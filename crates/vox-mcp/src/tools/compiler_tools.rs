//! Compiler, build, lint, and test tool handlers for the Vox MCP server.
//!
//! Covers: validate_file, run_tests, check_workspace, test_all, build_crate,
//! lint_crate, coverage_report, generate_vox_code.
//!
//! Subprocess and file reads use Tokio async I/O so [`super::handle_tool_call`] does not block
//! the runtime. TOESTUB runs inside [`tokio::task::spawn_blocking`] because the engine is synchronous.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::Instant;

use crate::params::{
    DiagnosticInfo, RunTestsParams, ToolResult, ValidateFileParams, ValidateResponse,
};
use crate::server::ServerState;
use tower_lsp::lsp_types::DiagnosticSeverity;

const REM_CARGO_DISABLED: &str = "Bind the MCP server to a Cargo workspace or package root, or use repository capabilities that enable Cargo tools.";
const REM_CARGO_TEST: &str = "Read STDOUT/STDERR for failing tests; run `cargo test` locally with the same filter and fix code or env.";
const REM_CARGO_SPAWN: &str = "Ensure `cargo` is installed and on PATH for the MCP process (see build-environment docs for agent shells).";
const REM_CARGO_CHECK: &str = "Fix compiler errors shown in stderr; run `cargo check --workspace` locally for full diagnostics.";
const REM_CARGO_BUILD: &str = "Fix build errors in stderr; verify features, targets, and that no concurrent build holds file locks.";
const REM_COVERAGE: &str =
    "Install `cargo-llvm-cov` (`cargo install cargo-llvm-cov`) or run coverage outside MCP.";
const REM_GEN_PROMPT: &str = "Provide a non-empty `prompt` describing the `.vox` code to generate.";
const REM_MCP_MODEL_LOCK: &str =
    "Retry; restart the MCP server if `mcp_chat_model_override` stays poisoned.";
const REM_MCP_MODEL_RESOLVE: &str = "Run `list_models`, ensure Ollama/API routes work, and check `vox clavis doctor` for inference secrets.";
const REM_LLM_COMPLETION: &str = "Check inference logs, rate limits, and backend health; verify API keys via `vox clavis doctor`.";
const REM_CODEGEN_REPAIR: &str = "Simplify the ask, paste compiler errors explicitly, lower constraints, or set `validate:false` for a raw draft.";
const REM_CODEGEN_STALL: &str = "Diagnostics did not change across retries — rephrase the prompt or disable validation temporarily.";
const RUNTIME_GENERATION_KPI_SCHEMA: &str = "vox_runtime_generation_kpi_v1";

fn runtime_generation_kpi(
    success: bool,
    validate_requested: bool,
    compile_pass: Option<bool>,
    canonical_pass: Option<bool>,
    attempts: u64,
    repair_stalled: bool,
    time_to_first_valid_ms: Option<u128>,
    output_tokens: u64,
    surface_contract_failures: u64,
    failure_category: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "schema": RUNTIME_GENERATION_KPI_SCHEMA,
        "schema_version": 1,
        "surface": "mcp",
        "success": success,
        "validate_requested": validate_requested,
        "compile_pass": compile_pass,
        "canonical_pass": canonical_pass,
        "attempts": attempts,
        "repair_stalled": repair_stalled,
        "time_to_first_valid_ms": time_to_first_valid_ms.map(|v| v as u64),
        "output_tokens": output_tokens,
        "surface_contract_failures": surface_contract_failures,
        "strictness_failures": surface_contract_failures,
        "canonicalization_failures": null,
        "failure_category": failure_category,
    })
}

async fn record_expensive_op(state: &ServerState) {
    if let Ok(mut sm) = state.session_manager.try_lock() {
        let sid = sm.list_sessions().first().map(|s| s.id.clone());
        if let Some(id) = sid {
            let _ = sm.record_expensive_op(&id);
        }
    }
}

