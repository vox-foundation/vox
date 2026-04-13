//! Agent lifecycle: spawn, retire, session mapping, handoff, pause/resume, heartbeat.
//!
//! All methods here operate on the `agents` / `agent_handles` maps and the supporting
//! subsystems (lock manager, affinity map, scope guard, heartbeat monitor).  No async
//! task dispatch lives here — that belongs in [`super::task_dispatch`].

use crate::locks::LockKind;
use crate::orchestrator::OrchestratorError;
use crate::services::MessageGateway;
use crate::types::{AgentId, AgentTask, TaskId, TaskStatus};

impl crate::orchestrator::Orchestrator {
    /// Spawn a new named agent, probe host capabilities, and register it with the
    /// heartbeat monitor and event bus.
    pub fn spawn_agent(&self, name: &str) -> Result<AgentId, OrchestratorError> {
        let config = crate::sync_lock::rw_read(&*self.config);
        if crate::sync_lock::rw_read(&*self.agents).len() >= config.max_agents {
            return Err(OrchestratorError::MaxAgentsReached {
                max: config.max_agents,
            });
        }
        let default_caps = config.default_agent_capabilities.clone();
        drop(config);

        let agent_id = self.agent_id_gen.next();
        let mut queue = crate::queue::AgentQueue::new(agent_id, name);
        let probed = crate::capability_probe::probe_host_capabilities();
        queue.capabilities =
            crate::capability_probe::merge_agent_capabilities(&default_caps, probed);
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
        tokio::spawn(async move {
            bm.load_user_configured_budget(agent_id).await;
        });

        tracing::info!("Spawned agent {} (name: {})", agent_id, name);
        Ok(agent_id)
    }

    /// Spawn a transient (dynamic) agent, marking it for automatic retirement when idle.
    pub fn spawn_dynamic_agent(&self, name: &str) -> Result<AgentId, OrchestratorError> {
        self.spawn_dynamic_agent_with_parent(name, None, None, None)
    }

