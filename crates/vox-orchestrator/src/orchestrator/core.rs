//! Construction, database initialization, temporal context, and AI usage recording
//! for [`crate::orchestrator::Orchestrator`].
//!
//! Keeping these in a dedicated module reduces the size of [`super`] and keeps
//! all construction-time defaults in one place.



use crate::bulletin::BulletinBoard;
use crate::config::OrchestratorConfig;
use crate::groups::AffinityGroupRegistry;
use crate::locks::FileLockManager;
use crate::orchestrator::OrchestratorError;
use crate::scope::ScopeGuard;
use crate::types::{AgentId, AgentIdGenerator, TaskIdGenerator};
use crate::affinity::FileAffinityMap;
use dashmap::DashMap;

impl crate::orchestrator::Orchestrator {
    /// Create a new orchestrator with the given configuration.
    pub fn new(config: OrchestratorConfig) -> Self {
        let bulletin = BulletinBoard::new(config.bulletin_capacity);
        Self {
            config: parking_lot::RwLock::new(config.clone()),
            affinity_map: FileAffinityMap::new(),
            lock_manager: FileLockManager::new(),
            context_store: crate::context::ContextStore::new(),
            budget_manager: crate::budget::BudgetManager::new(),
            summary_manager: crate::summary::SummaryManager::new(),
            models: parking_lot::RwLock::new(crate::models::ModelRegistry::new()),
            bulletin,
            agents: DashMap::new(),
            groups: parking_lot::RwLock::new(AffinityGroupRegistry::defaults()),
            task_id_gen: TaskIdGenerator::new(),
            agent_id_gen: AgentIdGenerator::new(),
            task_assignments: DashMap::new(),
            qa_router: crate::qa::QARouter::new(),
            monitor: parking_lot::RwLock::new(crate::monitor::AiMonitor::new(
                config.continuation_cooldown_ms,
                config.max_auto_continuations,
                config.stale_threshold_ms,
            )),
            event_bus: crate::events::EventBus::new(1024),
            message_bus: parking_lot::RwLock::new(crate::a2a::MessageBus::new(100)),
            dynamic_agents: DashMap::new(),
            agent_handles: DashMap::new(),
            heartbeat_monitor: parking_lot::RwLock::new(crate::heartbeat::HeartbeatMonitor::new(config.stale_threshold_ms)),
            #[cfg(feature = "system-metrics")]
            sys: parking_lot::RwLock::new(sysinfo::System::new_all()),
            load_history: parking_lot::RwLock::new(std::collections::VecDeque::with_capacity(config.scaling_lookback_ticks)),
            scope_guard: parking_lot::RwLock::new(ScopeGuard::new(config.scope_enforcement)),
            task_traces: DashMap::new(),
            snapshot_store: parking_lot::RwLock::new(crate::snapshot::SnapshotStore::default()),
            oplog: parking_lot::RwLock::new(crate::oplog::OpLog::default()),
            conflict_manager: parking_lot::RwLock::new(crate::conflicts::ConflictManager::new()),
            workspace_manager: parking_lot::RwLock::new(crate::workspace::WorkspaceManager::new()),
            db: parking_lot::RwLock::new(None),
            last_rebalance_at: parking_lot::RwLock::new(None),
            last_activity_ms: std::sync::atomic::AtomicU64::new(crate::types::now_unix_ms()),
            remote_mesh_routing_hints: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Create an orchestrator with custom affinity groups.
    pub fn with_groups(config: OrchestratorConfig, groups: AffinityGroupRegistry) -> Self {
        let bulletin = BulletinBoard::new(config.bulletin_capacity);
        Self {
            config: parking_lot::RwLock::new(config.clone()),
            affinity_map: FileAffinityMap::new(),
            lock_manager: FileLockManager::new(),
            context_store: crate::context::ContextStore::new(),
            budget_manager: crate::budget::BudgetManager::new(),
            summary_manager: crate::summary::SummaryManager::new(),
            models: parking_lot::RwLock::new(crate::models::ModelRegistry::new()),
            bulletin,
            agents: DashMap::new(),
            groups: parking_lot::RwLock::new(groups),
            task_id_gen: TaskIdGenerator::new(),
            agent_id_gen: AgentIdGenerator::new(),
            task_assignments: DashMap::new(),
            qa_router: crate::qa::QARouter::new(),
            monitor: parking_lot::RwLock::new(crate::monitor::AiMonitor::new(
                config.continuation_cooldown_ms,
                config.max_auto_continuations,
                config.stale_threshold_ms,
            )),
            event_bus: crate::events::EventBus::new(1024),
            message_bus: parking_lot::RwLock::new(crate::a2a::MessageBus::new(100)),
            dynamic_agents: DashMap::new(),
            agent_handles: DashMap::new(),
            heartbeat_monitor: parking_lot::RwLock::new(crate::heartbeat::HeartbeatMonitor::new(config.stale_threshold_ms)),
            #[cfg(feature = "system-metrics")]
            sys: parking_lot::RwLock::new(sysinfo::System::new_all()),
            load_history: parking_lot::RwLock::new(std::collections::VecDeque::with_capacity(config.scaling_lookback_ticks)),
            scope_guard: parking_lot::RwLock::new(ScopeGuard::new(config.scope_enforcement)),
            task_traces: DashMap::new(),
            snapshot_store: parking_lot::RwLock::new(crate::snapshot::SnapshotStore::default()),
            oplog: parking_lot::RwLock::new(crate::oplog::OpLog::default()),
            conflict_manager: parking_lot::RwLock::new(crate::conflicts::ConflictManager::new()),
            workspace_manager: parking_lot::RwLock::new(crate::workspace::WorkspaceManager::new()),
            db: parking_lot::RwLock::new(None),
            last_rebalance_at: parking_lot::RwLock::new(None),
            last_activity_ms: std::sync::atomic::AtomicU64::new(crate::types::now_unix_ms()),
            remote_mesh_routing_hints: parking_lot::RwLock::new(Vec::new()),
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
        *self.db.write() = Some(db);
        Ok(())
    }

    /// Builder-style variant of [`Self::init_db`] (takes ownership, sets db, returns self).
    pub fn with_db(mut self, db: std::sync::Arc<vox_db::VoxDb>) -> Self {
        *self.db.get_mut() = Some(db);
        self
    }

    /// Access the underlying database handle if connected.
    pub fn db(&self) -> Option<std::sync::Arc<vox_db::VoxDb>> {
        self.db.read().clone()
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

    /// Run a HIR-interpreted durable workflow with a shared `Arc<Self>` handle.
    ///
    /// Feature-gated on `workflow-runtime`.
    #[cfg(feature = "workflow-runtime")]
    pub async fn run_workflow_arc(
        arc: std::sync::Arc<Self>,
        hir: &vox_compiler::hir::HirModule,
        workflow_name: &str,
        session_id: String,
    ) -> Result<Vec<serde_json::Value>, crate::workflow_runtime::WorkflowError> {
        use crate::orchestrator::workflow_bridge::OrchestratorWorkflowBridge;
        use crate::workflow_runtime::interpret_workflow_durable;

        let mut bridge = OrchestratorWorkflowBridge::new(arc, session_id);
        interpret_workflow_durable(hir, workflow_name, &mut bridge)
            .await
            .map_err(|e| crate::workflow_runtime::WorkflowError::InterpretError(e.to_string()))
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

        self.budget_manager
            .record_usage(agent_id, (input_tokens + output_tokens) as usize);
        self.budget_manager.record_cost(agent_id, cost_usd);

        self.oplog.write().record_ai_call(
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
