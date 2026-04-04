//! Token and USD budget caps per agent for LLM context and API spend.
//!
//! [`BudgetManager`] tracks usage, rollover, and alert thresholds so the
//! orchestrator can trigger summarization or block work before limits are exceeded.
//! Phase 15: also tracks developer attention budget and per-agent trust scores.
use std::sync::Arc;

use std::collections::HashMap;

use crate::attention::{AgentTrustScore, ApprovalOutcome, AttentionBudget, AttentionEvent};
use crate::fatigue_monitor::{FatigueEvent, FatigueMonitor};
use crate::sync_lock;
use crate::types::AgentId;

/// Per-agent budget allocation cap.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentBudgetAllocation {
    /// Maximum tokens allowed per period
    pub max_tokens: usize,
    /// Maximum cost in USD allowed per period
    pub max_cost_usd: f64,
    /// Alert when token usage exceeds this fraction (0.0–1.0)
    pub token_alert_threshold: f64,
    /// Alert when cost exceeds this fraction (0.0–1.0)
    pub cost_alert_threshold: f64,
    /// Rollover unused budget to the next period (as fraction 0.0–1.0)
    pub rollover_fraction: f64,
}

impl AgentBudgetAllocation {
    /// Builds default allocation with conservative alert thresholds and no rollover.
    pub fn new(max_tokens: usize, max_cost_usd: f64) -> Self {
        Self {
            max_tokens,
            max_cost_usd,
            token_alert_threshold: 0.8,
            cost_alert_threshold: 0.9,
            rollover_fraction: 0.0,
        }
    }

    /// Sets the fraction of unused tokens (0.0–1.0) carried into the next period.
    pub fn with_rollover(mut self, fraction: f64) -> Self {
        self.rollover_fraction = fraction.clamp(0.0, 1.0);
        self
    }

    /// Overrides token and cost alert fractions (each 0.0–1.0 of the respective cap).
    pub fn with_alert_thresholds(mut self, token: f64, cost: f64) -> Self {
        self.token_alert_threshold = token.clamp(0.0, 1.0);
        self.cost_alert_threshold = cost.clamp(0.0, 1.0);
        self
    }
}

/// Configuration for an agent's context budget.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextBudget {
    /// Agent this budget applies to.
    pub agent_id: AgentId,
    /// Model context window size used as the default token ceiling.
    pub model_max_tokens: usize,
    /// Tokens consumed in the current period toward the effective cap.
    pub tokens_used: usize,
    /// Cumulative cost in USD incurred by this agent.
    pub cost_usd: f64,
    /// Optional per-agent hard cap (None = use model_max_tokens as limit)
    pub allocation: Option<AgentBudgetAllocation>,
    /// Rolled-over bonus tokens from the previous period
    pub rollover_tokens: usize,
}

impl ContextBudget {
    /// Starts a budget with no allocation override and zero usage.
    pub fn new(agent_id: AgentId, max_tokens: usize) -> Self {
        Self {
            agent_id,
            model_max_tokens: max_tokens,
            tokens_used: 0,
            cost_usd: 0.0,
            allocation: None,
            rollover_tokens: 0,
        }
    }

    /// Token ceiling including optional allocation override and rollover bonus.
    pub fn effective_max_tokens(&self) -> usize {
        let base = self
            .allocation
            .as_ref()
            .map(|a| a.max_tokens)
            .unwrap_or(self.model_max_tokens);
        base.saturating_add(self.rollover_tokens)
    }

    /// Remaining tokens before hitting the effective cap.
    pub fn tokens_available(&self) -> usize {
        self.effective_max_tokens().saturating_sub(self.tokens_used)
    }

    /// Returns true when usage crosses the configured token alert threshold.
    pub fn should_summarize(&self) -> bool {
        let threshold = self
            .allocation
            .as_ref()
            .map(|a| a.token_alert_threshold)
            .unwrap_or(0.8);
        self.tokens_used as f64 > (self.effective_max_tokens() as f64 * threshold)
    }

    /// True if token usage exceeds the alert threshold.
    pub fn token_alert(&self) -> bool {
        self.should_summarize()
    }

