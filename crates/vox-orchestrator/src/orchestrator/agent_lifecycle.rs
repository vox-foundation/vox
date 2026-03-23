//! Agent lifecycle: spawn, retire, session mapping, handoff, pause/resume, heartbeat.
//!
//! All methods here operate on the `agents` / `agent_handles` maps and the supporting
//! subsystems (lock manager, affinity map, scope guard, heartbeat monitor).  No async
//! task dispatch lives here — that belongs in [`super::task_dispatch`].

use crate::locks::LockKind;
use crate::orchestrator::OrchestratorError;
use crate::services::MessageGateway;
use crate::types::{AgentId, AgentTask};

impl crate::orchestrator::Orchestrator {
    /// Spawn a new named agent, probe host capabilities, and register it with the
    /// heartbeat monitor and event bus.
    pub fn spawn_agent(&mut self, name: &str) -> Result<AgentId, OrchestratorError> {
        if self.agents.len() >= self.config.max_agents {
            return Err(OrchestratorError::MaxAgentsReached {
                max: self.config.max_agents,
            });
        }

        let agent_id = self.agent_id_gen.next();
        let mut queue = crate::queue::AgentQueue::new(agent_id, name);
        let probed = crate::capability_probe::probe_host_capabilities();
        queue.capabilities = crate::capability_probe::merge_agent_capabilities(
            &self.config.default_agent_capabilities,
            probed,
        );
        self.agents.insert(agent_id, queue);
        self.heartbeat_monitor.register(agent_id);
        MessageGateway::publish_agent_spawned(
            &mut self.bulletin,
            &self.event_bus,
            agent_id,
            name.to_string(),
        );
        tracing::info!("Spawned agent {} (name: {})", agent_id, name);
        Ok(agent_id)
    }

    /// Spawn a transient (dynamic) agent, marking it for automatic retirement when idle.
    pub fn spawn_dynamic_agent(&mut self, name: &str) -> Result<AgentId, OrchestratorError> {
        let agent_id = self.spawn_agent(name)?;
        self.dynamic_agents.insert(agent_id);
        tracing::info!("Agent {} marked as dynamic", agent_id);
        Ok(agent_id)
    }

    /// Replace cached remote mesh capability hints (from a background mesh poll).
    ///
    /// Does **not** enable remote task execution; see
    /// `OrchestratorConfig::mesh_routing_experimental`.
    pub fn set_remote_mesh_routing_hints(
        &mut self,
        hints: Vec<crate::mesh_federation::RemoteMeshRoutingHint>,
    ) {
        self.remote_mesh_routing_hints = hints;
    }

    /// Map an AI agent session ID to an existing orchestrator agent queue.
    pub fn map_agent_session(
        &mut self,
        agent_id: AgentId,
        session_id: String,
    ) -> Result<(), OrchestratorError> {
        let queue = self
            .agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        queue.set_agent_session(session_id.clone());
        tracing::info!("Mapped agent session {} to agent {}", session_id, agent_id);
        Ok(())
    }

    /// Bind a provider/model endpoint key to an agent for reliability tracking.
    pub fn set_agent_endpoint(&mut self, agent_id: AgentId, provider: &str, model: &str) {
        if let Some(queue) = self.agents.get_mut(&agent_id) {
            queue.endpoint_reliability_key = Some(format!("{}:{}", provider, model));
        }
    }

