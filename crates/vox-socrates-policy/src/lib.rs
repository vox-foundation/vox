//! Shared **Socrates** confidence policy for orchestrator, MCP, and TOESTUB review.
//!
//! Single source of truth for numeric thresholds so prompts, filters, and gates stay aligned.
//! See `docs/src/architecture/socrates-protocol-ssot.md`.

mod confidence_override;

pub use confidence_override::ConfidencePolicyOverride;

use serde::{Deserialize, Serialize};

/// Discrete risk band after calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RiskBand {
    /// Safe to answer with normal grounding requirements.
    High,
    /// Partial evidence — prefer clarification or tool use.
    #[default]
    Medium,
    /// Insufficient or contradictory — abstain or escalate.
    Low,
}

/// Final triage decision for a turn or task completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskDecision {
    /// Model may answer with normal grounding requirements.
    Answer,
    /// Prefer a clarifying question or tool use before committing.
    Ask,
    /// Refuse or escalate — evidence is insufficient or contradictory.
    Abstain,
}

/// Preferred shape for a clarification prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionKind {
    /// Bounded hypothesis set, highest diagnostic throughput per turn.
    MultipleChoice,
    /// Free-form clarification for broad/novel intent spaces.
    OpenEnded,
    /// Structured scalar/identifier entry (path, id, numeric bound, date).
    Entry,
}

/// Why a clarification loop terminated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClarificationStopReason {
    /// Confidence already meets target for safe continuation.
    ConfidenceSufficient,
    /// Risk gate does not allow additional questioning.
    RiskGateBlocked,
    /// Clarification loop reached turn budget.
    MaxClarificationTurns,
    /// No candidate reached minimum expected entropy reduction.
    MarginalGainTooLow,
    /// Candidate utility exceeded allowed user cost.
    UserCostTooHigh,
    /// Session clarification attention budget (wall-time analogue) is exhausted.
    AttentionBudgetExceeded,
}

/// Baseline interrupt recovery cost (Gloria Mark, UC Irvine, 2023): 23m15s — scales `expected_user_cost`.
pub const CLARIFICATION_INTERRUPT_COST_MS: u64 = 23_250;

/// Cost-aware questioning policy tuned for high information-per-turn interaction.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct QuestioningPolicy {
    /// Upper bound on clarification turns before forced stop.
    pub max_clarification_turns: u32,
    /// Minimum expected entropy reduction (bits) required to ask.
    pub min_information_gain_bits: f64,
    /// Ceiling for normalized expected user burden in `[0, 1]`.
    pub max_expected_user_cost: f64,
    /// Confidence target after which clarification should stop.
    pub target_confidence: f64,
    /// Soft ceiling on accumulated MCP clarification attention (ms) per session key (`0` = disable).
    pub max_clarification_attention_ms: u64,
}

impl Default for QuestioningPolicy {
    fn default() -> Self {
        Self {
            max_clarification_turns: 3,
            min_information_gain_bits: 0.08,
            max_expected_user_cost: 0.80,
            target_confidence: 0.72,
            max_clarification_attention_ms: 3_600_000,
        }
    }
}

/// Candidate prompt metadata for information-theoretic selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionCandidate {
    /// Prompt text shown to the user.
    pub prompt: String,
    /// Prompt interaction type.
    pub question_kind: QuestionKind,
    /// Estimated entropy reduction if this question is asked.
    pub expected_information_gain_bits: f64,
    /// Estimated user effort/time burden normalized to `[0, 1]`.
    pub expected_user_cost: f64,
}

/// Output of clarification-selection policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionSelection {
    /// Whether the system should ask a clarification question now.
    pub question_needed: bool,
    /// Chosen question kind when `question_needed = true`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question_kind: Option<QuestionKind>,
    /// Chosen prompt when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Estimated entropy reduction of chosen question.
    pub expected_information_gain_bits: f64,
    /// Estimated user effort of chosen question.
    pub expected_user_cost: f64,
    /// Utility score used for tie-breaking (`gain / max(cost, eps)`).
    pub utility_bits_per_cost: f64,
    /// Optional stop reason when no question is asked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<ClarificationStopReason>,
}

/// Shannon entropy in bits from a probability vector.
#[must_use]
pub fn shannon_entropy_bits(probabilities: &[f64]) -> f64 {
    let mass: f64 = probabilities.iter().copied().filter(|p| *p > 0.0).sum();
    if mass <= f64::EPSILON {
        return 0.0;
    }
    probabilities
        .iter()
        .copied()
        .filter(|p| *p > 0.0)
        .map(|p| {
            let pn = p / mass;
            -(pn * pn.log2())
        })
        .sum()
}