    /// True if cost exceeds the cost alert threshold.
    pub fn cost_alert(&self) -> bool {
        if let Some(ref alloc) = self.allocation {
            self.cost_usd > alloc.max_cost_usd * alloc.cost_alert_threshold
        } else {
            false
        }
    }

    /// True if cost has exceeded the allocation hard cap.
    pub fn cost_exceeded(&self) -> bool {
        if let Some(ref alloc) = self.allocation {
            self.cost_usd >= alloc.max_cost_usd
        } else {
            false
        }
    }

    /// Roll over unused budget for the next period.
    /// Returns the number of rollover tokens granted.
    pub fn rollover(&mut self) -> usize {
        let unused = self.tokens_available();
        let rollover = if let Some(ref alloc) = self.allocation {
            (unused as f64 * alloc.rollover_fraction).floor() as usize
        } else {
            0
        };
        // Reset usage for new period
        self.tokens_used = 0;
        self.rollover_tokens = rollover;
        rollover
    }
}

/// Unified budget signal for behavioral gating (tokens, cost, and attention).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BudgetSignal {
    /// Usage within normal operating range.
    Normal { usage_ratio: f64 },
    /// Token usage is high; consider summarization.
    HighLoad {
        usage_ratio: f64,
        tokens_remaining: usize,
    },
    /// Token usage is critical; block new work.
    Critical {
        usage_ratio: f64,
        tokens_remaining: usize,
    },
    /// USD cost cap exceeded.
    CostExceeded { cost_usd: f64, limit_usd: f64 },
    /// Attention budget high (> alert threshold).
    AttentionHigh {
        spent_ratio: f64,
        attention_remaining_ms: u64,
    },
    /// Attention budget fully exhausted.
    AttentionCritical {
        spent_ratio: f64,
        attention_remaining_ms: u64,
    },
}

/// Tracks agent context budgets globally.
#[derive(Clone, Default)]
pub struct BudgetManager {
    inner: Arc<std::sync::RwLock<HashMap<AgentId, ContextBudget>>>,
    pub db: Option<Arc<vox_db::VoxDb>>,
    /// Phase 15: session-level attention budget.
    attention: Arc<std::sync::RwLock<AttentionBudget>>,
    /// Phase 15: per-agent EWMA trust scores.
    trust_scores: Arc<std::sync::RwLock<HashMap<AgentId, AgentTrustScore>>>,
    /// Phase 16: continuous developer fatigue pacing.
    fatigue: Arc<std::sync::RwLock<FatigueMonitor>>,
}

impl BudgetManager {
    /// Creates an empty manager; call [`Self::reset`] before tracking an agent.
    pub fn new(db: Option<Arc<vox_db::VoxDb>>) -> Self {
        Self {
            inner: Arc::new(std::sync::RwLock::new(HashMap::new())),
            attention: Arc::new(std::sync::RwLock::new(AttentionBudget::default())),
            trust_scores: Arc::new(std::sync::RwLock::new(HashMap::new())),
            fatigue: Arc::new(std::sync::RwLock::new(FatigueMonitor::new())),
            db,
        }
    }

    /// Initialise the attention budget with a custom ceiling (call once at startup).
    pub fn init_attention(&self, max_attention_ms: u64) {
        sync_lock::rw_write(&*self.attention).max_attention_ms = max_attention_ms;
    }

    /// Register or reset an agent's budget.
    pub fn reset(&self, agent_id: AgentId, max_tokens: usize) {
        let mut map = sync_lock::rw_write(&*self.inner);
        map.insert(agent_id, ContextBudget::new(agent_id, max_tokens));
    }

    /// Set a per-agent allocation cap (overrides default limits).
    pub fn set_allocation(&self, agent_id: AgentId, allocation: AgentBudgetAllocation) {
        let mut map = sync_lock::rw_write(&*self.inner);
        let budget = map
            .entry(agent_id)
            .or_insert_with(|| ContextBudget::new(agent_id, allocation.max_tokens));
        budget.allocation = Some(allocation);
    }