    /// Spawn a transient agent with an optional explicit parent binding.
    ///
    /// This is groundwork for delegation-aware orchestration: current runtime behavior
    /// remains queue-centric, but topology snapshots can now surface parent/child links.
    pub fn spawn_dynamic_agent_with_parent(
        &self,
        name: &str,
        parent_agent_id: Option<AgentId>,
        reason: Option<&str>,
        source_task_id: Option<TaskId>,
    ) -> Result<AgentId, OrchestratorError> {
        if let Some(parent) = parent_agent_id {
            let parent_exists = crate::sync_lock::rw_read(&*self.agents).contains_key(&parent);
            if !parent_exists {
                return Err(OrchestratorError::DelegationParentNotFound(parent));
            }
        }
        let agent_id = self.spawn_agent(name)?;
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

    /// Replace cached remote mens capability hints (from a background mens poll).
    ///
    /// Does **not** enable remote task execution; see
    /// `OrchestratorConfig::populi_routing_experimental`.
    ///
    /// When experimental routing is on and the count of **federation-schedulable** remote nodes
    /// drops vs the previous snapshot, emits `tracing` (`target: vox.orchestrator.routing`,
    /// `decision = populi_remote_schedulable_decreased`). If
    /// [`OrchestratorConfig::populi_rebalance_on_remote_schedulable_drop`] or queued route replay
    /// should consult [`crate::populi_federation::PopuliRoutingHintUpdate`] from the return value
    /// (see `vox-mcp` federation poller).
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

    /// Retire an agent: release all locks/affinity/scope, drain its queue, and return remaining tasks.
    pub async fn retire_agent(
        &self,
        agent_id: AgentId,
    ) -> Result<Vec<AgentTask>, OrchestratorError> {
        let (remaining, session_id) = {
            let queue_lock = crate::sync_lock::rw_write(&*self.agents)
                .remove(&agent_id)
                .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
            let mut queue = crate::sync_lock::rw_write(&*queue_lock);

            self.lock_manager.release_all(agent_id);
            self.affinity_map.release_all(agent_id);
            crate::sync_lock::rw_write(&*self.scope_guard).clear_scope(agent_id);
            crate::sync_lock::rw_write(&*self.dynamic_agents).remove(&agent_id);
            {
                let mut delegations = crate::sync_lock::rw_write(&*self.agent_delegations);
                delegations.remove(&agent_id);
                delegations.retain(|_, binding| binding.parent_agent_id != agent_id);
            }
            crate::sync_lock::rw_write(&*self.dynamic_spawn_context).remove(&agent_id);
            crate::sync_lock::rw_write(&*self.agent_handles).remove(&agent_id);
            crate::sync_lock::rw_write(&*self.heartbeat_monitor).unregister(agent_id);

            let mut remaining = queue.drain_tasks();

            // Re-queue the in-progress task if it was interrupted by retirement
            if let Some(mut task) = queue.in_progress.take() {
                task.status = TaskStatus::Queued;
                remaining.insert(0, task);
            }

            let sid = queue.agent_session_id.clone();
            (remaining, sid)
        };

        // If this agent was mapped to a session, flush its socrates thinking context to DB.
        if let Some(sid) = session_id {
            let key = crate::socrates::session_context_envelope_key(&sid);
            let envelope_opt = crate::sync_lock::rw_read(&*self.context_store)
                .get(&key)
                .clone();
            if let Some(envelope_json) = envelope_opt {
                let db_opt = crate::sync_lock::rw_read(&*self.db).clone();
                if let Some(db) = db_opt {
                    let sid_clone = sid.clone();
                    let data_clone = envelope_json.clone();
                    self.persist_with_retry_meta("session_context_retirement_flush", None, move || {
                        let db = db.clone();
                        let sid = sid_clone.clone();
                        let data = data_clone.clone();
                        async move {
                            db.save_memory(vox_db::SaveMemoryParams {
                                agent_id: "orchestrator",
                                session_id: &sid,
                                memory_type: "socrates_session_context",
                                content: "Durable context envelope flushed on agent retirement scaling event",
                                metadata: Some(&data),
                                importance: 1.0,
                                vcs_snapshot_id: None,
                            })
                            .await
                            .map(|_| ())
                        }
                    })
                    .await;
                }
            }
        }

        MessageGateway::publish_agent_retired(&self.event_bus, agent_id);
        tracing::info!(
            "Retired agent {} — {} tasks to redistribute",
            agent_id,
            remaining.len()
        );
        Ok(remaining)
    }

    /// Cancel a queued task, or a Populi remote-delegated in-progress task.
    ///
    /// Remote-delegated tasks emit a best-effort [`crate::a2a::relay_remote_task_cancel`] when a
    /// Tokio runtime handle is available.
    pub fn cancel_task(&self, task_id: crate::types::TaskId) -> Result<(), OrchestratorError> {
        let agent_id = crate::sync_lock::rw_read(&self.task_assignments)
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        let agents = crate::sync_lock::rw_read(&self.agents);
        let queue_lock = agents
            .get(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        let mut queue = crate::sync_lock::rw_write(queue_lock);

        if let Some(task) = queue.current_task()
            && task.id == task_id
            && task.populi_remote_delegate.is_some()
        {
            #[cfg(feature = "populi-transport")]
            let delegate = task.populi_remote_delegate.clone();
            #[cfg(feature = "populi-transport")]
            let idempotency_key = delegate.as_ref().map(|d| d.idempotency_key.clone());
            let taken = queue.take_in_progress_if(task_id);
            if taken.is_none() {
                return Err(OrchestratorError::TaskNotFound(task_id));
            }
            let still_claimed_by_queue = |path: &std::path::Path, q: &crate::queue::AgentQueue| {
                q.current_task()
                    .is_some_and(|t| t.write_files().iter().any(|p| p.as_path() == path))
                    || q.tasks()
                        .iter()
                        .any(|t| t.write_files().iter().any(|p| p.as_path() == path))
            };
            if let Some(ref t) = taken {
                for path in t.write_files() {
                    if !still_claimed_by_queue(path, &queue) {
                        self.lock_manager.release(path, agent_id);
                        self.affinity_map.release(path);
                        crate::sync_lock::rw_write(&*self.scope_guard).revoke_file(agent_id, path);
                    }
                }
            }
            crate::sync_lock::rw_write(&self.task_assignments).remove(&task_id);
            tracing::info!(
                "Cancelled Populi remote-delegated task {} from agent {}",
                task_id,
                agent_id
            );
            #[cfg(feature = "populi-transport")]
            if let (Some(key), Ok(handle)) =
                (idempotency_key, tokio::runtime::Handle::try_current())
            {
                let cfg = crate::sync_lock::rw_read(&*self.config).clone();
                if cfg.populi_remote_execute_experimental {
                    if let (Some(base), Some(recv_s)) = (
                        cfg.populi_control_url
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string()),
                        cfg.populi_remote_execute_receiver_agent
                            .as_deref()
                            .map(str::trim)
                            .filter(|s| !s.is_empty()),
                    ) {
                        if let Ok(recv_id) = recv_s.parse::<u64>() {
                            let send_id = cfg
                                .populi_remote_execute_sender_agent
                                .as_deref()
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                                .and_then(|s| s.parse::<u64>().ok())
                                .unwrap_or(1);
                            let cancel = crate::a2a::RemoteTaskCancel {
                                idempotency_key: key,
                                task_id: task_id.0,
                                reason: Some("orchestrator_cancel".to_string()),
                            };
                            let timeout_ms = cfg.populi_http_timeout_ms.max(500);
                            let tid = task_id.0;
                            let lease_id = delegate.as_ref().and_then(|d| d.lease_id.clone());
                            let claimer_node_id =
                                delegate.as_ref().and_then(|d| d.claimer_node_id.clone());
                            handle.spawn(async move {
                                let client =
                                    vox_populi::http_client::PopuliHttpClient::new_with_timeout(
                                        &base,
                                        std::time::Duration::from_millis(timeout_ms),
                                    )
                                    .with_env_deliver_token();
                                if let Err(e) = crate::a2a::relay_remote_task_cancel(
                                    &client,
                                    crate::types::AgentId(send_id),
                                    crate::types::AgentId(recv_id),
                                    &cancel,
                                )
                                .await
                                {
                                    tracing::debug!(
                                        error = %e,
                                        task_id = tid,
                                        "populi remote_task_cancel relay failed (best-effort)"
                                    );
                                }
                                if let (Some(lease_id), Some(claimer_node_id)) =
                                    (lease_id, claimer_node_id)
                                {
                                    let _ = client
                                        .exec_lease_release(
                                            &vox_populi::transport::RemoteExecLeaseReleaseRequest {
                                                lease_id,
                                                claimer_node_id,
                                            },
                                        )
                                        .await;
                                }
                            });
                        }
                    }
                }
            }
            return Ok(());
        }

        if let Some(task) = queue.cancel(task_id) {
            let still_claimed_by_queue = |path: &std::path::Path, q: &crate::queue::AgentQueue| {
                q.current_task()
                    .is_some_and(|t| t.write_files().iter().any(|p| p.as_path() == path))
                    || q.tasks()
                        .iter()
                        .any(|t| t.write_files().iter().any(|p| p.as_path() == path))
            };
            for path in task.write_files() {
                if !still_claimed_by_queue(path, &queue) {
                    self.lock_manager.release(path, agent_id);
                    self.affinity_map.release(path);
                    crate::sync_lock::rw_write(&*self.scope_guard).revoke_file(agent_id, path);
                }
            }
            crate::sync_lock::rw_write(&self.task_assignments).remove(&task_id);
            tracing::info!("Cancelled task {} from agent {}", task_id, agent_id);
            Ok(())
        } else {
            Err(OrchestratorError::TaskNotFound(task_id))
        }
    }

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

    /// Register a `vox-runtime` process handle for an agent.
    pub fn register_agent_handle(&self, agent_id: AgentId, handle: vox_runtime::ProcessHandle) {
        crate::sync_lock::rw_write(&*self.agent_handles).insert(agent_id, handle);
    }

    /// Accept a structured handoff payload from another agent, spawning a target agent if needed.
    pub fn accept_handoff(
        &self,
        payload: crate::handoff::HandoffPayload,
    ) -> Result<AgentId, OrchestratorError> {
        let from_agent = payload.from_agent;
        if let Err(err) = crate::handoff::validate_handoff_invariants(&payload) {
            let reason = err.to_string();
            self.event_bus
                .emit(crate::events::AgentEventKind::AgentHandoffRejected {
                    from: from_agent,
                    reason: reason.clone(),
                });
            return Err(OrchestratorError::HandoffInvariant(reason));
        }
        let (has_context_envelope, has_harness_spec, handoff_session_id, handoff_thread_id) =
            crate::handoff::handoff_context_event_metadata(&payload);
        if let Some((_, context_json)) = payload
            .metadata
            .iter()
            .rev()
            .find(|(k, _)| k == crate::handoff::CONTEXT_ENVELOPE_JSON_METADATA_KEY)
        {
            if let Ok(env) = serde_json::from_str::<crate::ContextEnvelope>(context_json) {
                let cfg = crate::sync_lock::rw_read(&*self.config).clone();
                let repo = crate::lineage::repository_id();
                if let Some(session_id) = env
                    .subject
                    .session_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let expectations = crate::context_lifecycle::ContextIngestExpectations {
                        repository_id: repo.as_str(),
                        session_id: Some(session_id),
                    };
                    if let Err(e) = crate::context_lifecycle::apply_context_lifecycle_policy(
                        &cfg,
                        &env,
                        expectations,
                        crate::context_lifecycle::ContextIngestSource::InternalHandoffAccept,
                    ) {
                        return Err(OrchestratorError::HandoffInvariant(e));
                    }
                    let key = crate::socrates::session_context_envelope_key(session_id);
                    let existing = crate::sync_lock::rw_read(&*self.context_store).get(&key);
                    let merged =
                        match crate::context_lifecycle::merge_context_envelope_for_session_store(
                            existing.as_deref(),
                            &env,
                            cfg.context_lifecycle_shadow,
                        ) {
                            Ok(m) => m,
                            Err(e) => return Err(OrchestratorError::HandoffInvariant(e)),
                        };
                    if let Err(e) = crate::context_lifecycle::apply_context_lifecycle_policy(
                        &cfg,
                        &merged,
                        expectations,
                        crate::context_lifecycle::ContextIngestSource::SessionStoreWrite,
                    ) {
                        return Err(OrchestratorError::HandoffInvariant(e));
                    }
                    let merged_json = match serde_json::to_string(&merged) {
                        Ok(s) => s,
                        Err(e) => {
                            return Err(OrchestratorError::HandoffInvariant(e.to_string()));
                        }
                    };
                    crate::sync_lock::rw_write(&*self.context_store).set(
                        from_agent,
                        key,
                        merged_json,
                        3600,
                    );
                }
            }
        }

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
            self.event_bus
                .emit(crate::events::AgentEventKind::AgentHandoffRejected {
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
            let agents = crate::sync_lock::rw_read(&*self.agents);
            if agents.contains_key(&id) {
                id
            } else {
                drop(agents);
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
            crate::sync_lock::rw_write(&self.scope_guard).assign_file(target_id, path.clone());
            let _ = self
                .lock_manager
                .try_acquire(path, target_id, LockKind::Exclusive);
        }

        let resumed_ids: Vec<crate::types::TaskId> = payload.pending_tasks.clone();
        if target_id != from_agent {
            crate::sync_lock::rw_write(&*self.agent_delegations).insert(
                target_id,
                crate::topology::AgentDelegationBinding {
                    parent_agent_id: from_agent,
                    source_task_id: None,
                    reason: "handoff_accept".to_string(),
                },
            );
            self.record_lineage_event(
                "task_delegated",
                None,
                Some(target_id),
                None,
                None,
                None,
                None,
                Some(serde_json::json!({
                    "reason": "handoff_accept",
                    "from_agent": from_agent
                })),
            );
        }

        self.event_bus
            .emit(crate::events::AgentEventKind::AgentHandoffAccepted {
                agent_id: target_id,
                from: from_agent,
                plan_summary: payload.plan_summary.clone(),
                has_context_envelope,
                has_harness_spec,
                session_id: handoff_session_id,
                thread_id: handoff_thread_id,
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
        &self,
        task_id: crate::types::TaskId,
        new_priority: crate::types::TaskPriority,
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
    pub fn drain_agent(&self, agent_id: AgentId) -> Result<Vec<AgentTask>, OrchestratorError> {
        let agents = crate::sync_lock::rw_read(&self.agents);
        let queue_lock = agents
            .get(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        let mut queue = crate::sync_lock::rw_write(queue_lock);

        let remaining = queue.drain_tasks();
        let mut assignments = crate::sync_lock::rw_write(&self.task_assignments);
        for task in &remaining {
            assignments.remove(&task.id);
        }

        tracing::info!("Drained {} tasks from agent {}", remaining.len(), agent_id);
        Ok(remaining)
    }

    /// Pause an agent's queue (new tasks are held, not dispatched).
    pub fn pause_agent(&self, agent_id: AgentId) -> Result<(), OrchestratorError> {
        let agents = crate::sync_lock::rw_read(&self.agents);
        let queue_lock = agents
            .get(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        crate::sync_lock::rw_write(queue_lock).pause();
        Ok(())
    }

    /// Resume a previously paused agent queue.
    pub fn resume_agent(&self, agent_id: AgentId) -> Result<(), OrchestratorError> {
        let agents = crate::sync_lock::rw_read(&self.agents);
        let queue_lock = agents
            .get(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        crate::sync_lock::rw_write(queue_lock).resume();
        Ok(())
    }

    /// Record a heartbeat from an agent and update the continuation monitor.
    pub fn heartbeat(&self, agent_id: AgentId, activity: crate::events::AgentActivity) {
        crate::sync_lock::rw_write(&self.heartbeat_monitor).heartbeat(agent_id, activity);
        crate::sync_lock::rw_write(&self.monitor).record_activity(agent_id);
    }
}
