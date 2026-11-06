//! Phase 78: End-to-End Text-In / Text-Out Multi-Spoke Verification & Round-Trip Evals (`SPOKE-01..06`).
//!
//! Pushes real prompts in and validates real text/code/tool outputs out across all 5 canonical
//! Mens spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`),
//! producing per-case [`SpokeRoundtripReceipt`]s, admitted [`TrainingRecordV2`] records, and
//! the unified [`MensSpokeScorecardV1`] report (`contracts/reports/mens-spoke-scorecard.v1.json`).

use serde::{Deserialize, Serialize};

pub(super) use crate::tool_workflow_corpus::{
    classify_shell_safety_floor, redact_trajectory_secrets,
};
pub(super) use crate::training::{OracleReceipt, RecordProvenance, SpokeKind, TrainingRecordV2};

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
    #[serde(alias = "objective", alias = "task_goal")]
    pub goal: String,
    /// Ordered execution steps.
    #[serde(alias = "plan_steps", alias = "tasks")]
    pub steps: Vec<String>,
    /// Target files touched by the plan.
    #[serde(alias = "target_files", alias = "paths")]
    pub files: Vec<String>,
    /// Token budget allocated to the plan.
    #[serde(default, alias = "max_tokens", alias = "token_budget")]
    pub budget_tokens: u64,
    /// Optional explicit step cap.
    #[serde(default)]
    pub max_steps: Option<usize>,
    /// Concrete verification command / gate (e.g. `timeout 900s cargo test -p vox-corpus`).
    #[serde(
        alias = "verification_command",
        alias = "test_command",
        alias = "verify_command"
    )]
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

mod verifiers_core;
pub use verifiers_core::*;

mod verifiers_shell_mcp;
pub use verifiers_shell_mcp::*;

mod verifiers_harness;
pub use verifiers_harness::*;

mod suites_five_spoke;
pub use suites_five_spoke::*;

mod verifiers_ext_spokes;
pub use verifiers_ext_spokes::*;

mod organic_types;
pub use organic_types::*;
use organic_types::{
    discover_workspace_evidence, discover_workspace_evidence_with_mode, workspace_root_dir,
};

mod organic_oracles;
use organic_oracles::verify_deep_compilation_for_spoke;
pub use organic_oracles::*;

mod organic_eval;
pub use organic_eval::*;

mod suites_organic_part1;
use suites_organic_part1::organic_suite_24_part1;
pub use suites_organic_part1::*;

mod suites_organic_part2;
pub use suites_organic_part2::*;

mod suites_organic_adversarial;
pub use suites_organic_adversarial::*;

#[cfg(test)]
mod tests;
