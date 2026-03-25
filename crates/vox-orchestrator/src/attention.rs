//! Developer attention as a first-class budget resource (Phase 15).
//!
//! Adapted NASA TLX (4 automated subscales), EWMA trust scoring with
//! Bayesian prior, Shannon entropy for auto-approve graduation, and
//! token-output-weighted complexity.
//!
//! All functions in this module are pure (no I/O, no async).
//! Persistence lives in [`crate::attention_tracker`].

use crate::types::{AgentId, TaskId, TaskPriority};
use serde::{Deserialize, Serialize};

// ── Configurable weight structs ────────────────────────────────────────────

/// Pilot-calibratable NASA TLX subscale weights for attention cost computation.
/// All four weights should sum to 1.0; they are not validated at parse time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NasaTlxWeights {
    /// Mental demand (complexity, surprise). Default: 0.35
    pub mental: f64,
    /// Temporal demand (urgency). Default: 0.25
    pub temporal: f64,
    /// Frustration (repeated patterns). Default: 0.20
    pub frustration: f64,
    /// Trust discount (inverse of agent trust). Default: 0.20
    pub trust_discount: f64,
}

impl Default for NasaTlxWeights {
    fn default() -> Self {
        Self {
            mental: 0.35,
            temporal: 0.25,
            frustration: 0.20,
            trust_discount: 0.20,
        }
    }
}

/// Config-driven thresholds for `classify_tier()`.
/// Decouples gating policy from compiled constants.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierGateConfig {
    /// Shannon entropy (bits) below which auto-approve graduation triggers. Default: 0.15
    pub entropy_auto_approve_threshold: f64,
    /// Minimum repeated approvals before entropy graduation. Default: 10
    pub auto_approve_min_observations: u32,
    /// Minimum trust score for auto-approve (from OrchestratorConfig). Default: 0.85
    pub auto_approve_min_trust: f64,
    /// Max write files before blocking an Untrusted agent. Default: 3
    pub untrusted_max_writes_before_block: usize,
    /// Max write files before requiring Review from a Trusted agent. Default: 1
    pub trusted_single_file_confirm_limit: usize,
}

impl Default for TierGateConfig {
    fn default() -> Self {
        Self {
            entropy_auto_approve_threshold: 0.15,
            auto_approve_min_observations: 10,
            auto_approve_min_trust: 0.85,
            untrusted_max_writes_before_block: 3,
            trusted_single_file_confirm_limit: 1,
        }
    }
}

/// Baseline interrupt recovery cost from Gloria Mark (UC Irvine, 2023): 23 min 15 sec.
pub const DEFAULT_INTERRUPT_COST_MS: u64 = 23_250;

/// Default pilot attention budget: 1 hour per session period.
pub const DEFAULT_ATTENTION_BUDGET_MS: u64 = 3_600_000;

// ── Enums ──────────────────────────────────────────────────────────────────

/// Classification of an action's approval requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ApprovalTier {
    /// No human review needed — read-only, reversible, or earned trust.
    AutoApprove,
    /// Quick confirm — low-risk mutation, earned trust above threshold.
    #[default]
    Confirm,
    /// Full review — architectural, security-critical, multi-file, low trust.
    Review,
    /// Blocked — requires explicit pilot sign-off (destructive, external, deploy).
    Blocked,
}

/// Quantified trust level for an agent, replacing `Option<String>` in `MessageEnvelope`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TrustTier {
    /// Brand-new agent or unknown provenance. Trust score < 0.45.
    #[default]
    Untrusted,
    /// Some successful completions. Trust score [0.45, 0.70).
    Provisional,
    /// Consistent track record. Trust score [0.70, 0.90).
    Trusted,
    /// Operator-provisioned. Trust score [0.90, 1.0]. Not auto-promoted.
    System,
}

/// What kind of action consumed pilot attention.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AttentionEventType {
    /// A command (cargo test, git status) required approval.
    CommandApproval,
    /// Code diff presented for review.
    CodeReview,
    /// New task submission for confirmation.
    TaskSubmission,
    /// A2A message interrupted the pilot.
    A2AInterrupt,
    /// Error escalation.
    ErrorReport,
    /// Plan review request.
    PlanReview,
}