/// Expected information gain in bits for one candidate question.
///
/// `prior_hypothesis_probs` and each posterior row can be unnormalized (they are normalized internally).
/// `outcome_probabilities` should match `posterior_hypothesis_probs.len()`.
#[must_use]
pub fn expected_information_gain_bits(
    prior_hypothesis_probs: &[f64],
    posterior_hypothesis_probs: &[Vec<f64>],
    outcome_probabilities: &[f64],
) -> f64 {
    if posterior_hypothesis_probs.is_empty()
        || posterior_hypothesis_probs.len() != outcome_probabilities.len()
    {
        return 0.0;
    }
    let prior_h = shannon_entropy_bits(prior_hypothesis_probs);
    let outcome_mass: f64 = outcome_probabilities
        .iter()
        .copied()
        .filter(|p| *p > 0.0)
        .sum();
    if outcome_mass <= f64::EPSILON {
        return 0.0;
    }
    let expected_post_h: f64 = posterior_hypothesis_probs
        .iter()
        .zip(outcome_probabilities.iter().copied())
        .filter(|(_, p)| *p > 0.0)
        .map(|(posterior, p)| (p / outcome_mass) * shannon_entropy_bits(posterior))
        .sum();
    (prior_h - expected_post_h).max(0.0)
}

/// Numeric policy for confidence, abstention, and review filtering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfidencePolicy {
    /// TOESTUB structured review: minimum `ReviewFinding.confidence` (0–100).
    pub min_review_finding_confidence: u8,
    /// Prompt text should instruct the model to report only at or above this (0–100).
    pub min_prompt_report_confidence: u8,
    /// Normalized score below this → [`RiskDecision::Abstain`].
    pub abstain_threshold: f64,
    /// Normalized score below this (but ≥ abstain) → [`RiskDecision::Ask`].
    pub ask_for_help_threshold: f64,
    /// If contradiction_ratio exceeds this, force abstain regardless of score.
    pub max_contradiction_ratio_for_answer: f64,
    /// Minimum normalized confidence to persist research-like artifacts (0–1).
    pub min_persist_confidence: f64,
    /// Minimum normalized confidence to emit training pairs (0–1).
    pub min_training_pair_confidence: f64,
}

impl ConfidencePolicy {
    /// Default TOESTUB / review finding floor (0–100); must match [`Self::default`].
    pub const DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE: u8 = 80;
    /// Default prompt report floor (0–100); must match [`Self::default`].
    pub const DEFAULT_MIN_PROMPT_REPORT_CONFIDENCE: u8 = 80;
    /// Default abstain cutoff on normalized confidence; must match [`Self::default`].
    pub const DEFAULT_ABSTAIN_THRESHOLD: f64 = 0.35;
    /// Default “ask” band lower bound; must match [`Self::default`].
    pub const DEFAULT_ASK_FOR_HELP_THRESHOLD: f64 = 0.55;
    /// Default contradiction ratio that forces abstain; must match [`Self::default`].
    pub const DEFAULT_MAX_CONTRADICTION_RATIO_FOR_ANSWER: f64 = 0.40;
    /// Default minimum normalized confidence to persist research-like artifacts; must match [`Self::default`].
    pub const DEFAULT_MIN_PERSIST_CONFIDENCE: f64 = 0.60;
    /// Default minimum normalized confidence to emit training pairs; must match [`Self::default`].
    pub const DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE: f64 = 0.75;

    /// Global default used across the workspace unless overridden.
    #[must_use]
    pub fn workspace_default() -> Self {
        Self::default()
    }

    /// Merge [`ConfidencePolicyOverride`] fields onto this policy (unset override fields keep base values).
    #[must_use]
    pub fn with_overrides(&self, o: &ConfidencePolicyOverride) -> Self {
        let mut out = *self;
        if let Some(v) = o.min_review_finding_confidence {
            out.min_review_finding_confidence = v;
        }
        if let Some(v) = o.min_prompt_report_confidence {
            out.min_prompt_report_confidence = v;
        }
        if let Some(v) = o.abstain_threshold {
            out.abstain_threshold = v;
        }
        if let Some(v) = o.ask_for_help_threshold {
            out.ask_for_help_threshold = v;
        }
        if let Some(v) = o.max_contradiction_ratio_for_answer {
            out.max_contradiction_ratio_for_answer = v;
        }
        if let Some(v) = o.min_persist_confidence {
            out.min_persist_confidence = v;
        }
        if let Some(v) = o.min_training_pair_confidence {
            out.min_training_pair_confidence = v;
        }
        out
    }

