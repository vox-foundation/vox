use super::{AgentSummary, Orchestrator, OrchestratorStatus, TaskTraceStep};
use crate::affinity::FileAffinityMap;
use crate::bulletin::BulletinBoard;
use crate::config::OrchestratorConfig;
use crate::locks::FileLockManager;
use crate::queue::AgentQueue;
use crate::types::{AgentId, TaskId};
use std::collections::HashMap;

impl Orchestrator {
    pub fn status(&self) -> OrchestratorStatus {
        let agents_map = crate::sync_lock::rw_read(&self.agents);
        let dynamic_agents = crate::sync_lock::rw_read(&self.dynamic_agents);
        let agents: Vec<AgentSummary> = agents_map
            .iter()
            .map(|(id, queue_lock)| {
                let queue = crate::sync_lock::rw_read(queue_lock);
                AgentSummary {
                    id: *id,
                    name: queue.name.clone(),
                    queued: queue.len(),
                    urgent_count: queue.depth_by_priority(crate::types::TaskPriority::Urgent),
                    normal_count: queue.depth_by_priority(crate::types::TaskPriority::Normal),
                    background_count: queue
                        .depth_by_priority(crate::types::TaskPriority::Background),
                    in_progress: queue.has_in_progress(),
                    completed: queue.completed_count(),
                    paused: queue.is_paused(),
                    owned_files: self.affinity_map.files_for_agent(*id).len(),
                    dynamic: dynamic_agents.contains(id),
                    weighted_load: queue.weighted_load(),
                    agent_session_id: queue.agent_session_id.clone(),
                }
            })
            .collect();

        let dynamic_count = dynamic_agents.len();
        let reserved_count = agents_map.len().saturating_sub(dynamic_count);
        drop(agents_map);
        drop(dynamic_agents);

        #[allow(unused_mut)]
        let mut total_weighted_load: f64 = agents.iter().map(|a| a.weighted_load).sum();

        let config = crate::sync_lock::rw_read(&self.config);

        // Integrate system resources if configured
        #[cfg(feature = "system-metrics")]
        if config.resource_weight > 0.0 {
            let sys = crate::sync_lock::rw_read(&self.sys);
            let cpu_usage = sys.global_cpu_usage() as f64 / 100.0;
            let mem_usage = sys.used_memory() as f64 / sys.total_memory().max(1) as f64;
            let mut resource_factor = cpu_usage * config.resource_cpu_multiplier
                + mem_usage * config.resource_mem_multiplier;
            if config.resource_exponent != 1.0 {
                resource_factor = resource_factor.powf(config.resource_exponent);
            }
            total_weighted_load *= 1.0 + (resource_factor * config.resource_weight);
        }

        let history = crate::sync_lock::rw_read(&self.load_history);
        let predicted_load = if history.is_empty() {
            total_weighted_load
        } else {
            let avg: f64 = history.iter().copied().sum::<f64>() / history.len() as f64;
            if history.len() >= 2 {
                let last = *history.back().unwrap();
                let trend = last - avg;
                (last + trend).max(0.0)
            } else {
                avg
            }
        };
        drop(history);

        OrchestratorStatus {
            enabled: config.enabled,
            agent_count: crate::sync_lock::rw_read(&self.agents).len(),
            total_queued: agents.iter().map(|a| a.queued).sum(),
            total_in_progress: agents.iter().filter(|a| a.in_progress).count(),
            total_completed: agents.iter().map(|a| a.completed).sum(),
            locked_files: self.lock_manager.active_lock_count(),
            total_contention: self.lock_manager.contention_count(),
            total_weighted_load,
            predicted_load,
            reserved_agents: reserved_count,
            dynamic_agents: dynamic_count,
            context_entries: crate::sync_lock::rw_read(&self.context_store).entries(),
            agents,
        }
    }

    /// Get a shared lock to an agent's queue.
    pub fn agent_queue(
        &self,
        agent_id: AgentId,
    ) -> Option<std::sync::Arc<std::sync::RwLock<AgentQueue>>> {
        crate::sync_lock::rw_read(&self.agents)
            .get(&agent_id)
            .cloned()
    }

    /// Get a shared lock to an agent's queue (alias for agent_queue).
    pub fn get_agent_queue_mut(
        &self,
        agent_id: AgentId,
    ) -> Option<std::sync::Arc<std::sync::RwLock<AgentQueue>>> {
        self.agent_queue(agent_id)
    }

