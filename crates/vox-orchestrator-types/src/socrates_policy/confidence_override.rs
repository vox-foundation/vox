//! Optional per-deployment overrides merged onto [`crate::ConfidencePolicy`].

use serde::{Deserialize, Serialize};

/// Optional per-deployment overrides (TOML / env) merged onto [`crate::ConfidencePolicy::workspace_default`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfidencePolicyOverride {
    /// Overrides [`crate::ConfidencePolicy::min_review_finding_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_review_finding_confidence: Option<u8>,
    /// Overrides [`crate::ConfidencePolicy::min_prompt_report_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_prompt_report_confidence: Option<u8>,
    /// Overrides [`crate::ConfidencePolicy::abstain_threshold`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abstain_threshold: Option<f64>,
    /// Overrides [`crate::ConfidencePolicy::ask_for_help_threshold`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_for_help_threshold: Option<f64>,
    /// Overrides [`crate::ConfidencePolicy::max_contradiction_ratio_for_answer`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_contradiction_ratio_for_answer: Option<f64>,
    /// Overrides [`crate::ConfidencePolicy::min_persist_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_persist_confidence: Option<f64>,
    /// Overrides [`crate::ConfidencePolicy::min_training_pair_confidence`] when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_training_pair_confidence: Option<f64>,
}
