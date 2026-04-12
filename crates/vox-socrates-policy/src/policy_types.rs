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

/// Output of a research-need assessment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocratesResearchDecision {
    /// Whether the system should trigger external web research (Tavily).
    pub should_research: bool,
    /// Reason string for the dispatch decision (e.g. "Coverage Paradox", "Low Confidence").
    pub trigger: String,
    /// Optional query refinement for the research call.
    pub suggested_query: Option<String>,
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
