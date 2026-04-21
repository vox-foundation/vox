use crate::orchestrator::OrchestratorError;
use crate::types::AgentId;

impl crate::orchestrator::Orchestrator {
    /// Replace cached remote mens capability hints (from a background mens poll).
    pub fn set_remote_populi_routing_hints(
        &self,
        hints: Vec<crate::populi_federation::RemotePopuliRoutingHint>,
    ) -> crate::populi_federation::PopuliRoutingHintUpdate {
        let new_nodes = hints.len();
        let new_schedulable = hints
            .iter()
            .filter(|h| h.is_federation_schedulable())
            .count();
        let new_gpu_eligible = hints
            .iter()
            .filter(|h| h.is_federation_gpu_eligible())
            .count();

        let mut slot = crate::sync_lock::rw_write(&*self.remote_populi_routing_hints);
        let prev = slot.clone();
        let prev_schedulable = prev
            .iter()
            .filter(|h| h.is_federation_schedulable())
            .count();
        let prev_gpu_eligible = prev
            .iter()
            .filter(|h| h.is_federation_gpu_eligible())
            .count();
        let prev_nodes = prev.len();
        *slot = hints;
        drop(slot);

        let populi_exp = crate::sync_lock::rw_read(&*self.config).populi_routing_experimental;
        if populi_exp && prev_schedulable > new_schedulable {
            tracing::info!(
                target: "vox.orchestrator.routing",
                decision = "populi_remote_schedulable_decreased",
                prev_schedulable,
                new_schedulable,
                prev_nodes,
                new_nodes,
                "populi federation: schedulable remote node count dropped"
            );
        }
        tracing::debug!(
            target: "vox.orchestrator.routing",
            remote_nodes = new_nodes,
            remote_schedulable = new_schedulable,
            remote_gpu_eligible = new_gpu_eligible,
            "populi federation hint snapshot applied"
        );
        crate::populi_federation::PopuliRoutingHintUpdate {
            prev_schedulable,
            new_schedulable,
            prev_gpu_eligible,
            new_gpu_eligible,
        }
    }

    /// Map an AI agent session ID to an existing orchestrator agent queue.
    pub fn map_agent_session(
        &self,
        agent_id: AgentId,
        session_id: String,
    ) -> Result<(), OrchestratorError> {
        let agents = crate::sync_lock::rw_read(&*self.agents);
        let queue_lock = agents
            .get(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        let mut queue = crate::sync_lock::rw_write(&**queue_lock);
        queue.set_agent_session(session_id.clone());
        tracing::info!("Mapped agent session {} to agent {}", session_id, agent_id);
        Ok(())
    }

    /// Bind a provider/model endpoint key to an agent for reliability tracking.
    pub fn set_agent_endpoint(&self, agent_id: AgentId, provider: &str, model: &str) {
        let agents = crate::sync_lock::rw_read(&*self.agents);
        if let Some(queue_lock) = agents.get(&agent_id) {
            let mut queue = crate::sync_lock::rw_write(&**queue_lock);
            queue.endpoint_reliability_key = Some(format!("{}:{}", provider, model));
        }
    }

    /// Register a `vox-runtime` process handle for an agent.
    #[cfg(feature = "runtime")]
    pub fn register_agent_handle(&self, agent_id: AgentId, handle: vox_runtime::ProcessHandle) {
        crate::sync_lock::rw_write(&*self.agent_handles).insert(agent_id, handle);
    }
}
