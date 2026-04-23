use crate::orchestrator::OrchestratorError;
use crate::services::MessageGateway;
use crate::types::{AgentId, TaskId};

impl crate::orchestrator::Orchestrator {
    /// Spawn a new named agent with specific capability requirements.
    pub fn spawn_agent_with_hints(
        &self,
        name: &str,
        hints: Option<crate::contract::TaskCapabilityHints>,
    ) -> Result<AgentId, OrchestratorError> {
        let config = crate::sync_lock::rw_read(&*self.config);
        if crate::sync_lock::rw_read(&*self.agents).len() >= config.max_agents {
            return Err(OrchestratorError::MaxAgentsReached {
                max: config.max_agents,
            });
        }
        let mut caps = config.default_agent_capabilities.clone();
        drop(config);

        if let Some(h) = hints {
            caps = crate::capability_probe::merge_agent_capabilities(&caps, h);
        }

        let agent_id = self.agent_id_gen.next();
        let mut queue = crate::queue::AgentQueue::new(agent_id, name);
        let probed = crate::capability_probe::probe_host_capabilities();
        queue.capabilities = crate::capability_probe::merge_agent_capabilities(&caps, probed);
        crate::sync_lock::rw_write(&*self.agents)
            .insert(agent_id, std::sync::Arc::new(std::sync::RwLock::new(queue)));
        crate::sync_lock::rw_write(&*self.heartbeat_monitor).register(agent_id);
        MessageGateway::publish_agent_spawned(
            &self.bulletin,
            &self.event_bus,
            agent_id,
            name.to_string(),
        );

        let bm = crate::sync_lock::rw_read(&*self.budget_manager).clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                bm.load_user_configured_budget(agent_id).await;
            });
        }

        tracing::info!("Spawned agent {} (name: {})", agent_id, name);
        Ok(agent_id)
    }

    /// Spawn a new named agent using default capabilities.
    pub fn spawn_agent(&self, name: &str) -> Result<AgentId, OrchestratorError> {
        self.spawn_agent_with_hints(name, None)
    }

    /// Spawn a transient (dynamic) agent, marking it for automatic retirement when idle.
    pub fn spawn_dynamic_agent(&self, name: &str) -> Result<AgentId, OrchestratorError> {
        self.spawn_dynamic_agent_with_parent(name, None, None, None, None)
    }

    /// Spawn a transient agent with an optional explicit parent binding.
    pub fn spawn_dynamic_agent_with_parent(
        &self,
        name: &str,
        parent_agent_id: Option<AgentId>,
        reason: Option<&str>,
        source_task_id: Option<TaskId>,
        hints: Option<crate::contract::TaskCapabilityHints>,
    ) -> Result<AgentId, OrchestratorError> {
        if let Some(parent) = parent_agent_id {
            let parent_exists = crate::sync_lock::rw_read(&*self.agents).contains_key(&parent);
            if !parent_exists {
                return Err(OrchestratorError::DelegationParentNotFound(parent));
            }
        }
        let agent_id = self.spawn_agent_with_hints(name, hints)?;
        crate::sync_lock::rw_write(&*self.dynamic_agents).insert(agent_id);
        let spawn_reason = reason
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("dynamic_spawn")
            .to_string();
        crate::sync_lock::rw_write(&*self.dynamic_spawn_context).insert(
            agent_id,
            crate::topology::DynamicSpawnContext {
                source_task_id,
                reason: spawn_reason.clone(),
            },
        );
        if let Some(parent) = parent_agent_id {
            let binding = crate::topology::AgentDelegationBinding {
                parent_agent_id: parent,
                source_task_id,
                reason: spawn_reason.clone(),
            };
            crate::sync_lock::rw_write(&*self.agent_delegations).insert(agent_id, binding);

            self.record_lineage_event(
                "task_delegated",
                source_task_id,
                Some(agent_id),
                None,
                None,
                None,
                None,
                Some(serde_json::json!({
                    "reason": spawn_reason,
                    "is_dynamic": true
                })),
            );
        }
        tracing::info!("Agent {} marked as dynamic", agent_id,);
        Ok(agent_id)
    }
}
