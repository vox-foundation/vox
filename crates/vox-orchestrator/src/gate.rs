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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
}

/// A gate that enforces budgets via the `BudgetManager`.
pub struct BudgetGate<'a> {
    budget_manager: &'a BudgetManager,
    usage_tracker: &'a UsageTracker<'a>,
}

impl<'a> BudgetGate<'a> {
    /// Wires budget caps together with persisted usage counters from Codex.
    pub fn new(budget_manager: &'a BudgetManager, usage_tracker: &'a UsageTracker<'a>) -> Self {
        Self {
            budget_manager,
            usage_tracker,
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
        // 1. Check in-memory budget
        if let Some(budget) = self.budget_manager.check_budget(agent_id) {
            if budget.cost_exceeded() {
                return GateResult::BudgetExceeded {
                    message: format!(
                        "Agent {} has exceeded its cost budget of ${:.2}",
                        agent_id,
                        budget.allocation.map(|a| a.max_cost_usd).unwrap_or(0.0)
                    ),
                };
            }
        }

        // 2. Check persisted usage tracker for rate limits
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
