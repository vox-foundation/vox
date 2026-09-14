//! Multi-wave iterative deep research execution.
//!
//! Coordinates progressive research waves:
//! - Wave 1: Reconnaissance (broad coverage, initial claim extraction)
//! - Wave 2: Contradiction isolation & targeted deep-dives
//! - Wave 3: Adversarial validation & sandboxed empirical execution
//!
//! Enforces mathematical multi-factor stability stopping criteria:
//! S = 0.40 * (N_supported / N_total) + 0.30 * mean_resample_stability + 0.30 * (1.0 - N_unresolved / N_total)
//!
//! Early termination rule:
//! S >= 0.85 AND N_unresolved == 0 AND (N_supported / N_total) >= 0.60.

use serde::{Deserialize, Serialize};

use crate::research::domain::codegen::verify_rust_code_in_sandbox;
use crate::research::types::{ClaimVerdict, EvidenceSpan, SpanType, Verdict};

/// Classification of source or claim conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionCategory {
    DirectFactualOpposition,
    NumericDiscrepancy,
    TemporalVersionMismatch,
    ScopeOrPlatformDifference,
}

/// Resolution status of a detected contradiction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionStatus {
    Unresolved,
    ResolvedByAuthority {
        winning_claim_id: u64,
        authoritative_url: String,
        rationale: String,
    },
    ResolvedByEmpiricalSandbox {
        winning_claim_id: u64,
        compiler_stdout: String,
    },
    DismissedAsContextDifference {
        explanation: String,
    },
    IrreconcilableDispute,
}

/// A structured record of contradictory findings across sources or claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionRecord {
    pub contradiction_id: u64,
    pub claim_id_a: u64,
    pub claim_text_a: String,
    pub source_url_a: String,
    pub claim_id_b: u64,
    pub claim_text_b: String,
    pub source_url_b: String,
    pub category: ContradictionCategory,
    pub severity: f64,
    pub status: ContradictionStatus,
    pub disambiguation_query: Option<String>,
}

/// Reason for research wave execution termination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminationReason {
    StabilityThresholdMet,
    MaxWavesReached,
    BudgetExhausted,
    StagnantContradiction,
}

/// State and tracker for multi-wave deep research.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveExecutionPlan {
    pub session_id: String,
    pub max_waves: usize,
    pub current_wave: usize,
    pub stability_threshold: f64,
    pub unresolved_contradictions: Vec<ContradictionRecord>,
    pub resolved_verdicts: Vec<ClaimVerdict>,
    pub max_sandbox_runs: usize,
    pub sandbox_runs_performed: usize,
    pub termination_reason: Option<TerminationReason>,
}

impl WaveExecutionPlan {
    pub fn new(session_id: impl Into<String>, max_waves: usize) -> Self {
        Self {
            session_id: session_id.into(),
            max_waves: max_waves.max(1),
            current_wave: 1,
            stability_threshold: 0.85,
            unresolved_contradictions: Vec::new(),
            resolved_verdicts: Vec::new(),
            max_sandbox_runs: 5,
            sandbox_runs_performed: 0,
            termination_reason: None,
        }
    }

    /// Computes multi-factor stability metric:
    /// S = 0.40 * (N_supported / N_total) + 0.30 * mean_resample_stability + 0.30 * (1.0 - N_unresolved / N_total)
    pub fn compute_stability(&self) -> f64 {
        let total = self.resolved_verdicts.len();
        if total == 0 {
            return 0.0;
        }

        let supported_count = self
            .resolved_verdicts
            .iter()
            .filter(|v| v.verdict == Verdict::Supported)
            .count();
        let supported_ratio = supported_count as f64 / total as f64;

        let mean_resample_stability = self
            .resolved_verdicts
            .iter()
            .map(|v| v.resample_stability)
            .sum::<f64>()
            / total as f64;

        let unresolved_count = self
            .unresolved_contradictions
            .iter()
            .filter(|c| matches!(c.status, ContradictionStatus::Unresolved))
            .count();
        let contradiction_penalty = (unresolved_count as f64 / total as f64).min(1.0);
        let contradiction_term = (1.0 - contradiction_penalty).clamp(0.0, 1.0);

        (0.40 * supported_ratio + 0.30 * mean_resample_stability + 0.30 * contradiction_term)
            .clamp(0.0, 1.0)
    }

