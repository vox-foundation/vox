//! Composite confidence fusion for Socrates invocation decision (D3).
//!
//! Blends five evidence signals into a single score, then applies thresholds
//! from [`FusionConfig`] to decide whether to invoke Socrates or answer directly.
//! Thresholds mirror `contracts/orchestration/socrates-fusion.v1.yaml`.
//! All checks are pure: no async, no I/O.

use serde::{Deserialize, Serialize};

use crate::socrates::SocratesTaskContext;

/// Weights applied to each evidence signal. Must sum to 1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionWeights {
    pub evidence_quality: f64,
    pub citation_coverage: f64,
    pub source_diversity: f64,
    pub contradiction_penalty: f64,
    pub entropy_score: f64,
}

impl Default for FusionWeights {
    fn default() -> Self {
        Self {
            evidence_quality: 0.35,
            citation_coverage: 0.25,
            source_diversity: 0.15,
            contradiction_penalty: 0.15,
            entropy_score: 0.10,
        }
    }
}

/// Raw signal values extracted before weighting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionInputs {
    /// Retrieval-side evidence quality proxy in `[0, 1]`.
    pub evidence_quality: f64,
    /// Retrieval-side citation coverage proxy in `[0, 1]`.
    pub citation_coverage: f64,
    /// Number of distinct source corpora (normalized to `[0, 1]` via saturation at 5).
    pub source_diversity_norm: f64,
    /// Contradiction mass in `[0, 1]`; inverted before weighting.
    pub contradiction_ratio: f64,
    /// Entropy-derived confidence in `[0, 1]` from the most recent completion text.
    pub entropy_score: f64,
}

impl FusionInputs {
    /// Build inputs from a [`SocratesTaskContext`] and an optional completion text entropy.
    #[must_use]
    pub fn from_task_context(ctx: &SocratesTaskContext, completion_entropy_score: Option<f64>) -> Self {
        let contradiction_ratio = match ctx.contradiction_hints {
            0 => 0.0,
            1 => 0.15,
            2 => 0.28,
            n => ((n as f64) * 0.22).min(1.0),
        };
        let source_diversity_norm = (ctx.source_diversity as f64 / 5.0).clamp(0.0, 1.0);
        Self {
            evidence_quality: ctx.evidence_quality.clamp(0.0, 1.0),
            citation_coverage: ctx.citation_coverage.clamp(0.0, 1.0),
            source_diversity_norm,
            contradiction_ratio,
            entropy_score: completion_entropy_score.unwrap_or(0.5).clamp(0.0, 1.0),
        }
    }
}

/// Outcome of the fusion step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FusionDecision {
    /// Score ≥ `answer_threshold`; confidence sufficient to answer directly.
    AnswerDirectly,
    /// Score in `[invoke_threshold, answer_threshold)`; invoke Socrates to strengthen evidence.
    InvokeSocrates,
    /// Score < `invoke_threshold`; insufficient confidence, block or escalate.
    Insufficient,
}

impl std::fmt::Display for FusionDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AnswerDirectly => write!(f, "answer-directly"),
            Self::InvokeSocrates => write!(f, "invoke-socrates"),
            Self::Insufficient => write!(f, "insufficient"),
        }
    }
}

/// Threshold configuration. Defaults mirror contract defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    /// Score ≥ this → answer directly.
    pub answer_threshold: f64,
    /// Score in `[invoke_threshold, answer_threshold)` → invoke Socrates.
    pub invoke_threshold: f64,
    pub weights: FusionWeights,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            answer_threshold: 0.75,
            invoke_threshold: 0.55,
            weights: FusionWeights::default(),
        }
    }
}

/// Pure, allocation-free confidence fuser.
pub struct ConfidenceFuser {
    config: FusionConfig,
}

impl ConfidenceFuser {
    pub fn new(config: FusionConfig) -> Self {
        Self { config }
    }

    /// Compute the composite confidence score from pre-computed inputs.
    ///
    /// Score = Σ(signal_i × weight_i) where contradiction is inverted first.
    #[must_use]
    #[inline]
    pub fn score(&self, inputs: &FusionInputs) -> f64 {
        let w = &self.config.weights;
        let contradiction_contribution = (1.0 - inputs.contradiction_ratio) * w.contradiction_penalty;
        (inputs.evidence_quality * w.evidence_quality
            + inputs.citation_coverage * w.citation_coverage
            + inputs.source_diversity_norm * w.source_diversity
            + contradiction_contribution
            + inputs.entropy_score * w.entropy_score)
            .clamp(0.0, 1.0)
    }

    /// Apply thresholds to a pre-computed score and return the routing decision.
    #[must_use]
    #[inline]
    pub fn decide(&self, score: f64) -> FusionDecision {
        if score >= self.config.answer_threshold {
            FusionDecision::AnswerDirectly
        } else if score >= self.config.invoke_threshold {
            FusionDecision::InvokeSocrates
        } else {
            FusionDecision::Insufficient
        }
    }