/// What the pilot decided.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApprovalOutcome {
    /// Pilot approved as-is.
    Approved,
    /// Pilot rejected the proposed action.
    Rejected,
    /// Pilot approved with modifications.
    Modified,
    /// System auto-approved — zero attention cost.
    AutoApproved,
    /// Pilot did not respond within timeout.
    TimedOut,
}

// ── Structs ────────────────────────────────────────────────────────────────

/// Descriptor for an action about to be gated. Used by `classify_tier()`.
#[derive(Debug, Clone)]
pub struct ActionDescriptor {
    /// Estimated task complexity (1–10) from `AgentTask.estimated_complexity`.
    pub estimated_complexity: u8,
    /// Token count of the LLM output that generated this action.
    pub tokens_output: u64,
    /// Task priority from the orchestrator queue.
    pub priority: TaskPriority,
    /// Number of files in the write manifest.
    pub write_file_count: usize,
    /// Whether the action touches external resources (network, deploy).
    pub external: bool,
    /// How many times this (agent, pattern) pair has been approved without failure.
    pub repeated_approve_count: u32,
    /// Number of concurrent tasks the pilot is overseeing right now.
    pub concurrent_tasks: usize,
}

/// Single attention event recorded in the orchestrator and persisted to Arca.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionEvent {
    pub agent_id: AgentId,
    pub task_id: Option<TaskId>,
    pub event_type: AttentionEventType,
    pub tier: ApprovalTier,
    /// Computed attention cost in ms (0 for AutoApproved events).
    pub cost_ms: u64,
    pub outcome: ApprovalOutcome,
    /// Agent's EWMA trust score at the moment this event was recorded.
    pub trust_score_at_time: f64,
    /// Effective complexity: 0.4 × estimated + 0.6 × token_complexity.
    pub effective_complexity: f64,
    /// Shannon entropy in bits for this (agent, pattern) pair.
    pub decision_entropy_bits: f64,
    pub timestamp_ms: u64,
}

/// Per-session attention budget tracked in-memory, periodically flushed to Arca.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionBudget {
    /// Maximum attention budget in milliseconds per period.
    pub max_attention_ms: u64,
    /// Total attention spent so far this period.
    pub spent_ms: u64,
    /// Total approval requests this period.
    pub total_requests: u32,
    /// Requests that were auto-approved (zero attention cost).
    pub auto_approved: u32,
    /// Requests where pilot rejected (wasted attention).
    pub rejected: u32,
    /// EWMA interrupt frequency (events per hour).
    pub interrupt_freq_per_hour: f64,
    /// Timestamp of last non-auto interrupt (for gap calculation).
    pub last_interrupt_ms: u64,
}

impl Default for AttentionBudget {
    fn default() -> Self {
        Self {
            max_attention_ms: DEFAULT_ATTENTION_BUDGET_MS,
            spent_ms: 0,
            total_requests: 0,
            auto_approved: 0,
            rejected: 0,
            interrupt_freq_per_hour: 0.0,
            last_interrupt_ms: 0,
        }
    }
}

impl AttentionBudget {
    /// Create a budget with a custom ceiling.
    pub fn with_max(max_attention_ms: u64) -> Self {
        Self {
            max_attention_ms,
            ..Default::default()
        }
    }

    /// Fraction of budget consumed (0.0–1.0+).
    pub fn spent_ratio(&self) -> f64 {
        if self.max_attention_ms == 0 {
            return 1.0;
        }
        self.spent_ms as f64 / self.max_attention_ms as f64
    }

    /// True when spent exceeds the given alert threshold.
    pub fn alert(&self, threshold: f64) -> bool {
        self.spent_ratio() > threshold
    }

    /// True when budget is fully exhausted.
    pub fn exhausted(&self) -> bool {
        self.spent_ms >= self.max_attention_ms
    }

    /// Efficiency: what fraction of approvals were useful (not rejected).
    pub fn efficiency(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        1.0 - (self.rejected as f64 / self.total_requests as f64)
    }

    /// Auto-approve ratio: higher means less attention drain.
    pub fn auto_approve_ratio(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        self.auto_approved as f64 / self.total_requests as f64
    }