fn cargo_unavailable_message(state: &ServerState) -> Option<String> {
    let c = &state.repository.capabilities;
    if c.cargo_workspace || c.cargo_package {
        None
    } else {
        Some(
            "Repository root is not a Cargo package or workspace; Cargo MCP tools are disabled for this repository."
                .to_string(),
        )
    }
}

/// Validate a .vox file using the full compiler pipeline (lexer → parser → typeck → HIR).
pub async fn validate_file(state: &ServerState, params: ValidateFileParams) -> String {
    let path = match super::workspace_path::resolve_existing_path_in_repository(state, &params.path)
    {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::<ValidateResponse>::err_with_remediation(
                e.message(),
                e.remediation(),
            )
            .to_json();
        }
    };

    let text = match tokio::fs::read_to_string(&path).await {
        Ok(t) => t,
        Err(e) => {
            return ToolResult::<ValidateResponse>::err_with_remediation(
                format!("failed to read file: {e}"),
                super::workspace_path::REM_VALIDATE_IO,
            )
            .to_json();
        }
    };

    let correlation_id = vox_oratio::trace::new_correlation_id();
    tracing::debug!(
        target: "vox_mcp_speech",
        correlation_id = %correlation_id,
        path = %params.path,
        bytes = text.len(),
        "validate_file: running HIR validation"
    );
    let diagnostics = vox_lsp::validate_document_with_hir(&text);
    let infos: Vec<DiagnosticInfo> = diagnostics
        .iter()
        .map(|d| DiagnosticInfo {
            severity: match d.severity {
                Some(s) if s == tower_lsp::lsp_types::DiagnosticSeverity::ERROR => {
                    "error".to_string()
                }
                _ => "warning".to_string(),
            },
            message: d.message.clone(),
            source: d.source.clone().unwrap_or_default(),
            start_line: d.range.start.line,
            start_col: d.range.start.character,
            end_line: d.range.end.line,
            end_col: d.range.end.character,
        })
        .collect();

    ToolResult::ok(ValidateResponse {
        count: infos.len(),
        diagnostics: infos,
        hir_validation_included: true,
        correlation_id: Some(correlation_id),
    })
    .to_json()
}

/// Run `cargo test` for a specific crate.
pub async fn run_tests(state: &ServerState, params: RunTestsParams) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err_with_remediation(msg, REM_CARGO_DISABLED).to_json();
    }
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(&state.repository.root);
    cmd.arg("test");
    if state.repository.capabilities.cargo_workspace {
        cmd.arg("-p").arg(&params.crate_name);
    }

    if let Some(filter) = &params.test_filter {
        cmd.arg(filter);
    }

    cmd.args(["--", "--nocapture"]);

    match cmd.output().await {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("STDOUT:\n{stdout}\n\nSTDERR:\n{stderr}");

            if output.status.success() {
                ToolResult::ok(combined).to_json()
            } else {
                ToolResult::<String>::err_with_remediation(combined, REM_CARGO_TEST).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("failed to run cargo test: {e}"),
            REM_CARGO_SPAWN,
        )
        .to_json(),
    }
}

/// Run `cargo check` for the entire workspace.
pub async fn check_workspace(state: &ServerState) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err_with_remediation(msg, REM_CARGO_DISABLED).to_json();
    }
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(&state.repository.root);
    cmd.arg("check").arg("--message-format=short");
    if state.repository.capabilities.cargo_workspace {
        cmd.arg("--workspace");
    }

    match cmd.output().await {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if output.status.success() {
                record_expensive_op(state).await;
                ToolResult::ok("workspace check passed".to_string()).to_json()
            } else {
                ToolResult::<String>::err_with_remediation(
                    format!("check failed:\n{stderr}"),
                    REM_CARGO_CHECK,
                )
                .to_json()
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("failed to run cargo check: {e}"),
            REM_CARGO_SPAWN,
        )
        .to_json(),
    }
}