    /// Classify a normalized confidence in `[0,1]` and contradiction ratio in `[0,1]`.
    #[must_use]
    pub fn classify_risk(&self, confidence: f64, contradiction_ratio: f64) -> RiskBand {
        let c = confidence.clamp(0.0, 1.0);
        let cr = contradiction_ratio.clamp(0.0, 1.0);
        if cr > self.max_contradiction_ratio_for_answer || c < self.abstain_threshold {
            RiskBand::Low
        } else if c < self.ask_for_help_threshold {
            RiskBand::Medium
        } else {
            RiskBand::High
        }
    }

    /// Map calibrated signal to answer / ask / abstain.
    #[must_use]
    pub fn evaluate_risk_decision(
        &self,
        confidence: f64,
        contradiction_ratio: f64,
    ) -> RiskDecision {
        let band = self.classify_risk(confidence, contradiction_ratio);
        match band {
            RiskBand::High => RiskDecision::Answer,
            RiskBand::Medium => RiskDecision::Ask,
            RiskBand::Low => RiskDecision::Abstain,
        }
    }

    /// Pick the highest utility clarification question under risk/time/cost constraints.
    #[must_use]
    pub fn select_clarification_question(
        &self,
        confidence: f64,
        contradiction_ratio: f64,
        clarification_turn_index: u32,
        candidates: &[QuestionCandidate],
        questioning: QuestioningPolicy,
        spent_clarification_attention_ms: u64,
        max_clarification_attention_ms: u64,
    ) -> QuestionSelection {
        let decision = self.evaluate_risk_decision(confidence, contradiction_ratio);
        if confidence >= questioning.target_confidence || decision == RiskDecision::Answer {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::ConfidenceSufficient),
            };
        }
        if decision == RiskDecision::Abstain {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::RiskGateBlocked),
            };
        }
        if max_clarification_attention_ms > 0
            && spent_clarification_attention_ms >= max_clarification_attention_ms
        {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::AttentionBudgetExceeded),
            };
        }
        if clarification_turn_index >= questioning.max_clarification_turns {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::MaxClarificationTurns),
            };
        }

        let mut best_idx: Option<usize> = None;
        let mut best_utility = f64::MIN;
        for (idx, c) in candidates.iter().enumerate() {
            if c.expected_user_cost > questioning.max_expected_user_cost {
                continue;
            }
            if c.expected_information_gain_bits < questioning.min_information_gain_bits {
                continue;
            }
            let utility = c.expected_information_gain_bits / c.expected_user_cost.max(1e-6);
            if utility > best_utility {
                best_utility = utility;
                best_idx = Some(idx);
            }
        }

        if let Some(idx) = best_idx {
            let c = &candidates[idx];
            return QuestionSelection {
                question_needed: true,
                question_kind: Some(c.question_kind),
                prompt: Some(c.prompt.clone()),
                expected_information_gain_bits: c.expected_information_gain_bits,
                expected_user_cost: c.expected_user_cost,
                utility_bits_per_cost: best_utility,
                stop_reason: None,
            };
        }

        let cost_blocked = candidates
            .iter()
            .any(|c| c.expected_user_cost > questioning.max_expected_user_cost);
        QuestionSelection {
            question_needed: false,
            question_kind: None,
            prompt: None,
            expected_information_gain_bits: 0.0,
            expected_user_cost: 0.0,
            utility_bits_per_cost: 0.0,
            stop_reason: Some(if cost_blocked {
                ClarificationStopReason::UserCostTooHigh
            } else {
                ClarificationStopReason::MarginalGainTooLow
            }),
        }
    }
}

