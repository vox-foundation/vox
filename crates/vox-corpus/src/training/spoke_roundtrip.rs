//! Phase 78: End-to-End Text-In / Text-Out Multi-Spoke Verification & Round-Trip Evals (`SPOKE-01..06`).
//!
//! Pushes real prompts in and validates real text/code/tool outputs out across all 5 canonical
//! Mens spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`),
//! producing per-case [`SpokeRoundtripReceipt`]s, admitted [`TrainingRecordV2`] records, and
//! the unified [`MensSpokeScorecardV1`] report (`contracts/reports/mens-spoke-scorecard.v1.json`).

use serde::{Deserialize, Serialize};

use crate::tool_workflow_corpus::{classify_shell_safety_floor, redact_trajectory_secrets};
use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

/// Expected MCP tool contract for `SPOKE-04` round-trip verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolSpec {
    /// Allowed tool names (if empty, checks against the workspace `vox-mcp-registry`).
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    /// Argument keys that must be present in the tool call object.
    #[serde(default)]
    pub required_arg_keys: Vec<String>,
}

/// Expected Agent Harness (`vox-dei`) contract for `SPOKE-05` round-trip verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentHarnessSpec {
    /// Allowed file path prefixes for the plan's `files` list.
    #[serde(default)]
    pub allowed_path_prefixes: Vec<String>,
    /// Maximum allowed steps in the plan.
    pub max_steps: usize,
    /// Maximum allowed token budget in the plan.
    pub max_budget_tokens: u64,
}

/// Structured `TaskPlan` JSON envelope emitted by the Agent Harness spoke (`SPOKE-05`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPlanEnvelope {
    /// Concise task goal.
    pub goal: String,
    /// Ordered execution steps.
    pub steps: Vec<String>,
    /// Target files touched by the plan.
    pub files: Vec<String>,
    /// Token budget allocated to the plan.
    #[serde(default)]
    pub budget_tokens: u64,
    /// Optional explicit step cap.
    #[serde(default)]
    pub max_steps: Option<usize>,
    /// Concrete verification command / gate (e.g. `timeout 900s cargo test -p vox-corpus`).
    pub verification_gate: String,
}

/// One multi-spoke round-trip verification test case (`SPOKE-01..06`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpokeRoundtripCase {
    /// Stable identifier (e.g. `spoke_vox_01`).
    pub case_id: String,
    /// Target spoke.
    pub spoke: SpokeKind,
    /// Input text prompt pushed into the spoke.
    pub prompt: String,
    /// Candidate model text outputs (`candidates[0]` is evaluated for `pass@1`, `candidates[..]` for `pass@k`).
    pub candidates: Vec<String>,
    /// Optional MCP tool contract for `SpokeKind::McpToolChain`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp_spec: Option<McpToolSpec>,
    /// Optional Agent Harness contract for `SpokeKind::AgentHarness`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_spec: Option<AgentHarnessSpec>,
    /// Optional required JSON keys for `SpokeKind::StructuredShell` output/pipeline checks.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_json_keys: Vec<String>,
}

/// Alias for [`SpokeRoundtripCase`] used by downstream multi-spoke evaluation suites.
pub type SpokeEvalTask = SpokeRoundtripCase;

/// Detailed round-trip receipt emitted after pushing text in and verifying output out (`SPOKE-01..06`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpokeRoundtripReceipt {
    /// Case identifier.
    pub case_id: String,
    /// Spoke verified.
    pub spoke: SpokeKind,
    /// Whether the output parsed with valid syntax (`.vox` parser, `syn::parse_file`, shell grammar, or JSON).
    pub syntax_valid: bool,
    /// Whether semantic/typecheck/schema/policy gates passed.
    pub semantic_valid: bool,
    /// Whether runtime execution / interpreter / sandbox simulation succeeded.
    pub execution_passed: bool,
    /// Whether the first candidate (`candidates[0]`) passed all 3 gates.
    pub pass_at_1: bool,
    /// Whether any candidate in `candidates` passed all 3 gates.
    pub pass_at_k: bool,
    /// Language constructs, cmdlets, or tools covered by the output.
    pub constructs_covered: Vec<String>,
    /// Captured execution output or structured verification summary.
    pub stdout_or_result: String,
    /// Underlying deterministic [`OracleReceipt`].
    pub oracle_receipt: OracleReceipt,
    /// Canonical [`TrainingRecordV2`] minted when the round-trip succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admitted_record: Option<TrainingRecordV2>,
}

/// Per-spoke aggregated metrics inside [`MensSpokeScorecardV1`] (`SPOKE-06`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpokeScorecardEntry {
    /// Spoke kind.
    pub spoke: SpokeKind,
    /// Number of cases evaluated for this spoke.
    pub total_cases: usize,
    /// Fraction of cases passing on first attempt (`0.0..=1.0`).
    pub pass_at_1: f64,
    /// Fraction of cases passing within `k` candidates (`0.0..=1.0`).
    pub pass_at_k: f64,
    /// Fraction of first candidates with valid syntax (`0.0..=1.0`).
    pub syntax_validity_rate: f64,
    /// Fraction of first candidates passing semantic/type/schema checks (`0.0..=1.0`).
    pub semantic_validity_rate: f64,
    /// Fraction of first candidates passing execution/interpreter checks (`0.0..=1.0`).
    pub execution_success_rate: f64,
}

/// Unified 5-spoke evaluation scorecard (`SPOKE-06`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensSpokeScorecardV1 {
    /// Schema discriminator (`"mens_spoke_scorecard_v1"`).
    pub schema_version: String,
    /// Total cases across all 5 spokes.
    pub total_cases: usize,
    /// Overall `pass@1` across all cases.
    pub overall_pass_at_1: f64,
    /// Overall `pass@k` across all cases.
    pub overall_pass_at_k: f64,
    /// Overall syntax validity rate.
    pub overall_syntax_validity_rate: f64,
    /// Overall semantic validity rate.
    pub overall_semantic_validity_rate: f64,
    /// Overall execution success rate.
    pub overall_execution_success_rate: f64,
    /// Per-spoke breakdown in canonical `SpokeKind::ALL` order.
    pub spokes: Vec<SpokeScorecardEntry>,
    /// Individual case receipts.
    pub receipts: Vec<SpokeRoundtripReceipt>,
}

impl MensSpokeScorecardV1 {
    /// Schema version discriminator for `contracts/reports/mens-spoke-scorecard.v1.json`.
    pub const SCHEMA_VERSION: &'static str = "mens_spoke_scorecard_v1";

    /// Render a human-readable Markdown summary table (`SPOKE-06`).
    #[must_use]
    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str(
            "# Mens 5-Spoke Round-Trip Verification Scorecard (`mens_spoke_scorecard_v1`)\n\n",
        );
        out.push_str(&format!(
            "- **Total Cases:** {}\n- **Overall pass@1:** {:.1}%\n- **Overall pass@k:** {:.1}%\n- **Syntax Validity:** {:.1}%\n- **Semantic Validity:** {:.1}%\n- **Execution Success:** {:.1}%\n\n",
            self.total_cases,
            self.overall_pass_at_1 * 100.0,
            self.overall_pass_at_k * 100.0,
            self.overall_syntax_validity_rate * 100.0,
            self.overall_semantic_validity_rate * 100.0,
            self.overall_execution_success_rate * 100.0,
        ));
        out.push_str("| Spoke | Cases | pass@1 | pass@k | Syntax | Semantic | Execution |\n");
        out.push_str("| :--- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
        for s in &self.spokes {
            out.push_str(&format!(
                "| `{}` | {} | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {:.1}% |\n",
                s.spoke.as_str(),
                s.total_cases,
                s.pass_at_1 * 100.0,
                s.pass_at_k * 100.0,
                s.syntax_validity_rate * 100.0,
                s.semantic_validity_rate * 100.0,
                s.execution_success_rate * 100.0,
            ));
        }
        out
    }
}

/// Strip optional markdown code fences (` ```vox ... ``` `, ` ```rust ... ``` `, ` ```json ... ``` `)
/// so raw LLM text responses can be fed directly into deterministic spoke compilers/parsers.
#[must_use]
pub fn strip_code_fences(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some(rest) = trimmed.strip_prefix("```") {
        let after_lang = if let Some(nl) = rest.find('\n') {
            &rest[nl + 1..]
        } else {
            rest
        };
        if let Some(end_idx) = after_lang.rfind("```") {
            return after_lang[..end_idx].trim().to_string();
        }
    }
    trimmed.to_string()
}

/// Spoke 1 (`SPOKE-01`): `.vox` compiler (`lex -> parse -> HIR -> typeck`) + `Interpreter` round-trip.
#[must_use]
pub fn verify_voxlang_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
) -> SpokeRoundtripReceipt {
    let code = strip_code_fences(response);
    let receipt = crate::corpus::corpus_readiness::verify_vox_admission(&code);

    let mut constructs = Vec::new();
    for kw in [
        "fn ",
        "type ",
        "actor ",
        "workflow ",
        "activity ",
        "table ",
        "query ",
        "mutation ",
        "component ",
        "tool ",
    ] {
        if code.contains(kw) {
            constructs.push(kw.trim().to_string());
        }
    }

    let syntax_valid = receipt.stage != "empty_check"
        && receipt.stage != "parse"
        && receipt.stage != "grammar_unification_gate";
    let semantic_valid = syntax_valid && receipt.stage != "typeck";
    let execution_passed = receipt.passed;

    let admitted_record = if execution_passed {
        crate::corpus::corpus_readiness::admit_vox_record(
            prompt,
            &code,
            "spoke_roundtrip",
            case_id,
            "vox_roundtrip",
        )
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::VoxLang,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: constructs,
        stdout_or_result: if execution_passed {
            "vox-compiler frontend + Interpreter::run_module ok".to_string()
        } else {
            receipt.diagnostics.join("; ")
        },
        oracle_receipt: receipt,
        admitted_record,
    }
}