    /// Record token usage for an agent.
    pub fn record_usage(&self, agent_id: AgentId, tokens: usize) {
        let mut map = sync_lock::rw_write(&*self.inner);
        if let Some(budget) = map.get_mut(&agent_id) {
            budget.tokens_used = budget.tokens_used.saturating_add(tokens);
        } else {
            let mut budget = ContextBudget::new(agent_id, 100_000);
            budget.tokens_used = tokens;
            map.insert(agent_id, budget);
        }
    }

    /// Record cost in USD for an agent (e.g., from an OpenRouter API call).
    pub fn record_cost(&self, agent_id: AgentId, cost_usd: f64) {
        let mut map = sync_lock::rw_write(&*self.inner);
        if let Some(budget) = map.get_mut(&agent_id) {
            budget.cost_usd += cost_usd;
        } else {
            let mut budget = ContextBudget::new(agent_id, 100_000);
            budget.cost_usd = cost_usd;
            map.insert(agent_id, budget);
        }
    }

    /// Check an agent's remaining budget.
    pub fn check_budget(&self, agent_id: AgentId) -> Option<ContextBudget> {
        let map = sync_lock::rw_read(&*self.inner);
        map.get(&agent_id).cloned()
    }

    /// Check if the agent is approaching context limits and should summarize.
    pub fn should_summarize(&self, agent_id: AgentId) -> bool {
        let map = sync_lock::rw_read(&*self.inner);
        map.get(&agent_id)
            .map(|b| b.should_summarize())
            .unwrap_or(false)
    }

    /// Get all agents that currently have an active cost or token alert.
    pub fn agents_in_alert(&self) -> Vec<(AgentId, bool, bool)> {
        let map = sync_lock::rw_read(&*self.inner);
        map.values()
            .filter(|b| b.token_alert() || b.cost_alert())
            .map(|b| (b.agent_id, b.token_alert(), b.cost_alert()))
            .collect()
    }

    /// Trigger a period rollover for all agents.
    /// Returns map of agent_id → rollover_tokens_granted.
    pub fn rollover_all(&self) -> HashMap<AgentId, usize> {
        let mut map = sync_lock::rw_write(&*self.inner);
        map.values_mut()
            .map(|b| (b.agent_id, b.rollover()))
            .collect()
    }

    /// Total cost across all agents.
    pub fn total_cost_usd(&self) -> f64 {
        let map = sync_lock::rw_read(&*self.inner);
        map.values().map(|b| b.cost_usd).sum()
    }

    /// Cumulative cost in USD for a specific agent.
    pub fn cost_usd(&self, agent_id: AgentId) -> f64 {
        let map = sync_lock::rw_read(&*self.inner);
        map.get(&agent_id).map(|b| b.cost_usd).unwrap_or(0.0)
    }

    // ── Phase 15: Attention budget ────────────────────────────────────────────

    /// Record one attention event. Updates EWMA interrupt frequency and budget counters.
    pub fn record_attention(&self, event: &AttentionEvent) {
        let mut att = sync_lock::rw_write(&*self.attention);
        att.total_requests += 1;
        att.spent_ms = att.spent_ms.saturating_add(event.cost_ms);
        match event.outcome {
            ApprovalOutcome::AutoApproved => att.auto_approved += 1,
            ApprovalOutcome::Rejected => att.rejected += 1,
            _ => {}
        }
        // EWMA interrupt frequency: freq_t = 0.2 × (1/gap_hr) + 0.8 × freq_{t-1}
        if att.last_interrupt_ms > 0 && event.outcome != ApprovalOutcome::AutoApproved {
            let gap_ms = event.timestamp_ms.saturating_sub(att.last_interrupt_ms);
            let gap_hours = gap_ms as f64 / 3_600_000.0;
            if gap_hours > 0.0 {
                let inst = 1.0 / gap_hours;
                att.interrupt_freq_per_hour = 0.2 * inst + 0.8 * att.interrupt_freq_per_hour;
            }
        }
        if event.outcome != ApprovalOutcome::AutoApproved {
            att.last_interrupt_ms = event.timestamp_ms;
        }
    }

