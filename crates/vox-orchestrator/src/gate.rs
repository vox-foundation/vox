//! Parity Gates for usage tracking and rate limiting.
//!
//! Inspired by Greater Fool's "Gates" system, this module provides
//! middleware to intercept AI requests and enforce budgets/limits.

use crate::budget::BudgetManager;
use crate::types::AgentId;
use crate::usage::{DEFAULT_RATE_LIMIT_RETRY_SECS, LlmUsageKey, UsageTracker};
use async_trait::async_trait;

/// A gate that can allow or deny an AI request.
#[async_trait]
pub trait Gate: Send + Sync {
    /// Check if the request is allowed.
    async fn allow(
        &self,
        agent_id: AgentId,
        usage: &LlmUsageKey,
        estimated_tokens: u64,
    ) -> GateResult;

    /// Record the actual usage after a successful request.
    async fn record_usage(
        &self,
        agent_id: AgentId,
        usage: &LlmUsageKey,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    );
}

/// Result of a gate check.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GateResult {
    /// Request allowed.
    Allowed,
    /// Request denied due to budget exhaustion.
    BudgetExceeded {
        /// Human-readable explanation shown to the caller.
        message: String,
    },
    /// Request denied due to rate limiting.
    RateLimited {
        /// Hint for when to retry; `None` if the backend did not specify.
        retry_after_secs: Option<u64>,
    },
    /// Request denied because the pilot's attention budget is exhausted (Phase 15).
    AttentionExhausted {
        /// Human-readable explanation.
        message: String,
        /// Milliseconds of attention consumed this session.
        spent_ms: u64,
        /// Configured maximum for this session.
        max_ms: u64,
    },
}

/// A gate that enforces budgets via the `BudgetManager`.
pub struct BudgetGate<'a> {
    budget_manager: &'a BudgetManager,
    usage_tracker: &'a UsageTracker<'a>,
}

impl<'a> BudgetGate<'a> {
    /// Wires budget caps together with persisted usage counters from Codex.
    pub fn new(budget_manager: &'a BudgetManager, usage_tracker: &'a UsageTracker<'a>) -> Self {
        Self { budget_manager, usage_tracker }
    }

    /// Static check for in-memory token/cost budget. Stateless; no DB access.
    pub fn check(
        manager: &BudgetManager,
        agent_id: AgentId,
        _config: &crate::config::OrchestratorConfig,
    ) -> GateResult {
        if let Some(budget) = manager.check_budget(agent_id) {
            if budget.cost_exceeded() {
                return GateResult::BudgetExceeded {
                    message: format!(
                        "Cost budget exceeded: ${:.4} used (cap: ${:.4})",
                        budget.cost_usd,
                        budget.allocation.as_ref().map(|a| a.max_cost_usd).unwrap_or(0.0)
                    ),
                };
            }
            if budget.tokens_available() == 0 {
                let cap = budget.effective_max_tokens();
                return GateResult::BudgetExceeded {
                    message: format!(
                        "Token budget exceeded: {} of {} tokens used",
                        budget.tokens_used, cap
                    ),
                };
            }
        }
        GateResult::Allowed
    }

    /// Check whether the pilot's attention budget allows a new interrupt.
    /// Returns `GateResult::Allowed` when `attention_enabled = false` (shadow mode).
    pub fn check_attention(
        manager: &BudgetManager,
        config: &crate::config::OrchestratorConfig,
    ) -> GateResult {
        if !config.attention_enabled {
            return GateResult::Allowed;
        }
        let snap = manager.attention_snapshot();
        if snap.exhausted() {
            GateResult::AttentionExhausted {
                message: format!(
                    "Attention budget exhausted: {}ms of {}ms used this session. \
                     Consider a break before continuing.",
                    snap.spent_ms, snap.max_attention_ms
                ),
                spent_ms: snap.spent_ms,
                max_ms: snap.max_attention_ms,
            }
        } else {
            GateResult::Allowed
        }
    }
}

#[async_trait]
impl<'a> Gate for BudgetGate<'a> {
    async fn allow(
        &self,
        agent_id: AgentId,
        usage: &LlmUsageKey,
        _estimated_tokens: u64,
    ) -> GateResult {
        // 1. In-memory token/cost budget check
        let result = BudgetGate::check(
            self.budget_manager,
            agent_id,
            &crate::config::OrchestratorConfig::default(),
        );
        if result != GateResult::Allowed {
            return result;
        }

        // 2. Persisted usage tracker for rate limits
        let budgets = match self.usage_tracker.remaining_all().await {
            Ok(b) => b,
            Err(_) => return GateResult::Allowed, // Fail open if DB is down
        };

        if let Some(b) = budgets
            .iter()
            .find(|b| b.provider == usage.provider && b.model == usage.model)
        {
            if b.rate_limited {
                return GateResult::RateLimited {
                    retry_after_secs: Some(DEFAULT_RATE_LIMIT_RETRY_SECS),
                };
            }
            if b.remaining == 0 {
                return GateResult::BudgetExceeded {
                    message: format!("Provider {}/{} daily limit reached", b.provider, b.model),
                };
            }
        }

        GateResult::Allowed
    }

    async fn record_usage(
        &self,
        agent_id: AgentId,
        usage: &LlmUsageKey,
        tokens_in: u64,
        tokens_out: u64,
        cost_usd: f64,
    ) {
        // Record in memory (budget manager)
        self.budget_manager
            .record_usage(agent_id, (tokens_in + tokens_out) as usize);
        self.budget_manager.record_cost(agent_id, cost_usd);

        // Record in DB (usage tracker) using the same keys as [`LIMITS`].
        let _ = self
            .usage_tracker
            .record_call(
                &usage.provider,
                &usage.model,
                tokens_in,
                tokens_out,
                cost_usd,
            )
            .await;
    }
}
