use crate::orchestrator::OrchestratorError;
use crate::types::{TaskId, TaskStatus};

impl crate::orchestrator::Orchestrator {
    /// Deterministically move a remote-held task back to the local queue after lease failure/loss.
    pub fn fallback_populi_remote_task_locally(
        &self,
        task_id: TaskId,
        reason: &str,
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

        let Some(mut task) = queue.take_in_progress_if(task_id) else {
            return Err(OrchestratorError::TaskNotFound(task_id));
        };
        task.populi_remote_delegate = None;
        task.debug_iterations = 0;
        task.toestub_iterations = 0;
        task.socrates_iterations = 0;
        task.status = TaskStatus::Queued;
        queue.enqueue(task);
        tracing::info!(
            task_id = task_id.0,
            agent_id = agent_id.0,
            reason,
            placement_reason =
                crate::populi_remote::PlacementReasonCode::LocalQueueFallbackAfterRemoteRelayError
                    .as_str(),
            "moved remote-held task back to local queue after lease transition failure"
        );
        Ok(())
    }
}
