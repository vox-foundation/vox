use crate::types::AgentId;

use super::budget::{ActionDescriptor, ApprovalTier, TierGateConfig, TrustTier};

/// Per-agent trust score with Kalman-filter update and hysteresis demotion.
///
/// The Kalman filter converges faster than EWMA for agents with consistent histories
/// (Task 62) while the `variance` field enables UCB exploration (Task 61).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentTrustScore {
    pub agent_id: AgentId,
    /// Kalman-filtered trust estimate ∈ [0.0, 1.0].
    pub trust_score: f64,
    pub tier: TrustTier,
    pub total_outcomes: u32,
    pub successful_outcomes: u32,
    /// Consecutive events below current tier's lower bound (hysteresis counter).
    pub below_tier_streak: u32,
    pub last_updated_ms: u64,
    /// Kalman estimate variance ∈ [0.0, 1.0] — high = uncertain = more exploration (Task 60).
    pub variance: f64,
    /// Manual override flag (Task 64)
    pub is_override: bool,
}

impl AgentTrustScore {
    /// Create a new agent with Empirical Bayes prior: trust = 0.5, variance = 0.25 (Task 63).
    ///
    /// The high initial variance (0.25) drives UCB exploration for new agents so they receive
    /// tasks before their performance is fully characterized.
    pub fn new(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            trust_score: 0.5,
            tier: TrustTier::Untrusted,
            total_outcomes: 0,
            successful_outcomes: 0,
            below_tier_streak: 0,
            last_updated_ms: crate::types::now_unix_ms(),
            variance: 0.25,
            is_override: false,
        }
    }

    /// Update trust with a discrete Kalman filter step (Task 62).
    ///
    /// The Kalman gain `K = P / (P + R)` (measurement noise R = 0.1) adapts the update
    /// magnitude to the current variance, converging faster than a fixed-α EWMA when
    /// the agent is consistent.
    ///
    /// `provisional_min` and `trusted_min` come from `OrchestratorConfig`.
    pub fn record_outcome(
        &mut self,
        success: bool,
        _alpha: f64,
        provisional_min: u32,
        trusted_min: u32,
    ) -> f64 {
        const MEASUREMENT_NOISE: f64 = 0.10;
        const PROCESS_NOISE: f64 = 0.005;

        if self.is_override {
            return self.trust_score;
        }

        let observation = if success { 1.0_f64 } else { 0.0_f64 };

        // Prediction step: variance grows by process noise
        let p_pred = (self.variance + PROCESS_NOISE).min(1.0);

        // Update step: Kalman gain
        let k = p_pred / (p_pred + MEASUREMENT_NOISE);
        self.trust_score =
            (self.trust_score + k * (observation - self.trust_score)).clamp(0.0, 1.0);
        self.variance = (1.0 - k) * p_pred;

        self.total_outcomes += 1;
        if success {
            self.successful_outcomes += 1;
        }
        self.last_updated_ms = crate::types::now_unix_ms();
        self.update_tier(provisional_min, trusted_min);
        self.trust_score
    }

    /// UCB (Upper Confidence Bound) score for exploration-driven routing (Task 61).
    ///
    /// Combines the Kalman trust estimate with an exploration bonus proportional to `variance`.
    /// Agents with high uncertainty receive a bonus that encourages the router to sample them,
    /// spreading load more evenly than pure greedy selection.
    pub fn ucb_score(&self, exploration_weight: f64) -> f64 {
        // UCB1-style: μ + c * σ  where σ = sqrt(variance)
        (self.trust_score + exploration_weight * self.variance.sqrt()).clamp(0.0, 2.0)
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