    /// Current budget signal including the attention dimension.
    pub fn attention_signal(&self, alert_threshold: f64) -> BudgetSignal {
        let att = sync_lock::rw_read(&*self.attention);
        let ratio = att.spent_ratio();
        let remaining = att.max_attention_ms.saturating_sub(att.spent_ms);
        if ratio >= 1.0 {
            BudgetSignal::AttentionCritical {
                spent_ratio: ratio,
                attention_remaining_ms: 0,
            }
        } else if ratio > alert_threshold {
            BudgetSignal::AttentionHigh {
                spent_ratio: ratio,
                attention_remaining_ms: remaining,
            }
        } else {
            BudgetSignal::Normal { usage_ratio: ratio }
        }
    }

    /// Snapshot of the current attention budget (for MCP / serialization).
    pub fn attention_snapshot(&self) -> AttentionBudget {
        sync_lock::rw_read(&*self.attention).clone()
    }

    /// Add MCP Socrates questioning wall-time into global [`AttentionBudget::spent_ms`].
    ///
    /// Does not create an [`AttentionEvent`] (no interrupt EWMA); use for observability parity with
    /// `vox_mcp::ServerState::record_questioning_attention_spend` when mirroring is enabled.
    pub fn add_questioning_attention_debit_ms(&self, delta_ms: u64) {
        if delta_ms == 0 {
            return;
        }
        let mut att = sync_lock::rw_write(&*self.attention);
        att.spent_ms = att.spent_ms.saturating_add(delta_ms);
    }

    /// Update EWMA trust score for an agent. Returns the new trust score.
    /// `provisional_min` and `trusted_min` are read from `OrchestratorConfig`.
    pub fn record_trust_outcome(
        &self,
        agent_id: AgentId,
        success: bool,
        alpha: f64,
        provisional_min: u32,
        trusted_min: u32,
    ) -> f64 {
        let mut scores = sync_lock::rw_write(&*self.trust_scores);
        let entry = scores
            .entry(agent_id)
            .or_insert_with(|| AgentTrustScore::new(agent_id));
        entry.record_outcome(success, alpha, provisional_min, trusted_min)
    }

    /// Snapshot of all agent trust scores (for routing injection).
    pub fn trust_snapshot(&self) -> HashMap<AgentId, AgentTrustScore> {
        sync_lock::rw_read(&*self.trust_scores).clone()
    }

    // ── Phase 16: Fatigue and Pacing ───────────────────────────────────────────

    /// Record an IDE context switch. Evaluates cognitive thrashing and burnout.
    /// Returns a `FatigueEvent` if the human needs immediate pacing intervention.
    pub fn record_ide_context_switch(&self, timestamp_ms: u64) -> Option<FatigueEvent> {
        let mut f_mon = sync_lock::rw_write(&*self.fatigue);
        f_mon.record_context_switch(timestamp_ms);

        let att = sync_lock::rw_read(&*self.attention);
        f_mon.evaluate_fatigue(att.spent_ratio())
    }

