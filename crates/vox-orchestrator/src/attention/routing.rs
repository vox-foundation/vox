use crate::types::AgentId;

use super::budget::{
    ActionDescriptor, ApprovalTier, TierGateConfig, TrustTier,
};

/// Per-agent trust score with EWMA smoothing and hysteresis demotion.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
