use crate::orchestrator::OrchestratorError;
use crate::types::{AgentId, AgentTask, TaskId, TaskStatus};

impl crate::orchestrator::Orchestrator {
    /// Flag a task as "Suspect" by the user, triggering a resolution loop.
    pub fn doubt_task(
        &self,
        task_id: TaskId,
        reason: Option<String>,
    ) -> Result<(), OrchestratorError> {
        let agent_id = crate::sync_lock::rw_read(&self.task_assignments)
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        let agents = crate::sync_lock::rw_read(&self.agents);
        let queue_lock = agents
            .get(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        let mut queue = crate::sync_lock::rw_write(queue_lock);

        // If in progress, take it out
        let mut task = if let Some(t) = queue.take_in_progress_if(task_id) {
            t
        } else {
            // Check if it's in the queue
            queue
                .take_queued(task_id)
                .ok_or(OrchestratorError::TaskNotFound(task_id))?
        };

        // Update context envelope to explicitly force Verification mode for the agent's prompt
        if let Some(ref sid) = task.session_id {
            let key = crate::socrates::session_context_envelope_key(sid);
            let env_opt = crate::sync_lock::rw_read(&*self.context_store).get(&key);
            if let Some(env_json) = env_opt {
                if let Ok(mut env) = serde_json::from_str::<crate::ContextEnvelope>(&env_json) {
                    env.operating_mode = Some(crate::context_envelope::OperatingMode::Verification { reason: reason.clone() });
                    if let Ok(new_json) = serde_json::to_string(&env) {
                        crate::sync_lock::rw_write(&*self.context_store).set(agent_id, key, new_json, 3600);
                    }
                }
            }
        }

        // Change the role to explicitly focus on verification
        task.execution_role = Some(crate::reconstruction::AgentExecutionRole::Verifier);
        task.status = TaskStatus::Doubted(reason.clone());

        tracing::info!(
            target: "vox.orchestrator.tasks",
            %task_id,
            %agent_id,
            reason = ?reason,
            "Task doubted: enforcing rigid Second Pass compilation/validation compliance."
        );

        // Emit event for hud and ludus
        self.event_bus
            .emit(crate::events::AgentEventKind::TaskDoubted {
                task_id,
                agent_id,
                reason: reason.clone(),
            });
            
        self.bulletin.publish(crate::types::AgentMessage::TaskDoubted {
            task_id,
            agent_id,
            reason: reason.clone(),
        });

        // The Implementation Plan requires that we re-enqueue it and let explicitly-enforced
        // terminal checks clear the Verification mode before it can be marked complete.
        queue.enqueue(task);

        tracing::info!(
            "Task {} doubted by human for agent {}: {:?}",
            task_id,
            agent_id,
            reason
        );
        Ok(())
    }

    /// Dequeue a task in Doubted status for a specific agent.
    pub fn dequeue_doubted(&self, agent_id: AgentId) -> Option<AgentTask> {
        let agents = crate::sync_lock::rw_read(&self.agents);
        let queue_lock = agents.get(&agent_id)?;
        let mut queue = crate::sync_lock::rw_write(queue_lock);
        queue.dequeue_doubted()
    }
}