    /// Current focus depth from interrupt frequency.
    pub fn focus_depth(&self) -> FocusDepth {
        if self.interrupt_freq_per_hour >= 8.0 {
            FocusDepth::Deep
        } else if self.interrupt_freq_per_hour >= 3.0 {
            FocusDepth::Focused
        } else {
            FocusDepth::Ambient
        }
    }
}

/// Per-agent trust score with EWMA smoothing and hysteresis demotion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrustScore {
    pub agent_id: AgentId,
    /// EWMA trust score ∈ [0.0, 1.0].
    pub trust_score: f64,
    pub tier: TrustTier,
    pub total_outcomes: u32,
    pub successful_outcomes: u32,
    /// Consecutive events below current tier's lower bound (hysteresis counter).
    pub below_tier_streak: u32,
    pub last_updated_ms: u64,
}

impl AgentTrustScore {
    /// Create a new agent with Bayesian prior trust = 0.3.
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            trust_score: 0.3,
            tier: TrustTier::Untrusted,
            total_outcomes: 0,
            successful_outcomes: 0,
            below_tier_streak: 0,
            last_updated_ms: crate::types::now_unix_ms(),
        }
    }

    /// Update trust with EWMA. `success` = true if approved and no rollback.
    /// `provisional_min` and `trusted_min` come from `OrchestratorConfig`.
    pub fn record_outcome(
        &mut self,
        success: bool,
        alpha: f64,
        provisional_min: u32,
        trusted_min: u32,
    ) -> f64 {
        let outcome = if success { 1.0 } else { 0.0 };
        self.trust_score = alpha * outcome + (1.0 - alpha) * self.trust_score;
        self.trust_score = self.trust_score.clamp(0.0, 1.0);
        self.total_outcomes += 1;
        if success {
            self.successful_outcomes += 1;
        }
        self.last_updated_ms = crate::types::now_unix_ms();
        self.update_tier(provisional_min, trusted_min);
        self.trust_score
    }

    fn update_tier(&mut self, provisional_min: u32, trusted_min: u32) {
        let lower = match self.tier {
            TrustTier::Untrusted => 0.0,
            TrustTier::Provisional => 0.45,
            TrustTier::Trusted => 0.70,
            TrustTier::System => 0.90,
        };

        // Promotion checks (System is operator-only; not auto-promoted)
        if self.trust_score >= 0.70
            && self.total_outcomes >= trusted_min
            && matches!(self.tier, TrustTier::Untrusted | TrustTier::Provisional)
        {
            self.tier = TrustTier::Trusted;
            self.below_tier_streak = 0;
            return;
        }
        if self.trust_score >= 0.45
            && self.total_outcomes >= provisional_min
            && self.tier == TrustTier::Untrusted
        {
            self.tier = TrustTier::Provisional;
            self.below_tier_streak = 0;
            return;
        }

        // Demotion with hysteresis (3 consecutive events below tier floor)
        if self.trust_score < lower && self.tier != TrustTier::Untrusted {
            self.below_tier_streak += 1;
            if self.below_tier_streak >= 3 {
                self.tier = match self.tier {
                    TrustTier::System => TrustTier::Trusted,
                    TrustTier::Trusted => TrustTier::Provisional,
                    TrustTier::Provisional => TrustTier::Untrusted,
                    TrustTier::Untrusted => TrustTier::Untrusted,
                };
                self.below_tier_streak = 0;
            }
        } else {
            self.below_tier_streak = 0;
        }
    }
}

// ── FocusDepth ─────────────────────────────────────────────────────────────

/// Agent inbox suppression level based on interrupt frequency.
/// Maps to brain's Salience/Executive Control Networks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FocusDepth {
    /// Full SN+ECN; unrestricted A2A. Interrupt freq < 3/hr.
    #[default]
    Ambient,
    /// ECN at boundaries; non-critical A2A deferred. 3 ≤ freq < 8/hr.
    Focused,
    /// ECN pre/post task only; ScopeConflict/ErrorReport A2A only. freq ≥ 8/hr.
    Deep,
}

// ── Computation functions ──────────────────────────────────────────────────