    /// Evaluates whether the operator is currently fatigued (for Socratic Lockout logic).
    pub fn is_fatigued(&self) -> bool {
        let f_mon = sync_lock::rw_read(&*self.fatigue);
        let att = sync_lock::rw_read(&*self.attention);
        f_mon.evaluate_fatigue(att.spent_ratio()).is_some()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_tracking_and_summarization() {
        let manager = BudgetManager::new(None);
        let agent = AgentId(1);

        manager.reset(agent, 10_000);
        let b = manager.check_budget(agent).unwrap();
        assert_eq!(b.tokens_available(), 10_000);

        manager.record_usage(agent, 7_000);
        assert!(!manager.should_summarize(agent));

        manager.record_usage(agent, 2_000);
        // Total = 9_000, 90% full.
        assert!(manager.should_summarize(agent));
        assert_eq!(
            manager.check_budget(agent).unwrap().tokens_available(),
            1_000
        );
    }

    #[test]
    fn per_agent_allocation_cap() {
        let mgr = BudgetManager::new(None);
        let agent = AgentId(2);
        let alloc = AgentBudgetAllocation::new(5_000, 1.00).with_alert_thresholds(0.6, 0.8);
        mgr.set_allocation(agent, alloc);

        mgr.record_usage(agent, 3_001); // > 60% of 5000
        let b = mgr.check_budget(agent).unwrap();
        assert!(b.token_alert());
    }

    #[test]
    fn cost_alert_fires_above_threshold() {
        let mgr = BudgetManager::new(None);
        let agent = AgentId(3);
        let alloc = AgentBudgetAllocation::new(100_000, 10.0).with_alert_thresholds(0.8, 0.9);
        mgr.set_allocation(agent, alloc);
        mgr.record_cost(agent, 9.01); // 90.1% of $10.0
        let b = mgr.check_budget(agent).unwrap();
        assert!(b.cost_alert());
        assert!(!b.cost_exceeded());

        mgr.record_cost(agent, 1.0); // now > $10.0
        let b = mgr.check_budget(agent).unwrap();
        assert!(b.cost_exceeded());
    }

    #[test]
    fn rollover_grants_unused_tokens() {
        let mgr = BudgetManager::new(None);
        let agent = AgentId(4);
        let alloc = AgentBudgetAllocation::new(10_000, 5.0).with_rollover(0.5);
        mgr.set_allocation(agent, alloc);
        mgr.record_usage(agent, 4_000); // used 4000, 6000 remaining
        let rollovers = mgr.rollover_all();
        let granted = rollovers[&agent];
        assert_eq!(granted, 3_000, "50% of 6000 unused = 3000 rollover");

        // After rollover, usage resets but rollover_tokens = 3000
        let b = mgr.check_budget(agent).unwrap();
        assert_eq!(b.tokens_used, 0);
        assert_eq!(b.rollover_tokens, 3_000);
        assert_eq!(b.effective_max_tokens(), 13_000);
    }

    #[test]
    fn total_cost_aggregation() {
        let mgr = BudgetManager::new(None);
        mgr.record_cost(AgentId(1), 0.50);
        mgr.record_cost(AgentId(2), 0.75);
        mgr.record_cost(AgentId(3), 0.25);
        let total = mgr.total_cost_usd();
        assert!((total - 1.50).abs() < 1e-9);
    }

    #[test]
    fn attention_signal_escalates_to_high() {
        use crate::attention::{ApprovalTier, AttentionEventType};
        let mgr = BudgetManager::new(None);
        // Fill 75% of the default 1-hour budget (alert threshold 0.7)
        let cost_ms = (crate::attention::DEFAULT_ATTENTION_BUDGET_MS as f64 * 0.75) as u64;
        let event = AttentionEvent {
            agent_id: AgentId(1),
            task_id: None,
            event_type: AttentionEventType::CodeReview,
            tier: ApprovalTier::Review,
            cost_ms,
            outcome: ApprovalOutcome::Approved,
            trust_score_at_time: 0.5,
            effective_complexity: 5.0,
            decision_entropy_bits: 0.5,
            timestamp_ms: 1_000_000,
            channel: None,
            policy_reason: None,
        };
        mgr.record_attention(&event);
        let signal = mgr.attention_signal(0.7);
        assert!(
            matches!(signal, BudgetSignal::AttentionHigh { .. }),
            "expected AttentionHigh, got {:?}",
            signal
        );
    }

    #[test]
    fn attention_signal_normal_below_threshold() {
        let mgr = BudgetManager::new(None);
        let signal = mgr.attention_signal(0.7);
        assert!(matches!(signal, BudgetSignal::Normal { .. }));
    }

    #[test]
    fn questioning_attention_debit_updates_spent_ms() {
        let mgr = BudgetManager::new(None);
        let before = mgr.attention_snapshot().spent_ms;
        mgr.add_questioning_attention_debit_ms(500);
        assert_eq!(
            mgr.attention_snapshot().spent_ms,
            before.saturating_add(500)
        );
    }

    #[test]
    fn record_trust_outcome_updates_score() {
        let mgr = BudgetManager::new(None);
        let agent = AgentId(42);
        for _ in 0..10 {
            mgr.record_trust_outcome(agent, true, 0.2, 5, 20);
        }
        let snap = mgr.trust_snapshot();
        let score = snap[&agent].trust_score;
        assert!(
            score > 0.3,
            "trust score should increase after successes, got {score:.3}"
        );
    }
}
