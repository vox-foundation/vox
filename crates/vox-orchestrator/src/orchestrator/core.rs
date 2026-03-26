//! Construction, database initialization, temporal context, and AI usage recording
//! for [`crate::orchestrator::Orchestrator`].
//!
//! Keeping these in a dedicated module reduces the size of [`super`] and keeps
//! all construction-time defaults in one place.

use std::collections::HashMap;

use crate::affinity::FileAffinityMap;
use crate::bulletin::BulletinBoard;
use crate::config::OrchestratorConfig;
use crate::groups::AffinityGroupRegistry;
use crate::locks::FileLockManager;
use crate::orchestrator::OrchestratorError;
use crate::scope::ScopeGuard;
use crate::types::{AgentId, AgentIdGenerator, TaskIdGenerator};

impl crate::orchestrator::Orchestrator {
    pub fn new(config: OrchestratorConfig) -> Self {
        let bulletin = BulletinBoard::new(config.bulletin_capacity);
        Self {
            config: std::sync::Arc::new(std::sync::RwLock::new(config.clone())),
            affinity_map: FileAffinityMap::new(),
            lock_manager: FileLockManager::new(),
            context_store: std::sync::Arc::new(std::sync::RwLock::new(
                crate::context::ContextStore::new(),
            )),
            budget_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::budget::BudgetManager::new(),
            )),
            summary_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::summary::SummaryManager::new(),
            )),
            models: std::sync::Arc::new(
                std::sync::RwLock::new(crate::models::ModelRegistry::new()),
            ),
            bulletin,
            agents: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            groups: std::sync::Arc::new(std::sync::RwLock::new(AffinityGroupRegistry::defaults())),
            task_id_gen: TaskIdGenerator::new(),
            agent_id_gen: AgentIdGenerator::new(),
            task_assignments: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            qa_router: std::sync::Arc::new(std::sync::RwLock::new(crate::qa::QARouter::new())),
            monitor: std::sync::Arc::new(std::sync::RwLock::new(crate::monitor::AiMonitor::new(
                config.continuation_cooldown_ms,
                config.max_auto_continuations,
                config.stale_threshold_ms,
            ))),
            event_bus: crate::events::EventBus::new(1024),
            message_bus: crate::a2a::MessageBus::new(100),
            dynamic_agents: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashSet::new(),
            )),
            agent_handles: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            heartbeat_monitor: std::sync::Arc::new(std::sync::RwLock::new(
                crate::heartbeat::HeartbeatMonitor::new(config.stale_threshold_ms),
            )),
            #[cfg(feature = "system-metrics")]
            sys: std::sync::Arc::new(std::sync::RwLock::new(sysinfo::System::new_all())),
            load_history: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::VecDeque::with_capacity(config.scaling_lookback_ticks),
            )),
            scope_guard: std::sync::Arc::new(std::sync::RwLock::new(ScopeGuard::new(
                config.scope_enforcement,
            ))),
            task_traces: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            snapshot_store: std::sync::Arc::new(std::sync::RwLock::new(
                crate::snapshot::SnapshotStore::default(),
            )),
            oplog: std::sync::Arc::new(std::sync::RwLock::new(crate::oplog::OpLog::default())),
            conflict_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::conflicts::ConflictManager::new(),
            )),
            workspace_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::workspace::WorkspaceManager::new(),
            )),
            db: std::sync::Arc::new(std::sync::RwLock::new(None)),
            last_rebalance_at: std::sync::Arc::new(std::sync::RwLock::new(None)),
            last_activity_ms: std::sync::atomic::AtomicU64::new(crate::types::now_unix_ms()),
            remote_populi_routing_hints: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    pub fn with_groups(config: OrchestratorConfig, groups: AffinityGroupRegistry) -> Self {
        let bulletin = BulletinBoard::new(config.bulletin_capacity);
        Self {
            config: std::sync::Arc::new(std::sync::RwLock::new(config.clone())),
            affinity_map: FileAffinityMap::new(),
            lock_manager: FileLockManager::new(),
            context_store: std::sync::Arc::new(std::sync::RwLock::new(
                crate::context::ContextStore::new(),
            )),
            budget_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::budget::BudgetManager::new(),
            )),
            summary_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::summary::SummaryManager::new(),
            )),
            models: std::sync::Arc::new(
                std::sync::RwLock::new(crate::models::ModelRegistry::new()),
            ),
            bulletin,
            agents: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            groups: std::sync::Arc::new(std::sync::RwLock::new(groups)),
            task_id_gen: TaskIdGenerator::new(),
            agent_id_gen: AgentIdGenerator::new(),
            task_assignments: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            qa_router: std::sync::Arc::new(std::sync::RwLock::new(crate::qa::QARouter::new())),
            monitor: std::sync::Arc::new(std::sync::RwLock::new(crate::monitor::AiMonitor::new(
                config.continuation_cooldown_ms,
                config.max_auto_continuations,
                config.stale_threshold_ms,
            ))),
            event_bus: crate::events::EventBus::new(1024),
            message_bus: crate::a2a::MessageBus::new(100),
            dynamic_agents: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::HashSet::new(),
            )),
            agent_handles: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            heartbeat_monitor: std::sync::Arc::new(std::sync::RwLock::new(
                crate::heartbeat::HeartbeatMonitor::new(config.stale_threshold_ms),
            )),
            #[cfg(feature = "system-metrics")]
            sys: std::sync::Arc::new(std::sync::RwLock::new(sysinfo::System::new_all())),
            load_history: std::sync::Arc::new(std::sync::RwLock::new(
                std::collections::VecDeque::with_capacity(config.scaling_lookback_ticks),
            )),
            scope_guard: std::sync::Arc::new(std::sync::RwLock::new(ScopeGuard::new(
                config.scope_enforcement,
            ))),
            task_traces: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            snapshot_store: std::sync::Arc::new(std::sync::RwLock::new(
                crate::snapshot::SnapshotStore::default(),
            )),
            oplog: std::sync::Arc::new(std::sync::RwLock::new(crate::oplog::OpLog::default())),
            conflict_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::conflicts::ConflictManager::new(),
            )),
            workspace_manager: std::sync::Arc::new(std::sync::RwLock::new(
                crate::workspace::WorkspaceManager::new(),
            )),
            db: std::sync::Arc::new(std::sync::RwLock::new(None)),
            last_rebalance_at: std::sync::Arc::new(std::sync::RwLock::new(None)),
            last_activity_ms: std::sync::atomic::AtomicU64::new(crate::types::now_unix_ms()),
            remote_populi_routing_hints: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
        }
    }

    /// Initialize the orchestrator database schema and set the DB handle.
    pub async fn init_db(
        &self,
        db: std::sync::Arc<vox_db::VoxDb>,
    ) -> Result<(), OrchestratorError> {
        db.sync_schema_from_digest(&crate::schema::orchestrator_schema())
            .await
            .map_err(|e| OrchestratorError::DatabaseError(format!("DB sync failed: {}", e)))?;

        crate::sync_lock::rw_write(&*self.db).replace(db.clone());
        match db.sqlite_capabilities_snapshot().await {
            Ok(p) => {
                tracing::debug!(
                    journal_mode = %p.journal_mode,
                    foreign_keys_on = p.foreign_keys_on,
                    fts5_reported = p.fts5_reported,
                    "sqlite capabilities (orchestrator init_db)"
                );
            }
            Err(e) => {
                tracing::debug!(error = %e, "sqlite capability probe failed during orchestrator init_db");
            }
        }
        Ok(())
    }

    /// Builder-style variant of [`Self::init_db`] (takes ownership, sets db, returns self).
    pub fn with_db(self, db: std::sync::Arc<vox_db::VoxDb>) -> Self {
        crate::sync_lock::rw_write(&*self.db).replace(db);
        self
    }

    /// Access the underlying database handle if connected.
    pub fn db(&self) -> Option<std::sync::Arc<vox_db::VoxDb>> {
        crate::sync_lock::rw_read(&*self.db).clone()
    }

    /// Update the global activity timestamp.
    pub fn record_activity(&self) {
        self.last_activity_ms.store(
            crate::types::now_unix_ms(),
            std::sync::atomic::Ordering::Relaxed,
        );
    }

    /// Get the global last activity timestamp in milliseconds.
    pub fn last_activity_ms(&self) -> u64 {
        self.last_activity_ms
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Access the internal context store.
    pub fn context_store(&self) -> std::sync::Arc<std::sync::RwLock<crate::context::ContextStore>> {
        self.context_store.clone()
    }

    /// Build temporal context string for system-prompt injection.
    ///
    /// Combines the session-level temporal summary with the task's wall-clock age.
    pub fn build_temporal_context(
        session: &crate::session::Session,
        task: &crate::types::AgentTask,
    ) -> String {
        let mut base = session.temporal_summary();
        let elapsed_secs = task
            .created_at
            .map(|i| std::time::Instant::now().duration_since(i).as_secs())
            .unwrap_or_else(|| {
                crate::types::now_unix_ms().saturating_sub(task.created_at_ms) / 1000
            });
        base.push_str(&format!(" Task created: {}s ago.", elapsed_secs));
        base
    }

    /// Record a single AI model call: emits [`crate::events::AgentEventKind::CostIncurred`],
    /// updates the in-memory budget, and appends an oplog entry — all in one atomic call.
    ///
    /// This is the **single integration point** for cost/token tracking. Call it after every
    /// LLM API response; do not scatter individual updates across subsystems.
    pub fn record_ai_usage(
        &self,
        agent_id: AgentId,
        provider: impl Into<String> + Clone,
        model: impl Into<String> + Clone,
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f64,
    ) {
        let provider_str: String = provider.into();
        let model_str: String = model.into();

        let idle_ms = crate::types::now_unix_ms().saturating_sub(self.last_activity_ms());
        let temporal_context = serde_json::json!({
            "idle_secs": idle_ms / 1000,
            "date": chrono::Local::now().to_rfc3339(),
        });

        self.event_bus
            .emit(crate::events::AgentEventKind::CostIncurred {
                agent_id,
                provider: provider_str.clone(),
                model: model_str.clone(),
                input_tokens,
                output_tokens,
                cost_usd,
                temporal_context: Some(temporal_context),
            });

        {
            let budget = crate::sync_lock::rw_write(&*self.budget_manager);
            budget.record_usage(agent_id, (input_tokens + output_tokens) as usize);
            budget.record_cost(agent_id, cost_usd);
        }

        crate::sync_lock::rw_write(&*self.oplog).record_ai_call(
            agent_id,
            &provider_str,
            &model_str,
            input_tokens,
            output_tokens,
            cost_usd,
        );

        tracing::debug!(
            "AI usage recorded: agent={} {}/{} in={} out={} cost=${:.6}",
            agent_id,
            provider_str,
            model_str,
            input_tokens,
            output_tokens,
            cost_usd
        );
    }
}