    /// Get a shared lock to the budget manager.
    pub fn budget_manager_handle(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::budget::BudgetManager>> {
        std::sync::Arc::clone(&self.budget_manager)
    }

    pub fn config_handle(&self) -> std::sync::Arc<std::sync::RwLock<OrchestratorConfig>> {
        std::sync::Arc::clone(&self.config)
    }

    /// Get a reference to the bulletin board (for subscribing).
    pub fn bulletin(&self) -> &BulletinBoard {
        &self.bulletin
    }

    /// No longer needed: bulletin is thread-safe.
    pub fn bulletin_mut(&self) -> &BulletinBoard {
        &self.bulletin
    }

    /// Access the file affinity map.
    pub fn affinity_map(&self) -> &FileAffinityMap {
        &self.affinity_map
    }

    /// Access the QA Router handle.
    pub fn qa_router_handle(&self) -> std::sync::Arc<std::sync::RwLock<crate::qa::QARouter>> {
        std::sync::Arc::clone(&self.qa_router)
    }

    /// Affinity map has internal locking.
    pub fn affinity_map_mut(&self) -> &FileAffinityMap {
        &self.affinity_map
    }

    /// Get a reference to the lock manager.
    pub fn lock_manager(&self) -> &FileLockManager {
        &self.lock_manager
    }

    /// List all agent IDs.
    pub fn agent_ids(&self) -> Vec<AgentId> {
        crate::sync_lock::rw_read(&self.agents)
            .keys()
            .copied()
            .collect()
    }

    /// List all tasks (queued or in-progress) from all agents.
    pub fn all_tasks(&self) -> Vec<crate::types::AgentTask> {
        let mut all = Vec::new();
        let agents = crate::sync_lock::rw_read(&self.agents);
        for queue_lock in agents.values() {
            let queue = crate::sync_lock::rw_read(queue_lock);
            if let Some(task) = &queue.current_task() {
                all.push((*task).clone());
            }
            for task in queue.tasks() {
                all.push(task.clone());
            }
        }
        all
    }

    /// Get a copy of the task assignments map.
    pub fn task_assignments_copy(&self) -> HashMap<TaskId, AgentId> {
        crate::sync_lock::rw_read(&self.task_assignments).clone()
    }

    /// Agent currently assigned this task, if any (used before completion clears routing state).
    pub fn agent_assigned_to_task(&self, task_id: TaskId) -> Option<AgentId> {
        crate::sync_lock::rw_read(&self.task_assignments)
            .get(&task_id)
            .copied()
    }

    /// Get the lifecycle timeline for a task (ingress → route → outcome), if recorded.
    pub fn task_trace(&self, task_id: TaskId) -> Option<Vec<TaskTraceStep>> {
        crate::sync_lock::rw_read(&self.task_traces)
            .get(&task_id)
            .cloned()
    }

    /// Get a handle to the shared context store.
    pub fn context_handle(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::context::ContextStore>> {
        std::sync::Arc::clone(&self.context_store)
    }

    /// Get a handle to the budget manager.
    pub fn budget_handle(&self) -> std::sync::Arc<std::sync::RwLock<crate::budget::BudgetManager>> {
        std::sync::Arc::clone(&self.budget_manager)
    }

    /// Get a handle to the summary manager.
    pub fn summary_handle(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::summary::SummaryManager>> {
        std::sync::Arc::clone(&self.summary_manager)
    }

    /// Access the model registry handle.
    pub fn models_handle(&self) -> std::sync::Arc<std::sync::RwLock<crate::models::ModelRegistry>> {
        std::sync::Arc::clone(&self.models)
    }

    /// Access the event bus
    pub fn event_bus(&self) -> &crate::events::EventBus {
        &self.event_bus
    }

    /// Access the A2A message bus
    pub fn message_bus(&self) -> &crate::a2a::MessageBus {
        &self.message_bus
    }

    /// Message bus has shared methods or can be locked internally if wrap is needed.
    pub fn message_bus_mut(&self) -> &crate::a2a::MessageBus {
        &self.message_bus
    }

    // -- JJ-inspired subsystem accessors --

    /// Access the auto-snapshot store handle.
    pub fn snapshot_store_handle(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::snapshot::SnapshotStore>> {
        std::sync::Arc::clone(&self.snapshot_store)
    }

    /// Alias for snapshot_store_handle.
    pub fn snapshot_store_mut(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::snapshot::SnapshotStore>> {
        self.snapshot_store_handle()
    }

    /// Access the operation log handle.
    pub fn oplog_handle(&self) -> std::sync::Arc<std::sync::RwLock<crate::oplog::OpLog>> {
        std::sync::Arc::clone(&self.oplog)
    }

    /// Alias for oplog_handle.
    pub fn oplog_mut(&self) -> std::sync::Arc<std::sync::RwLock<crate::oplog::OpLog>> {
        self.oplog_handle()
    }

    /// Access the conflict manager handle.
    pub fn conflict_manager_handle(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::conflicts::ConflictManager>> {
        std::sync::Arc::clone(&self.conflict_manager)
    }

    /// Alias for conflict_manager_handle.
    pub fn conflict_manager_mut(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::conflicts::ConflictManager>> {
        self.conflict_manager_handle()
    }

    /// Access the workspace manager handle.
    pub fn workspace_manager_handle(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::workspace::WorkspaceManager>> {
        std::sync::Arc::clone(&self.workspace_manager)
    }

    /// Alias for workspace_manager_handle.
    pub fn workspace_manager_mut(
        &self,
    ) -> std::sync::Arc<std::sync::RwLock<crate::workspace::WorkspaceManager>> {
        self.workspace_manager_handle()
    }
}
