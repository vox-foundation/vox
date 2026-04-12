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
            budget_manager: std::sync::Arc::new(std::sync::RwLock::new({
                let bm = crate::budget::BudgetManager::new(None);
                bm.init_holistic_budgets(
                    config.attention_budget_ms,
                    config.financial_cost_budget_micros,
                    config.execution_time_budget_multiplier,
                );
                bm
            })),
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
            agent_delegations: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            dynamic_spawn_context: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
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
            tavily_credits_used: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            remote_populi_routing_hints: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
            stop_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
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
            budget_manager: std::sync::Arc::new(std::sync::RwLock::new({
                let bm = crate::budget::BudgetManager::new(None);
                bm.init_holistic_budgets(
                    config.attention_budget_ms,
                    config.financial_cost_budget_micros,
                    config.execution_time_budget_multiplier,
                );
                bm
            })),
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
            agent_delegations: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            dynamic_spawn_context: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
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
            tavily_credits_used: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            remote_populi_routing_hints: std::sync::Arc::new(std::sync::RwLock::new(Vec::new())),
            stop_flag: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Check if the orchestrator is in an emergency stop state.
    pub fn is_stopped(&self) -> bool {
        self.stop_flag.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Trigger a global emergency stop across the orchestrator.
    pub fn emergency_stop(&self, reason: Option<String>) {
        self.stop_flag
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.event_bus
            .emit(crate::events::AgentEventKind::EmergencyStop { reason });
        tracing::warn!("Orchestrator emergency stop triggered.");
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

    /// Record a lineage event to the persistent Codex store asynchronously if attached.
    pub fn record_lineage_event(
        &self,
        kind: &str,
        task_id: Option<crate::TaskId>,
        agent_id: Option<crate::AgentId>,
        session_id: Option<String>,
        workflow_id: Option<String>,
        plan_session_id: Option<String>,
        plan_node_id: Option<String>,
        payload: Option<serde_json::Value>,
    ) {
        let Some(db) = self.db() else { return };
        let repo = crate::lineage::repository_id();
        let kind = kind.to_string();
        let tid = task_id.map(|t| t.0 as i64).unwrap_or(0);
        let aid = agent_id.map(|a| a.0 as i64);
        let payload_str = payload.map(|p| p.to_string());

        tokio::spawn(async move {
            if let Err(e) = db
                .append_orchestration_lineage_event(
                    &repo,
                    &kind,
                    tid,
                    aid,
                    session_id.as_deref(),
                    workflow_id.as_deref(),
                    plan_session_id.as_deref(),
                    plan_node_id.as_deref(),
                    payload_str.as_deref(),
                )
                .await
            {
                tracing::debug!(error = %e, "lineage persistence failed");
            }
        });
    }

    /// Laplace-smoothed task reliability from Codex `agent_reliability`, when DB is attached.
    pub fn lookup_agent_reliability_sync(&self, agent_id: crate::types::AgentId) -> Option<f64> {
        let db = self.db()?;
        let sid = agent_id.0.to_string();
        db.block_on(async { db.get_agent_reliability(&sid).await })
            .ok()
            .flatten()
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
    pub async fn record_ai_usage(
        &self,
        agent_id: AgentId,
        provider: impl Into<String> + Clone,
        model: impl Into<String> + Clone,
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f64,
        header_cost_usd: Option<f64>,
    ) {
        let provider_str: String = provider.into();
        let model_str: String = model.into();

        let mut breakeven_crossed = false;
        if cost_usd == 0.0 || provider_str == "ollama" || provider_str == "mens" {
            let budget = crate::sync_lock::rw_read(&*self.budget_manager);
            let prev = budget.local_inference_tokens();
            let total_tokens = (input_tokens + output_tokens) as u64;
            budget.record_local_inference_tokens(total_tokens);
            let current = prev + total_tokens;
            let threshold = crate::sync_lock::rw_read(&*self.config).local_breakeven_tokens;
            if prev <= threshold && current > threshold {
                breakeven_crossed = true;
            }
        }

        let idle_ms = crate::types::now_unix_ms().saturating_sub(self.last_activity_ms());
        let mut temporal_context = serde_json::json!({
            "idle_secs": idle_ms / 1000,
            "date": chrono::Local::now().to_rfc3339(),
        });
        if breakeven_crossed {
            temporal_context["local_breakeven_crossed"] = serde_json::json!(true);
        }

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

        if let Some(db) = self.db() {
            let tracker = crate::usage::UsageTracker::new_ref(&*db);
            let _ = tracker
                .record_call_detailed(
                    &provider_str,
                    &model_str,
                    input_tokens as u64,
                    output_tokens as u64,
                    header_cost_usd.unwrap_or(cost_usd),
                    None,
                    header_cost_usd,
                    Some(cost_usd),
                    header_cost_usd.or(Some(cost_usd)),
                    Some(if header_cost_usd.is_some() {
                        "openrouter_header"
                    } else {
                        "heuristic"
                    }),
                    None,
                    Some(&agent_id.to_string()),
                )
                .await;
        }

        let (op_id, entry_meta) = {
            let mut oplog = crate::sync_lock::rw_write(&*self.oplog);
            let op_id = oplog.record_ai_call(
                agent_id,
                &provider_str,
                &model_str,
                input_tokens,
                output_tokens,
                cost_usd,
            );
            let entry_meta = oplog.get(op_id).map(|entry| {
                (
                    entry.kind.clone(),
                    entry.description.clone(),
                    entry.predecessor_hash.clone(),
                    entry.model_id.clone(),
                    entry.change_id,
                    entry.timestamp_ms,
                )
            });
            (op_id, entry_meta)
        };
        self.persist_oplog_entry(agent_id, op_id, entry_meta).await;

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

    /// Record a flat telemetry event to `agent_telemetry_flat` in the database.
    ///
    /// This is the primary entry point for recording high-volume agent observations
    /// into the flattened table for SQL-based evaluation.
    pub async fn record_telemetry(
        &self,
        agent_id: AgentId,
        event_kind: &str,
        model_id: Option<&str>,
        provider: Option<&str>,
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
        cost_usd: Option<f64>,
        payload: Option<serde_json::Value>,
    ) {
        let Some(db) = self.db() else { return };
        let repo = crate::lineage::repository_id();
        let aid = agent_id.0.to_string();
        // TODO: Resolve session_id from task or context
        let sid = "canonical-session"; 
        let payload_json = payload.map(|p| p.to_string());

        let res = db
            .insert_telemetry_flat_raw(
                &aid,
                sid,
                &repo,
                event_kind,
                None, // tool_name
                model_id,
                provider,
                None, // duration_ms
                input_tokens.map(|t| t as i64),
                output_tokens.map(|t| t as i64),
                cost_usd,
                payload_json.as_deref(),
            )
            .await;

        if let Err(e) = res {
            tracing::warn!(error = %e, kind = event_kind, "failed to record flat telemetry; outbox enqueue pending");
            // Hardening: Enqueue to outbox for retry if DB is busy/down
            self.enqueue_telemetry_outbox(
                agent_id,
                sid,
                event_kind,
                model_id,
                provider,
                input_tokens,
                output_tokens,
                cost_usd,
                payload_json.as_deref(),
            ).await;
        }
    }

    async fn enqueue_telemetry_outbox(
        &self,
        agent_id: AgentId,
        session_id: &str,
        event_kind: &str,
        model_id: Option<&str>,
        provider: Option<&str>,
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
        cost_usd: Option<f64>,
        payload_json: Option<&str>,
    ) {
        let store = crate::sync_lock::rw_read(&*self.context_store);
        let key = crate::orchestrator::persistence_outbox::PERSISTENCE_OUTBOX_KEY.to_string();
        let mut queue = store
            .get(&key)
            .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
            .unwrap_or_default();

        let entry = serde_json::json!({
            "lane": "telemetry/flat",
            "error": "db_unavailable",
            "first_seen_unix_ms": crate::types::now_unix_ms(),
            "retry_count": 0,
            "replay": {
                "op": "insert_telemetry_flat_raw",
                "agent_id": agent_id.0.to_string(),
                "session_id": session_id,
                "event_kind": event_kind,
                "model_id": model_id,
                "provider": provider,
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
                "cost_usd": cost_usd,
                "payload_json": payload_json,
            }
        });

        queue.push(entry);
        if let Ok(raw) = serde_json::to_string(&queue) {
            store.set(AgentId(0), key, raw, 0);
        }
    }
}
