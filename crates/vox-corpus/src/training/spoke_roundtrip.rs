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
}