/// Run `cargo test` for the entire workspace.
pub async fn test_all(state: &ServerState) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err_with_remediation(msg, REM_CARGO_DISABLED).to_json();
    }
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(&state.repository.root);
    cmd.arg("test");
    if state.repository.capabilities.cargo_workspace {
        cmd.arg("--workspace");
    }

    match cmd.output().await {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let combined = format!("STDOUT:\n{stdout}\n\nSTDERR:\n{stderr}");

            if output.status.success() {
                record_expensive_op(state).await;
                ToolResult::ok(combined).to_json()
            } else {
                ToolResult::<String>::err_with_remediation(combined, REM_CARGO_TEST).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("failed to run cargo test --workspace: {e}"),
            REM_CARGO_SPAWN,
        )
        .to_json(),
    }
}

/// Run `cargo build` for a crate or the whole workspace.
pub async fn build_crate(state: &ServerState, crate_name: Option<&str>) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err_with_remediation(msg, REM_CARGO_DISABLED).to_json();
    }
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(&state.repository.root);
    cmd.arg("build");
    if let Some(c) = crate_name {
        cmd.args(["-p", c]);
    } else if state.repository.capabilities.cargo_workspace {
        cmd.arg("--workspace");
    }

    match cmd.output().await {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if output.status.success() {
                record_expensive_op(state).await;
                ToolResult::ok(format!("Build succeeded.\n{stdout}")).to_json()
            } else {
                ToolResult::<String>::err_with_remediation(
                    format!("Build failed:\n{stderr}"),
                    REM_CARGO_BUILD,
                )
                .to_json()
            }
        }
        Err(e) => ToolResult::<String>::err_with_remediation(
            format!("failed to run cargo build: {e}"),
            REM_CARGO_SPAWN,
        )
        .to_json(),
    }
}

/// Run `cargo clippy` and TOESTUB for a crate or the whole workspace.
pub async fn lint_crate(state: &ServerState, crate_name: Option<&str>) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err_with_remediation(msg, REM_CARGO_DISABLED).to_json();
    }

    let repo_root = state.repository.root.clone();
    let workspace = state.repository.capabilities.cargo_workspace;
    let crate_owned = crate_name.map(str::to_string);

    let mut clippy_cmd = tokio::process::Command::new("cargo");
    clippy_cmd.current_dir(&repo_root);
    clippy_cmd.arg("clippy");
    if let Some(ref c) = crate_owned {
        clippy_cmd.args(["-p", c.as_str()]);
    } else if workspace {
        clippy_cmd.arg("--workspace");
    }
    clippy_cmd.args(["--", "-D", "warnings"]);

    let clippy_out = match clippy_cmd.output().await {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if output.status.success() {
                "Clippy clean.".to_string()
            } else {
                format!("Clippy errors:\n{stderr}")
            }
        }
        Err(e) => format!("failed to run cargo clippy: {e}"),
    };

    let root_for_ts = if let Some(ref c) = crate_owned {
        let p = repo_root.join("crates").join(c);
        if tokio::fs::try_exists(&p).await.unwrap_or(false) {
            p
        } else {
            repo_root.clone()
        }
    } else {
        repo_root.clone()
    };

    use vox_toestub::{Severity, ToestubConfig, ToestubEngine};

    let ts_report = match tokio::task::spawn_blocking(move || {
        let ts_config = ToestubConfig {
            roots: vec![root_for_ts],
            min_severity: Severity::Warning,
            schema_path: Some(PathBuf::from("vox-schema.json")),
            ..ToestubConfig::default()
        };
        let ts_engine = ToestubEngine::new(ts_config);
        let (_, report) = ts_engine.run_and_report();
        report
    })
    .await
    {
        Ok(r) => r,
        Err(e) => format!("TOESTUB task failed: {e}"),
    };

    let combined = format!(
        "### 📎 Clippy Results\n{}\n\n### 🦶 TOESTUB Architectural Scan\n{}",
        clippy_out, ts_report
    );

    record_expensive_op(state).await;
    ToolResult::ok(combined).to_json()
}

