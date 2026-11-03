//! Automated evaluation helpers for MENS-trained emitters (`Mn-T12`, `EVAL-01`).
//!
//! Evaluates all 5 Vox spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`,
//! `AgentHarness`) end-to-end with per-spoke `pass@1`, `pass@k`, syntax validity, semantic
//! validity, and execution rates.

use crate::corpus_score::pass_at_k;
use serde::{Deserialize, Serialize};

/// Normalised compile outcome for a single eval case.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompileVerdict {
    Pass,
    Fail { diagnostic_summary: String },
}

#[must_use]
pub fn summarize_placeholder() -> CompileVerdict {
    CompileVerdict::Pass
}

/// Canonical 5-spoke evaluation dimension (`EVAL-01`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MensEvalSpoke {
    VoxLang,
    RustWorkspace,
    StructuredShell,
    McpToolChain,
    AgentHarness,
}

impl MensEvalSpoke {
    /// All 5 spokes in canonical evaluation order.
    pub const ALL: [Self; 5] = [
        Self::VoxLang,
        Self::RustWorkspace,
        Self::StructuredShell,
        Self::McpToolChain,
        Self::AgentHarness,
    ];

    /// Stable snake_case identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VoxLang => "vox_lang",
            Self::RustWorkspace => "rust_workspace",
            Self::StructuredShell => "structured_shell",
            Self::McpToolChain => "mcp_tool_chain",
            Self::AgentHarness => "agent_harness",
        }
    }
}

/// Outcome of a single candidate attempt on a spoke problem (`EVAL-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpokeAttemptOutcome {
    /// Problem/case identifier.
    pub case_id: String,
    /// Target spoke.
    pub spoke: MensEvalSpoke,
    /// Whether the candidate parsed syntactically (`vox::lexer+parser`, `syn::parse_file`, JSON parse, etc.).
    pub syntax_valid: bool,
    /// Whether semantic/policy checks passed (`typeck`, no banned imports, shell safety floor, tool schema, plan bounds).
    pub semantic_valid: bool,
    /// Whether execution / round-trip verification succeeded (`--interp`, oracle pass).
    pub execution_passed: bool,
    /// Optional diagnostic summary on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic_summary: Option<String>,
}

impl SpokeAttemptOutcome {
    /// Full end-to-end pass requires syntax + semantic + execution validity.
    #[must_use]
    pub const fn is_full_pass(&self) -> bool {
        self.syntax_valid && self.semantic_valid && self.execution_passed
    }
}

/// Multi-sample attempts for a single benchmark problem (`EVAL-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpokeProblemInput {
    /// Problem identifier.
    pub case_id: String,
    /// Target spoke.
    pub spoke: MensEvalSpoke,
    /// Candidate attempts (`attempts[0]` is first attempt for deterministic `pass@1`, all attempts for unbiased `pass@k`).
    pub attempts: Vec<SpokeAttemptOutcome>,
}

/// Aggregated metrics for one spoke (`EVAL-01`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpokeEvalMetrics {
    /// Target spoke.
    pub spoke: MensEvalSpoke,
    /// Number of problems evaluated for this spoke.
    pub n_problems: usize,
    /// Mean `pass@1` across problems in this spoke.
    pub pass_at_1: f64,
    /// Mean `pass@k` across problems in this spoke.
    pub pass_at_k: f64,
    /// Fraction of all candidate attempts with `syntax_valid == true`.
    pub syntax_validity_rate: f64,
    /// Fraction of all candidate attempts with `semantic_valid == true`.
    pub semantic_validity_rate: f64,
    /// Fraction of all candidate attempts with `execution_passed == true`.
    pub execution_rate: f64,
}

/// Versioned 5-spoke evaluation scorecard (`EVAL-01`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensFiveSpokeScorecardV1 {
    /// Schema discriminator (`"mens_five_spoke_scorecard_v1"`).
    pub schema_version: String,
    /// Model revision identifier under test.
    pub model_id: String,
    /// Dataset snapshot Trusty URI (`trusty://...`).
    pub snapshot_uri: String,
    /// Adapter checkpoint Trusty URI (`trusty://blake3/...`).
    pub adapter_trusty_uri: String,
    /// `k` used for `pass@k`.
    pub k: usize,
    /// Per-spoke metrics in [`MensEvalSpoke::ALL`] order.
    pub spokes: Vec<SpokeEvalMetrics>,
    /// Macro-averaged `pass@1` across all 5 spokes.
    pub overall_pass_at_1: f64,
    /// Macro-averaged `pass@k` across all 5 spokes.
    pub overall_pass_at_k: f64,
    /// True when every spoke in [`MensEvalSpoke::ALL`] has `n_problems > 0`.
    pub all_five_spokes_covered: bool,
}

impl MensFiveSpokeScorecardV1 {
    /// Canonical schema discriminator.
    pub const SCHEMA_VERSION: &'static str = "mens_five_spoke_scorecard_v1";
}

