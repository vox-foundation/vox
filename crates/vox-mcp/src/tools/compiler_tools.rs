//! Compiler, build, lint, and test tool handlers for the Vox MCP server.
//!
//! Covers: validate_file, run_tests, check_workspace, test_all, build_crate,
//! lint_crate, coverage_report, generate_vox_code.
//!
//! Subprocess and file reads use Tokio async I/O so [`super::handle_tool_call`] does not block
//! the runtime. TOESTUB runs inside [`tokio::task::spawn_blocking`] because the engine is synchronous.

use std::path::PathBuf;

use crate::params::{
    DiagnosticInfo, RunTestsParams, ToolResult, ValidateFileParams, ValidateResponse,
};
use crate::server::ServerState;
use tower_lsp::lsp_types::DiagnosticSeverity;

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
pub async fn validate_file(params: ValidateFileParams) -> String {
    let path = PathBuf::from(&params.path);

    let exists = match tokio::fs::try_exists(&path).await {
        Ok(e) => e,
        Err(e) => {
            return ToolResult::<ValidateResponse>::err(format!("failed to stat file: {e}"))
                .to_json();
        }
    };

    if !exists {
        return ToolResult::<ValidateResponse>::err(format!("file not found: {}", params.path))
            .to_json();
    }

    let text = match tokio::fs::read_to_string(&path).await {
        Ok(t) => t,
        Err(e) => {
            return ToolResult::<ValidateResponse>::err(format!("failed to read file: {e}"))
                .to_json();
        }
    };

    let diagnostics = vox_lsp::validate_document(&text);
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
    })
    .to_json()
}

/// Run `cargo test` for a specific crate.
pub async fn run_tests(state: &ServerState, params: RunTestsParams) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err(msg).to_json();
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
                ToolResult::<String>::err(combined).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err(format!("failed to run cargo test: {e}")).to_json(),
    }
}

/// Run `cargo check` for the entire workspace.
pub async fn check_workspace(state: &ServerState) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err(msg).to_json();
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
                ToolResult::<String>::err(format!("check failed:\n{stderr}")).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err(format!("failed to run cargo check: {e}")).to_json(),
    }
}

/// Run `cargo test` for the entire workspace.
pub async fn test_all(state: &ServerState) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err(msg).to_json();
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
                ToolResult::<String>::err(combined).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err(format!("failed to run cargo test --workspace: {e}"))
            .to_json(),
    }
}

/// Run `cargo build` for a crate or the whole workspace.
pub async fn build_crate(state: &ServerState, crate_name: Option<&str>) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err(msg).to_json();
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
                ToolResult::<String>::err(format!("Build failed:\n{stderr}")).to_json()
            }
        }
        Err(e) => ToolResult::<String>::err(format!("failed to run cargo build: {e}")).to_json(),
    }
}

/// Run `cargo clippy` and TOESTUB for a crate or the whole workspace.
pub async fn lint_crate(state: &ServerState, crate_name: Option<&str>) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err(msg).to_json();
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

/// Run `cargo llvm-cov` or `cargo tarpaulin` for code coverage.
pub async fn coverage_report(state: &ServerState, crate_name: Option<&str>) -> String {
    if let Some(msg) = cargo_unavailable_message(state) {
        return ToolResult::<String>::err(msg).to_json();
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
        _ => ToolResult::<String>::err(
            "Coverage tool (llvm-cov or tarpaulin) not installed. Run `cargo install cargo-llvm-cov`."
                .to_string(),
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
        .min(5);

    if prompt.is_empty() {
        return ToolResult::<String>::err("Missing 'prompt' parameter").to_json();
    }

    let mut current_prompt = prompt.to_string();
    let mut retry_count = 0;

    loop {
        let system_prompt = format!(
            "You are an expert compiler engineer. Generate VALD .vox code.\n\n\
             Rules:\n\
             - Only output the code, no explanation.\n\
             - Wrap in a ```vox code block.\n\
             {}\n",
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
            Err(e) => return ToolResult::<String>::err(e.to_string()).to_json(),
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
            Err(e) => return ToolResult::<String>::err(format!("No model: {e}")).to_json(),
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
            Err(e) => return ToolResult::<String>::err(format!("LLM error: {e}")).to_json(),
        };

        // Robust fence stripping: strip the language tag and then any trailing ```
        let block = completion.trim();
        completion = if block.starts_with("```vox") {
            block
                .strip_prefix("```vox")
                .unwrap_or(block)
                .strip_suffix("```")
                .unwrap_or(block)
                .trim()
                .to_string()
        } else if block.starts_with("```") {
            block
                .strip_prefix("```")
                .unwrap_or(block)
                .strip_suffix("```")
                .unwrap_or(block)
                .trim()
                .to_string()
        } else {
            block.to_string()
        };

        if !validate_flag {
            return ToolResult::ok(completion).to_json();
        }

        let diagnostics = vox_lsp::validate_document(&completion);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.severity == Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR))
            .collect();

        if errors.is_empty() {
            return ToolResult::ok(completion).to_json();
        }

        retry_count += 1;
        if retry_count > max_retries {
            let err_msgs: Vec<_> = errors.iter().map(|e| &e.message).collect();
            return ToolResult::<String>::err(format!(
                "Failed to generate valid code after {} retries. Errors: {:?}",
                max_retries, err_msgs
            ))
            .to_json();
        }

        // Add diagnostics to prompt for retry
        let mut feedback = String::from(
            "\n\nThe previous generation had these errors. Fix them and re-generate ONLY the corrected .vox code:\n",
        );
        for (i, err) in errors.iter().enumerate() {
            feedback.push_str(&format!(
                "{}. [L{}:C{}] {}\n",
                i + 1,
                err.range.start.line,
                err.range.start.character,
                err.message
            ));
        }
        current_prompt.push_str(&feedback);
    }
}
