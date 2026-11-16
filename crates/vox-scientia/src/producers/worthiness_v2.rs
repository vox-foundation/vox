//! Two-stage Novelty x Utility worthiness gate (`DISC-04`).
//!
//! Enforces BOTH novelty clearance (`Novel | PossiblyNovel`) AND verified utility
//! proof (`empirical_delta | ci_improvement | reproducibility_proof | regression_fix`
//! + `claim_grounding >= 0.70`) before surfacing as `strong_candidate`.

use crate::inspect_bridge::novelty::NoveltyVerdict;
use serde::{Deserialize, Serialize};
use vox_research_events::schema_types::{
    WorthinessActionItem, WorthinessProfile, WorthinessSignalItem,
    WorthinessSignalsV2 as ContractWorthinessSignalsV2,
};

/// Minimum claim grounding ratio required by the utility gate.
pub const MIN_CLAIM_GROUNDING: f64 = 0.70;

/// Minimum utility score required before surfacing to `scientia_discovery_inbox`.
pub const MIN_UTILITY_SCORE: f64 = 0.70;

/// Two-stage Novelty x Utility signals for a discovery candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct WorthinessSignalsV2 {
    pub novelty_verdict: NoveltyVerdict,
    pub novelty_score: f64,
    pub empirical_delta: bool,
    pub ci_improvement: bool,
    pub reproducibility_proof: bool,
    pub regression_fix: bool,
    pub claim_grounding: f64,
    pub profile: WorthinessProfile,
}

impl Default for WorthinessSignalsV2 {
    fn default() -> Self {
        Self {
            novelty_verdict: NoveltyVerdict::InsufficientEvidence {
                reason: "novelty check not run".into(),
            },
            novelty_score: 0.0,
            empirical_delta: false,
            ci_improvement: false,
            reproducibility_proof: false,
            regression_fix: false,
            claim_grounding: 0.0,
            profile: WorthinessProfile::Repository,
        }
    }
}

/// Intake decision produced by the two-stage Novelty x Utility gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorthinessTierV2 {
    StrongCandidate,
    HoldInLedger,
}

impl WorthinessTierV2 {
    /// Canonical `scientia_discovery_inbox` intake tier string.
    #[must_use]
    pub const fn as_intake_tier_str(self) -> &'static str {
        match self {
            Self::StrongCandidate => "strong_candidate",
            Self::HoldInLedger => "low_signal",
        }
    }
}

/// Full evaluation breakdown from `evaluate_worthiness_v2`.
#[derive(Debug, Clone, PartialEq)]
pub struct WorthinessEvaluationV2 {
    pub novelty_passed: bool,
    pub utility_passed: bool,
    pub novelty_score: f64,
    pub utility_score: f64,
    pub composite_score: f64,
    pub tier: WorthinessTierV2,
    pub gate_reasons: Vec<String>,
    pub contract_signals: ContractWorthinessSignalsV2,
}

impl WorthinessEvaluationV2 {
    /// Whether the candidate should be surfaced to `scientia_discovery_inbox`.
    #[must_use]
    pub fn should_surface_to_inbox(&self) -> bool {
        self.tier == WorthinessTierV2::StrongCandidate
    }
}

