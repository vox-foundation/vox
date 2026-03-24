use parking_lot::{RwLockReadGuard, RwLockWriteGuard};
use dashmap::DashMap;
use dashmap::mapref::one::{Ref, RefMut};
use super::{AgentSummary, Orchestrator, OrchestratorStatus, TaskTraceStep};
use crate::affinity::FileAffinityMap;
use crate::bulletin::BulletinBoard;
use crate::config::OrchestratorConfig;
use crate::locks::FileLockManager;
use crate::queue::AgentQueue;
use crate::types::{AgentId, TaskId};


impl Orchestrator {
    pub fn status(&self) -> OrchestratorStatus {
        let config = self.config.read();
        let agents: Vec<AgentSummary> = self
            .agents
            .iter()
            .map(|pair| {
                let id = pair.key();
                let queue = pair.value();
                AgentSummary {
                    id: *id,
                    name: queue.name.clone(),
                    queued: queue.len(),
                    urgent_count: queue.depth_by_priority(crate::types::TaskPriority::Urgent),
                    normal_count: queue.depth_by_priority(crate::types::TaskPriority::Normal),
                    background_count: queue.depth_by_priority(crate::types::TaskPriority::Background),
                    in_progress: queue.has_in_progress(),
                    completed: queue.completed_count(),
                    paused: queue.is_paused(),
                    owned_files: self.affinity_map.files_for_agent(*id).len(),
                    dynamic: self.dynamic_agents.contains_key(id),
                    weighted_load: queue.weighted_load(),
                    agent_session_id: queue.agent_session_id.clone(),
                }
            })
            .collect();

        let dynamic_count = self.dynamic_agents.len();
        let reserved_count = self.agents.len().saturating_sub(dynamic_count);

        #[allow(unused_mut)]
        let mut total_weighted_load: f64 = agents.iter().map(|a| a.weighted_load).sum();

        // Integrate system resources if configured
        #[cfg(feature = "system-metrics")]
        if config.resource_weight > 0.0 {
            let sys = self.sys.read();
            let cpu_usage = sys.global_cpu_usage() as f64 / 100.0;
            let mem_usage = sys.used_memory() as f64 / sys.total_memory().max(1) as f64;
            let mut resource_factor = cpu_usage * config.resource_cpu_multiplier
                + mem_usage * config.resource_mem_multiplier;
            if config.resource_exponent != 1.0 {
                resource_factor = resource_factor.powf(config.resource_exponent);
            }
            total_weighted_load *= 1.0 + (resource_factor * config.resource_weight);
        }

        let load_history = self.load_history.read();
        let predicted_load = if load_history.is_empty() {
            total_weighted_load
        } else {
            let avg: f64 =
                load_history.iter().copied().sum::<f64>() / load_history.len() as f64;
            if load_history.len() >= 2 {
                let last = *load_history.back().unwrap();
                let trend = last - avg;
                (last + trend).max(0.0)
            } else {
                avg
            }
        };

        OrchestratorStatus {
            enabled: config.enabled,
            agent_count: self.agents.len(),
            total_queued: agents.iter().map(|a| a.queued).sum(),
            total_in_progress: agents.iter().filter(|a| a.in_progress).count(),
            total_completed: agents.iter().map(|a| a.completed).sum(),
            locked_files: self.lock_manager.active_lock_count(),
            total_contention: self.lock_manager.contention_count(),
            total_weighted_load,
            predicted_load,
            reserved_agents: reserved_count,
            dynamic_agents: dynamic_count,
            context_entries: self.context_store.entries(),
            agents,
        }
    }

