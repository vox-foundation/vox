use std::sync::Arc;
use std::collections::{HashMap, VecDeque};
use crate::attention::{AgentTrustScore, ApprovalOutcome, AttentionBudget, AttentionEvent};
use crate::fatigue_monitor::{FatigueEvent, FatigueMonitor};
use crate::sync_lock;
use crate::types::AgentId;

/// Per-agent budget allocation cap.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentBudgetAllocation {
    pub max_tokens: usize,
    pub max_cost_usd: f64,
    pub token_alert_threshold: f64,
    pub cost_alert_threshold: f64,
    pub rollover_fraction: f64,
}

impl AgentBudgetAllocation {
    pub fn new(max_tokens: usize, max_cost_usd: f64) -> Self {
        Self {
            max_tokens,
            max_cost_usd,
            token_alert_threshold: 0.8,
            cost_alert_threshold: 0.9,
            rollover_fraction: 0.0,
        }
    }

    pub fn with_rollover(mut self, fraction: f64) -> Self {
        self.rollover_fraction = fraction.clamp(0.0, 1.0);
        self
    }

    pub fn with_alert_thresholds(mut self, token: f64, cost: f64) -> Self {
        self.token_alert_threshold = token.clamp(0.0, 1.0);
        self.cost_alert_threshold = cost.clamp(0.0, 1.0);
        self
    }
}

/// Configuration for an agent's context budget.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextBudget {
    pub agent_id: AgentId,
    pub model_max_tokens: usize,
    pub tokens_used: usize,
    pub cost_usd: f64,
    pub allocation: Option<AgentBudgetAllocation>,
    pub rollover_tokens: usize,
}

impl ContextBudget {
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

    pub fn effective_max_tokens(&self) -> usize {
        let base = self
            .allocation
            .as_ref()
            .map(|a| a.max_tokens)
            .unwrap_or(self.model_max_tokens);
        base.saturating_add(self.rollover_tokens)
    }

    pub fn tokens_available(&self) -> usize {
        self.effective_max_tokens().saturating_sub(self.tokens_used)
    }

    pub fn should_summarize(&self) -> bool {
        let threshold = self
            .allocation
            .as_ref()
            .map(|a| a.token_alert_threshold)
            .unwrap_or(0.8);
        self.tokens_used as f64 > (self.effective_max_tokens() as f64 * threshold)
    }

    pub fn token_alert(&self) -> bool {
        self.should_summarize()
    }

    pub fn cost_alert(&self) -> bool {
        if let Some(ref alloc) = self.allocation {
            self.cost_usd > alloc.max_cost_usd * alloc.cost_alert_threshold
        } else {
            false
        }
    }

    pub fn cost_exceeded(&self) -> bool {
        if let Some(ref alloc) = self.allocation {
            self.cost_usd >= alloc.max_cost_usd
        } else {
            false
        }
    }

    pub fn rollover(&mut self) -> usize {
        let unused = self.tokens_available();
        let rollover = if let Some(ref alloc) = self.allocation {
            (unused as f64 * alloc.rollover_fraction).floor() as usize
        } else {
            0
        };
        self.tokens_used = 0;
        self.rollover_tokens = rollover;
        rollover
    }
}

/// Unified budget signal for behavioral gating (tokens, cost, and attention).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BudgetSignal {
    Normal { usage_ratio: f64 },
    HighLoad {
        usage_ratio: f64,
        tokens_remaining: usize,
    },
    Critical {
        usage_ratio: f64,
        tokens_remaining: usize,
    },
    CostExceeded { cost_usd: f64, limit_usd: f64 },
    AttentionHigh {
        spent_ratio: f64,
        attention_remaining_ms: u64,
    },
    AttentionCritical {
        spent_ratio: f64,
        attention_remaining_ms: u64,
    },
    ToolLatencyHigh {
        tool_key: String,
        recommended_budget_ms: u64,
        p90_ms: f64,
        timeout_rate: f64,
    },
    ToolLatencyUnknown {
        tool_key: String,
        default_budget_ms: u64,
    },
}

/// Tracks agent context budgets globally.
#[derive(Clone, Default)]
pub struct BudgetManager {
    pub(crate) inner: Arc<std::sync::RwLock<HashMap<AgentId, ContextBudget>>>,
    pub db: Arc<std::sync::RwLock<Option<Arc<vox_db::VoxDb>>>>,
    pub(crate) attention: Arc<std::sync::RwLock<AttentionBudget>>,
    pub(crate) attention_events: Arc<std::sync::RwLock<VecDeque<AttentionEvent>>>,
    pub(crate) trust_scores: Arc<std::sync::RwLock<HashMap<AgentId, AgentTrustScore>>>,
    pub(crate) fatigue: Arc<std::sync::RwLock<FatigueMonitor>>,
    pub(crate) max_financial_cost_micros: Arc<std::sync::atomic::AtomicI64>,
    pub(crate) global_financial_cost_micros: Arc<std::sync::atomic::AtomicI64>,
    pub(crate) execution_time_budget_multiplier: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) local_inference_tokens: Arc<std::sync::atomic::AtomicU64>,
}

