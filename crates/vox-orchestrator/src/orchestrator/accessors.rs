use std::collections::HashMap;
use super::{AgentSummary, Orchestrator, OrchestratorStatus, TaskTraceStep};
use crate::affinity::FileAffinityMap;
use crate::bulletin::BulletinBoard;
use crate::config::OrchestratorConfig;
use crate::locks::FileLockManager;
use crate::queue::AgentQueue;
use crate::types::{AgentId, TaskId};

impl Orchestrator {
    pub fn status(&self) -> OrchestratorStatus {
        let agents: Vec<AgentSummary> = self
            .agents
            .iter()
            .map(|(id, queue)| AgentSummary {
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
                dynamic: self.dynamic_agents.contains(id),
                weighted_load: queue.weighted_load(),
                agent_session_id: queue.agent_session_id.clone(),
            })
            .collect();

        let dynamic_count = self.dynamic_agents.len();
        let reserved_count = self.agents.len().saturating_sub(dynamic_count);

        #[allow(unused_mut)]
        let mut total_weighted_load: f64 = agents.iter().map(|a| a.weighted_load).sum();

        // Integrate system resources if configured
        #[cfg(feature = "system-metrics")]
        if self.config.resource_weight > 0.0 {
            let cpu_usage = self.sys.global_cpu_usage() as f64 / 100.0;
            let mem_usage = self.sys.used_memory() as f64 / self.sys.total_memory().max(1) as f64;
            let mut resource_factor = cpu_usage * self.config.resource_cpu_multiplier
                + mem_usage * self.config.resource_mem_multiplier;
            if self.config.resource_exponent != 1.0 {
                resource_factor = resource_factor.powf(self.config.resource_exponent);
            }
            total_weighted_load *= 1.0 + (resource_factor * self.config.resource_weight);
        }

        let predicted_load = if self.load_history.is_empty() {
            total_weighted_load
        } else {
            let avg: f64 =
                self.load_history.iter().copied().sum::<f64>() / self.load_history.len() as f64;
            if self.load_history.len() >= 2 {
                let last = *self.load_history.back().unwrap();
                let trend = last - avg;
                (last + trend).max(0.0)
            } else {
                avg
            }
        };

        OrchestratorStatus {
            enabled: self.config.enabled,
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
    pub fn agent_queue(&self, agent_id: AgentId) -> Option<&AgentQueue> {
        self.agents.get(&agent_id)
    }

    /// Get a mutable reference to an agent's queue.
    pub fn get_agent_queue_mut(&mut self, agent_id: AgentId) -> Option<&mut AgentQueue> {
        self.agents.get_mut(&agent_id)
    }

    /// Get a reference to the budget manager.
    pub fn budget_manager(&self) -> &crate::budget::BudgetManager {
        &self.budget_manager
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &OrchestratorConfig {
        &self.config
    }

    /// Get a mutable reference to the configuration to allow run-time tuning.
    pub fn config_mut(&mut self) -> &mut OrchestratorConfig {
        &mut self.config
    }

    /// Get a reference to the bulletin board (for subscribing).
    pub fn bulletin(&self) -> &BulletinBoard {
        &self.bulletin
    }

    /// Get a mutable reference to the bulletin board (for publishing).
    pub fn bulletin_mut(&mut self) -> &mut BulletinBoard {
        &mut self.bulletin
    }

    /// Access the file affinity map.
    pub fn affinity_map(&self) -> &FileAffinityMap {
        &self.affinity_map
    }

    /// Access the QA Router.
    pub fn qa_router(&self) -> &crate::qa::QARouter {
        &self.qa_router
    }

    /// Access the file affinity map mutably.
    pub fn affinity_map_mut(&mut self) -> &mut FileAffinityMap {
        &mut self.affinity_map
    }

    /// Get a reference to the lock manager.
    pub fn lock_manager(&self) -> &FileLockManager {
        &self.lock_manager
    }

    /// List all agent IDs.
    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().copied().collect()
    }

    /// Get a reference to task → agent assignment map.
    pub fn task_assignments(&self) -> &HashMap<TaskId, AgentId> {
        &self.task_assignments
    }

    /// Get the lifecycle timeline for a task (ingress → route → outcome), if recorded.
    pub fn task_trace(&self, task_id: TaskId) -> Option<&Vec<TaskTraceStep>> {
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
    pub fn models(&self) -> &crate::models::ModelRegistry {
        &self.models
    }

    /// Mutable access for updating model registry overrides at runtime.
    pub fn models_mut(&mut self) -> &mut crate::models::ModelRegistry {
        &mut self.models
    }

    /// Access the event bus
    pub fn event_bus(&self) -> &crate::events::EventBus {
        &self.event_bus
    }

    /// Access the A2A message bus
    pub fn message_bus(&self) -> &crate::a2a::MessageBus {
        &self.message_bus
    }

    /// Access the A2A message bus mutably (for ack, etc.)
    pub fn message_bus_mut(&mut self) -> &mut crate::a2a::MessageBus {
        &mut self.message_bus
    }

    // -- JJ-inspired subsystem accessors --

    /// Access the auto-snapshot store.
    pub fn snapshot_store(&self) -> &crate::snapshot::SnapshotStore {
        &self.snapshot_store
    }

    /// Access the auto-snapshot store mutably.
    pub fn snapshot_store_mut(&mut self) -> &mut crate::snapshot::SnapshotStore {
        &mut self.snapshot_store
    }

    /// Access the operation log.
    pub fn oplog(&self) -> &crate::oplog::OpLog {
        &self.oplog
    }

    /// Access the operation log mutably.
    pub fn oplog_mut(&mut self) -> &mut crate::oplog::OpLog {
        &mut self.oplog
    }

    /// Access the conflict manager.
    pub fn conflict_manager(&self) -> &crate::conflicts::ConflictManager {
        &self.conflict_manager
    }

    /// Access the conflict manager mutably.
    pub fn conflict_manager_mut(&mut self) -> &mut crate::conflicts::ConflictManager {
        &mut self.conflict_manager
    }

    /// Access the workspace manager.
    pub fn workspace_manager(&self) -> &crate::workspace::WorkspaceManager {
        &self.workspace_manager
    }

    /// Access the workspace manager mutably.
    pub fn workspace_manager_mut(&mut self) -> &mut crate::workspace::WorkspaceManager {
        &mut self.workspace_manager
    }


}