    /// Get a reference to an agent's queue.
    pub fn agent_queue(&self, agent_id: AgentId) -> Option<Ref<'_, AgentId, AgentQueue>> {
        self.agents.get(&agent_id)
    }

    /// Get a mutable reference to an agent's queue.
    pub fn get_agent_queue_mut(&self, agent_id: AgentId) -> Option<RefMut<'_, AgentId, AgentQueue>> {
        self.agents.get_mut(&agent_id)
    }

    /// Get a reference to the budget manager.
    pub fn budget_manager(&self) -> &crate::budget::BudgetManager {
        &self.budget_manager
    }

    /// Get a read guard to the configuration.
    pub fn config(&self) -> RwLockReadGuard<'_, OrchestratorConfig> {
        self.config.read()
    }

    /// Get a write guard to the configuration.
    pub fn config_mut(&self) -> RwLockWriteGuard<'_, OrchestratorConfig> {
        self.config.write()
    }

    /// Get a reference to the bulletin board (for subscribing).
    pub fn bulletin(&self) -> &BulletinBoard {
        &self.bulletin
    }

    /// Get a reference to the bulletin board.
    pub fn bulletin_mut(&self) -> &BulletinBoard {
        &self.bulletin
    }

    /// Access the file affinity map.
    pub fn affinity_map(&self) -> &FileAffinityMap {
        &self.affinity_map
    }

    /// Access the QA Router.
    pub fn qa_router(&self) -> &crate::qa::QARouter {
        &self.qa_router
    }

    /// Access the file affinity map.
    pub fn affinity_map_mut(&self) -> &FileAffinityMap {
        &self.affinity_map
    }

    /// Get a reference to the lock manager.
    pub fn lock_manager(&self) -> &FileLockManager {
        &self.lock_manager
    }

    /// List all agent IDs.
    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.iter().map(|pair| *pair.key()).collect()
    }

    /// List all tasks (queued or in-progress) from all agents.
    pub fn all_tasks(&self) -> Vec<crate::types::AgentTask> {
        let mut all = Vec::new();
        for pair in self.agents.iter() {
            let queue = pair.value();
            if let Some(task) = &queue.current_task() {
                all.push((*task).clone());
            }
            for task in queue.tasks() {
                all.push(task.clone());
            }
        }
        all
    }

    /// Find a specific task by its unique ID across all buffered queues.
    pub fn get_task(&self, task_id: TaskId) -> Option<crate::types::AgentTask> {
        for pair in self.agents.iter() {
            let queue = pair.value();
            if let Some(task) = queue.current_task() {
                if task.id == task_id {
                    return Some((*task).clone());
                }
            }
            if let Some(task) = queue.tasks().iter().find(|t| t.id == task_id) {
                return Some(task.clone());
            }
        }
        None
    }

    /// Get the execution status of a task by its ID.
    pub fn task_status(&self, task_id: TaskId) -> Option<crate::types::TaskStatus> {
        self.get_task(task_id).map(|t| t.status)
    }

    /// Get a reference to task → agent assignment map.
    pub fn task_assignments(&self) -> &DashMap<TaskId, AgentId> {
        &self.task_assignments
    }

    /// Get the lifecycle timeline for a task (ingress → route → outcome), if recorded.
    pub fn task_trace(&self, task_id: TaskId) -> Option<Ref<'_, TaskId, Vec<TaskTraceStep>>> {
        self.task_traces.get(&task_id)
    }

    /// Get a reference to the shared context store.
    pub fn context(&self) -> &crate::context::ContextStore {
        &self.context_store
    }

    /// Get a reference to the budget manager.
    pub fn budget(&self) -> &crate::budget::BudgetManager {
        &self.budget_manager
    }

    /// Get a reference to the summary manager.
    pub fn summary(&self) -> &crate::summary::SummaryManager {
        &self.summary_manager
    }

    /// Access the model registry.
    pub fn models(&self) -> RwLockReadGuard<'_, crate::models::ModelRegistry> {
        self.models.read()
    }

    /// Access the model registry.
    pub fn models_mut(&self) -> RwLockWriteGuard<'_, crate::models::ModelRegistry> {
        self.models.write()
    }

    /// Access the event bus
    pub fn event_bus(&self) -> &crate::events::EventBus {
        &self.event_bus
    }

    /// Get a read guard to the A2A message bus.
    pub fn message_bus(&self) -> RwLockReadGuard<'_, crate::a2a::MessageBus> {
        self.message_bus.read()
    }

    /// Get a write guard to the A2A message bus.
    pub fn message_bus_mut(&self) -> RwLockWriteGuard<'_, crate::a2a::MessageBus> {
        self.message_bus.write()
    }

    // -- JJ-inspired subsystem accessors --

    /// Get a read guard to the auto-snapshot store.
    pub fn snapshot_store(&self) -> RwLockReadGuard<'_, crate::snapshot::SnapshotStore> {
        self.snapshot_store.read()
    }

    /// Get a write guard to the auto-snapshot store.
    pub fn snapshot_store_mut(&self) -> RwLockWriteGuard<'_, crate::snapshot::SnapshotStore> {
        self.snapshot_store.write()
    }

    /// Get a read guard to the operation log.
    pub fn oplog(&self) -> RwLockReadGuard<'_, crate::oplog::OpLog> {
        self.oplog.read()
    }

    /// Get a write guard to the operation log.
    pub fn oplog_mut(&self) -> RwLockWriteGuard<'_, crate::oplog::OpLog> {
        self.oplog.write()
    }

    /// Get a read guard to the conflict manager.
    pub fn conflict_manager(&self) -> RwLockReadGuard<'_, crate::conflicts::ConflictManager> {
        self.conflict_manager.read()
    }

    /// Get a write guard to the conflict manager.
    pub fn conflict_manager_mut(&self) -> RwLockWriteGuard<'_, crate::conflicts::ConflictManager> {
        self.conflict_manager.write()
    }

    /// Get a read guard to the workspace manager.
    pub fn workspace_manager(&self) -> RwLockReadGuard<'_, crate::workspace::WorkspaceManager> {
        self.workspace_manager.read()
    }

    /// Get a write guard to the workspace manager.
    pub fn workspace_manager_mut(&self) -> RwLockWriteGuard<'_, crate::workspace::WorkspaceManager> {
        self.workspace_manager.write()
    }
}