impl Default for ConfidencePolicy {
    fn default() -> Self {
        Self {
            min_review_finding_confidence: Self::DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE,
            min_prompt_report_confidence: Self::DEFAULT_MIN_PROMPT_REPORT_CONFIDENCE,
            abstain_threshold: Self::DEFAULT_ABSTAIN_THRESHOLD,
            ask_for_help_threshold: Self::DEFAULT_ASK_FOR_HELP_THRESHOLD,
            max_contradiction_ratio_for_answer: Self::DEFAULT_MAX_CONTRADICTION_RATIO_FOR_ANSWER,
            min_persist_confidence: Self::DEFAULT_MIN_PERSIST_CONFIDENCE,
            min_training_pair_confidence: Self::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constants_match_struct_fields() {
        let p = ConfidencePolicy::workspace_default();
        assert_eq!(
            p.min_review_finding_confidence,
            ConfidencePolicy::DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE
        );
        assert_eq!(
            p.min_persist_confidence,
            ConfidencePolicy::DEFAULT_MIN_PERSIST_CONFIDENCE
        );
        assert_eq!(
            p.min_training_pair_confidence,
            ConfidencePolicy::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE
        );
    }

    #[test]
    fn default_policy_symmetric_review_thresholds() {
        let p = ConfidencePolicy::default();
        assert_eq!(
            p.min_review_finding_confidence,
            p.min_prompt_report_confidence
        );
    }

    #[test]
    fn high_confidence_answers() {
        let p = ConfidencePolicy::default();
        assert_eq!(p.evaluate_risk_decision(0.9, 0.0), RiskDecision::Answer);
    }

    #[test]
    fn contradiction_forces_abstain() {
        let p = ConfidencePolicy::default();
        assert_eq!(p.evaluate_risk_decision(0.99, 0.99), RiskDecision::Abstain);
    }

    #[test]
    fn medium_confidence_requests_clarification() {
        let p = ConfidencePolicy::default();
        let c = (p.abstain_threshold + p.ask_for_help_threshold) / 2.0;
        assert_eq!(p.evaluate_risk_decision(c, 0.0), RiskDecision::Ask);
    }

    #[test]
    fn confidence_monotonicity_for_fixed_contradiction() {
        let p = ConfidencePolicy::default();
        let low = p.evaluate_risk_decision(0.10, 0.0);
        let mid = p.evaluate_risk_decision(0.50, 0.0);
        let hi = p.evaluate_risk_decision(0.95, 0.0);
        assert_eq!(low, RiskDecision::Abstain);
        assert!(matches!(mid, RiskDecision::Ask | RiskDecision::Answer));
        assert_eq!(hi, RiskDecision::Answer);
    }

    #[test]
    fn entropy_uniform_over_four_states_is_two_bits() {
        let h = shannon_entropy_bits(&[0.25, 0.25, 0.25, 0.25]);
        assert!((h - 2.0).abs() < 1e-9, "h={h}");
    }

    #[test]
    fn expected_information_gain_positive_when_posteriors_reduce_uncertainty() {
        let prior = [0.5, 0.5];
        let post = vec![vec![0.9, 0.1], vec![0.1, 0.9]];
        let outcome_probs = [0.5, 0.5];
        let gain = expected_information_gain_bits(&prior, &post, &outcome_probs);
        assert!(gain > 0.0, "gain={gain}");
    }

    #[test]
    fn select_clarification_uses_best_gain_per_cost() {
        let p = ConfidencePolicy::default();
        let qp = QuestioningPolicy::default();
        let candidates = vec![
            QuestionCandidate {
                prompt: "Do you want only docs or code changes?".into(),
                question_kind: QuestionKind::MultipleChoice,
                expected_information_gain_bits: 0.20,
                expected_user_cost: 0.20,
            },
            QuestionCandidate {
                prompt: "What is your broader goal?".into(),
                question_kind: QuestionKind::OpenEnded,
                expected_information_gain_bits: 0.28,
                expected_user_cost: 0.55,
            },
        ];
        let sel =
            p.select_clarification_question(0.45, 0.0, 0, &candidates, qp, 0, 0);
        assert!(sel.question_needed);
        assert_eq!(sel.question_kind, Some(QuestionKind::MultipleChoice));
    }

    #[test]
    fn select_clarification_stops_when_max_turns_reached() {
        let p = ConfidencePolicy::default();
        let qp = QuestioningPolicy {
            max_clarification_turns: 1,
            ..QuestioningPolicy::default()
        };
        let candidates = vec![QuestionCandidate {
            prompt: "confirm scope".into(),
            question_kind: QuestionKind::Entry,
            expected_information_gain_bits: 0.5,
            expected_user_cost: 0.1,
        }];
        let sel =
            p.select_clarification_question(0.40, 0.0, 1, &candidates, qp, 0, 0);
        assert!(!sel.question_needed);
        assert_eq!(
            sel.stop_reason,
            Some(ClarificationStopReason::MaxClarificationTurns)
        );
    }

    #[test]
    fn select_clarification_stops_when_attention_budget_spent() {
        let p = ConfidencePolicy::default();
        let qp = QuestioningPolicy::default();
        let candidates = vec![QuestionCandidate {
            prompt: "confirm scope".into(),
            question_kind: QuestionKind::Entry,
            expected_information_gain_bits: 0.5,
            expected_user_cost: 0.1,
        }];
        let sel = p.select_clarification_question(
            0.40,
            0.0,
            0,
            &candidates,
            qp,
            10_000,
            10_000,
        );
        assert!(!sel.question_needed);
        assert_eq!(
            sel.stop_reason,
            Some(ClarificationStopReason::AttentionBudgetExceeded)
        );
    }
}