fn clean_unit(v: f64) -> f64 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Evaluate two-stage Novelty x Utility worthiness gate.
///
/// Requires BOTH:
/// 1. Novelty clearance (`Novel` or `PossiblyNovel` with `novelty_score > 0.0`)
/// 2. Verified utility proof (at least one concrete utility proof AND
///    `claim_grounding >= 0.70`, yielding `utility_score >= 0.70`)
#[must_use]
pub fn evaluate_worthiness_v2(signals: &WorthinessSignalsV2) -> WorthinessEvaluationV2 {
    let novelty_score = match &signals.novelty_verdict {
        NoveltyVerdict::Novel => clean_unit(signals.novelty_score),
        NoveltyVerdict::PossiblyNovel { closest_score } => {
            let inferred = clean_unit(1.0 - clean_unit(*closest_score));
            if signals.novelty_score > 0.0 {
                clean_unit(signals.novelty_score)
            } else {
                inferred
            }
        }
        NoveltyVerdict::NotNovel { .. }
        | NoveltyVerdict::InsufficientEvidence { .. }
        | NoveltyVerdict::Contradicted { .. } => 0.0,
    };

    let novelty_passed = matches!(
        signals.novelty_verdict,
        NoveltyVerdict::Novel | NoveltyVerdict::PossiblyNovel { .. }
    ) && novelty_score > 0.0;

    let grounding = clean_unit(signals.claim_grounding);
    let proof_count = usize::from(signals.empirical_delta)
        + usize::from(signals.ci_improvement)
        + usize::from(signals.reproducibility_proof)
        + usize::from(signals.regression_fix);

    let proof_tier = match proof_count {
        0 => 0.0,
        1 => 0.74,
        2 => 0.86,
        3 => 0.94,
        _ => 1.0,
    };

    let utility_score = if proof_count > 0 {
        clean_unit(0.65 * proof_tier + 0.35 * grounding)
    } else {
        clean_unit(0.50 * grounding)
    };

    let grounding_passed = grounding >= MIN_CLAIM_GROUNDING;
    let utility_passed = proof_count > 0 && grounding_passed && utility_score >= MIN_UTILITY_SCORE;

    let composite_score = if novelty_passed && utility_passed {
        clean_unit(novelty_score * utility_score)
    } else {
        0.0
    };

    let tier = if novelty_passed && utility_passed && composite_score > 0.0 {
        WorthinessTierV2::StrongCandidate
    } else {
        WorthinessTierV2::HoldInLedger
    };

    let mut gate_reasons = Vec::new();
    let novelty_reason = match &signals.novelty_verdict {
        NoveltyVerdict::Novel => "novelty_cleared",
        NoveltyVerdict::PossiblyNovel { .. } => "novelty_possibly_novel",
        NoveltyVerdict::NotNovel { .. } => "prior_art_not_novel",
        NoveltyVerdict::InsufficientEvidence { .. } => "novelty_insufficient_evidence",
        NoveltyVerdict::Contradicted { .. } => "novelty_contradicted",
    };
    if !novelty_passed {
        gate_reasons.push(novelty_reason.to_string());
    }
    if proof_count == 0 {
        gate_reasons.push("missing_utility_proof".to_string());
    }
    if !grounding_passed {
        gate_reasons.push("claim_grounding_below_0_70".to_string());
    }

    let mut next_actions = Vec::new();
    if !novelty_passed {
        next_actions.push(WorthinessActionItem {
            id: "act-novelty".to_string(),
            priority: 1,
            action: "Resolve prior-art overlap or gather retrieval evidence".to_string(),
        });
    }
    if proof_count == 0 {
        next_actions.push(WorthinessActionItem {
            id: "act-utility-proof".to_string(),
            priority: 2,
            action: "Attach verified empirical delta, CI gain, repro proof, or regression fix"
                .to_string(),
        });
    }
    if !grounding_passed {
        next_actions.push(WorthinessActionItem {
            id: "act-grounding".to_string(),
            priority: 3,
            action: "Raise claim grounding verification to >= 0.70".to_string(),
        });
    }

    let contract_signals = ContractWorthinessSignalsV2 {
        version: "2".to_string(),
        profile: signals.profile.clone(),
        hard_gate: vec![
            WorthinessSignalItem {
                id: "hg-novelty-clearance".to_string(),
                passed: novelty_passed,
                score: novelty_score,
                reason_code: novelty_reason.to_string(),
                details: None,
            },
            WorthinessSignalItem {
                id: "hg-utility-proof".to_string(),
                passed: utility_passed,
                score: utility_score,
                reason_code: if utility_passed {
                    "utility_verified".to_string()
                } else if proof_count == 0 {
                    "missing_utility_proof".to_string()
                } else {
                    "claim_grounding_below_0_70".to_string()
                },
                details: Some(format!(
                    "proof_axes={proof_count},claim_grounding={grounding:.2}"
                )),
            },
        ],
        soft_gate: vec![WorthinessSignalItem {
            id: "sg-novelty-x-utility".to_string(),
            passed: tier == WorthinessTierV2::StrongCandidate,
            score: composite_score,
            reason_code: tier.as_intake_tier_str().to_string(),
            details: None,
        }],
        diagnostic: vec![WorthinessSignalItem {
            id: "diag-grounding".to_string(),
            passed: grounding_passed,
            score: grounding,
            reason_code: "claim_grounding".to_string(),
            details: None,
        }],
        next_actions,
    };

    WorthinessEvaluationV2 {
        novelty_passed,
        utility_passed,
        novelty_score,
        utility_score,
        composite_score,
        tier,
        gate_reasons,
        contract_signals,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_novelty_low_utility_is_held_in_ledger() {
        let eval = evaluate_worthiness_v2(&WorthinessSignalsV2 {
            novelty_verdict: NoveltyVerdict::Novel,
            novelty_score: 0.92,
            empirical_delta: false,
            ci_improvement: false,
            reproducibility_proof: false,
            regression_fix: false,
            claim_grounding: 0.85,
            profile: WorthinessProfile::Repository,
        });
        assert!(eval.novelty_passed);
        assert!(!eval.utility_passed);
        assert_eq!(eval.tier, WorthinessTierV2::HoldInLedger);
        assert!(!eval.should_surface_to_inbox());
        assert_eq!(eval.composite_score, 0.0);
    }

    #[test]
    fn failed_novelty_high_utility_is_held_in_ledger() {
        let eval = evaluate_worthiness_v2(&WorthinessSignalsV2 {
            novelty_verdict: NoveltyVerdict::NotNovel {
                closest_hit_uri: "https://openalex.org/W1".into(),
                similarity: 0.91,
            },
            novelty_score: 0.09,
            empirical_delta: true,
            ci_improvement: true,
            reproducibility_proof: true,
            regression_fix: false,
            claim_grounding: 0.95,
            profile: WorthinessProfile::Repository,
        });
        assert!(!eval.novelty_passed);
        assert!(eval.utility_passed);
        assert_eq!(eval.tier, WorthinessTierV2::HoldInLedger);
        assert!(!eval.should_surface_to_inbox());
    }

    #[test]
    fn ungrounded_utility_below_0_70_is_held() {
        let eval = evaluate_worthiness_v2(&WorthinessSignalsV2 {
            novelty_verdict: NoveltyVerdict::Novel,
            novelty_score: 0.88,
            empirical_delta: true,
            ci_improvement: false,
            reproducibility_proof: false,
            regression_fix: false,
            claim_grounding: 0.55,
            profile: WorthinessProfile::Repository,
        });
        assert!(eval.novelty_passed);
        assert!(!eval.utility_passed);
        assert_eq!(eval.tier, WorthinessTierV2::HoldInLedger);
    }

    #[test]
    fn novelty_and_utility_both_pass_surfaces_strong_candidate() {
        let eval = evaluate_worthiness_v2(&WorthinessSignalsV2 {
            novelty_verdict: NoveltyVerdict::Novel,
            novelty_score: 0.90,
            empirical_delta: true,
            ci_improvement: true,
            reproducibility_proof: false,
            regression_fix: false,
            claim_grounding: 0.82,
            profile: WorthinessProfile::Preprint,
        });
        assert!(eval.novelty_passed);
        assert!(eval.utility_passed);
        assert_eq!(eval.tier, WorthinessTierV2::StrongCandidate);
        assert_eq!(eval.tier.as_intake_tier_str(), "strong_candidate");
        assert!(eval.should_surface_to_inbox());
        assert!(eval.utility_score >= 0.80);
        assert!((eval.composite_score - eval.novelty_score * eval.utility_score).abs() < 1e-9);
    }

    #[test]
    fn possibly_novel_with_verified_regression_fix_passes() {
        let eval = evaluate_worthiness_v2(&WorthinessSignalsV2 {
            novelty_verdict: NoveltyVerdict::PossiblyNovel {
                closest_score: 0.58,
            },
            novelty_score: 0.0,
            empirical_delta: false,
            ci_improvement: false,
            reproducibility_proof: false,
            regression_fix: true,
            claim_grounding: 0.72,
            profile: WorthinessProfile::Repository,
        });
        assert!(eval.novelty_passed);
        assert!(eval.utility_passed);
        assert_eq!(eval.tier, WorthinessTierV2::StrongCandidate);
        assert!((eval.novelty_score - 0.42).abs() < 1e-9);
    }
}
