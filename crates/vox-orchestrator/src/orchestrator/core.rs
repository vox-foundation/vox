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
    /// Create a new orchestrator with the given configuration.
    pub fn new(config: OrchestratorConfig) -> Self {
        let bulletin = BulletinBoard::new(config.bulletin_capacity);
        Self {
            config: config.clone(),
            affinity_map: FileAffinityMap::new(),
            lock_manager: FileLockManager::new(),
            context_store: crate::context::ContextStore::new(),
            budget_manager: crate::budget::BudgetManager::new(),
            summary_manager: crate::summary::SummaryManager::new(),
            models: crate::models::ModelRegistry::new(),
            bulletin,
            agents: HashMap::new(),
            groups: AffinityGroupRegistry::defaults(),
            task_id_gen: TaskIdGenerator::new(),
            agent_id_gen: AgentIdGenerator::new(),
            task_assignments: HashMap::new(),
            qa_router: crate::qa::QARouter::new(),
            monitor: crate::monitor::AiMonitor::new(
                config.continuation_cooldown_ms,
                config.max_auto_continuations,
                config.stale_threshold_ms,
            ),
            event_bus: crate::events::EventBus::new(1024),
            message_bus: crate::a2a::MessageBus::new(100),
            dynamic_agents: std::collections::HashSet::new(),
            agent_handles: HashMap::new(),
            heartbeat_monitor: crate::heartbeat::HeartbeatMonitor::new(config.stale_threshold_ms),
            #[cfg(feature = "system-metrics")]
            sys: sysinfo::System::new_all(),
            load_history: std::collections::VecDeque::with_capacity(config.scaling_lookback_ticks),
            scope_guard: ScopeGuard::new(config.scope_enforcement),
            task_traces: HashMap::new(),
            snapshot_store: crate::snapshot::SnapshotStore::default(),
            oplog: crate::oplog::OpLog::default(),
            conflict_manager: crate::conflicts::ConflictManager::new(),
            workspace_manager: crate::workspace::WorkspaceManager::new(),
            db: None,
            last_rebalance_at: None,
            last_activity_ms: std::sync::atomic::AtomicU64::new(crate::types::now_unix_ms()),
            remote_mesh_routing_hints: Vec::new(),
        }
    }

    /// Create an orchestrator with custom affinity groups.
    pub fn with_groups(config: OrchestratorConfig, groups: AffinityGroupRegistry) -> Self {
        let bulletin = BulletinBoard::new(config.bulletin_capacity);
        Self {
            config: config.clone(),
            affinity_map: FileAffinityMap::new(),
            lock_manager: FileLockManager::new(),
            context_store: crate::context::ContextStore::new(),
            budget_manager: crate::budget::BudgetManager::new(),
            summary_manager: crate::summary::SummaryManager::new(),
            models: crate::models::ModelRegistry::new(),
            bulletin,
            agents: HashMap::new(),
            groups,
            task_id_gen: TaskIdGenerator::new(),
            agent_id_gen: AgentIdGenerator::new(),
            task_assignments: HashMap::new(),
            qa_router: crate::qa::QARouter::new(),
            monitor: crate::monitor::AiMonitor::new(
                config.continuation_cooldown_ms,
                config.max_auto_continuations,
                config.stale_threshold_ms,
            ),
            event_bus: crate::events::EventBus::new(1024),
            message_bus: crate::a2a::MessageBus::new(100),
            dynamic_agents: std::collections::HashSet::new(),
            agent_handles: HashMap::new(),
            heartbeat_monitor: crate::heartbeat::HeartbeatMonitor::new(config.stale_threshold_ms),
            #[cfg(feature = "system-metrics")]
            sys: sysinfo::System::new_all(),
            load_history: std::collections::VecDeque::with_capacity(config.scaling_lookback_ticks),
            scope_guard: ScopeGuard::new(config.scope_enforcement),
            task_traces: HashMap::new(),
            snapshot_store: crate::snapshot::SnapshotStore::default(),
            oplog: crate::oplog::OpLog::default(),
            conflict_manager: crate::conflicts::ConflictManager::new(),
            workspace_manager: crate::workspace::WorkspaceManager::new(),
            db: None,
            last_rebalance_at: None,
            last_activity_ms: std::sync::atomic::AtomicU64::new(crate::types::now_unix_ms()),
            remote_mesh_routing_hints: Vec::new(),
        }
    }

    /// Initialize the orchestrator database schema and set the DB handle.
    pub async fn init_db(
        &mut self,
        db: std::sync::Arc<vox_db::VoxDb>,
    ) -> Result<(), OrchestratorError> {
        db.sync_schema_from_digest(&crate::schema::orchestrator_schema())
            .await
            .map_err(|e| OrchestratorError::DatabaseError(format!("DB sync failed: {}", e)))?;
        self.db = Some(db);
        Ok(())
    }

    /// Builder-style variant of [`Self::init_db`] (takes ownership, sets db, returns self).
    pub fn with_db(mut self, db: std::sync::Arc<vox_db::VoxDb>) -> Self {
        self.db = Some(db);
        self
    }

    /// Access the underlying database handle if connected.
    pub fn db(&self) -> Option<&vox_db::VoxDb> {
        self.db.as_deref()
    }

    /// Update the global activity timestamp.
    pub fn record_activity(&self) {
        self.last_activity_ms.store(crate::types::now_unix_ms(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Get the global last activity timestamp in milliseconds.
    pub fn last_activity_ms(&self) -> u64 {
        self.last_activity_ms.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Access the internal context store.
    pub fn context_store(&self) -> &crate::context::ContextStore {
        &self.context_store
    }

    /// Build temporal context string for system-prompt injection.
    ///
    /// Combines the session-level temporal summary with the task's wall-clock age.
    pub fn build_temporal_context(
        session: &crate::session::Session,
        task: &crate::types::AgentTask,
    ) -> String {
        let mut base = session.temporal_summary();
        let elapsed_secs = task.created_at
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
        &mut self,
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

        self.budget_manager
            .record_usage(agent_id, (input_tokens + output_tokens) as usize);
        self.budget_manager.record_cost(agent_id, cost_usd);

        self.oplog.record_ai_call(
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