impl BudgetManager {
    pub fn new(db: Option<Arc<vox_db::VoxDb>>) -> Self {
        Self {
            inner: Arc::new(std::sync::RwLock::new(HashMap::new())),
            attention: Arc::new(std::sync::RwLock::new(AttentionBudget::default())),
            attention_events: Arc::new(std::sync::RwLock::new(VecDeque::new())),
            trust_scores: Arc::new(std::sync::RwLock::new(HashMap::new())),
            fatigue: Arc::new(std::sync::RwLock::new(FatigueMonitor::new())),
            max_financial_cost_micros: Arc::new(std::sync::atomic::AtomicI64::new(50_000)),
            global_financial_cost_micros: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            execution_time_budget_multiplier: Arc::new(std::sync::atomic::AtomicU64::new(
                1.5f64.to_bits(),
            )),
            local_inference_tokens: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            db: Arc::new(std::sync::RwLock::new(db)),
        }
    }

    pub fn db(&self) -> Option<Arc<vox_db::VoxDb>> {
        sync_lock::rw_read(&*self.db).clone()
    }

    pub fn init_holistic_budgets(
        &self,
        max_attention_ms: u64,
        financial_cost_budget_micros: i64,
        execution_time_multiplier: f64,
    ) {
        self.init_attention(max_attention_ms);
        self.max_financial_cost_micros.store(
            financial_cost_budget_micros,
            std::sync::atomic::Ordering::Relaxed,
        );
        self.execution_time_budget_multiplier.store(
            execution_time_multiplier.to_bits(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    pub fn init_attention(&self, max_attention_ms: u64) {
        sync_lock::rw_write(&*self.attention).max_attention_ms = max_attention_ms;
    }

    pub fn reset_attention(&self) {
        let mut att = sync_lock::rw_write(&*self.attention);
        att.spent_ms = 0;
    }

    pub fn reset(&self, agent_id: AgentId, max_tokens: usize) {
        let mut map = sync_lock::rw_write(&*self.inner);
        map.insert(agent_id, ContextBudget::new(agent_id, max_tokens));
    }

    pub fn set_allocation(&self, agent_id: AgentId, allocation: AgentBudgetAllocation) {
        let mut map = sync_lock::rw_write(&*self.inner);
        let budget = map
            .entry(agent_id)
            .or_insert_with(|| ContextBudget::new(agent_id, allocation.max_tokens));
        budget.allocation = Some(allocation);
    }

    pub fn execution_time_budget_multiplier(&self) -> f64 {
        f64::from_bits(
            self.execution_time_budget_multiplier
                .load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    pub fn max_financial_cost_micros(&self) -> i64 {
        self.max_financial_cost_micros
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn global_financial_cost_micros(&self) -> i64 {
        self.global_financial_cost_micros
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn record_local_inference_tokens(&self, tokens: u64) {
        self.local_inference_tokens
            .fetch_add(tokens, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn local_inference_tokens(&self) -> u64 {
        self.local_inference_tokens
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn record_inbox_suppression(&self, count: u32) {
        let mut att = sync_lock::rw_write(&*self.attention);
        att.inbox_suppressed_count = att.inbox_suppressed_count.saturating_add(count);
    }

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

    pub fn record_cost(&self, agent_id: AgentId, cost_usd: f64) {
        let mut map = sync_lock::rw_write(&*self.inner);
        if let Some(budget) = map.get_mut(&agent_id) {
            budget.cost_usd += cost_usd;
        } else {
            let mut budget = ContextBudget::new(agent_id, 100_000);
            budget.cost_usd = cost_usd;
            map.insert(agent_id, budget);
        }
        let inc_micros = (cost_usd * 1_000_000.0).round() as i64;
        self.global_financial_cost_micros
            .fetch_add(inc_micros, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn check_budget(&self, agent_id: AgentId) -> Option<ContextBudget> {
        let map = sync_lock::rw_read(&*self.inner);
        map.get(&agent_id).cloned()
    }

    pub fn should_summarize(&self, agent_id: AgentId) -> bool {
        let map = sync_lock::rw_read(&*self.inner);
        map.get(&agent_id)
            .map(|b| b.should_summarize())
            .unwrap_or(false)
    }

    pub fn agents_in_alert(&self) -> Vec<(AgentId, bool, bool)> {
        let map = sync_lock::rw_read(&*self.inner);
        map.values()
            .filter(|b| b.token_alert() || b.cost_alert())
            .map(|b| (b.agent_id, b.token_alert(), b.cost_alert()))
            .collect()
    }

    pub fn agent_budget_signal(&self, agent_id: AgentId) -> BudgetSignal {
        let map = sync_lock::rw_read(&*self.inner);
        if let Some(b) = map.get(&agent_id) {
            let ratio = b.tokens_used as f64 / b.effective_max_tokens().max(1) as f64;
            if b.cost_exceeded() {
                BudgetSignal::CostExceeded {
                    cost_usd: b.cost_usd,
                    limit_usd: b.allocation.as_ref().map(|a| a.max_cost_usd).unwrap_or(0.0),
                }
            } else if ratio >= 1.0 {
                BudgetSignal::Critical {
                    usage_ratio: ratio,
                    tokens_remaining: 0,
                }
            } else if b.token_alert() {
                BudgetSignal::HighLoad {
                    usage_ratio: ratio,
                    tokens_remaining: b.tokens_available(),
                }
            } else if b.cost_alert() {
                BudgetSignal::HighLoad {
                    usage_ratio: ratio,
                    tokens_remaining: b.tokens_available(),
                }
            } else {
                BudgetSignal::Normal { usage_ratio: ratio }
            }
        } else {
            BudgetSignal::Normal { usage_ratio: 0.0 }
        }
    }

    pub fn rollover_all(&self) -> HashMap<AgentId, usize> {
        let mut map = sync_lock::rw_write(&*self.inner);
        map.values_mut()
            .map(|b| (b.agent_id, b.rollover()))
            .collect()
    }

    pub fn total_cost_usd(&self) -> f64 {
        let map = sync_lock::rw_read(&*self.inner);
        map.values().map(|b| b.cost_usd).sum()
    }

    pub fn cost_usd(&self, agent_id: AgentId) -> f64 {
        let map = sync_lock::rw_read(&*self.inner);
        map.get(&agent_id).map(|b| b.cost_usd).unwrap_or(0.0)
    }

    pub fn record_attention(&self, event: &AttentionEvent) {
        let mut att = sync_lock::rw_write(&*self.attention);
        att.total_requests += 1;
        att.spent_ms = att.spent_ms.saturating_add(event.cost_ms);
        match event.outcome {
            ApprovalOutcome::AutoApproved => att.auto_approved += 1,
            ApprovalOutcome::Rejected => att.rejected += 1,
            _ => {}
        }
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
        drop(att);

        let mut ring = sync_lock::rw_write(&*self.attention_events);
        if ring.len() >= 100 {
            ring.pop_back();
        }
        ring.push_front(event.clone());
    }

    pub fn attention_events_snapshot(&self, limit: usize) -> Vec<AttentionEvent> {
        let ring = sync_lock::rw_read(&*self.attention_events);
        ring.iter().take(limit).cloned().collect()
    }

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

    pub fn attention_snapshot(&self) -> AttentionBudget {
        sync_lock::rw_read(&*self.attention).clone()
    }

    pub fn add_questioning_attention_debit_ms(&self, delta_ms: u64) {
        if delta_ms == 0 {
            return;
        }
        let mut att = sync_lock::rw_write(&*self.attention);
        att.spent_ms = att.spent_ms.saturating_add(delta_ms);
    }

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

    pub fn trust_snapshot(&self) -> HashMap<AgentId, AgentTrustScore> {
        sync_lock::rw_read(&*self.trust_scores).clone()
    }

    pub fn force_trust_score(&self, agent_id: AgentId, score: f64) {
        let mut scores = sync_lock::rw_write(&*self.trust_scores);
        let entry = scores
            .entry(agent_id)
            .or_insert_with(|| AgentTrustScore::new(agent_id));
        entry.trust_score = score.clamp(0.0, 1.0);
        entry.is_override = true;
    }

    pub fn is_fatigued(&self) -> bool {
        let f_mon = sync_lock::rw_read(&*self.fatigue);
        let att = sync_lock::rw_read(&*self.attention);
        f_mon.evaluate_fatigue(att.spent_ratio()).is_some()
    }

    pub fn record_ide_context_switch(&self, timestamp_ms: u64) -> Option<FatigueEvent> {
        let mut f_mon = sync_lock::rw_write(&*self.fatigue);
        f_mon.record_context_switch(timestamp_ms);
        let att = sync_lock::rw_read(&*self.attention);
        f_mon.evaluate_fatigue(att.spent_ratio())
    }
}

mod persistence;