    /// Evaluates early exit condition:
    /// S >= 0.85 AND N_unresolved == 0 AND (N_supported / N_total) >= 0.60.
    pub fn should_early_terminate(&self) -> bool {
        let total = self.resolved_verdicts.len();
        if total == 0 {
            return false;
        }

        let supported_count = self
            .resolved_verdicts
            .iter()
            .filter(|v| v.verdict == Verdict::Supported)
            .count();
        let supported_ratio = supported_count as f64 / total as f64;

        let unresolved_count = self
            .unresolved_contradictions
            .iter()
            .filter(|c| matches!(c.status, ContradictionStatus::Unresolved))
            .count();

        let s = self.compute_stability();
        s >= self.stability_threshold && unresolved_count == 0 && supported_ratio >= 0.60
    }

    /// Detects contradictions among a set of verdicts and extracts disambiguation queries for Wave 2.
    pub fn detect_contradictions(&mut self, verdicts: &[ClaimVerdict]) {
        self.resolved_verdicts = verdicts.to_vec();
        let mut new_records = Vec::new();

        for i in 0..verdicts.len() {
            for j in (i + 1)..verdicts.len() {
                let v_a = &verdicts[i];
                let v_b = &verdicts[j];

                // Direct opposition: one supported, one contradicted on overlapping topic
                let words_a: std::collections::HashSet<_> = v_a
                    .claim
                    .text
                    .split_whitespace()
                    .map(|w| w.to_ascii_lowercase())
                    .filter(|w| w.len() > 3)
                    .collect();
                let words_b: std::collections::HashSet<_> = v_b
                    .claim
                    .text
                    .split_whitespace()
                    .map(|w| w.to_ascii_lowercase())
                    .filter(|w| w.len() > 3)
                    .collect();

                let common = words_a.intersection(&words_b).count();
                if common >= 2 {
                    let is_opposite = (v_a.verdict == Verdict::Supported
                        && v_b.verdict == Verdict::Contradicted)
                        || (v_a.verdict == Verdict::Contradicted
                            && v_b.verdict == Verdict::Supported);

                    let is_numeric_mismatch = v_a.claim.is_numeric
                        && v_b.claim.is_numeric
                        && v_a.claim.text != v_b.claim.text;

                    if is_opposite || is_numeric_mismatch {
                        let category = if is_numeric_mismatch {
                            ContradictionCategory::NumericDiscrepancy
                        } else if v_a.claim.is_recent || v_b.claim.is_recent {
                            ContradictionCategory::TemporalVersionMismatch
                        } else {
                            ContradictionCategory::DirectFactualOpposition
                        };

                        let id = ((v_a.claim.claim_id as u64) << 32) | (v_b.claim.claim_id as u64);
                        let disambiguation_query = Some(format!(
                            "{} vs {} difference verification truth",
                            v_a.claim.text, v_b.claim.text
                        ));

                        new_records.push(ContradictionRecord {
                            contradiction_id: id,
                            claim_id_a: v_a.claim.claim_id,
                            claim_text_a: v_a.claim.text.clone(),
                            source_url_a: v_a
                                .evidence_spans
                                .first()
                                .map(|s| format!("source-{}", s.source_id))
                                .unwrap_or_default(),
                            claim_id_b: v_b.claim.claim_id,
                            claim_text_b: v_b.claim.text.clone(),
                            source_url_b: v_b
                                .evidence_spans
                                .first()
                                .map(|s| format!("source-{}", s.source_id))
                                .unwrap_or_default(),
                            category,
                            severity: 0.85,
                            status: ContradictionStatus::Unresolved,
                            disambiguation_query,
                        });
                    }
                }
            }
        }

        self.unresolved_contradictions = new_records;
    }

    /// Returns subqueries needed for Wave 2 disambiguation.
    pub fn generate_disambiguation_subqueries(&self) -> Vec<String> {
        self.unresolved_contradictions
            .iter()
            .filter(|c| matches!(c.status, ContradictionStatus::Unresolved))
            .filter_map(|c| c.disambiguation_query.clone())
            .take(4)
            .collect()
    }