/// Run `cargo llvm-cov` for a text coverage summary.
pub async fn coverage_report(state: &ServerState, crate_name: Option<&str>) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err_with_remediation(msg, REM_CARGO_DISABLED).to_json();
    }
    let mut cmd = tokio::process::Command::new("cargo");
    cmd.current_dir(&state.repository.root);
    cmd.arg("llvm-cov");
    if let Some(c) = crate_name {
        cmd.args(["-p", c]);
    }
    cmd.args(["--text"]);

    match cmd.output().await {
        Ok(output) if output.status.success() => {
            ToolResult::ok(String::from_utf8_lossy(&output.stdout).to_string()).to_json()
        }
        _ => ToolResult::<String>::err_with_remediation(
            "cargo-llvm-cov is not installed or failed. Run `cargo install cargo-llvm-cov` and ensure `rustup component add llvm-tools-preview`."
                .to_string(),
            REM_COVERAGE,
        )
        .to_json(),
    }
}

/// Generate validated Vox code from a prompt using the native LLM bridge.
pub async fn generate_vox_code(state: &ServerState, args: serde_json::Value) -> String {
    let prompt = args.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let session_id = args
        .get("session_id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let validate_flag = args
        .get("validate")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let max_retries = args
        .get("max_retries")
        .and_then(|v| v.as_u64())
        .unwrap_or(2)
        .min(crate::speech_constraints::SPEECH_CODE_MAX_REPAIR_ATTEMPTS as u64);
    let output_surface_mode = match args
        .get("output_surface_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("raw_code_only")
    {
        "fenced_transport" => crate::speech_constraints::OutputSurfaceMode::FencedTransport,
        _ => crate::speech_constraints::OutputSurfaceMode::RawCodeOnly,
    };

    if prompt.is_empty() {
        return ToolResult::<String>::err_with_remediation(
            "Missing 'prompt' parameter",
            REM_GEN_PROMPT,
        )
        .to_json();
    }

    let mut current_prompt = prompt.to_string();
    let mut retry_count = 0u64;
    let mut prev_error_sig: Option<u64> = None;
    let started = Instant::now();
    let mut first_valid_ms: Option<u128> = None;
    let mut surface_contract_failures: u64 = 0;
    let mut repair_stalled = false;

    let grammar_addon =
        crate::speech_constraints::grammar_artifact_prompt_addon(&state.repository.root);
    let decode_policy = crate::speech_constraints::ConstrainedDecodePolicy::from_env();
    decode_policy.note_delegation_target();

    loop {
        let hint_stub = crate::speech_constraints::TypeHintStub;
        let output_surface_instruction = match output_surface_mode {
            crate::speech_constraints::OutputSurfaceMode::RawCodeOnly => {
                "- Return ONLY raw .vox code (no markdown fences, no prose).\n"
            }
            crate::speech_constraints::OutputSurfaceMode::FencedTransport => {
                "- Wrap output in a ```vox fenced code block.\n"
            }
        };
        let system_prompt = format!(
            "You are an expert compiler engineer. Generate VALD .vox code.\n\n\
             Rules:\n\
             - Only output the code, no explanation.\n\
             {}\
             {}{}\
             {}\n",
            output_surface_instruction,
            grammar_addon,
            hint_stub.system_prompt_addon(),
            crate::tools::chat_tools::ANTI_LAZINESS_RIDER
        );

        let resolution_template = crate::llm_bridge::McpChatModelResolution {
            complexity: 2,
            ..Default::default()
        };

        let pref = match crate::sync_poison::poison_rw_read(
            state.mcp_chat_model_override.read(),
            "mcp_chat_model_override",
        ) {
            Ok(g) => g.clone(),
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    e.to_string(),
                    REM_MCP_MODEL_LOCK,
                )
                .to_json();
            }
        };
        let (model, free_only) = match crate::tools::chat_model_resolve::resolve_chat_llm_model(
            state,
            &current_prompt,
            resolution_template.clone(),
            session_id.as_deref(),
        )
        .await
        {
            Ok(pair) => pair,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("No model: {e}"),
                    REM_MCP_MODEL_RESOLVE,
                )
                .to_json();
            }
        };

        let routing = crate::llm_bridge::McpInferRouting {
            user_prompt: &current_prompt,
            sticky_model_pref: pref.as_deref(),
            resolution_template,
            free_only,
            allow_cloud_ollama_fallback: true,
            user_id: session_id.as_deref(),
        };

        let (mut completion, _, _) = match crate::llm_bridge::mcp_infer_completion(
            state,
            model,
            "vox_generate_code",
            &system_prompt,
            &routing,
            2048,
            0.1,
            false,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return ToolResult::<String>::err_with_remediation(
                    format!("LLM error: {e}"),
                    REM_LLM_COMPLETION,
                )
                .to_json();
            }
        };

        let normalized = vox_compiler::generated_vox::normalize_generated_vox(
            &completion,
            match output_surface_mode {
                crate::speech_constraints::OutputSurfaceMode::RawCodeOnly => {
                    vox_compiler::generated_vox::OutputSurfaceMode::RawCodeOnly
                }
                crate::speech_constraints::OutputSurfaceMode::FencedTransport => {
                    vox_compiler::generated_vox::OutputSurfaceMode::FencedTransport
                }
            },
        );
        completion = normalized.normalized;
        if decode_policy.enabled {
            if !normalized.surface_contract_ok
                || !decode_policy.surface_contract_ok(&completion, output_surface_mode)
            {
                surface_contract_failures += 1;
                retry_count += 1;
                if retry_count > max_retries {
                    let kpi = runtime_generation_kpi(
                        false,
                        validate_flag,
                        None,
                        None,
                        retry_count,
                        repair_stalled,
                        first_valid_ms,
                        completion.split_whitespace().count() as u64,
                        surface_contract_failures,
                        Some("surface_contract"),
                    );
                    return ToolResult::<String>::err_with_remediation_meta(
                        "constrained-decode policy rejected non-code surface repeatedly"
                            .to_string(),
                        REM_CODEGEN_REPAIR,
                        serde_json::json!({
                            "repair": {
                                "attempts": retry_count,
                                "failure_category": "surface_contract",
                                "decode_policy": "grammar_surface_guard"
                            },
                            "runtime_generation_kpi": kpi
                        }),
                    )
                    .to_json();
                }
                current_prompt.push_str(
                    "\n\nYour previous output violated output-surface constraints. Regenerate using only the required output mode.",
                );
                continue;
            }
        }

        if !validate_flag {
            let kpi = runtime_generation_kpi(
                true,
                false,
                None,
                None,
                retry_count + 1,
                repair_stalled,
                Some(started.elapsed().as_millis()),
                completion.split_whitespace().count() as u64,
                surface_contract_failures,
                None,
            );
            tracing::info!(
                target: "vox_mcp_speech",
                time_to_first_valid_ms = started.elapsed().as_millis() as u64,
                repair_attempts = retry_count + 1,
                repair_stalled,
                output_tokens = completion.split_whitespace().count() as u64,
                surface_contract_failures,
                "vox_generate_code runtime_kpi"
            );
            return ToolResult::ok_with_meta(
                completion,
                serde_json::json!({ "runtime_generation_kpi": kpi }),
            )
            .to_json();
        }

        let validation = vox_compiler::generated_vox::validate_generated_vox(&completion, true);
        let errors: Vec<_> = validation.errors.iter().collect();

        if errors.is_empty() {
            if first_valid_ms.is_none() {
                first_valid_ms = Some(started.elapsed().as_millis());
            }
            let canonicalized = validation.canonicalized;
            let canonical_pass = canonicalized.is_some();
            let canonical = canonicalized.unwrap_or_else(|| completion.clone());
            let kpi = runtime_generation_kpi(
                true,
                true,
                Some(true),
                Some(canonical_pass),
                retry_count + 1,
                repair_stalled,
                first_valid_ms,
                canonical.split_whitespace().count() as u64,
                surface_contract_failures,
                None,
            );
            tracing::info!(
                target: "vox_mcp_speech",
                time_to_first_valid_ms = first_valid_ms.unwrap_or(0) as u64,
                repair_attempts = retry_count + 1,
                repair_stalled,
                output_tokens = canonical.split_whitespace().count() as u64,
                surface_contract_failures,
                "vox_generate_code runtime_kpi"
            );
            return ToolResult::ok_with_meta(
                canonical,
                serde_json::json!({ "runtime_generation_kpi": kpi }),
            )
            .to_json();
        }

        let snapshot: Vec<serde_json::Value> = errors
            .iter()
            .map(|e| {
                serde_json::json!({
                    "severity": "error",
                    "message": e.message,
                    "code": e.code,
                    "category": e.category,
                })
            })
            .collect();
        let mut rows: Vec<(String, String, String)> = errors
            .iter()
            .map(|e| {
                (
                    e.category.to_string(),
                    e.message.split_whitespace().collect::<Vec<_>>().join(" "),
                    e.code.clone().unwrap_or_default(),
                )
            })
            .collect();
        rows.sort();
        let mut hasher = DefaultHasher::new();
        for row in rows {
            row.hash(&mut hasher);
        }
        let sig = hasher.finish();
        if prev_error_sig == Some(sig) {
            repair_stalled = true;
            let kpi = runtime_generation_kpi(
                false,
                true,
                Some(false),
                Some(false),
                retry_count,
                true,
                first_valid_ms,
                completion.split_whitespace().count() as u64,
                surface_contract_failures,
                Some("repair_stall"),
            );
            return ToolResult::<String>::err_with_remediation_meta(
                format!(
                    "repair loop stalled: diagnostics unchanged after retry (signature={sig:#x})"
                ),
                REM_CODEGEN_STALL,
                serde_json::json!({
                    "diagnostics_snapshot": snapshot,
                    "repair": { "attempts": retry_count, "stalled": true },
                    "runtime_generation_kpi": kpi
                }),
            )
            .to_json();
        }
        prev_error_sig = Some(sig);

        retry_count += 1;
        if retry_count > max_retries {
            let err_msgs: Vec<_> = errors.iter().map(|e| &e.message).collect();
            let kpi = runtime_generation_kpi(
                false,
                true,
                Some(false),
                Some(false),
                retry_count,
                false,
                first_valid_ms,
                completion.split_whitespace().count() as u64,
                surface_contract_failures,
                Some("semantic"),
            );
            return ToolResult::<String>::err_with_remediation_meta(
                format!(
                    "Failed to generate valid code after {} retries. Errors: {:?}",
                    max_retries, err_msgs
                ),
                REM_CODEGEN_REPAIR,
                serde_json::json!({
                    "diagnostics_snapshot": snapshot,
                    "repair": {
                        "attempts": retry_count,
                        "stalled": false,
                        "failure_category": "semantic"
                    },
                    "runtime_generation_kpi": kpi
                }),
            )
            .to_json();
        }

        // Add diagnostics to prompt for retry
        let mut feedback = String::from(
            "\n\nThe previous generation had these errors. Fix them and re-generate ONLY the corrected .vox code:\n",
        );
        for (i, err) in errors.iter().enumerate() {
            feedback.push_str(&format!("{}. [{}] {}\n", i + 1, err.category, err.message));
        }
        if let Ok(json) =
            serde_json::to_string_pretty(&serde_json::json!({ "diagnostics_snapshot": &snapshot }))
        {
            feedback.push_str("\nStructured diagnostics (JSON):\n");
            feedback.push_str(&json);
            feedback.push('\n');
        }
        current_prompt.push_str(&feedback);
    }
}