    /// Retire an agent: release all locks/affinity/scope, drain its queue, and return remaining tasks.
    pub fn retire_agent(
        &mut self,
        agent_id: AgentId,
    ) -> Result<Vec<AgentTask>, OrchestratorError> {
        let mut queue = self
            .agents
            .remove(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        self.lock_manager.release_all(agent_id);
        self.affinity_map.release_all(agent_id);
        self.scope_guard.clear_scope(agent_id);
        self.dynamic_agents.remove(&agent_id);
        self.agent_handles.remove(&agent_id);
        self.heartbeat_monitor.unregister(agent_id);

        let remaining = queue.drain_tasks();
        MessageGateway::publish_agent_retired(&self.event_bus, agent_id);
        tracing::info!(
            "Retired agent {} — {} tasks to redistribute",
            agent_id,
            remaining.len()
        );
        Ok(remaining)
    }

    /// Cancel a queued task. Returns an error if the task is in-progress or not found.
    pub fn cancel_task(
        &mut self,
        task_id: crate::types::TaskId,
    ) -> Result<(), OrchestratorError> {
        let agent_id = self
            .task_assignments
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        let queue = self
            .agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        if let Some(_task) = queue.cancel(task_id) {
            self.task_assignments.remove(&task_id);
            tracing::info!("Cancelled task {} from agent {}", task_id, agent_id);
            Ok(())
        } else {
            Err(OrchestratorError::TaskNotFound(task_id))
        }
    }

    /// Register a `vox-runtime` process handle for an agent.
    pub fn register_agent_handle(
        &mut self,
        agent_id: AgentId,
        handle: vox_runtime::ProcessHandle,
    ) {
        self.agent_handles.insert(agent_id, handle);
    }

    /// Accept a structured handoff payload from another agent, spawning a target agent if needed.
    pub fn accept_handoff(
        &mut self,
        payload: crate::handoff::HandoffPayload,
    ) -> Result<AgentId, OrchestratorError> {
        let from_agent = payload.from_agent;

        // Check for staleness/expiration
        let now = crate::types::now_unix_ms();
        let age_ms = now.saturating_sub(payload.created_at);
        let timeout = payload.timeout_ms.unwrap_or(3_600_000); // 1 hour default

        if age_ms > timeout {
            let reason = format!(
                "Handoff from {} is stale (age: {}s, timeout: {}s)",
                from_agent,
                age_ms / 1000,
                timeout / 1000
            );
            self.event_bus.emit(crate::events::AgentEventKind::AgentHandoffRejected {
                from: from_agent,
                reason: reason.clone(),
            });
            tracing::warn!("{}", reason);
            return Err(OrchestratorError::StaleHandoff {
                agent_id: from_agent,
                age_ms,
                timeout_ms: timeout,
            });
        }

        let target_id = if let Some(id) = payload.to_agent {
            if self.agents.contains_key(&id) {
                id
            } else {
                match self.spawn_agent(&format!("ResumingAgent-{}", id.0)) {
                    Ok(new_id) => new_id,
                    Err(e) => {
                        self.event_bus
                            .emit(crate::events::AgentEventKind::AgentHandoffRejected {
                                from: from_agent,
                                reason: format!("Spawn failed: {}", e),
                            });
                        return Err(e);
                    }
                }
            }
        } else {
            match self.spawn_agent("AdaptiveResumer") {
                Ok(new_id) => new_id,
                Err(e) => {
                    self.event_bus
                        .emit(crate::events::AgentEventKind::AgentHandoffRejected {
                            from: from_agent,
                            reason: format!("Spawn failed: {}", e),
                        });
                    return Err(e);
                }
            }
        };

        for path in &payload.owned_files {
            self.affinity_map.assign(path, target_id);
            self.scope_guard.assign_file(target_id, path.clone());
            let _ = self
                .lock_manager
                .try_acquire(path, target_id, LockKind::Exclusive);
        }

        let resumed_ids: Vec<crate::types::TaskId> = payload.pending_tasks.clone();

        self.event_bus
            .emit(crate::events::AgentEventKind::AgentHandoffAccepted {
                agent_id: target_id,
                from: from_agent,
                plan_summary: payload.plan_summary.clone(),
            });

        tracing::info!(
            "Agent {} accepted handoff from {} ({} tasks resumed: {:?})",
            target_id,
            from_agent,
            resumed_ids.len(),
            resumed_ids
        );
        Ok(target_id)
    }

    /// Reorder a queued task with a new priority.
    pub fn reorder_task(
        &mut self,
        task_id: crate::types::TaskId,
        new_priority: crate::types::TaskPriority,
    ) -> Result<(), OrchestratorError> {
        let agent_id = self
            .task_assignments
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        let queue = self
            .agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        if queue.reorder(task_id, new_priority) {
            tracing::info!(
                "Reordered task {} to priority {:?} on agent {}",
                task_id,
                new_priority,
                agent_id
            );
            Ok(())
        } else {
            Err(OrchestratorError::TaskNotFound(task_id))
        }
    }

    /// Drain all queued tasks from an agent without retiring it.
    pub fn drain_agent(
        &mut self,
        agent_id: AgentId,
    ) -> Result<Vec<AgentTask>, OrchestratorError> {
        let queue = self
            .agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        let remaining = queue.drain_tasks();
        for task in &remaining {
            self.task_assignments.remove(&task.id);
        }

        tracing::info!("Drained {} tasks from agent {}", remaining.len(), agent_id);
        Ok(remaining)
    }

    /// Pause an agent's queue (new tasks are held, not dispatched).
    pub fn pause_agent(&mut self, agent_id: AgentId) -> Result<(), OrchestratorError> {
        self.agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?
            .pause();
        Ok(())
    }

    /// Resume a previously paused agent queue.
    pub fn resume_agent(&mut self, agent_id: AgentId) -> Result<(), OrchestratorError> {
        self.agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?
            .resume();
        Ok(())
    }

    /// Record a heartbeat from an agent and update the continuation monitor.
    pub fn heartbeat(&mut self, agent_id: AgentId, activity: crate::events::AgentActivity) {
        self.heartbeat_monitor.heartbeat(agent_id, activity);
        self.monitor.record_activity(agent_id);
    }
}