    /// Compiler Sandbox Overrule Hierarchy for CodeGen research.
    ///
    /// If `rustc` fails, it unconditionally overrules any LLM verdict:
    /// - Marks claim as `Verdict::Contradicted`
    /// - Locks confidence to `1.00`
    /// - Injects compiler stderr into evidence spans
    pub fn apply_sandbox_verdict_overrule(
        &mut self,
        claim_id: u64,
        passed: bool,
        stderr: &str,
        stdout: &str,
    ) {
        self.sandbox_runs_performed += 1;

        if let Some(verdict) = self
            .resolved_verdicts
            .iter_mut()
            .find(|v| v.claim.claim_id == claim_id)
        {
            if !passed {
                verdict.verdict = Verdict::Contradicted;
                verdict.confidence = 1.0;
                verdict.contradicting_count += 1;
                verdict.evidence_spans.push(EvidenceSpan {
                    source_id: 999999, // Canonical synthetic sandbox source
                    span_start: 0,
                    span_end: stderr.len(),
                    text: format!("Compiler execution failed (empirical overrule): {stderr}"),
                    span_type: SpanType::Contradicting,
                });
            } else {
                verdict.verdict = Verdict::Supported;
                verdict.confidence = 1.0;
                verdict.supporting_count += 1;
            }
        }

        // Update corresponding contradiction status if present
        for c in &mut self.unresolved_contradictions {
            if c.claim_id_a == claim_id || c.claim_id_b == claim_id {
                let winning_id = if passed {
                    claim_id
                } else if c.claim_id_a == claim_id {
                    c.claim_id_b
                } else {
                    c.claim_id_a
                };
                let rationale = if passed {
                    format!("Empirical compilation succeeded: {stdout}")
                } else {
                    format!("Claim {claim_id} failed compilation: {stderr}")
                };
                c.status = ContradictionStatus::ResolvedByEmpiricalSandbox {
                    winning_claim_id: winning_id,
                    compiler_stdout: rationale,
                };
            }
        }
    }

    /// Applies the outcome of a code self-correction attempt to claim verdicts and matching contradictions.
    pub fn apply_sandbox_self_correction(
        &mut self,
        claim_id: u64,
        correction: &crate::research::domain::codegen::CodeSelfCorrectionResult,
    ) {
        self.sandbox_runs_performed += 1;

        if let Some(verdict) = self
            .resolved_verdicts
            .iter_mut()
            .find(|v| v.claim.claim_id == claim_id)
        {
            if correction.final_passed {
                verdict.verdict = Verdict::Supported;
                verdict.confidence = 1.0;
                verdict.supporting_count += 1;
                let note = if correction.iterations > 0 {
                    format!(
                        "Self-corrected via compiler sandbox (iteration {}): {}\nRepaired Code:\n{}\nOriginal Error: {}",
                        correction.iterations,
                        correction
                            .repair_explanation
                            .as_deref()
                            .unwrap_or("syntax/type auto-fix"),
                        correction.corrected_code.as_deref().unwrap_or_default(),
                        correction.initial_error.as_deref().unwrap_or_default(),
                    )
                } else {
                    format!(
                        "Empirically verified via compiler sandbox:\n{}",
                        correction.corrected_code.as_deref().unwrap_or_default()
                    )
                };
                verdict.evidence_spans.push(EvidenceSpan {
                    source_id: 999999,
                    span_start: 0,
                    span_end: note.len(),
                    text: note,
                    span_type: SpanType::Supporting,
                });
            } else {
                let err_msg = correction
                    .final_error
                    .as_deref()
                    .or(correction.initial_error.as_deref())
                    .unwrap_or("Compiler execution failed");
                let is_harness_artifact = err_msg.contains("E0432")
                    || err_msg.contains("E0433")
                    || err_msg.contains("cannot find function `main`");
                if is_harness_artifact {
                    verdict.verdict = Verdict::Unverified;
                    verdict.confidence = 0.50;
                } else {
                    verdict.verdict = Verdict::Contradicted;
                    verdict.confidence = 1.0;
                    verdict.contradicting_count += 1;
                    verdict.evidence_spans.push(EvidenceSpan {
                        source_id: 999999,
                        span_start: 0,
                        span_end: err_msg.len(),
                        text: format!("Compiler execution failed (empirical overrule): {err_msg}"),
                        span_type: SpanType::Contradicting,
                    });
                }
            }
        }

        // Update contradictions matching this claim_id so early exit does not deadlock
        for c in &mut self.unresolved_contradictions {
            if c.claim_id_a == claim_id || c.claim_id_b == claim_id {
                let winning_id = if correction.final_passed {
                    claim_id
                } else if c.claim_id_a == claim_id {
                    c.claim_id_b
                } else {
                    c.claim_id_a
                };
                let rationale = if correction.final_passed {
                    format!(
                        "Empirical compilation succeeded: {}",
                        correction.corrected_code.as_deref().unwrap_or_default()
                    )
                } else {
                    format!(
                        "Compilation failed: {}",
                        correction.final_error.as_deref().unwrap_or("error")
                    )
                };
                c.status = ContradictionStatus::ResolvedByEmpiricalSandbox {
                    winning_claim_id: winning_id,
                    compiler_stdout: rationale,
                };
            }
        }
    }