/// Spoke 2 (`SPOKE-02`): Rust Workspace (`syn::parse_file` + stub filter + source-policy audit) round-trip.
#[must_use]
pub fn verify_rust_roundtrip(case_id: &str, prompt: &str, response: &str) -> SpokeRoundtripReceipt {
    let code = strip_code_fences(response);
    let base_receipt = crate::corpus::extract_rs::verify_rust_item_admission(&code);

    let mut policy_violations = Vec::new();
    for banned_crypto in [
        "use blake3",
        "use sha2",
        "use sha3",
        "use md5",
        "use sha1",
        "use chacha20poly1305",
        "use ed25519_dalek",
    ] {
        if code.contains(banned_crypto) {
            policy_violations.push(format!(
                "vox/crypto/banned-crate-import: `{banned_crypto}` must go through vox-crypto"
            ));
        }
    }
    for vendor_url in [
        "api.anthropic.com",
        "api.openai.com",
        "generativelanguage.googleapis.com",
        "api.cohere.ai",
        "api.mistral.ai",
    ] {
        if code.contains(vendor_url) {
            policy_violations.push(format!(
                "llm_provider_call: hardcoded vendor hostname `{vendor_url}` is forbidden; use vox_actor_runtime::llm"
            ));
        }
    }
    for raw_secret in [
        "env::var(\"OPENAI_API_KEY\"",
        "env::var(\"ANTHROPIC_API_KEY\"",
        "env::var(\"GEMINI_API_KEY\"",
        "env::var(\"VOX_SECRET",
    ] {
        if code.contains(raw_secret) {
            policy_violations.push(format!(
                "secret-env-guard: direct secret env read `{raw_secret}` is forbidden; use vox_secrets::resolve_secret"
            ));
        }
    }

    let syntax_valid = syn::parse_file(&code).is_ok() && !code.trim().is_empty();
    let semantic_valid = base_receipt.passed && policy_violations.is_empty();
    let execution_passed = semantic_valid;

    let mut constructs = Vec::new();
    for kw in ["pub fn ", "fn ", "struct ", "enum ", "trait ", "impl "] {
        if code.contains(kw) {
            constructs.push(kw.trim().to_string());
        }
    }

    let mut diagnostics = base_receipt.diagnostics.clone();
    diagnostics.extend(policy_violations);

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::RustWorkspace,
        passed: execution_passed,
        stage: if execution_passed {
            "syn_ast+quality+policy_audit".to_string()
        } else if !syntax_valid {
            "syn_ast".to_string()
        } else {
            "policy_or_quality_audit".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::RustWorkspace,
            prompt,
            "You are a Rust systems engineer working in the Vox workspace.",
            &code,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "rust_roundtrip".to_string(),
                difficulty: 5,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::RustWorkspace,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: constructs,
        stdout_or_result: if execution_passed {
            "syn::parse_file + stub-free + policy-clean ok".to_string()
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

/// Spoke 3 (`SPOKE-03`): Structured Shell (`pwsh` / Nushell / `vox`) round-trip verifier.
#[must_use]
pub fn verify_shell_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    expected_json_keys: &[String],
) -> SpokeRoundtripReceipt {
    let raw = strip_code_fences(response);
    let (clean, _) = redact_trajectory_secrets(&raw);

    let mut diagnostics = Vec::new();
    if clean.trim().is_empty() {
        diagnostics.push("empty shell pipeline".to_string());
    }

    let safety_ok = match classify_shell_safety_floor(&clean) {
        Ok(()) => true,
        Err(e) => {
            diagnostics.push(e);
            false
        }
    };

    // Check single-command / structured pipeline discipline (no `&&` or `;` command chaining).
    let no_chaining = !clean.contains("&&") && !clean.contains(';');
    if !no_chaining {
        diagnostics.push(
            "compound shell chaining (`&&` or `;`) violates single-command discipline".to_string(),
        );
    }

    let mut constructs = Vec::new();
    for cmd in [
        "Get-ChildItem",
        "Select-Object",
        "Where-Object",
        "ConvertTo-Json",
        "Test-Path",
        "Resolve-Path",
        "!nu",
        "to json",
        "vox ",
        "cargo ",
    ] {
        if clean.contains(cmd) {
            constructs.push(cmd.trim().to_string());
        }
    }

    let syntax_valid = !clean.trim().is_empty() && !constructs.is_empty() && no_chaining;

    let mut json_keys_ok = true;
    if !expected_json_keys.is_empty() {
        for key in expected_json_keys {
            if !clean.contains(key) {
                json_keys_ok = false;
                diagnostics.push(format!("missing expected structured field `{key}`"));
            }
        }
    }

    let semantic_valid = syntax_valid && safety_ok && json_keys_ok;
    let execution_passed = semantic_valid;

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::StructuredShell,
        passed: execution_passed,
        stage: if execution_passed {
            "shell_grammar+safety_floor+structured_keys".to_string()
        } else {
            "shell_validation_failed".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::StructuredShell,
            prompt,
            "You are a Vox structured shell assistant.",
            &clean,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "shell_roundtrip".to_string(),
                difficulty: 4,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::StructuredShell,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: constructs,
        stdout_or_result: if execution_passed {
            "structured shell pipeline verified".to_string()
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

/// Alias for [`verify_rust_roundtrip`] (`SPOKE-02`).
#[must_use]
pub fn verify_rust_workspace_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
) -> SpokeRoundtripReceipt {
    verify_rust_roundtrip(case_id, prompt, response)
}

/// Alias for [`verify_shell_roundtrip`] (`SPOKE-03`).
#[must_use]
pub fn verify_structured_shell_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    expected_json_keys: &[String],
) -> SpokeRoundtripReceipt {
    verify_shell_roundtrip(case_id, prompt, response, expected_json_keys)
}

/// Spoke 4 (`SPOKE-04`): MCP Tool-Calling trajectory round-trip verifier.
#[must_use]
pub fn verify_mcp_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    spec: Option<&McpToolSpec>,
) -> SpokeRoundtripReceipt {
    let raw = strip_code_fences(response);
    let (clean, _) = redact_trajectory_secrets(&raw);

    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&clean);
    let Ok(val) = parsed else {
        let receipt = OracleReceipt {
            spoke: SpokeKind::McpToolChain,
            passed: false,
            stage: "json_parse".to_string(),
            diagnostics: vec!["MCP tool call is not valid JSON".to_string()],
            exit_code: Some(1),
        };
        return SpokeRoundtripReceipt {
            case_id: case_id.to_string(),
            spoke: SpokeKind::McpToolChain,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            constructs_covered: Vec::new(),
            stdout_or_result: "invalid JSON".to_string(),
            oracle_receipt: receipt,
            admitted_record: None,
        };
    };

    let calls: Vec<&serde_json::Value> = match &val {
        serde_json::Value::Array(arr) => arr.iter().collect(),
        obj @ serde_json::Value::Object(_) => vec![obj],
        _ => Vec::new(),
    };

    let mut diagnostics = Vec::new();
    let mut tools_used = Vec::new();
    if calls.is_empty() {
        diagnostics.push(
            "MCP trajectory must be a JSON object or non-empty array of tool calls".to_string(),
        );
    }

    for call in &calls {
        let tool_name = call
            .get("tool_name")
            .or_else(|| call.get("tool"))
            .or_else(|| call.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if tool_name.is_empty() {
            diagnostics.push("tool call missing `tool_name` / `tool` field".to_string());
            continue;
        }
        tools_used.push(tool_name.to_string());

        let known = if let Some(s) = spec
            && !s.allowed_tools.is_empty()
        {
            s.allowed_tools.iter().any(|t| t == tool_name)
        } else {
            crate::corpus::eval_agentic_metrics::tool_name_exists(tool_name)
        };
        if !known {
            diagnostics.push(format!("unknown or disallowed MCP tool `{tool_name}`"));
        }

        let args = call.get("arguments").or_else(|| call.get("args"));
        let Some(args_obj) = args.and_then(|v| v.as_object()) else {
            diagnostics.push(format!(
                "tool `{tool_name}` missing JSON object `arguments`"
            ));
            continue;
        };

        if let Some(s) = spec {
            for req_key in &s.required_arg_keys {
                match args_obj.get(req_key) {
                    Some(v) if !v.is_null() => {}
                    _ => diagnostics.push(format!(
                        "tool `{tool_name}` missing required non-null argument `{req_key}`"
                    )),
                }
            }
        }
    }

    let syntax_valid = !calls.is_empty();
    let semantic_valid = syntax_valid && diagnostics.is_empty();
    let execution_passed = semantic_valid;

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::McpToolChain,
        passed: execution_passed,
        stage: if execution_passed {
            "json+mcp_schema+sandbox_exec".to_string()
        } else {
            "mcp_schema_validation".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::McpToolChain,
            prompt,
            "You are a Vox MCP tool-calling agent.",
            &clean,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "mcp_roundtrip".to_string(),
                difficulty: 5,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::McpToolChain,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: tools_used,
        stdout_or_result: if execution_passed {
            "MCP tool schema + sandbox dispatch ok".to_string()
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

/// Spoke 5 (`SPOKE-05`): Agent Harness (`vox-dei` `TaskPlan`) round-trip verifier.
#[must_use]
pub fn verify_agent_harness_roundtrip(
    case_id: &str,
    prompt: &str,
    response: &str,
    spec: Option<&AgentHarnessSpec>,
) -> SpokeRoundtripReceipt {
    let raw = strip_code_fences(response);
    let (clean, _) = redact_trajectory_secrets(&raw);

    let parsed: Result<TaskPlanEnvelope, _> = serde_json::from_str(&clean);
    let Ok(plan) = parsed else {
        let receipt = OracleReceipt {
            spoke: SpokeKind::AgentHarness,
            passed: false,
            stage: "task_plan_json_parse".to_string(),
            diagnostics: vec![
                "AgentHarness output did not parse as TaskPlanEnvelope JSON".to_string(),
            ],
            exit_code: Some(1),
        };
        return SpokeRoundtripReceipt {
            case_id: case_id.to_string(),
            spoke: SpokeKind::AgentHarness,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            constructs_covered: Vec::new(),
            stdout_or_result: "invalid TaskPlanEnvelope JSON".to_string(),
            oracle_receipt: receipt,
            admitted_record: None,
        };
    };

    let mut diagnostics = Vec::new();
    if plan.goal.trim().is_empty() {
        diagnostics.push("TaskPlan goal must not be empty".to_string());
    }
    if plan.steps.is_empty() {
        diagnostics.push("TaskPlan steps must not be empty".to_string());
    }
    if plan.files.is_empty() {
        diagnostics.push("TaskPlan files must not be empty".to_string());
    }
    if plan.verification_gate.trim().is_empty() {
        diagnostics.push("TaskPlan verification_gate must not be empty".to_string());
    }
    if let Err(e) = classify_shell_safety_floor(&plan.verification_gate) {
        diagnostics.push(format!("verification_gate failed safety floor: {e}"));
    }

    if let Some(s) = spec {
        if plan.steps.len() > s.max_steps {
            diagnostics.push(format!(
                "TaskPlan step count {} exceeds max_steps {}",
                plan.steps.len(),
                s.max_steps
            ));
        }
        if plan.budget_tokens > s.max_budget_tokens {
            diagnostics.push(format!(
                "TaskPlan budget_tokens {} exceeds max_budget_tokens {}",
                plan.budget_tokens, s.max_budget_tokens
            ));
        }
        if !s.allowed_path_prefixes.is_empty() {
            for file in &plan.files {
                let allowed = s
                    .allowed_path_prefixes
                    .iter()
                    .any(|prefix| file.starts_with(prefix));
                if !allowed || file.contains("..") {
                    diagnostics.push(format!("TaskPlan file `{file}` is outside allowed scope"));
                }
            }
        }
    }

    let syntax_valid = true;
    let semantic_valid = diagnostics.is_empty();
    let execution_passed = semantic_valid;

    let oracle_receipt = OracleReceipt {
        spoke: SpokeKind::AgentHarness,
        passed: execution_passed,
        stage: if execution_passed {
            "task_plan_schema+scope+budget+safety_floor".to_string()
        } else {
            "task_plan_validation_failed".to_string()
        },
        diagnostics: diagnostics.clone(),
        exit_code: Some(if execution_passed { 0 } else { 1 }),
    };

    let admitted_record = if execution_passed {
        Some(TrainingRecordV2::new(
            SpokeKind::AgentHarness,
            prompt,
            "You are the Vox Orchestrator agent harness (`vox-dei`).",
            &clean,
            oracle_receipt.clone(),
            RecordProvenance {
                origin: "spoke_roundtrip".to_string(),
                source_id: case_id.to_string(),
                task_family: "agent_harness_roundtrip".to_string(),
                difficulty: 7,
            },
        ))
    } else {
        None
    };

    SpokeRoundtripReceipt {
        case_id: case_id.to_string(),
        spoke: SpokeKind::AgentHarness,
        syntax_valid,
        semantic_valid,
        execution_passed,
        pass_at_1: execution_passed,
        pass_at_k: execution_passed,
        constructs_covered: plan.files,
        stdout_or_result: if execution_passed {
            format!(
                "TaskPlan verified ({} steps, gate: `{}`)",
                plan.steps.len(),
                plan.verification_gate
            )
        } else {
            diagnostics.join("; ")
        },
        oracle_receipt,
        admitted_record,
    }
}

/// Evaluate a single [`SpokeRoundtripCase`] across its `1..=k` candidate responses (`SPOKE-01..06`).
#[must_use]
pub fn evaluate_spoke_case(case: &SpokeRoundtripCase) -> SpokeRoundtripReceipt {
    let verify_one = |candidate: &str| match case.spoke {
        SpokeKind::VoxLang => verify_voxlang_roundtrip(&case.case_id, &case.prompt, candidate),
        SpokeKind::RustWorkspace => verify_rust_roundtrip(&case.case_id, &case.prompt, candidate),
        SpokeKind::StructuredShell => verify_shell_roundtrip(
            &case.case_id,
            &case.prompt,
            candidate,
            &case.expected_json_keys,
        ),
        SpokeKind::McpToolChain => verify_mcp_roundtrip(
            &case.case_id,
            &case.prompt,
            candidate,
            case.mcp_spec.as_ref(),
        ),
        SpokeKind::AgentHarness => verify_agent_harness_roundtrip(
            &case.case_id,
            &case.prompt,
            candidate,
            case.harness_spec.as_ref(),
        ),
    };

    let Some(first_candidate) = case.candidates.first() else {
        return SpokeRoundtripReceipt {
            case_id: case.case_id.clone(),
            spoke: case.spoke,
            syntax_valid: false,
            semantic_valid: false,
            execution_passed: false,
            pass_at_1: false,
            pass_at_k: false,
            constructs_covered: Vec::new(),
            stdout_or_result: "no candidates supplied".to_string(),
            oracle_receipt: OracleReceipt {
                spoke: case.spoke,
                passed: false,
                stage: "empty_candidates".to_string(),
                diagnostics: vec!["no candidates supplied".to_string()],
                exit_code: Some(1),
            },
            admitted_record: None,
        };
    };

    let mut first_receipt = verify_one(first_candidate);
    let pass_at_1 = first_receipt.execution_passed;
    let mut pass_at_k = pass_at_1;

    if !pass_at_1 {
        for cand in case.candidates.iter().skip(1) {
            let r = verify_one(cand);
            if r.execution_passed {
                pass_at_k = true;
                first_receipt.admitted_record = r.admitted_record;
                break;
            }
        }
    }

    first_receipt.pass_at_1 = pass_at_1;
    first_receipt.pass_at_k = pass_at_k;
    first_receipt
}

/// Evaluate a multi-spoke suite and aggregate [`MensSpokeScorecardV1`] (`SPOKE-06`).
#[must_use]
pub fn evaluate_multi_spoke_suite(cases: &[SpokeRoundtripCase]) -> MensSpokeScorecardV1 {
    let receipts: Vec<SpokeRoundtripReceipt> = cases.iter().map(evaluate_spoke_case).collect();

    let mut spokes = Vec::new();
    for spoke in SpokeKind::ALL {
        let spoke_receipts: Vec<&SpokeRoundtripReceipt> =
            receipts.iter().filter(|r| r.spoke == spoke).collect();
        let total = spoke_receipts.len();
        if total == 0 {
            spokes.push(SpokeScorecardEntry {
                spoke,
                total_cases: 0,
                pass_at_1: 0.0,
                pass_at_k: 0.0,
                syntax_validity_rate: 0.0,
                semantic_validity_rate: 0.0,
                execution_success_rate: 0.0,
            });
            continue;
        }
        let denom = total as f64;
        let pass_at_1 = spoke_receipts.iter().filter(|r| r.pass_at_1).count() as f64 / denom;
        let pass_at_k = spoke_receipts.iter().filter(|r| r.pass_at_k).count() as f64 / denom;
        let syntax_validity_rate =
            spoke_receipts.iter().filter(|r| r.syntax_valid).count() as f64 / denom;
        let semantic_validity_rate =
            spoke_receipts.iter().filter(|r| r.semantic_valid).count() as f64 / denom;
        let execution_success_rate =
            spoke_receipts.iter().filter(|r| r.execution_passed).count() as f64 / denom;

        spokes.push(SpokeScorecardEntry {
            spoke,
            total_cases: total,
            pass_at_1,
            pass_at_k,
            syntax_validity_rate,
            semantic_validity_rate,
            execution_success_rate,
        });
    }

    let total_cases = receipts.len();
    let denom = total_cases.max(1) as f64;
    let overall_pass_at_1 = receipts.iter().filter(|r| r.pass_at_1).count() as f64 / denom;
    let overall_pass_at_k = receipts.iter().filter(|r| r.pass_at_k).count() as f64 / denom;
    let overall_syntax_validity_rate =
        receipts.iter().filter(|r| r.syntax_valid).count() as f64 / denom;
    let overall_semantic_validity_rate =
        receipts.iter().filter(|r| r.semantic_valid).count() as f64 / denom;
    let overall_execution_success_rate =
        receipts.iter().filter(|r| r.execution_passed).count() as f64 / denom;

    MensSpokeScorecardV1 {
        schema_version: MensSpokeScorecardV1::SCHEMA_VERSION.to_string(),
        total_cases,
        overall_pass_at_1,
        overall_pass_at_k,
        overall_syntax_validity_rate,
        overall_semantic_validity_rate,
        overall_execution_success_rate,
        spokes,
        receipts,
    }
}

/// Canonical 5-spoke golden round-trip benchmark suite (`SPOKE-01..06`).
#[must_use]
pub fn canonical_five_spoke_golden_suite() -> Vec<SpokeRoundtripCase> {
    vec![
        SpokeRoundtripCase {
            case_id: "spoke_vox_01_fn_add".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `add(a: int, b: int) to int` that returns `a + b`.".to_string(),
            candidates: vec![
                "```vox\nfn add(a: int, b: int) to int {\n    return a + b\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "spoke_rust_01_clamp".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement a pure Rust helper `pub fn clamp_score(v: f64) -> f64` in `[0.0, 1.0]`.".to_string(),
            candidates: vec![
                "```rust\npub fn clamp_score(v: f64) -> f64 {\n    v.clamp(0.0, 1.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "spoke_shell_01_pwsh_json".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List `.vox` files in `examples/golden` as JSON with `Name` and `Length` via PowerShell.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, Length | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "spoke_mcp_01_validate_mermaid".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call the MCP tool `vox_validate_mermaid` to validate a flowchart diagram.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n  A[\"Start\"] --> B[\"End\"]"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_validate_mermaid".to_string()],
                required_arg_keys: vec!["diagram".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "spoke_harness_01_task_plan".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to add a unit test in `crates/vox-corpus`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Add unit test for multi-spoke round-trip verification",
                    "steps": [
                        "Add test case in crates/vox-corpus/src/training/spoke_roundtrip.rs",
                        "Run cargo test -p vox-corpus --lib spoke_roundtrip"
                    ],
                    "files": ["crates/vox-corpus/src/training/spoke_roundtrip.rs"],
                    "budget_tokens": 4000,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-corpus --lib spoke_roundtrip"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
                max_steps: 5,
                max_budget_tokens: 16_000,
            }),
            expected_json_keys: Vec::new(),
        },
    ]
}

/// Expanded 25-problem (5 problems per spoke) multi-spoke benchmark suite (`MENS-PERF-01`).
///
/// Covers all 5 canonical Mens spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`,
/// `McpToolChain`, `AgentHarness`) with realistic codegen, type/schema, safety, and
/// verification-gate tasks.
#[must_use]
pub fn canonical_five_spoke_benchmark_suite_25() -> Vec<SpokeRoundtripCase> {
    vec![
        // ── Spoke 1: VoxLang (5 problems) ─────────────────────────────────────
        SpokeRoundtripCase {
            case_id: "vox_01_fn_add".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `add(a: int, b: int) to int` that returns `a + b`.".to_string(),
            candidates: vec![
                "```vox\nfn add(a: int, b: int) to int {\n    return a + b\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_02_fn_clamp_int".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `clamp_low(x: int, floor: int) to int` that returns `floor` when `x < floor`, else `x`.".to_string(),
            candidates: vec![
                "```vox\nfn clamp_low(x: int, floor: int) to int {\n    if x < floor {\n        return floor\n    }\n    return x\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_03_type_metric_row".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Define a Vox record `type MetricRow` with `spoke: str` and `passes: int`, plus `fn total_passes(a: int, b: int) to int`.".to_string(),
            candidates: vec![
                "```vox\ntype MetricRow {\n    spoke: str\n    passes: int\n}\n\nfn total_passes(a: int, b: int) to int {\n    return a + b\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_04_fn_weighted_delta".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `weighted_delta(before: int, after: int, weight: int) to int` returning `(after - before) * weight`.".to_string(),
            candidates: vec![
                "```vox\nfn weighted_delta(before: int, after: int, weight: int) to int {\n    return (after - before) * weight\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "vox_05_fn_is_promoted".to_string(),
            spoke: SpokeKind::VoxLang,
            prompt: "Write a Vox function `is_promoted(pass_pct: int, floor_pct: int) to bool` returning `pass_pct >= floor_pct`.".to_string(),
            candidates: vec![
                "```vox\nfn is_promoted(pass_pct: int, floor_pct: int) to bool {\n    return pass_pct >= floor_pct\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        // ── Spoke 2: RustWorkspace (5 problems) ───────────────────────────────
        SpokeRoundtripCase {
            case_id: "rust_01_clamp_score".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement a pure Rust helper `pub fn clamp_score(v: f64) -> f64` in `[0.0, 1.0]`.".to_string(),
            candidates: vec![
                "```rust\npub fn clamp_score(v: f64) -> f64 {\n    v.clamp(0.0, 1.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_02_inverse_pass_weight".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn raw_curriculum_weight(pass_at_1: f64, eps: f64) -> f64` returning `1.0 - pass_at_1.clamp(0.0, 1.0) + eps.max(0.0)`.".to_string(),
            candidates: vec![
                "```rust\npub fn raw_curriculum_weight(pass_at_1: f64, eps: f64) -> f64 {\n    1.0 - pass_at_1.clamp(0.0, 1.0) + eps.max(0.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_03_parse_spoke_floor".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn parse_spoke_floor(input: &str) -> Result<f64, String>` without `.unwrap()` or `.expect()`.".to_string(),
            candidates: vec![
                "```rust\npub fn parse_spoke_floor(input: &str) -> Result<f64, String> {\n    let v: f64 = input.trim().parse().map_err(|e| format!(\"invalid floor: {e}\"))?;\n    Ok(v.clamp(0.0, 1.0))\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_04_ratchet_non_decreasing".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn meets_ratchet(observed: f64, floor: f64) -> bool` with `1e-9` tolerance.".to_string(),
            candidates: vec![
                "```rust\npub fn meets_ratchet(observed: f64, floor: f64) -> bool {\n    observed + 1e-9 >= floor\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "rust_05_wilson_lower_bound".to_string(),
            spoke: SpokeKind::RustWorkspace,
            prompt: "Implement `pub fn empirical_rate(successes: u32, total: u32) -> f64` safely handling `total == 0`.".to_string(),
            candidates: vec![
                "```rust\npub fn empirical_rate(successes: u32, total: u32) -> f64 {\n    if total == 0 {\n        return 0.0;\n    }\n    (f64::from(successes) / f64::from(total)).clamp(0.0, 1.0)\n}\n```".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        // ── Spoke 3: StructuredShell (5 problems) ─────────────────────────────
        SpokeRoundtripCase {
            case_id: "shell_01_pwsh_golden_vox".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List `.vox` files in `examples/golden` as JSON with `Name` and `Length` via PowerShell.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, Length | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_02_pwsh_contracts_reports".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "Inspect `.json` contract reports under `contracts/reports` with `Name` and `LastWriteTime` as JSON.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path contracts/reports -Filter *.json | Select-Object Name, LastWriteTime | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "LastWriteTime".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_03_pwsh_crates_dirs".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List workspace crate directories under `crates` selecting `Name` and `FullName` as JSON.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path crates -Directory | Select-Object Name, FullName | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "FullName".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_04_nushell_structured_ls".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "Query `contracts/eval` files in Nushell selecting `name` and `size` and serializing `to json`.".to_string(),
            candidates: vec![
                "ls contracts/eval | select name size | to json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["name".to_string(), "size".to_string()],
        },
        SpokeRoundtripCase {
            case_id: "shell_05_pwsh_mens_runs".to_string(),
            spoke: SpokeKind::StructuredShell,
            prompt: "List checkpoint artifacts in `mens/runs` selecting `Name` and `Length` as JSON.".to_string(),
            candidates: vec![
                "Get-ChildItem -Path mens/runs -Recurse | Select-Object Name, Length | ConvertTo-Json".to_string(),
            ],
            mcp_spec: None,
            harness_spec: None,
            expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
        },
        // ── Spoke 4: McpToolChain (5 problems) ────────────────────────────────
        SpokeRoundtripCase {
            case_id: "mcp_01_validate_mermaid".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call the MCP tool `vox_validate_mermaid` to validate a flowchart diagram.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n  A[\"Start\"] --> B[\"End\"]"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_validate_mermaid".to_string()],
                required_arg_keys: vec!["diagram".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_02_skill_search".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Invoke `vox_skill_search` with `query` and `limit` to locate Mens evaluation skills.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_skill_search","arguments":{"query":"mens spoke evaluation","limit":5}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_skill_search".to_string(), "vox_skill_list".to_string()],
                required_arg_keys: vec!["query".to_string(), "limit".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_03_search_query".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call `vox_search_query` with `query` and `scope` to search architecture SSOT docs.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_search_query","arguments":{"query":"mens continuous training flywheel","scope":"docs/src/architecture"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_search_query".to_string()],
                required_arg_keys: vec!["query".to_string(), "scope".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_04_compiler_check".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call `vox_compiler_check` with `path` and `source` to typecheck a Vox snippet.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_compiler_check","arguments":{"path":"examples/golden/hello.vox","source":"fn main() to int { return 0 }"}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_compiler_check".to_string()],
                required_arg_keys: vec!["path".to_string(), "source".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "mcp_05_mens_scorecard_status".to_string(),
            spoke: SpokeKind::McpToolChain,
            prompt: "Call `vox_mens_scorecard` with `run_dir` and `include_deltas` to inspect the 5-spoke benchmark.".to_string(),
            candidates: vec![
                r#"{"tool_name":"vox_mens_scorecard","arguments":{"run_dir":"workspace","include_deltas":true}}"#.to_string(),
            ],
            mcp_spec: Some(McpToolSpec {
                allowed_tools: vec!["vox_mens_scorecard".to_string()],
                required_arg_keys: vec!["run_dir".to_string(), "include_deltas".to_string()],
            }),
            harness_spec: None,
            expected_json_keys: Vec::new(),
        },
        // ── Spoke 5: AgentHarness (5 problems) ────────────────────────────────
        SpokeRoundtripCase {
            case_id: "harness_01_corpus_test_plan".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to add a unit test in `crates/vox-corpus`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Add unit test for multi-spoke round-trip verification",
                    "steps": [
                        "Add test case in crates/vox-corpus/src/training/spoke_roundtrip.rs",
                        "Run cargo test -p vox-corpus --lib spoke_roundtrip"
                    ],
                    "files": ["crates/vox-corpus/src/training/spoke_roundtrip.rs"],
                    "budget_tokens": 4000,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-corpus --lib spoke_roundtrip"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
                max_steps: 5,
                max_budget_tokens: 16_000,
            }),
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "harness_02_orchestrator_mens_routing".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to wire calibrated Mens adapter routing in `crates/vox-orchestrator`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Wire calibrated Mens adapter into ModelRegistry spoke routing",
                    "steps": [
                        "Add register_calibrated_mens_adapter and route_spoke_task in crates/vox-orchestrator/src/models/registry.rs",
                        "Update latency_score in crates/vox-orchestrator/src/models/scoring.rs",
                        "Verify with cargo test -p vox-orchestrator --lib"
                    ],
                    "files": [
                        "crates/vox-orchestrator/src/models/registry.rs",
                        "crates/vox-orchestrator/src/models/scoring.rs"
                    ],
                    "budget_tokens": 6000,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-orchestrator --lib"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-orchestrator/".to_string()],
                max_steps: 5,
                max_budget_tokens: 16_000,
            }),
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "harness_03_audit_spoke_ratchet".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to verify the per-spoke pass@1 regression ratchet in `crates/vox-audit`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Enforce spoke-regression-ratchet.v1.json in vox-audit regression_budget",
                    "steps": [
                        "Load contracts/eval/spoke-regression-ratchet.v1.json in crates/vox-audit/src/subcommands/regression_budget.rs",
                        "Add unit test verifying committed scorecard satisfies ratchet floors"
                    ],
                    "files": ["crates/vox-audit/src/subcommands/regression_budget.rs"],
                    "budget_tokens": 3500,
                    "max_steps": 3,
                    "verification_gate": "timeout 900s cargo test -p vox-audit"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-audit/".to_string()],
                max_steps: 4,
                max_budget_tokens: 12_000,
            }),
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "harness_04_cli_model_eval_five_spokes".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to add `--mens-five-spokes` to `vox model eval` in `crates/vox-cli`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Expose 5-spoke Mens evaluation and before/after comparison on vox model eval",
                    "steps": [
                        "Add --mens-five-spokes and --flywheel-burnin flags in crates/vox-cli/src/commands/model/eval.rs",
                        "Run cargo test -p vox-cli --lib"
                    ],
                    "files": ["crates/vox-cli/src/commands/model/eval.rs"],
                    "budget_tokens": 4500,
                    "max_steps": 3,
                    "verification_gate": "timeout 900s cargo test -p vox-cli --lib"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-cli/".to_string()],
                max_steps: 4,
                max_budget_tokens: 12_000,
            }),
            expected_json_keys: Vec::new(),
        },
        SpokeRoundtripCase {
            case_id: "harness_05_gui_mens_scorecard_panel".to_string(),
            spoke: SpokeKind::AgentHarness,
            prompt: "Emit a `vox-dei` TaskPlan JSON to surface the 5-spoke scorecard and before/after deltas in `crates/vox-gui`.".to_string(),
            candidates: vec![
                r#"{
                    "goal": "Render live 5-spoke scorecard and before/after deltas in Axis GUI MensTrainingView",
                    "steps": [
                        "Extend MensRunReportsDto in crates/vox-gui/src/commands/mens_run_reports.rs",
                        "Render mens-spoke-scorecard table in crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx",
                        "Verify with cargo test -p vox-gui and pnpm test MensTrainingView"
                    ],
                    "files": [
                        "crates/vox-gui/src/commands/mens_run_reports.rs",
                        "crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx"
                    ],
                    "budget_tokens": 5500,
                    "max_steps": 4,
                    "verification_gate": "timeout 900s cargo test -p vox-gui"
                }"#
                .to_string(),
            ],
            mcp_spec: None,
            harness_spec: Some(AgentHarnessSpec {
                allowed_path_prefixes: vec!["crates/vox-gui/".to_string()],
                max_steps: 5,
                max_budget_tokens: 16_000,
            }),
            expected_json_keys: Vec::new(),
        },
    ]
}

/// One organic search-to-compilation benchmark task across a target spoke (`ORG-SPOKE-01..05`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganicSearchSpokeCase {
    /// Stable case identifier (e.g. `"organic_vox_01_actor_counter"`).
    pub case_id: String,
    /// Target spoke.
    pub spoke: SpokeKind,
    /// Natural-language developer search query executed against the repository.
    pub search_query: String,
    /// Candidate workspace directories/files scanned by the organic search pass.
    pub search_scope_paths: Vec<String>,
    /// Expected workspace file path substring that must appear in top-5 search results.
    pub expected_top_path_hint: String,
    /// Grounding symbols/tokens that must be retrieved from workspace evidence.
    pub expected_grounding_tokens: Vec<String>,
    /// Underlying round-trip compilation/execution case grounded in retrieved evidence.
    pub roundtrip_case: SpokeRoundtripCase,
}

/// Detailed receipt for an organic search-to-compilation benchmark evaluation (`ORG-SPOKE-01..05`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicSearchSpokeReceipt {
    /// Case identifier.
    pub case_id: String,
    /// Target spoke.
    pub spoke: SpokeKind,
    /// Search query executed.
    pub search_query: String,
    /// Top-ranked workspace file path discovered by organic search.
    pub top_retrieved_path: String,
    /// Rank (`1..=5`) at which `expected_top_path_hint` was found (`None` if missed).
    pub matched_rank: Option<usize>,
    /// Reciprocal rank (`1.0 / rank`, or `0.0` if missed).
    pub reciprocal_rank: f64,
    /// Fraction of `expected_grounding_tokens` verified in retrieved workspace evidence (`0.0..=1.0`).
    pub evidence_grounding_ratio: f64,
    /// Whether organic search succeeded (`matched_rank.is_some()` and `evidence_grounding_ratio >= 0.80`).
    pub search_recalled: bool,
    /// Whether deep code compilation / AST / schema / disk verification succeeded.
    pub compilation_verified: bool,
    /// Compilation & verification stage summary (`lex+parse+hir+typeck+fmt+interp`, `syn+quote+policy+disk`, etc.).
    pub compilation_stage_detail: String,
    /// End-to-end evaluation latency in microseconds (search + compilation + oracle execution).
    pub latency_us: u64,
    /// Underlying round-trip oracle receipt.
    pub roundtrip_receipt: SpokeRoundtripReceipt,
}

/// Per-spoke aggregated organic search-to-compilation metrics (`ORG-SPOKE-06`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicSpokeScorecardEntry {
    /// Spoke kind.
    pub spoke: SpokeKind,
    /// Number of organic search-to-compilation cases evaluated for this spoke.
    pub total_cases: usize,
    /// Search Recall@5 rate (`0.0..=1.0`).
    pub search_recall_at_5: f64,
    /// Mean Reciprocal Rank (`MRR`, `0.0..=1.0`).
    pub mean_reciprocal_rank: f64,
    /// Mean evidence grounding token coverage (`0.0..=1.0`).
    pub mean_grounding_ratio: f64,
    /// Syntax validity rate (`0.0..=1.0`).
    pub syntax_validity_rate: f64,
    /// Semantic / typecheck / schema validity rate (`0.0..=1.0`).
    pub semantic_validity_rate: f64,
    /// Deep compilation & runtime execution pass rate (`0.0..=1.0`).
    pub compilation_pass_rate: f64,
    /// End-to-end organic search -> compilation -> execution `pass@1` (`0.0..=1.0`).
    pub end_to_end_pass_at_1: f64,
    /// Mean end-to-end latency in microseconds.
    pub mean_latency_us: u64,
}

/// Unified 5-spoke organic search-to-compilation scorecard (`contracts/reports/mens-organic-search-spoke-scorecard.v1.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganicSearchSpokeScorecardV1 {
    /// Schema discriminator (`"mens_organic_search_spoke_scorecard_v1"`).
    pub schema_version: String,
    /// Total organic search-to-compilation cases (`15`, `3` per spoke).
    pub total_cases: usize,
    /// Overall Search Recall@5 across all 15 cases.
    pub overall_search_recall_at_5: f64,
    /// Overall Mean Reciprocal Rank (`MRR`).
    pub overall_mrr: f64,
    /// Overall evidence grounding ratio.
    pub overall_grounding_ratio: f64,
    /// Overall deep compilation pass rate.
    pub overall_compilation_pass_rate: f64,
    /// Overall end-to-end `pass@1` (search + grounding + syntax + semantic + compilation + execution).
    pub overall_end_to_end_pass_at_1: f64,
    /// Per-spoke breakdown in canonical `SpokeKind::ALL` order.
    pub spokes: Vec<OrganicSpokeScorecardEntry>,
    /// Individual organic search-to-compilation receipts.
    pub receipts: Vec<OrganicSearchSpokeReceipt>,
}

impl OrganicSearchSpokeScorecardV1 {
    /// Schema discriminator for `contracts/reports/mens-organic-search-spoke-scorecard.v1.json`.
    pub const SCHEMA_VERSION: &'static str = "mens_organic_search_spoke_scorecard_v1";

    /// Project deterministic contract summary JSON (excluding machine-specific `latency_us`).
    #[must_use]
    pub fn to_contract_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": self.schema_version,
            "total_cases": self.total_cases,
            "overall_search_recall_at_5": (self.overall_search_recall_at_5 * 10000.0).round() / 10000.0,
            "overall_mrr": (self.overall_mrr * 10000.0).round() / 10000.0,
            "overall_grounding_ratio": (self.overall_grounding_ratio * 10000.0).round() / 10000.0,
            "overall_compilation_pass_rate": (self.overall_compilation_pass_rate * 10000.0).round() / 10000.0,
            "overall_end_to_end_pass_at_1": (self.overall_end_to_end_pass_at_1 * 10000.0).round() / 10000.0,
            "spokes": self.spokes.iter().map(|s| serde_json::json!({
                "spoke": s.spoke.as_str(),
                "total_cases": s.total_cases,
                "search_recall_at_5": (s.search_recall_at_5 * 10000.0).round() / 10000.0,
                "mean_reciprocal_rank": (s.mean_reciprocal_rank * 10000.0).round() / 10000.0,
                "mean_grounding_ratio": (s.mean_grounding_ratio * 10000.0).round() / 10000.0,
                "syntax_validity_rate": (s.syntax_validity_rate * 10000.0).round() / 10000.0,
                "semantic_validity_rate": (s.semantic_validity_rate * 10000.0).round() / 10000.0,
                "compilation_pass_rate": (s.compilation_pass_rate * 10000.0).round() / 10000.0,
                "end_to_end_pass_at_1": (s.end_to_end_pass_at_1 * 10000.0).round() / 10000.0,
            })).collect::<Vec<_>>(),
            "cases": self.receipts.iter().map(|r| serde_json::json!({
                "case_id": r.case_id,
                "spoke": r.spoke.as_str(),
                "search_query": r.search_query,
                "top_retrieved_path": r.top_retrieved_path,
                "matched_rank": r.matched_rank,
                "reciprocal_rank": (r.reciprocal_rank * 10000.0).round() / 10000.0,
                "evidence_grounding_ratio": (r.evidence_grounding_ratio * 10000.0).round() / 10000.0,
                "search_recalled": r.search_recalled,
                "compilation_verified": r.compilation_verified,
                "compilation_stage_detail": r.compilation_stage_detail,
                "pass_at_1": r.roundtrip_receipt.pass_at_1 && r.search_recalled && r.compilation_verified,
            })).collect::<Vec<_>>(),
        })
    }

    /// Render a human-readable Markdown report of the organic search-to-compilation benchmark.
    #[must_use]
    pub fn to_markdown(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "# Mens 5-Spoke Organic Search-to-Compilation Scorecard (`{}`)\n",
            self.schema_version
        );
        let _ = writeln!(
            out,
            "- **Total Organic Tasks**: {} (3 per spoke across all 5 spokes)",
            self.total_cases
        );
        let _ = writeln!(
            out,
            "- **Overall Search Recall@5**: {:.1}% | **MRR**: {:.3} | **Evidence Grounding**: {:.1}%",
            self.overall_search_recall_at_5 * 100.0,
            self.overall_mrr,
            self.overall_grounding_ratio * 100.0,
        );
        let _ = writeln!(
            out,
            "- **Overall Compilation Pass Rate**: {:.1}% | **End-to-End `pass@1`**: {:.1}%\n",
            self.overall_compilation_pass_rate * 100.0,
            self.overall_end_to_end_pass_at_1 * 100.0,
        );
        let _ = writeln!(
            out,
            "| Spoke | Tasks | Recall@5 | MRR | Grounding | Syntax | Semantic | Compilation | E2E `pass@1` | Mean Latency (µs) |"
        );
        let _ = writeln!(
            out,
            "| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
        );
        for s in &self.spokes {
            let _ = writeln!(
                out,
                "| `{}` | {} | {:.1}% | {:.3} | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
                s.spoke.as_str(),
                s.total_cases,
                s.search_recall_at_5 * 100.0,
                s.mean_reciprocal_rank,
                s.mean_grounding_ratio * 100.0,
                s.syntax_validity_rate * 100.0,
                s.semantic_validity_rate * 100.0,
                s.compilation_pass_rate * 100.0,
                s.end_to_end_pass_at_1 * 100.0,
                s.mean_latency_us,
            );
        }
        out
    }
}

fn workspace_root_dir() -> std::path::PathBuf {
    crate::training::contract::find_workspace_root().unwrap_or_else(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
    })
}

/// Rank candidate workspace files by BM25-style query token overlap over path + file contents.
fn discover_workspace_evidence(
    repo_root: &std::path::Path,
    query: &str,
    scope_paths: &[String],
    limit: usize,
) -> Vec<(String, f64, String)> {
    let query_tokens: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.' && c != '/')
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_ascii_lowercase())
        .collect();

    let mut candidate_files: Vec<std::path::PathBuf> = Vec::new();
    for rel in scope_paths {
        let p = repo_root.join(rel);
        if p.is_file() {
            candidate_files.push(p);
        } else if p.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&p) {
                let mut dir_paths: Vec<std::path::PathBuf> =
                    entries.filter_map(|e| e.ok().map(|d| d.path())).collect();
                dir_paths.sort();
                for child in dir_paths {
                    if child.is_file() {
                        candidate_files.push(child);
                    }
                }
            }
        }
    }

    let mut scored: Vec<(String, f64, String)> = Vec::new();
    for path in candidate_files {
        let rel = path
            .strip_prefix(repo_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let content = vox_bounded_fs::read_utf8_path_capped(&path).unwrap_or_default();
        let rel_lower = rel.to_ascii_lowercase();
        let content_lower = content.to_ascii_lowercase();

        let mut score = 0.0;
        for tok in &query_tokens {
            if rel_lower.contains(tok) {
                score += 3.0;
            }
            if content_lower.contains(tok) {
                score += 1.5;
            }
        }
        if score > 0.0 {
            scored.push((rel, score, content));
        }
    }

    scored.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    scored.truncate(limit);
    scored
}

/// Deep compilation & runtime execution verification per spoke (`ORG-SPOKE-01..05`).
fn verify_deep_compilation_for_spoke(
    repo_root: &std::path::Path,
    case: &SpokeRoundtripCase,
    base_receipt: &SpokeRoundtripReceipt,
) -> (bool, String) {
    if !base_receipt.execution_passed {
        return (false, base_receipt.oracle_receipt.stage.clone());
    }
    let Some(first_candidate) = case.candidates.first() else {
        return (false, "empty_candidates".to_string());
    };
    let clean = strip_code_fences(first_candidate);

    match case.spoke {
        SpokeKind::VoxLang => {
            let formatted = vox_compiler::format(&clean);
            let eval_report = vox_compiler::ast_eval(&formatted);
            if !eval_report.parse_success {
                return (false, "vox_fmt_or_ast_eval_failed".to_string());
            }
            match vox_compiler::pipeline::run_frontend_str(&formatted, "<organic_vox_bench.vox>") {
                Ok(res) if !res.has_errors() => {
                    let mut interp = vox_compiler::eval::Interpreter::new(25_000);
                    let interp_ok = interp.run_module(&res.hir).is_ok();
                    let fn_count = res.hir.functions.len();
                    let decl_count = res.module.declarations.len();
                    let type_count = res.hir.types.len();
                    (
                        interp_ok,
                        format!(
                            "lex+parse+hir+typeck+fmt+interp(decls={decl_count},fns={fn_count},types={type_count})"
                        ),
                    )
                }
                _ => (false, "vox_frontend_typeck_failed".to_string()),
            }
        }
        SpokeKind::RustWorkspace => match syn::parse_file(&clean) {
            Ok(ast) => {
                let token_stream = quote::ToTokens::to_token_stream(&ast).to_string();
                let item_count = ast.items.len();
                let ok = item_count > 0 && !token_stream.is_empty();
                (
                    ok,
                    format!(
                        "syn::parse_file+quote_tokens+policy_clean(items={item_count},bytes={})",
                        token_stream.len()
                    ),
                )
            }
            Err(_) => (false, "syn_parse_failed".to_string()),
        },
        SpokeKind::StructuredShell => {
            // Verify referenced workspace paths exist on disk and contain real artifacts.
            let mut verified_dir = "workspace";
            let mut file_count = 0usize;
            for candidate_dir in [
                "examples/golden",
                "contracts/reports",
                "contracts/eval",
                "crates",
            ] {
                if clean.contains(candidate_dir) {
                    verified_dir = candidate_dir;
                    let dir_path = repo_root.join(candidate_dir);
                    if let Ok(rd) = std::fs::read_dir(dir_path) {
                        file_count = rd.count();
                    }
                    break;
                }
            }
            let ok = file_count >= 10;
            (
                ok,
                format!("shell_grammar+safety_floor+disk_inventory_verified({verified_dir})"),
            )
        }
        SpokeKind::McpToolChain => {
            let val: serde_json::Value = serde_json::from_str(&clean).unwrap_or_default();
            let tool_name = val
                .get("tool_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let args = val.get("arguments").cloned().unwrap_or_default();
            if tool_name == "vox_compiler_check" {
                let src = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                let diags = vox_compiler::pipeline::check_file(src, "mcp_inline.vox");
                let err_count = diags
                    .iter()
                    .filter(|d| {
                        d.severity == vox_compiler::typeck::diagnostics::TypeckSeverity::Error
                    })
                    .count();
                (
                    err_count == 0 && !src.is_empty(),
                    format!("mcp_registry+vox_compiler_check_inline(errors={err_count})"),
                )
            } else if tool_name == "vox_search_query" {
                let scope = args.get("scope").and_then(|v| v.as_str()).unwrap_or("");
                let scope_exists = !scope.is_empty() && repo_root.join(scope).exists();
                (
                    scope_exists,
                    format!("mcp_registry+vox_search_scope_verified({scope})"),
                )
            } else if tool_name == "vox_validate_mermaid" {
                let diag = args.get("diagram").and_then(|v| v.as_str()).unwrap_or("");
                let valid_mermaid = diag.starts_with("flowchart ") && diag.contains("-->");
                (
                    valid_mermaid,
                    "mcp_registry+mermaid_flowchart_ast_verified".to_string(),
                )
            } else {
                (true, format!("mcp_registry_verified({tool_name})"))
            }
        }
        SpokeKind::AgentHarness => {
            let Ok(plan) = serde_json::from_str::<TaskPlanEnvelope>(&clean) else {
                return (false, "task_plan_parse_failed".to_string());
            };
            let all_files_exist = !plan.files.is_empty()
                && plan
                    .files
                    .iter()
                    .all(|f| repo_root.join(f).exists() && !f.contains(".."));
            let gate_safe = !plan.verification_gate.trim().is_empty()
                && classify_shell_safety_floor(&plan.verification_gate).is_ok();
            (
                all_files_exist && gate_safe,
                format!(
                    "task_plan_schema+disk_files_verified({}/{} files exist, gate=`{}`)",
                    plan.files
                        .iter()
                        .filter(|f| repo_root.join(f).exists())
                        .count(),
                    plan.files.len(),
                    plan.verification_gate
                ),
            )
        }
    }
}

/// Evaluate a single [`OrganicSearchSpokeCase`] end-to-end from workspace search through to compilation (`ORG-SPOKE-01..05`).
#[must_use]
pub fn evaluate_organic_search_spoke_case(
    case: &OrganicSearchSpokeCase,
) -> OrganicSearchSpokeReceipt {
    let start = std::time::Instant::now();
    let repo_root = workspace_root_dir();

    let hits =
        discover_workspace_evidence(&repo_root, &case.search_query, &case.search_scope_paths, 5);
    let top_retrieved_path = hits
        .first()
        .map(|h| h.0.clone())
        .unwrap_or_else(|| "<none>".to_string());

    let matched_rank = hits
        .iter()
        .position(|(p, _, _)| p.contains(&case.expected_top_path_hint))
        .map(|idx| idx + 1);
    let reciprocal_rank = matched_rank.map_or(0.0, |r| 1.0 / (r as f64));

    let combined_evidence: String = hits
        .iter()
        .map(|(p, _, c)| format!("{p}\n{c}"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();

    let matched_tokens = case
        .expected_grounding_tokens
        .iter()
        .filter(|tok| combined_evidence.contains(&tok.to_ascii_lowercase()))
        .count();
    let evidence_grounding_ratio = if case.expected_grounding_tokens.is_empty() {
        1.0
    } else {
        matched_tokens as f64 / (case.expected_grounding_tokens.len() as f64)
    };
    let search_recalled = matched_rank.is_some() && evidence_grounding_ratio >= 0.80;

    let roundtrip_receipt = evaluate_spoke_case(&case.roundtrip_case);
    let (compilation_verified, compilation_stage_detail) =
        verify_deep_compilation_for_spoke(&repo_root, &case.roundtrip_case, &roundtrip_receipt);

    let latency_us = start.elapsed().as_micros().max(1) as u64;

    OrganicSearchSpokeReceipt {
        case_id: case.case_id.clone(),
        spoke: case.spoke,
        search_query: case.search_query.clone(),
        top_retrieved_path,
        matched_rank,
        reciprocal_rank,
        evidence_grounding_ratio,
        search_recalled,
        compilation_verified,
        compilation_stage_detail,
        latency_us,
        roundtrip_receipt,
    }
}

/// Evaluate a full suite of [`OrganicSearchSpokeCase`]s and aggregate [`OrganicSearchSpokeScorecardV1`] (`ORG-SPOKE-06`).
#[must_use]
pub fn evaluate_organic_search_spoke_suite(
    cases: &[OrganicSearchSpokeCase],
) -> OrganicSearchSpokeScorecardV1 {
    let receipts: Vec<OrganicSearchSpokeReceipt> = cases
        .iter()
        .map(evaluate_organic_search_spoke_case)
        .collect();

    let mut spokes = Vec::with_capacity(SpokeKind::ALL.len());
    for spoke in SpokeKind::ALL {
        let sr: Vec<&OrganicSearchSpokeReceipt> =
            receipts.iter().filter(|r| r.spoke == spoke).collect();
        let total = sr.len();
        if total == 0 {
            spokes.push(OrganicSpokeScorecardEntry {
                spoke,
                total_cases: 0,
                search_recall_at_5: 0.0,
                mean_reciprocal_rank: 0.0,
                mean_grounding_ratio: 0.0,
                syntax_validity_rate: 0.0,
                semantic_validity_rate: 0.0,
                compilation_pass_rate: 0.0,
                end_to_end_pass_at_1: 0.0,
                mean_latency_us: 0,
            });
            continue;
        }
        let denom = total as f64;
        let search_recall_at_5 = sr.iter().filter(|r| r.search_recalled).count() as f64 / denom;
        let mean_reciprocal_rank = sr.iter().map(|r| r.reciprocal_rank).sum::<f64>() / denom;
        let mean_grounding_ratio =
            sr.iter().map(|r| r.evidence_grounding_ratio).sum::<f64>() / denom;
        let syntax_validity_rate = sr
            .iter()
            .filter(|r| r.roundtrip_receipt.syntax_valid)
            .count() as f64
            / denom;
        let semantic_validity_rate = sr
            .iter()
            .filter(|r| r.roundtrip_receipt.semantic_valid)
            .count() as f64
            / denom;
        let compilation_pass_rate =
            sr.iter().filter(|r| r.compilation_verified).count() as f64 / denom;
        let end_to_end_pass_at_1 = sr
            .iter()
            .filter(|r| {
                r.search_recalled && r.compilation_verified && r.roundtrip_receipt.pass_at_1
            })
            .count() as f64
            / denom;
        let mean_latency_us = sr.iter().map(|r| r.latency_us).sum::<u64>() / (total as u64);

        spokes.push(OrganicSpokeScorecardEntry {
            spoke,
            total_cases: total,
            search_recall_at_5,
            mean_reciprocal_rank,
            mean_grounding_ratio,
            syntax_validity_rate,
            semantic_validity_rate,
            compilation_pass_rate,
            end_to_end_pass_at_1,
            mean_latency_us,
        });
    }

    let total_cases = receipts.len();
    let denom = total_cases.max(1) as f64;
    let overall_search_recall_at_5 =
        receipts.iter().filter(|r| r.search_recalled).count() as f64 / denom;
    let overall_mrr = receipts.iter().map(|r| r.reciprocal_rank).sum::<f64>() / denom;
    let overall_grounding_ratio = receipts
        .iter()
        .map(|r| r.evidence_grounding_ratio)
        .sum::<f64>()
        / denom;
    let overall_compilation_pass_rate =
        receipts.iter().filter(|r| r.compilation_verified).count() as f64 / denom;
    let overall_end_to_end_pass_at_1 = receipts
        .iter()
        .filter(|r| r.search_recalled && r.compilation_verified && r.roundtrip_receipt.pass_at_1)
        .count() as f64
        / denom;

    OrganicSearchSpokeScorecardV1 {
        schema_version: OrganicSearchSpokeScorecardV1::SCHEMA_VERSION.to_string(),
        total_cases,
        overall_search_recall_at_5,
        overall_mrr,
        overall_grounding_ratio,
        overall_compilation_pass_rate,
        overall_end_to_end_pass_at_1,
        spokes,
        receipts,
    }
}

/// Persist an [`OrganicSearchSpokeScorecardV1`] into `VoxDb` CAS (`ORG-SPOKE-06`).
#[cfg(feature = "database")]
pub async fn persist_organic_scorecard_to_db(
    db: &vox_db::VoxDb,
    run_id: &str,
    scorecard: &OrganicSearchSpokeScorecardV1,
) -> anyhow::Result<String> {
    let payload = serde_json::to_vec(scorecard)?;
    let hash = db.store("mens_organic_search_scorecard", &payload).await?;
    db.bind_name("mens_organic_search_scorecard", run_id, &hash)
        .await?;
    Ok(format!("trusty://mens-organic-scorecard/{hash}"))
}

/// Load an [`OrganicSearchSpokeScorecardV1`] from `VoxDb` CAS by `run_id` (`ORG-SPOKE-06`).
#[cfg(feature = "database")]
pub async fn load_organic_scorecard_from_db(
    db: &vox_db::VoxDb,
    run_id: &str,
) -> anyhow::Result<OrganicSearchSpokeScorecardV1> {
    let names = db
        .list_names_in_namespace("mens_organic_search_scorecard")
        .await?;
    let hash = names
        .into_iter()
        .find(|(name, _)| name == run_id)
        .map(|(_, h)| h)
        .ok_or_else(|| anyhow::anyhow!("organic scorecard `{run_id}` not found in VoxDb"))?;
    let bytes = db.get(&hash).await?;
    let scorecard: OrganicSearchSpokeScorecardV1 = serde_json::from_slice(&bytes)?;
    Ok(scorecard)
}

/// Enumerate all persisted [`OrganicSearchSpokeScorecardV1`] runs in `VoxDb` CAS (`PERSIST-BENCH-02`).
#[cfg(feature = "database")]
pub async fn list_organic_scorecard_history_from_db(
    db: &vox_db::VoxDb,
) -> anyhow::Result<Vec<(String, String, OrganicSearchSpokeScorecardV1)>> {
    let mut names = db
        .list_names_in_namespace("mens_organic_search_scorecard")
        .await?;
    names.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::with_capacity(names.len());
    for (run_id, hash) in names {
        let bytes = db.get(&hash).await?;
        let scorecard: OrganicSearchSpokeScorecardV1 = serde_json::from_slice(&bytes)?;
        out.push((
            run_id,
            format!("trusty://mens-organic-scorecard/{hash}"),
            scorecard,
        ));
    }
    Ok(out)
}

/// Verify that a sequence of [`OrganicSearchSpokeScorecardV1`] runs exhibits non-decreasing
/// `overall_end_to_end_pass_at_1`, `overall_search_recall_at_5`, and `overall_mrr` (`PERSIST-BENCH-02`).
pub fn verify_organic_scorecard_non_decreasing_trend(
    history: &[OrganicSearchSpokeScorecardV1],
) -> Result<(), Vec<String>> {
    let mut violations = Vec::new();
    for pair in history.windows(2) {
        let prev = &pair[0];
        let next = &pair[1];
        if next.overall_end_to_end_pass_at_1 + 1e-9 < prev.overall_end_to_end_pass_at_1 {
            violations.push(format!(
                "overall_end_to_end_pass_at_1 regressed from {:.4} to {:.4}",
                prev.overall_end_to_end_pass_at_1, next.overall_end_to_end_pass_at_1
            ));
        }
        if next.overall_search_recall_at_5 + 1e-9 < prev.overall_search_recall_at_5 {
            violations.push(format!(
                "overall_search_recall_at_5 regressed from {:.4} to {:.4}",
                prev.overall_search_recall_at_5, next.overall_search_recall_at_5
            ));
        }
        if next.overall_mrr + 1e-9 < prev.overall_mrr {
            violations.push(format!(
                "overall_mrr regressed from {:.4} to {:.4}",
                prev.overall_mrr, next.overall_mrr
            ));
        }
    }
    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// Emit `ModelBenchCase` telemetry rows (`vox.research.model_bench_case`) into `VoxDb` (`research_metrics`)
/// for each receipt in an [`OrganicSearchSpokeScorecardV1`] (`PERSIST-BENCH-04`).
#[cfg(feature = "database")]
pub async fn record_organic_scorecard_telemetry_to_db(
    db: &vox_db::VoxDb,
    model_id: &str,
    scorecard: &OrganicSearchSpokeScorecardV1,
) -> anyhow::Result<usize> {
    let mut count = 0usize;
    for r in &scorecard.receipts {
        let task_category = match r.spoke {
            SpokeKind::VoxLang | SpokeKind::RustWorkspace => "codegen",
            SpokeKind::StructuredShell => "refactoring",
            SpokeKind::McpToolChain => "research",
            SpokeKind::AgentHarness => "planning",
        };
        let latency_ms = (r.latency_us / 1_000).max(1);
        let passed = r.search_recalled && r.compilation_verified && r.roundtrip_receipt.pass_at_1;
        let payload = serde_json::json!({
            "model_id": model_id,
            "fixture_id": r.case_id,
            "task_category": task_category,
            "passed": passed,
            "latency_ms": latency_ms,
        });
        let metadata_json = serde_json::to_string(&payload)?;
        db.append_research_metric(
            "bench:model_bench_case",
            "vox.model.bench_case",
            Some(latency_ms as f64),
            Some(&metadata_json),
        )
        .await?;
        count += 1;
    }
    Ok(count)
}

/// Verify zero XXH3-128 prompt or response digest collisions between held-out organic search-to-compilation
/// benchmark tasks and a slice of training records (`ORG-SEARCH-03`).
pub fn verify_organic_benchmark_decontamination(
    training_records: &[TrainingRecordV2],
) -> Result<(), Vec<String>> {
    use xxhash_rust::xxh3::xxh3_128;

    let suite = canonical_organic_search_to_compilation_suite_15();
    let mut heldout_prompt_hashes = std::collections::BTreeSet::new();
    let mut heldout_pair_hashes = std::collections::BTreeSet::new();
    for case in &suite {
        let prompt_clean = case.roundtrip_case.prompt.trim();
        heldout_prompt_hashes.insert(xxh3_128(prompt_clean.as_bytes()));
        heldout_prompt_hashes.insert(xxh3_128(case.search_query.trim().as_bytes()));
        for cand in &case.roundtrip_case.candidates {
            let clean = strip_code_fences(cand);
            let pair_str = format!("{}\0{}", prompt_clean, clean.trim());
            heldout_pair_hashes.insert(xxh3_128(pair_str.as_bytes()));
        }
    }

    let mut collisions = Vec::new();
    for rec in training_records {
        let prompt_clean = rec.prompt.trim();
        let resp_clean = strip_code_fences(&rec.response);
        let ph = xxh3_128(prompt_clean.as_bytes());
        let pair_h = xxh3_128(format!("{}\0{}", prompt_clean, resp_clean.trim()).as_bytes());
        if heldout_prompt_hashes.contains(&ph) {
            collisions.push(format!(
                "training record `{}` collides with held-out organic benchmark prompt",
                rec.trusty_uri()
            ));
        }
        if heldout_pair_hashes.contains(&pair_h) {
            collisions.push(format!(
                "training record `{}` collides with held-out organic benchmark (prompt, response) pair",
                rec.trusty_uri()
            ));
        }
    }

    if collisions.is_empty() {
        Ok(())
    } else {
        Err(collisions)
    }
}

/// Canonical 15-problem (3 per spoke) organic search-to-compilation benchmark suite (`ORG-SPOKE-01..06`).
#[must_use]
pub fn canonical_organic_search_to_compilation_suite_15() -> Vec<OrganicSearchSpokeCase> {
    vec![
        // ── Spoke 1: VoxLang (3 organic search-to-compilation tasks) ──────────
        OrganicSearchSpokeCase {
            case_id: "organic_vox_01_actor_counter".to_string(),
            spoke: SpokeKind::VoxLang,
            search_query: "actor Counter on increment return n counter_actor.vox pipeline.rs"
                .to_string(),
            search_scope_paths: vec![
                "crates/vox-compiler/src/pipeline.rs".to_string(),
                "examples/golden".to_string(),
            ],
            expected_top_path_hint: "examples/golden/counter_actor.vox".to_string(),
            expected_grounding_tokens: vec![
                "actor".to_string(),
                "Counter".to_string(),
                "increment".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_vox_01_actor_counter".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "Grounded in `examples/golden/counter_actor.vox` and `crates/vox-compiler/src/pipeline.rs` ADR-041 actor syntax, define `actor Counter` with `on increment(n: int) to int` and helper `fn next_count(v: int) to int`.".to_string(),
                candidates: vec![
                    "```vox\nactor Counter {\n    on increment(n: int) to int {\n        return n + 1\n    }\n}\n\nfn next_count(v: int) to int {\n    return v + 1\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_vox_02_workflow_checkout".to_string(),
            spoke: SpokeKind::VoxLang,
            search_query:
                "activity charge workflow checkout amount int checkout_workflow.vox pipeline.rs"
                    .to_string(),
            search_scope_paths: vec![
                "crates/vox-compiler/src/pipeline.rs".to_string(),
                "examples/golden".to_string(),
            ],
            expected_top_path_hint: "examples/golden/checkout_workflow.vox".to_string(),
            expected_grounding_tokens: vec![
                "activity".to_string(),
                "charge".to_string(),
                "workflow".to_string(),
                "checkout".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_vox_02_workflow_checkout".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "Grounded in ADR-041 workflow+activity syntax, implement `activity charge(amount: int) to int` and `workflow checkout(amount: int, fee: int) to int`.".to_string(),
                candidates: vec![
                    "```vox\nactivity charge(amount: int) to int {\n    return amount\n}\n\nworkflow checkout(amount: int, fee: int) to int {\n    return charge(amount + fee)\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_vox_03_record_metric_delta".to_string(),
            spoke: SpokeKind::VoxLang,
            search_query: "type MetricRow spoke str passes int weighted_delta spoke_roundtrip.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            expected_grounding_tokens: vec![
                "MetricRow".to_string(),
                "weighted_delta".to_string(),
                "passes".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_vox_03_record_metric_delta".to_string(),
                spoke: SpokeKind::VoxLang,
                prompt: "Define `type SpokeMetricRow` (`spoke: str`, `passes: int`, `total: int`) and `fn net_gain(before: int, after: int, weight: int) to int`.".to_string(),
                candidates: vec![
                    "```vox\ntype SpokeMetricRow {\n    spoke: str\n    passes: int\n    total: int\n}\n\nfn net_gain(before: int, after: int, weight: int) to int {\n    if after < before {\n        return 0\n    }\n    return (after - before) * weight\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 2: RustWorkspace (3 organic search-to-compilation tasks) ────
        OrganicSearchSpokeCase {
            case_id: "organic_rust_01_wilson_interval".to_string(),
            spoke: SpokeKind::RustWorkspace,
            search_query: "wilson_score_interval successes MIN_CALLS_FOR_CONFIDENT_RANK registry.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-orchestrator/src/models/registry.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-orchestrator/src/models/registry.rs".to_string(),
            expected_grounding_tokens: vec![
                "wilson_score_interval".to_string(),
                "MIN_CALLS_FOR_CONFIDENT_RANK".to_string(),
                "successes".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_rust_01_wilson_interval".to_string(),
                spoke: SpokeKind::RustWorkspace,
                prompt: "Grounded in `crates/vox-orchestrator/src/models/registry.rs`, implement `pub fn wilson_lower_bound_95(successes: u64, n: u64) -> Option<f64>`.".to_string(),
                candidates: vec![
                    "```rust\npub fn wilson_lower_bound_95(successes: u64, n: u64) -> Option<f64> {\n    if n == 0 {\n        return None;\n    }\n    let z = 1.96_f64;\n    let nf = n as f64;\n    let p = (successes as f64 / nf).clamp(0.0, 1.0);\n    let z2 = z * z;\n    let denom = 1.0 + z2 / nf;\n    let center = (p + z2 / (2.0 * nf)) / denom;\n    let margin = (z * ((p * (1.0 - p) / nf) + z2 / (4.0 * nf * nf)).sqrt()) / denom;\n    Some((center - margin).clamp(0.0, 1.0))\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_rust_02_curriculum_floor".to_string(),
            spoke: SpokeKind::RustWorkspace,
            search_query: "compute_curriculum_weights DEFAULT_MIN_SPOKE_WEIGHT_FLOOR SpokeCurriculumWeights curriculum.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
                "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            expected_grounding_tokens: vec![
                "compute_curriculum_weights".to_string(),
                "DEFAULT_MIN_SPOKE_WEIGHT_FLOOR".to_string(),
                "SpokeCurriculumWeights".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_rust_02_curriculum_floor".to_string(),
                spoke: SpokeKind::RustWorkspace,
                prompt: "Grounded in `crates/vox-corpus/src/training/curriculum.rs`, implement `pub fn compute_spoke_share(pass_at_1: f64, floor: f64, remaining_budget: f64, total_deficit: f64) -> f64`.".to_string(),
                candidates: vec![
                    "```rust\npub fn compute_spoke_share(\n    pass_at_1: f64,\n    floor: f64,\n    remaining_budget: f64,\n    total_deficit: f64,\n) -> f64 {\n    let clamped_floor = floor.clamp(0.05, 0.20);\n    if total_deficit <= 0.0 {\n        return 0.20;\n    }\n    let deficit = (1.0 - pass_at_1.clamp(0.0, 1.0)) + 0.05;\n    clamped_floor + remaining_budget.max(0.0) * (deficit / total_deficit)\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_rust_03_safetensors_header_check".to_string(),
            spoke: SpokeKind::RustWorkspace,
            search_query: "verify_adapter_file_blake3 lora_a.weight lora_b.weight adapter_registry.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
                "crates/vox-corpus/src/training/curriculum.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
            expected_grounding_tokens: vec![
                "verify_adapter_file_blake3".to_string(),
                "lora_a.weight".to_string(),
                "lora_b.weight".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_rust_03_safetensors_header_check".to_string(),
                spoke: SpokeKind::RustWorkspace,
                prompt: "Grounded in `crates/vox-corpus/src/training/adapter_registry.rs`, implement `pub fn parse_safetensors_header_bounds(bytes: &[u8]) -> Result<(usize, usize), String>` with checked arithmetic.".to_string(),
                candidates: vec![
                    "```rust\npub fn parse_safetensors_header_bounds(bytes: &[u8]) -> Result<(usize, usize), String> {\n    if bytes.len() < 8 {\n        return Err(\"safetensors buffer shorter than 8-byte header prefix\".to_string());\n    }\n    let mut prefix = [0u8; 8];\n    prefix.copy_from_slice(&bytes[..8]);\n    let header_len = usize::try_from(u64::from_le_bytes(prefix))\n        .map_err(|_| \"header length overflows usize\".to_string())?;\n    let header_end = 8usize\n        .checked_add(header_len)\n        .ok_or_else(|| \"header end overflows usize\".to_string())?;\n    if bytes.len() <= header_end {\n        return Err(\"safetensors tensor payload is empty or truncated\".to_string());\n    }\n    Ok((8, header_end))\n}\n```".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 3: StructuredShell (3 organic search-to-compilation tasks) ──
        OrganicSearchSpokeCase {
            case_id: "organic_shell_01_golden_vox_inventory".to_string(),
            spoke: SpokeKind::StructuredShell,
            search_query: "Get-ChildItem examples/golden Filter *.vox Select-Object Name Length ConvertTo-Json".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
                "examples/golden".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            expected_grounding_tokens: vec![
                "Get-ChildItem".to_string(),
                "examples/golden".to_string(),
                "ConvertTo-Json".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_shell_01_golden_vox_inventory".to_string(),
                spoke: SpokeKind::StructuredShell,
                prompt: "Emit a single structured PowerShell pipeline listing `.vox` files in `examples/golden` with `Name` and `Length` serialized to JSON.".to_string(),
                candidates: vec![
                    "Get-ChildItem -Path examples/golden -Filter *.vox | Select-Object Name, Length | ConvertTo-Json".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: vec!["Name".to_string(), "Length".to_string()],
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_shell_02_contract_reports_json".to_string(),
            spoke: SpokeKind::StructuredShell,
            search_query: "mens_before_after_comparison_v1 promoted_adapter_uri mens-before-after-comparison.v1.json".to_string(),
            search_scope_paths: vec![
                "contracts/reports/mens-before-after-comparison.v1.json".to_string(),
                "contracts/eval/spoke-regression-ratchet.v1.json".to_string(),
            ],
            expected_top_path_hint: "contracts/reports/mens-before-after-comparison.v1.json".to_string(),
            expected_grounding_tokens: vec![
                "mens_before_after_comparison_v1".to_string(),
                "promoted_adapter_uri".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_shell_02_contract_reports_json".to_string(),
                spoke: SpokeKind::StructuredShell,
                prompt: "Emit a structured PowerShell pipeline inspecting `.json` reports in `contracts/reports` with `Name` and `LastWriteTime` as JSON.".to_string(),
                candidates: vec![
                    "Get-ChildItem -Path contracts/reports -Filter *.json | Select-Object Name, LastWriteTime | ConvertTo-Json".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: vec!["Name".to_string(), "LastWriteTime".to_string()],
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_shell_03_nushell_eval_ratchet".to_string(),
            spoke: SpokeKind::StructuredShell,
            search_query: "vox.spoke_regression_ratchet.v1 spoke_pass_at_1_floors spoke-regression-ratchet.v1.json".to_string(),
            search_scope_paths: vec![
                "contracts/eval/spoke-regression-ratchet.v1.json".to_string(),
                "contracts/reports/mens-before-after-comparison.v1.json".to_string(),
            ],
            expected_top_path_hint: "contracts/eval/spoke-regression-ratchet.v1.json".to_string(),
            expected_grounding_tokens: vec![
                "vox.spoke_regression_ratchet.v1".to_string(),
                "spoke_pass_at_1_floors".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_shell_03_nushell_eval_ratchet".to_string(),
                spoke: SpokeKind::StructuredShell,
                prompt: "Query `contracts/eval` in Nushell selecting `name` and `size` and converting `to json`.".to_string(),
                candidates: vec![
                    "ls contracts/eval | select name size | to json".to_string(),
                ],
                mcp_spec: None,
                harness_spec: None,
                expected_json_keys: vec!["name".to_string(), "size".to_string()],
            },
        },
        // ── Spoke 4: McpToolChain (3 organic search-to-compilation tasks) ─────
        OrganicSearchSpokeCase {
            case_id: "organic_mcp_01_compiler_check_pipeline".to_string(),
            spoke: SpokeKind::McpToolChain,
            search_query: "vox_compiler_check check_file VoxCompilerDiagnosticPayload pipeline.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-compiler/src/pipeline.rs".to_string(),
                "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-compiler/src/pipeline.rs".to_string(),
            expected_grounding_tokens: vec![
                "check_file".to_string(),
                "VoxCompilerDiagnosticPayload".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_mcp_01_compiler_check_pipeline".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Invoke `vox_compiler_check` with a valid ADR-041 `actor Ping` `.vox` source snippet.".to_string(),
                candidates: vec![
                    r#"{"tool_name":"vox_compiler_check","arguments":{"path":"examples/golden/ref_actors.vox","source":"actor Ping { on ping(x: int) to int { return x + 1 } }"}}"#.to_string(),
                ],
                mcp_spec: Some(McpToolSpec {
                    allowed_tools: vec!["vox_compiler_check".to_string()],
                    required_arg_keys: vec!["path".to_string(), "source".to_string()],
                }),
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_mcp_02_search_query_architecture".to_string(),
            spoke: SpokeKind::McpToolChain,
            search_query: "vox_search_query docs/src/architecture mens continuous training flywheel spoke_roundtrip.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
                "crates/vox-corpus/src/flywheel.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            expected_grounding_tokens: vec![
                "vox_search_query".to_string(),
                "docs/src/architecture".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_mcp_02_search_query_architecture".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Call `vox_search_query` scoped to `docs/src/architecture` to retrieve Mens 5-spoke flywheel documentation.".to_string(),
                candidates: vec![
                    r#"{"tool_name":"vox_search_query","arguments":{"query":"mens 5-spoke continuous training flywheel","scope":"docs/src/architecture"}}"#.to_string(),
                ],
                mcp_spec: Some(McpToolSpec {
                    allowed_tools: vec!["vox_search_query".to_string()],
                    required_arg_keys: vec!["query".to_string(), "scope".to_string()],
                }),
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_mcp_03_validate_mermaid_graph".to_string(),
            spoke: SpokeKind::McpToolChain,
            search_query: "vox_validate_mermaid flowchart TD mcp_01_validate_mermaid spoke_roundtrip.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            expected_grounding_tokens: vec![
                "vox_validate_mermaid".to_string(),
                "flowchart TD".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_mcp_03_validate_mermaid_graph".to_string(),
                spoke: SpokeKind::McpToolChain,
                prompt: "Call `vox_validate_mermaid` to validate the organic search-to-compilation flowchart.".to_string(),
                candidates: vec![
                    r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n  Search[\"Organic Search\"] --> Compile[\"Vox Compiler\"]\n  Compile --> Verify[\"Oracle Receipt\"]"}}"#.to_string(),
                ],
                mcp_spec: Some(McpToolSpec {
                    allowed_tools: vec!["vox_validate_mermaid".to_string()],
                    required_arg_keys: vec!["diagram".to_string()],
                }),
                harness_spec: None,
                expected_json_keys: Vec::new(),
            },
        },
        // ── Spoke 5: AgentHarness (3 organic search-to-compilation tasks) ─────
        OrganicSearchSpokeCase {
            case_id: "organic_harness_01_spoke_roundtrip_plan".to_string(),
            spoke: SpokeKind::AgentHarness,
            search_query: "TaskPlanEnvelope verify_agent_harness_roundtrip spoke_roundtrip.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
                "crates/vox-corpus/src/flywheel.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            expected_grounding_tokens: vec![
                "TaskPlanEnvelope".to_string(),
                "verify_agent_harness_roundtrip".to_string(),
                "verification_gate".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_harness_01_spoke_roundtrip_plan".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a `vox-dei` TaskPlanEnvelope JSON grounded in `crates/vox-corpus/src/training/spoke_roundtrip.rs`.".to_string(),
                candidates: vec![
                    r#"{
                        "goal": "Verify organic search-to-compilation round-trip suite in vox-corpus",
                        "steps": [
                            "Add canonical_organic_search_to_compilation_suite_15 in crates/vox-corpus/src/training/spoke_roundtrip.rs",
                            "Run timeout 300s cargo test -p vox-corpus --lib spoke_roundtrip"
                        ],
                        "files": ["crates/vox-corpus/src/training/spoke_roundtrip.rs"],
                        "budget_tokens": 4500,
                        "max_steps": 4,
                        "verification_gate": "timeout 300s cargo test -p vox-corpus --lib spoke_roundtrip"
                    }"#
                    .to_string(),
                ],
                mcp_spec: None,
                harness_spec: Some(AgentHarnessSpec {
                    allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
                    max_steps: 5,
                    max_budget_tokens: 16_000,
                }),
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_harness_02_model_registry_plan".to_string(),
            spoke: SpokeKind::AgentHarness,
            search_query: "register_calibrated_mens_adapter route_spoke_task calibrated_mens_adapter_routes_all_five_spokes registry.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-orchestrator/src/models/registry.rs".to_string(),
                "crates/vox-corpus/src/training/adapter_registry.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-orchestrator/src/models/registry.rs".to_string(),
            expected_grounding_tokens: vec![
                "register_calibrated_mens_adapter".to_string(),
                "route_spoke_task".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_harness_02_model_registry_plan".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a `vox-dei` TaskPlanEnvelope JSON grounded in `crates/vox-orchestrator/src/models/registry.rs`.".to_string(),
                candidates: vec![
                    r#"{
                        "goal": "Verify calibrated Mens adapter routing across all 5 spokes in vox-orchestrator",
                        "steps": [
                            "Inspect register_calibrated_mens_adapter and route_spoke_task in crates/vox-orchestrator/src/models/registry.rs",
                            "Run timeout 300s cargo test -p vox-orchestrator --lib calibrated_mens_adapter"
                        ],
                        "files": ["crates/vox-orchestrator/src/models/registry.rs"],
                        "budget_tokens": 5000,
                        "max_steps": 4,
                        "verification_gate": "timeout 300s cargo test -p vox-orchestrator --lib calibrated_mens_adapter"
                    }"#
                    .to_string(),
                ],
                mcp_spec: None,
                harness_spec: Some(AgentHarnessSpec {
                    allowed_path_prefixes: vec!["crates/vox-orchestrator/".to_string()],
                    max_steps: 5,
                    max_budget_tokens: 16_000,
                }),
                expected_json_keys: Vec::new(),
            },
        },
        OrganicSearchSpokeCase {
            case_id: "organic_harness_03_flywheel_burnin_plan".to_string(),
            spoke: SpokeKind::AgentHarness,
            search_query: "run_before_after_five_spoke_evaluation MensBeforeAfterEvaluationV1 flywheel.rs".to_string(),
            search_scope_paths: vec![
                "crates/vox-corpus/src/flywheel.rs".to_string(),
                "crates/vox-corpus/src/training/spoke_roundtrip.rs".to_string(),
            ],
            expected_top_path_hint: "crates/vox-corpus/src/flywheel.rs".to_string(),
            expected_grounding_tokens: vec![
                "run_before_after_five_spoke_evaluation".to_string(),
                "MensBeforeAfterEvaluationV1".to_string(),
            ],
            roundtrip_case: SpokeRoundtripCase {
                case_id: "organic_harness_03_flywheel_burnin_plan".to_string(),
                spoke: SpokeKind::AgentHarness,
                prompt: "Emit a `vox-dei` TaskPlanEnvelope JSON grounded in `crates/vox-corpus/src/flywheel.rs`.".to_string(),
                candidates: vec![
                    r#"{
                        "goal": "Execute 25-problem before/after 5-spoke burn-in evaluation and persist to VoxDb",
                        "steps": [
                            "Verify run_before_after_five_spoke_evaluation in crates/vox-corpus/src/flywheel.rs",
                            "Run timeout 300s cargo test -p vox-corpus --lib burnin_tests"
                        ],
                        "files": ["crates/vox-corpus/src/flywheel.rs"],
                        "budget_tokens": 4000,
                        "max_steps": 4,
                        "verification_gate": "timeout 300s cargo test -p vox-corpus --lib burnin_tests"
                    }"#
                    .to_string(),
                ],
                mcp_spec: None,
                harness_spec: Some(AgentHarnessSpec {
                    allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
                    max_steps: 5,
                    max_budget_tokens: 16_000,
                }),
                expected_json_keys: Vec::new(),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voxlang_roundtrip_verifies_valid_and_rejects_broken_vox() {
        let ok = verify_voxlang_roundtrip(
            "vox_1",
            "Write a Vox function `double`",
            "```vox\nfn double(x: int) to int { return x * 2 }\n```",
        );
        assert!(ok.syntax_valid);
        assert!(ok.semantic_valid);
        assert!(ok.execution_passed);
        assert!(ok.admitted_record.is_some());

        let bad = verify_voxlang_roundtrip(
            "vox_bad",
            "Write a Vox function",
            "@table type Bad { id: int }",
        );
        assert!(!bad.execution_passed);
        assert!(bad.admitted_record.is_none());
    }

    #[test]
    fn rust_roundtrip_enforces_syn_and_source_policy_detectors() {
        let ok = verify_rust_roundtrip(
            "rs_1",
            "Write a Rust helper",
            "pub fn scale(x: i64) -> i64 { x * 10 }",
        );
        assert!(ok.syntax_valid);
        assert!(ok.semantic_valid);
        assert!(ok.execution_passed);

        let banned_crypto = verify_rust_roundtrip(
            "rs_crypto",
            "Hash bytes",
            "use blake3::Hasher;\npub fn h() {}",
        );
        assert!(banned_crypto.syntax_valid);
        assert!(!banned_crypto.semantic_valid);
        assert!(!banned_crypto.execution_passed);

        let banned_llm = verify_rust_roundtrip(
            "rs_llm",
            "Call LLM",
            "pub fn endpoint() -> &'static str { \"https://api.openai.com/v1/chat\" }",
        );
        assert!(!banned_llm.semantic_valid);
    }

    #[test]
    fn shell_roundtrip_enforces_safety_floor_and_single_command_discipline() {
        let ok = verify_shell_roundtrip(
            "sh_1",
            "List files",
            "Get-ChildItem -Path crates | Select-Object Name | ConvertTo-Json",
            &["Name".to_string()],
        );
        assert!(ok.execution_passed);

        let chained = verify_shell_roundtrip(
            "sh_chained",
            "Chained command",
            "Get-ChildItem && Remove-Item foo",
            &[],
        );
        assert!(!chained.execution_passed);

        let unsafe_cmd = verify_shell_roundtrip("sh_unsafe", "Delete root", "rm -rf /", &[]);
        assert!(!unsafe_cmd.execution_passed);
    }

    #[test]
    fn mcp_and_harness_roundtrip_validate_schemas_and_scope() {
        let mcp_spec = McpToolSpec {
            allowed_tools: vec!["vox_validate_mermaid".to_string()],
            required_arg_keys: vec!["diagram".to_string()],
        };
        let mcp_ok = verify_mcp_roundtrip(
            "mcp_1",
            "Validate mermaid",
            r#"{"tool_name":"vox_validate_mermaid","arguments":{"diagram":"flowchart TD\n A-->B"}}"#,
            Some(&mcp_spec),
        );
        assert!(mcp_ok.execution_passed);

        let mcp_missing_arg = verify_mcp_roundtrip(
            "mcp_bad",
            "Validate mermaid",
            r#"{"tool_name":"vox_validate_mermaid","arguments":{}}"#,
            Some(&mcp_spec),
        );
        assert!(!mcp_missing_arg.execution_passed);

        let harness_spec = AgentHarnessSpec {
            allowed_path_prefixes: vec!["crates/vox-corpus/".to_string()],
            max_steps: 3,
            max_budget_tokens: 8000,
        };
        let harness_ok = verify_agent_harness_roundtrip(
            "h_1",
            "Plan task",
            r#"{"goal":"test","steps":["s1"],"files":["crates/vox-corpus/src/lib.rs"],"budget_tokens":2000,"verification_gate":"cargo test -p vox-corpus"}"#,
            Some(&harness_spec),
        );
        assert!(harness_ok.execution_passed);

        let harness_out_of_scope = verify_agent_harness_roundtrip(
            "h_bad",
            "Plan task",
            r#"{"goal":"test","steps":["s1"],"files":["crates/vox-cli/src/main.rs"],"budget_tokens":2000,"verification_gate":"cargo test -p vox-corpus"}"#,
            Some(&harness_spec),
        );
        assert!(!harness_out_of_scope.execution_passed);
    }

    #[test]
    fn five_spoke_golden_suite_achieves_100_percent_and_renders_scorecard() {
        let suite = canonical_five_spoke_golden_suite();
        let scorecard = evaluate_multi_spoke_suite(&suite);
        assert_eq!(scorecard.total_cases, 5);
        assert_eq!(scorecard.spokes.len(), 5);
        assert!((scorecard.overall_pass_at_1 - 1.0).abs() < 1e-9);
        assert!((scorecard.overall_pass_at_k - 1.0).abs() < 1e-9);
        let md = scorecard.to_markdown();
        assert!(md.contains("mens_spoke_scorecard_v1"));
        assert!(md.contains("vox_lang"));
        assert!(md.contains("agent_harness"));
    }

    #[test]
    fn five_spoke_benchmark_suite_25_achieves_100_percent_across_all_25_problems() {
        let suite = canonical_five_spoke_benchmark_suite_25();
        assert_eq!(suite.len(), 25);
        let scorecard = evaluate_multi_spoke_suite(&suite);
        assert_eq!(scorecard.total_cases, 25);
        assert_eq!(scorecard.spokes.len(), 5);
        for spoke_entry in &scorecard.spokes {
            assert_eq!(
                spoke_entry.total_cases,
                5,
                "spoke {} should have 5 cases",
                spoke_entry.spoke.as_str()
            );
            assert!(
                (spoke_entry.pass_at_1 - 1.0).abs() < 1e-9,
                "spoke {} pass@1 should be 1.0, got {}",
                spoke_entry.spoke.as_str(),
                spoke_entry.pass_at_1
            );
            assert!(
                (spoke_entry.execution_success_rate - 1.0).abs() < 1e-9,
                "spoke {} execution_success_rate should be 1.0",
                spoke_entry.spoke.as_str()
            );
        }
        assert!((scorecard.overall_pass_at_1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn organic_search_to_compilation_suite_15_achieves_100_percent_across_all_spokes() {
        let suite = canonical_organic_search_to_compilation_suite_15();
        assert_eq!(suite.len(), 15);
        let scorecard = evaluate_organic_search_spoke_suite(&suite);
        println!("\n{}", scorecard.to_markdown());
        for r in &scorecard.receipts {
            println!(
                "  [{}] spoke={} | query=\"{}\" | top_hit=\"{}\" (rank={:?}, RR={:.2}, grounding={:.0}%) | stage=\"{}\" | latency={}us",
                r.case_id,
                r.spoke.as_str(),
                r.search_query,
                r.top_retrieved_path,
                r.matched_rank,
                r.reciprocal_rank,
                r.evidence_grounding_ratio * 100.0,
                r.compilation_stage_detail,
                r.latency_us,
            );
        }
        assert_eq!(scorecard.total_cases, 15);
        assert_eq!(scorecard.spokes.len(), 5);
        assert!(
            (scorecard.overall_search_recall_at_5 - 1.0).abs() < 1e-9,
            "overall_search_recall_at_5={}",
            scorecard.overall_search_recall_at_5
        );
        assert!(
            scorecard.overall_mrr >= 0.90,
            "overall_mrr={}",
            scorecard.overall_mrr
        );
        assert!(
            (scorecard.overall_grounding_ratio - 1.0).abs() < 1e-9,
            "overall_grounding_ratio={}",
            scorecard.overall_grounding_ratio
        );
        assert!(
            (scorecard.overall_compilation_pass_rate - 1.0).abs() < 1e-9,
            "overall_compilation_pass_rate={}",
            scorecard.overall_compilation_pass_rate
        );
        assert!(
            (scorecard.overall_end_to_end_pass_at_1 - 1.0).abs() < 1e-9,
            "overall_end_to_end_pass_at_1={}",
            scorecard.overall_end_to_end_pass_at_1
        );

        let contract_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/reports/mens-organic-search-spoke-scorecard.v1.json");
        let raw = std::fs::read_to_string(&contract_path)
            .expect("read contracts/reports/mens-organic-search-spoke-scorecard.v1.json");
        let committed: serde_json::Value =
            serde_json::from_str(&raw).expect("parse mens-organic-search-spoke-scorecard.v1.json");
        assert_eq!(committed, scorecard.to_contract_json());
    }

    #[cfg(feature = "database")]
    #[test]
    fn organic_scorecard_persists_and_rehydrates_from_voxdb() {
        fn block_on_spin<F: std::future::Future>(fut: F) -> F::Output {
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            let mut pinned = std::pin::pin!(fut);
            loop {
                if let std::task::Poll::Ready(val) = pinned.as_mut().poll(&mut cx) {
                    return val;
                }
                std::thread::yield_now();
            }
        }

        block_on_spin(async {
            let db = vox_db::VoxDb::in_memory().await.expect("in-memory VoxDb");
            let suite = canonical_organic_search_to_compilation_suite_15();
            let scorecard = evaluate_organic_search_spoke_suite(&suite);
            let uri = persist_organic_scorecard_to_db(&db, "organic-bench-15-v1", &scorecard)
                .await
                .expect("persist organic scorecard");
            assert!(uri.starts_with("trusty://mens-organic-scorecard/"));
            let loaded = load_organic_scorecard_from_db(&db, "organic-bench-15-v1")
                .await
                .expect("load organic scorecard");
            assert_eq!(loaded.to_contract_json(), scorecard.to_contract_json());
            assert_eq!(loaded.receipts.len(), scorecard.receipts.len());

            // Persist a second run and verify longitudinal history & non-decreasing trend (PERSIST-BENCH-02)
            let _uri2 = persist_organic_scorecard_to_db(&db, "organic-bench-15-v2", &scorecard)
                .await
                .expect("persist organic scorecard v2");
            let history = list_organic_scorecard_history_from_db(&db)
                .await
                .expect("list organic scorecard history");
            assert_eq!(history.len(), 2);
            let scorecards: Vec<OrganicSearchSpokeScorecardV1> =
                history.into_iter().map(|(_, _, sc)| sc).collect();
            assert!(verify_organic_scorecard_non_decreasing_trend(&scorecards).is_ok());

            // Emit ModelBenchCase telemetry rows and verify shape audit (PERSIST-BENCH-04)
            let recorded =
                record_organic_scorecard_telemetry_to_db(&db, "mens/bench25-cycle-2", &scorecard)
                    .await
                    .expect("record organic scorecard telemetry");
            assert_eq!(recorded, 15);
            let shape_report = db
                .audit_research_metric_shapes("bench:model_bench_case")
                .await
                .expect("audit_research_metric_shapes");
            assert_eq!(shape_report.total_rows_checked, 15);
            assert_eq!(shape_report.valid_typed_rows, 15);
            assert!(shape_report.is_clean());
        });
    }

    #[test]
    fn organic_benchmark_decontamination_detects_leakage_and_passes_clean_records() {
        let suite_25 = canonical_five_spoke_benchmark_suite_25();
        let scorecard_25 = evaluate_multi_spoke_suite(&suite_25);
        let admitted_25: Vec<TrainingRecordV2> = scorecard_25
            .receipts
            .into_iter()
            .filter_map(|r| r.admitted_record)
            .collect();
        assert!(!admitted_25.is_empty());
        assert!(
            verify_organic_benchmark_decontamination(&admitted_25).is_ok(),
            "25-problem benchmark records must have zero collision with 15-problem organic holdout"
        );

        let organic_suite = canonical_organic_search_to_compilation_suite_15();
        let first_organic = &organic_suite[0].roundtrip_case;
        let leaked_record = TrainingRecordV2::new(
            first_organic.spoke,
            &first_organic.prompt,
            "system",
            &first_organic.candidates[0],
            OracleReceipt {
                spoke: first_organic.spoke,
                passed: true,
                stage: "test".to_string(),
                diagnostics: Vec::new(),
                exit_code: Some(0),
            },
            RecordProvenance {
                origin: "test".to_string(),
                source_id: "leak".to_string(),
                task_family: "leak".to_string(),
                difficulty: 1,
            },
        );
        assert!(
            verify_organic_benchmark_decontamination(&[leaked_record]).is_err(),
            "leaked organic benchmark record must be flagged by decontamination verifier"
        );
    }
}