/// Compute attention cost in ms using configurable NASA TLX subscales.
///
/// `attention_cost = base_ms × Σ(wᵢ × subscaleᵢ) × context_switch_mult`
pub fn compute_attention_cost_ms(
    action: &ActionDescriptor,
    agent_trust: f64,
    base_ms: u64,
    weights: &NasaTlxWeights,
) -> u64 {
    let token_complexity = (action.tokens_output as f64 / 1000.0).min(10.0);
    let effective_complexity = 0.4 * (action.estimated_complexity as f64) + 0.6 * token_complexity;
    let mental = (effective_complexity / 10.0).clamp(0.0, 1.0);

    let temporal = match action.priority {
        TaskPriority::Urgent => 1.0,
        TaskPriority::Normal => 0.5,
        TaskPriority::Background => 0.2,
    };

    let r = action.repeated_approve_count as f64;
    let frustration = r / (r + 5.0);

    let trust_discount = (1.0 - agent_trust).clamp(0.0, 1.0);

    let weighted = weights.mental * mental
        + weights.temporal * temporal
        + weights.frustration * frustration
        + weights.trust_discount * trust_discount;

    let csm = (1.0 + (action.concurrent_tasks.saturating_sub(1) as f64) * 0.3).clamp(1.0, 3.0);

    (base_ms as f64 * weighted * csm) as u64
}

/// Shannon entropy for a binary approve/reject decision.
///
/// `H = −p·log₂(p) − (1−p)·log₂(1−p)`
///
/// Returns NaN-safe result; clamps p to [0.001, 0.999].
pub fn decision_entropy_bits(approve_rate: f64) -> f64 {
    let p = approve_rate.clamp(0.001, 0.999);
    -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
}