/// Evaluate all 5 spokes (`VoxLang`, `RustWorkspace`, `StructuredShell`, `McpToolChain`, `AgentHarness`)
/// from a problem set and produce a [`MensFiveSpokeScorecardV1`] (`EVAL-01`).
#[must_use]
pub fn evaluate_mens_five_spokes(
    model_id: &str,
    snapshot_uri: &str,
    adapter_trusty_uri: &str,
    k: usize,
    problems: &[SpokeProblemInput],
) -> MensFiveSpokeScorecardV1 {
    let eff_k = k.max(1);
    let mut spokes = Vec::with_capacity(MensEvalSpoke::ALL.len());
    let mut all_covered = true;

    for spoke in MensEvalSpoke::ALL {
        let spoke_problems: Vec<&SpokeProblemInput> =
            problems.iter().filter(|p| p.spoke == spoke).collect();
        let n_problems = spoke_problems.len();
        if n_problems == 0 {
            all_covered = false;
            spokes.push(SpokeEvalMetrics {
                spoke,
                n_problems: 0,
                pass_at_1: 0.0,
                pass_at_k: 0.0,
                syntax_validity_rate: 0.0,
                semantic_validity_rate: 0.0,
                execution_rate: 0.0,
            });
            continue;
        }

        let mut p1_sum = 0.0;
        let mut pk_sum = 0.0;
        let mut total_attempts = 0usize;
        let mut syntax_ok = 0usize;
        let mut semantic_ok = 0usize;
        let mut exec_ok = 0usize;

        for prob in spoke_problems {
            let n = prob.attempts.len();
            let c = prob.attempts.iter().filter(|a| a.is_full_pass()).count();
            if n > 0 {
                p1_sum += pass_at_k(n, c, 1);
                pk_sum += pass_at_k(n, c, eff_k.min(n));
            }
            for att in &prob.attempts {
                total_attempts += 1;
                if att.syntax_valid {
                    syntax_ok += 1;
                }
                if att.semantic_valid {
                    semantic_ok += 1;
                }
                if att.execution_passed {
                    exec_ok += 1;
                }
            }
        }

        let denom_prob = n_problems as f64;
        let denom_att = total_attempts.max(1) as f64;
        spokes.push(SpokeEvalMetrics {
            spoke,
            n_problems,
            pass_at_1: p1_sum / denom_prob,
            pass_at_k: pk_sum / denom_prob,
            syntax_validity_rate: syntax_ok as f64 / denom_att,
            semantic_validity_rate: semantic_ok as f64 / denom_att,
            execution_rate: exec_ok as f64 / denom_att,
        });
    }

    let overall_pass_at_1 =
        spokes.iter().map(|s| s.pass_at_1).sum::<f64>() / MensEvalSpoke::ALL.len() as f64;
    let overall_pass_at_k =
        spokes.iter().map(|s| s.pass_at_k).sum::<f64>() / MensEvalSpoke::ALL.len() as f64;

    MensFiveSpokeScorecardV1 {
        schema_version: MensFiveSpokeScorecardV1::SCHEMA_VERSION.to_string(),
        model_id: model_id.to_string(),
        snapshot_uri: snapshot_uri.to_string(),
        adapter_trusty_uri: adapter_trusty_uri.to_string(),
        k: eff_k,
        spokes,
        overall_pass_at_1,
        overall_pass_at_k,
        all_five_spokes_covered: all_covered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_ok() {
        assert_eq!(summarize_placeholder(), CompileVerdict::Pass);
    }

    #[test]
    fn evaluate_mens_five_spokes_computes_per_spoke_rates_and_coverage() {
        let problems: Vec<SpokeProblemInput> = MensEvalSpoke::ALL
            .iter()
            .map(|&spoke| SpokeProblemInput {
                case_id: format!("case_{}", spoke.as_str()),
                spoke,
                attempts: vec![
                    SpokeAttemptOutcome {
                        case_id: format!("case_{}", spoke.as_str()),
                        spoke,
                        syntax_valid: true,
                        semantic_valid: true,
                        execution_passed: true,
                        diagnostic_summary: None,
                    },
                    SpokeAttemptOutcome {
                        case_id: format!("case_{}", spoke.as_str()),
                        spoke,
                        syntax_valid: true,
                        semantic_valid: false,
                        execution_passed: false,
                        diagnostic_summary: Some("type mismatch".to_string()),
                    },
                ],
            })
            .collect();

        let card = evaluate_mens_five_spokes(
            "mens-qwen35-cycle-1",
            "trusty://vox-dataset/snap-1",
            "trusty://blake3/adapter-1",
            2,
            &problems,
        );
        assert!(card.all_five_spokes_covered);
        assert_eq!(card.spokes.len(), 5);
        assert!((card.overall_pass_at_1 - 0.5).abs() < 1e-9);
        assert!((card.overall_pass_at_k - 1.0).abs() < 1e-9);
        for s in &card.spokes {
            assert!((s.syntax_validity_rate - 1.0).abs() < 1e-9);
            assert!((s.semantic_validity_rate - 0.5).abs() < 1e-9);
            assert!((s.execution_rate - 0.5).abs() < 1e-9);
        }
    }
}