    /// Convenience: score + decide in one call.
    #[must_use]
    #[inline]
    pub fn evaluate(&self, inputs: &FusionInputs) -> (f64, FusionDecision) {
        let score = self.score(inputs);
        (score, self.decide(score))
    }
}

/// Metric payload emitted when a fusion decision is made.
/// Serialize-only — see `TripEvent` for rationale on the missing `Deserialize`.
#[derive(Debug, Clone, Serialize)]
pub struct FusionEvent {
    pub metric_type: &'static str,
    pub score: f64,
    pub decision: String,
    pub session_id: Option<String>,
}

impl FusionEvent {
    pub fn new(score: f64, decision: FusionDecision, session_id: Option<String>) -> Self {
        Self {
            metric_type: vox_db::research_metrics_contract::METRIC_TYPE_SOCRATES_FUSION,
            score,
            decision: decision.to_string(),
            session_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fuser() -> ConfidenceFuser {
        ConfidenceFuser::new(FusionConfig::default())
    }

    fn high_quality_inputs() -> FusionInputs {
        FusionInputs {
            evidence_quality: 0.9,
            citation_coverage: 0.85,
            source_diversity_norm: 0.8,
            contradiction_ratio: 0.0,
            entropy_score: 0.8,
        }
    }

    fn low_quality_inputs() -> FusionInputs {
        FusionInputs {
            evidence_quality: 0.1,
            citation_coverage: 0.1,
            source_diversity_norm: 0.0,
            contradiction_ratio: 0.8,
            entropy_score: 0.2,
        }
    }

    #[test]
    fn high_quality_answers_directly() {
        let f = fuser();
        let (_, decision) = f.evaluate(&high_quality_inputs());
        assert_eq!(decision, FusionDecision::AnswerDirectly);
    }

    #[test]
    fn low_quality_is_insufficient() {
        let f = fuser();
        let (_, decision) = f.evaluate(&low_quality_inputs());
        assert_eq!(decision, FusionDecision::Insufficient);
    }

    #[test]
    fn mid_quality_invokes_socrates() {
        let f = fuser();
        let inputs = FusionInputs {
            evidence_quality: 0.6,
            citation_coverage: 0.5,
            source_diversity_norm: 0.4,
            contradiction_ratio: 0.1,
            entropy_score: 0.6,
        };
        let (_, decision) = f.evaluate(&inputs);
        assert_eq!(decision, FusionDecision::InvokeSocrates);
    }

    #[test]
    fn contradiction_penalty_lowers_score() {
        let f = fuser();
        let base = FusionInputs {
            evidence_quality: 0.8,
            citation_coverage: 0.8,
            source_diversity_norm: 0.6,
            contradiction_ratio: 0.0,
            entropy_score: 0.7,
        };
        let contradicted = FusionInputs {
            contradiction_ratio: 0.8,
            ..base.clone()
        };
        assert!(f.score(&base) > f.score(&contradicted));
    }

    #[test]
    fn score_clamped_between_zero_and_one() {
        let f = fuser();
        let extreme = FusionInputs {
            evidence_quality: 2.0,
            citation_coverage: 2.0,
            source_diversity_norm: 2.0,
            contradiction_ratio: -1.0,
            entropy_score: 2.0,
        };
        let score = f.score(&extreme);
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn from_task_context_maps_fields() {
        let ctx = SocratesTaskContext {
            evidence_quality: 0.7,
            citation_coverage: 0.6,
            source_diversity: 3,
            contradiction_hints: 1,
            ..Default::default()
        };
        let inputs = FusionInputs::from_task_context(&ctx, Some(0.65));
        assert!((inputs.evidence_quality - 0.7).abs() < 1e-9);
        assert!((inputs.citation_coverage - 0.6).abs() < 1e-9);
        assert!((inputs.source_diversity_norm - 0.6).abs() < 1e-9);
        assert!((inputs.contradiction_ratio - 0.15).abs() < 1e-9);
        assert!((inputs.entropy_score - 0.65).abs() < 1e-9);
    }

    #[test]
    fn fusion_event_has_correct_metric_type() {
        let event = FusionEvent::new(0.8, FusionDecision::AnswerDirectly, None);
        assert_eq!(event.metric_type, "orch.socrates.fusion");
    }

    #[test]
    fn decide_at_exact_answer_threshold() {
        let f = fuser();
        assert_eq!(f.decide(0.75), FusionDecision::AnswerDirectly);
    }

    #[test]
    fn decide_just_below_answer_threshold() {
        let f = fuser();
        assert_eq!(f.decide(0.74), FusionDecision::InvokeSocrates);
    }

    #[test]
    fn decide_at_exact_invoke_threshold() {
        let f = fuser();
        assert_eq!(f.decide(0.55), FusionDecision::InvokeSocrates);
    }

    #[test]
    fn decide_just_below_invoke_threshold() {
        let f = fuser();
        assert_eq!(f.decide(0.54), FusionDecision::Insufficient);
    }
}