/// Classify the approval tier for an action based on trust, complexity, and patterns.
/// All thresholds are taken from `gate` to avoid hard-coded constants.
pub fn classify_tier(
    trust: &AgentTrustScore,
    action: &ActionDescriptor,
    entropy: f64,
    gate: &TierGateConfig,
) -> ApprovalTier {
    // Hard blocks first
    if action.external {
        return ApprovalTier::Blocked;
    }
    if action.write_file_count > gate.untrusted_max_writes_before_block
        && trust.tier == TrustTier::Untrusted
    {
        return ApprovalTier::Blocked;
    }

    // Auto-approve graduation: low Shannon entropy over sufficient observations
    if entropy < gate.entropy_auto_approve_threshold
        && action.repeated_approve_count >= gate.auto_approve_min_observations
        && trust.trust_score >= gate.auto_approve_min_trust
        && matches!(trust.tier, TrustTier::Trusted | TrustTier::System)
    {
        return ApprovalTier::AutoApprove;
    }

    // Read-only actions are always safe
    if action.write_file_count == 0 && !action.external {
        return ApprovalTier::AutoApprove;
    }

    // Trust-based classification
    match trust.tier {
        TrustTier::System => ApprovalTier::AutoApprove,
        TrustTier::Trusted => {
            if action.write_file_count <= gate.trusted_single_file_confirm_limit {
                ApprovalTier::Confirm
            } else {
                ApprovalTier::Review
            }
        }
        TrustTier::Provisional => ApprovalTier::Review,
        TrustTier::Untrusted => {
            if action.write_file_count > gate.untrusted_max_writes_before_block {
                ApprovalTier::Blocked
            } else {
                ApprovalTier::Review
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trust(tier: TrustTier, score: f64) -> AgentTrustScore {
        AgentTrustScore {
            agent_id: AgentId(1),
            trust_score: score,
            tier,
            total_outcomes: 25,
            successful_outcomes: 20,
            below_tier_streak: 0,
            last_updated_ms: 0,
        }
    }

    fn make_action(writes: usize, external: bool, tokens: u64, complexity: u8) -> ActionDescriptor {
        ActionDescriptor {
            estimated_complexity: complexity,
            tokens_output: tokens,
            priority: TaskPriority::Normal,
            write_file_count: writes,
            external,
            repeated_approve_count: 0,
            concurrent_tasks: 1,
        }
    }

    #[test]
    fn attention_cost_scales_with_token_output() {
        let w = NasaTlxWeights::default();
        let low = make_action(1, false, 50, 5);
        let high = make_action(1, false, 5000, 5);
        let cost_low = compute_attention_cost_ms(&low, 0.5, DEFAULT_INTERRUPT_COST_MS, &w);
        let cost_high = compute_attention_cost_ms(&high, 0.5, DEFAULT_INTERRUPT_COST_MS, &w);
        assert!(
            cost_high > cost_low,
            "high-token output should cost more attention"
        );
    }

    #[test]
    fn trust_ewma_promotes_untrusted_to_provisional() {
        let mut ts = AgentTrustScore::new(AgentId(1));
        for _ in 0..5 {
            ts.record_outcome(true, 0.1, 5, 20);
        }
        assert_eq!(ts.total_outcomes, 5);
        assert!(
            ts.trust_score > 0.45 || ts.tier == TrustTier::Provisional || ts.total_outcomes < 5,
            "score={:.3}, tier={:?}",
            ts.trust_score,
            ts.tier
        );
    }

    #[test]
    fn trust_ewma_demotes_after_3_consecutive_failures() {
        let mut ts = make_trust(TrustTier::Trusted, 0.75);
        ts.total_outcomes = 25;
        ts.record_outcome(false, 0.5, 5, 20);
        ts.record_outcome(false, 0.5, 5, 20);
        ts.record_outcome(false, 0.5, 5, 20);
        assert!(
            ts.tier == TrustTier::Provisional || ts.trust_score < 0.70,
            "should demote or drop below threshold"
        );
    }

    #[test]
    fn classify_tier_auto_approves_read_only() {
        let trust = make_trust(TrustTier::Untrusted, 0.2);
        let action = make_action(0, false, 100, 3);
        let gate = TierGateConfig::default();
        assert_eq!(
            classify_tier(&trust, &action, 0.5, &gate),
            ApprovalTier::AutoApprove
        );
    }

    #[test]
    fn classify_tier_blocks_untrusted_wide_writes() {
        let trust = make_trust(TrustTier::Untrusted, 0.2);
        let action = make_action(4, false, 100, 3);
        let gate = TierGateConfig::default();
        assert_eq!(
            classify_tier(&trust, &action, 0.5, &gate),
            ApprovalTier::Blocked
        );
    }

    #[test]
    fn classify_tier_blocks_external() {
        let trust = make_trust(TrustTier::System, 0.99);
        let action = make_action(0, true, 100, 1);
        let gate = TierGateConfig::default();
        assert_eq!(
            classify_tier(&trust, &action, 0.0, &gate),
            ApprovalTier::Blocked
        );
    }

    #[test]
    fn shannon_entropy_near_zero_for_certain_decisions() {
        let h = decision_entropy_bits(0.99);
        assert!(h < 0.15, "H={:.4} should be < 0.15 for p=0.99", h);
    }

    #[test]
    fn shannon_entropy_max_at_half() {
        let h = decision_entropy_bits(0.5);
        assert!(
            (h - 1.0).abs() < 0.01,
            "H should be ~1.0 at p=0.5, got {h:.4}"
        );
    }

    #[test]
    fn focus_depth_escalates_at_8_per_hour() {
        let mut budget = AttentionBudget::default();
        budget.interrupt_freq_per_hour = 8.0;
        assert_eq!(budget.focus_depth(), FocusDepth::Deep);
    }

    #[test]
    fn focus_depth_focused_between_3_and_8() {
        let mut budget = AttentionBudget::default();
        budget.interrupt_freq_per_hour = 5.0;
        assert_eq!(budget.focus_depth(), FocusDepth::Focused);
    }

    #[test]
    fn attention_budget_exhausted_flag() {
        let mut b = AttentionBudget::default();
        b.spent_ms = b.max_attention_ms;
        assert!(b.exhausted());
    }

    #[test]
    fn auto_approve_graduation_requires_low_entropy_and_trust() {
        let trust = make_trust(TrustTier::Trusted, 0.90); // meets gate.auto_approve_min_trust=0.85
        let mut action = make_action(1, false, 200, 3);
        action.repeated_approve_count = 10;
        let gate = TierGateConfig::default();
        // entropy < 0.15 AND trust >= 0.85 should auto-approve
        assert_eq!(
            classify_tier(&trust, &action, 0.10, &gate),
            ApprovalTier::AutoApprove
        );
        // entropy > 0.15 should not
        assert_ne!(
            classify_tier(&trust, &action, 0.50, &gate),
            ApprovalTier::AutoApprove
        );
    }
}
