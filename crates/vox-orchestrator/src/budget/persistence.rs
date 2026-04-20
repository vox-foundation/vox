use super::{AgentBudgetAllocation, BudgetManager};
use crate::types::AgentId;
use std::sync::Arc;

impl BudgetManager {
    /// Load the custom dollar caps or budgets powered by VoxDB.
    pub async fn load_user_configured_budget(&self, agent_id: AgentId) {
        if let Some(db) = self.db() {
            let key = format!("agent_budget.{}", agent_id.0);
            if let Ok(Some(val)) = db.get_user_preference("local_user", &key).await {
                if let Ok(alloc) = serde_json::from_str::<AgentBudgetAllocation>(&val) {
                    self.set_allocation(agent_id, alloc);
                }
            }
        }
    }

    /// Save the custom dollar caps or budgets powered by VoxDB.
    pub async fn set_and_persist_allocation(
        &self,
        agent_id: AgentId,
        allocation: AgentBudgetAllocation,
    ) {
        self.set_allocation(agent_id, allocation.clone());
        if let Some(db) = self.db() {
            let key = format!("agent_budget.{}", agent_id.0);
            if let Ok(val) = serde_json::to_string(&allocation) {
                let _ = db.set_user_preference("local_user", &key, &val).await;
            }
        }
    }

    /// Attach a database handle late.
    pub async fn attach_db(&self, db: Arc<vox_db::VoxDb>) {
        crate::sync_lock::rw_write(&*self.db).replace(db);
    }

    pub async fn query_tool_latency_signal(
        &self,
        tool_key: &str,
        repository_id: &str,
        timeout_rate_alert_threshold: f64,
        window_days: u32,
        safety_multiplier: f64,
        default_budget_ms: u64,
    ) -> super::BudgetSignal {
        let Some(db) = self.db() else {
            return super::BudgetSignal::ToolLatencyUnknown {
                tool_key: tool_key.to_string(),
                default_budget_ms,
            };
        };

        match db
            .query_tool_latency(tool_key, repository_id, window_days, safety_multiplier)
            .await
        {
            Ok(Some(profile)) => {
                if profile.timeout_rate > timeout_rate_alert_threshold {
                    super::BudgetSignal::ToolLatencyHigh {
                        tool_key: tool_key.to_string(),
                        recommended_budget_ms: profile.recommended_budget_ms,
                        p90_ms: profile.p90_ms,
                        timeout_rate: profile.timeout_rate,
                    }
                } else {
                    super::BudgetSignal::Normal {
                        usage_ratio: profile.timeout_rate,
                    }
                }
            }
            _ => super::BudgetSignal::ToolLatencyUnknown {
                tool_key: tool_key.to_string(),
                default_budget_ms,
            },
        }
    }

    pub async fn record_tool_execution_outcome(
        &self,
        tool_key: &str,
        repository_id: &str,
        duration_ms: u64,
        timed_out: bool,
        attempted_budget_ms: Option<u64>,
    ) {
        if let Some(db) = self.db() {
            if timed_out {
                let _ = db
                    .record_exec_timeout(tool_key, repository_id, duration_ms)
                    .await;
            } else {
                let record = vox_db::ExecTimeRecord {
                    tool_key,
                    repository_id,
                    duration_ms,
                    timeout_budget_ms: attempted_budget_ms,
                    compute_tokens_used: None,
                    vendor_cost_usd_micros: None,
                    attention_cost_ms: None,
                    outcome: vox_db::ExecOutcome::Success,
                };
                let _ = db.record_exec_time(&record).await;
            }
        }
    }
}
