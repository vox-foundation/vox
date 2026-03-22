//! Shared **Socrates** confidence policy for orchestrator, MCP, and TOESTUB review.
//!
//! Single source of truth for numeric thresholds so prompts, filters, and gates stay aligned.
//! See `docs/src/architecture/socrates-protocol-ssot.md`.

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

/// Optional per-deployment overrides (TOML / env) merged onto [`ConfidencePolicy::workspace_default`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfidencePolicyOverride {
    /// Overrides [`ConfidencePolicy::min_review_finding_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_review_finding_confidence: Option<u8>,
    /// Overrides [`ConfidencePolicy::min_prompt_report_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_prompt_report_confidence: Option<u8>,
    /// Overrides [`ConfidencePolicy::abstain_threshold`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abstain_threshold: Option<f64>,
    /// Overrides [`ConfidencePolicy::ask_for_help_threshold`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_for_help_threshold: Option<f64>,
    /// Overrides [`ConfidencePolicy::max_contradiction_ratio_for_answer`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_contradiction_ratio_for_answer: Option<f64>,
    /// Overrides [`ConfidencePolicy::min_persist_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_persist_confidence: Option<f64>,
    /// Overrides [`ConfidencePolicy::min_training_pair_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_training_pair_confidence: Option<f64>,
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
}