    /// Executes sandboxed compilation for a CodeGen claim if budget allows.
    pub async fn verify_claim_in_sandbox(
        &mut self,
        claim_id: u64,
        code_snippet: &str,
        dependencies: &[&str],
    ) -> anyhow::Result<bool> {
        if self.sandbox_runs_performed >= self.max_sandbox_runs {
            return Ok(false);
        }

        let res = verify_rust_code_in_sandbox(code_snippet, dependencies).await?;
        self.apply_sandbox_verdict_overrule(claim_id, res.passed, &res.stderr, &res.stdout);
        Ok(res.passed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research::claims::Claim;

    #[test]
    fn test_multi_factor_stability_and_early_exit() {
        let mut plan = WaveExecutionPlan::new("test-session-1", 3);

        // Initially no verdicts -> stability 0.0, no early exit
        assert_eq!(plan.compute_stability(), 0.0);
        assert!(!plan.should_early_terminate());

        // Add 5 supported verdicts with high resample stability (1.0) and 0 contradictions
        plan.resolved_verdicts = (1..=5)
            .map(|id| ClaimVerdict {
                claim: Claim {
                    claim_id: id,
                    text: format!("Claim {id} is valid."),
                    is_numeric: false,
                    is_recent: false,
                    is_named_event: false,
                },
                verdict: Verdict::Supported,
                confidence: 0.95,
                supporting_count: 3,
                contradicting_count: 0,
                evidence_spans: vec![],
                resample_stability: 1.0,
            })
            .collect();

        // S = 0.40*(5/5) + 0.30*(1.0) + 0.30*(1.0 - 0) = 0.40 + 0.30 + 0.30 = 1.0
        let s = plan.compute_stability();
        assert!((s - 1.0).abs() < 1e-6, "Stability should be 1.0, got {s}");
        assert!(
            plan.should_early_terminate(),
            "Early exit condition must be met"
        );

        // Introduce an unresolved contradiction
        plan.unresolved_contradictions.push(ContradictionRecord {
            contradiction_id: 100,
            claim_id_a: 1,
            claim_text_a: "Claim 1".into(),
            source_url_a: "url1".into(),
            claim_id_b: 2,
            claim_text_b: "Claim 2".into(),
            source_url_b: "url2".into(),
            category: ContradictionCategory::DirectFactualOpposition,
            severity: 0.9,
            status: ContradictionStatus::Unresolved,
            disambiguation_query: Some("Claim 1 vs Claim 2".into()),
        });

        // Unresolved contradiction must immediately block early exit!
        assert!(
            !plan.should_early_terminate(),
            "Unresolved contradictions must block early termination"
        );
    }

    #[test]
    fn test_compiler_sandbox_overrule_hierarchy() {
        let mut plan = WaveExecutionPlan::new("codegen-session-2", 3);
        let claim_id = 77;

        plan.resolved_verdicts.push(ClaimVerdict {
            claim: Claim {
                claim_id,
                text: "fn parse(s: &str) -> i32 is valid".into(),
                is_numeric: false,
                is_recent: false,
                is_named_event: false,
            },
            verdict: Verdict::Supported, // LLM hallucinated that broken code is supported
            confidence: 0.88,
            supporting_count: 1,
            contradicting_count: 0,
            evidence_spans: vec![],
            resample_stability: 0.9,
        });

        // Compiler fails on this code snippet
        plan.apply_sandbox_verdict_overrule(claim_id, false, "error[E0308]: mismatched types", "");

        let v = plan.resolved_verdicts.first().unwrap();
        assert_eq!(
            v.verdict,
            Verdict::Contradicted,
            "Compiler error must overrule LLM and mark Contradicted"
        );
        assert_eq!(
            v.confidence, 1.0,
            "Compiler overrule confidence must be locked to 1.0"
        );
        assert!(
            v.evidence_spans
                .iter()
                .any(|s| s.text.contains("mismatched types")),
            "Compiler stderr must be recorded in evidence spans"
        );
    }
}
