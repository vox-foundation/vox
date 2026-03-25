use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::locks::LockKind;
use crate::oplog::OperationKind;
use crate::scope::ScopeEnforcement;
use crate::services::{PolicyCheckResult, PolicyEngine, RouteResult, RoutingService};
use crate::types::{AccessKind, AgentId, AgentTask, FileAffinity, TaskId};

use super::super::super::{MAX_TASK_TRACES, Orchestrator, OrchestratorError, TaskTraceStep};

impl Orchestrator {
    /// Submit a batch of interdependent tasks (async).
    pub async fn submit_batch(
        &self,
        descriptors: Vec<crate::types::TaskDescriptor>,
    ) -> Result<Vec<TaskId>, OrchestratorError> {
        let (enabled, default_priority, scope_enforcement) = {
            let config = crate::sync_lock::rw_read(&*self.config);
            (
                config.enabled,
                config.default_priority,
                config.scope_enforcement,
            )
        };
        if !enabled {
            return Err(OrchestratorError::Disabled);
        }

        let mut assigned_ids: Vec<TaskId> = Vec::with_capacity(descriptors.len());

        // Pre-allocate task IDs
        for _ in 0..descriptors.len() {
            assigned_ids.push(self.task_id_gen.next());
        }

        let mut results = Vec::new();

        // Second pass: construct tasks with resolved IDs and submit
        for (i, mut desc) in descriptors.into_iter().enumerate() {
            let my_id = assigned_ids[i];

            // Resolve temporary deps into actual TaskIds
            for tmp_dep_idx in desc.temp_deps {
                if tmp_dep_idx < assigned_ids.len() {
                    desc.depends_on.push(assigned_ids[tmp_dep_idx]);
                } else {
                    tracing::warn!(
                        "Task descriptor {} referenced out-of-bounds temp dep {}",
                        i,
                        tmp_dep_idx
                    );
                }
            }

            let priority = desc.priority.unwrap_or(default_priority);
            let mut task = AgentTask::new(
                my_id,
                desc.description.clone(),
                priority,
                desc.file_manifest.clone(),
            );
            task.capability_requirements = desc.capability_requirements.clone();
            task.session_id = desc.session_id.clone();
            task.start(); // ensure started_at_ms is populated

            // Add all collected deps
            for dep in desc.depends_on {
                task = task.depends_on(dep);
            }

            // Route to best agent via RoutingService
            let agent_id = self
                .resolve_route(
                    &desc.file_manifest,
                    None,
                    desc.capability_requirements.as_ref(),
                )
                .await?;

            {
                let scope_guard_lock = (scope_enforcement != ScopeEnforcement::Disabled)
                    .then_some(crate::sync_lock::rw_read(&*self.scope_guard));
                let scope_guard_ref = scope_guard_lock.as_deref();
                match PolicyEngine::check_before_queue(
                    &self.lock_manager,
                    scope_guard_ref,
                    &self.event_bus,
                    &desc.file_manifest,
                    agent_id,
                ) {
                    PolicyCheckResult::Allowed => {}
                    PolicyCheckResult::LockConflict(e) => {
                        return Err(OrchestratorError::LockConflict(e));
                    }
                    PolicyCheckResult::ScopeDenied(msg) => {
                        return Err(OrchestratorError::ScopeDenied(msg));
                    }
                }
            }

            // Acquire locks and assign scope (after releasing scope read guard; see task_submit)
            for fa in &desc.file_manifest {
                if fa.access == AccessKind::Write {
                    let _ = self
                        .lock_manager
                        .try_acquire(&fa.path, agent_id, LockKind::Exclusive);
                    self.affinity_map.assign(&fa.path, agent_id);
                    crate::sync_lock::rw_write(&*self.scope_guard)
                        .assign_file(agent_id, fa.path.clone());
                }
            }

            // Capture pre-task snapshot for version control
            let snapshot_before = {
                let paths: Vec<PathBuf> =
                    desc.file_manifest.iter().map(|f| f.path.clone()).collect();
                self.capture_snapshot(
                    agent_id,
                    &paths,
                    format!("pre-task batch: {:.50}", task.description),
                )
                .await
            };

            self.record_operation(
                agent_id,
                OperationKind::TaskSubmit { task_id: my_id.0 },
                format!("Submitted batch task {}", my_id),
                Some(snapshot_before),
                None,
                None,
                None,
            )
            .await;

            self.record_activity();
            let session_id_for_retrieval = task.session_id.clone();
            // Enqueue
            let handle_to_notify = {
                let agents = crate::sync_lock::rw_read(&*self.agents);
                if let Some(queue_lock) = agents.get(&agent_id) {
                    let mut queue = crate::sync_lock::rw_write(&**queue_lock);
                    self.event_bus
                        .emit(crate::events::AgentEventKind::TaskSubmitted {
                            task_id: my_id,
                            agent_id,
                            description: task.description.clone(),
                            session_id: task.session_id.clone(),
                        });
                    queue.enqueue(task);
                    crate::sync_lock::rw_write(&*self.task_assignments).insert(my_id, agent_id);

                    // Grab the handle for notification outside the agents lock
                    crate::sync_lock::rw_read(&*self.agent_handles)
                        .get(&agent_id)
                        .cloned()
                } else {
                    None
                }
            };

            // Notify outside all locks
            if let Some(handle) = handle_to_notify {
                let json = serde_json::to_string(&crate::runtime::AgentCommand::ProcessQueue)
                    .unwrap_or_else(|e| {
                        tracing::warn!("serialize ProcessQueue: {e}");
                        "{}".to_string()
                    });
                let env = vox_runtime::mailbox::Envelope::Message(vox_runtime::mailbox::Message {
                    from: vox_runtime::Pid::new(),
                    payload: vox_runtime::mailbox::MessagePayload::Json(json),
                });
                const NOTIFY_TIMEOUT: Duration = Duration::from_secs(30);
                match tokio::time::timeout(NOTIFY_TIMEOUT, handle.send(env)).await {
                    Ok(send_res) => {
                        if let Err(e) = send_res {
                            tracing::warn!("submit_batch: agent notify send failed: {e:?}");
                        }
                    }
                    Err(_) => tracing::warn!(
                        "submit_batch: agent notify timed out after {:?}",
                        NOTIFY_TIMEOUT
                    ),
                }
            }

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            {
                let mut traces = crate::sync_lock::rw_write(&*self.task_traces);
                if traces.len() >= MAX_TASK_TRACES {
                    if let Some(min_id) = traces.keys().min().copied() {
                        traces.remove(&min_id);
                    }
                }
                traces.insert(
                    my_id,
                    vec![
                        TaskTraceStep {
                            stage: "ingress".to_string(),
                            timestamp_ms: now_ms,
                            detail: None,
                        },
                        TaskTraceStep {
                            stage: "routed".to_string(),
                            timestamp_ms: now_ms,
                            detail: Some(format!("agent {}", agent_id)),
                        },
                    ],
                );
            }

            self.attach_session_retrieval_envelope_if_present(my_id, &session_id_for_retrieval);

            results.push(my_id);
        }

        tracing::info!("Submitted batch of {} tasks", results.len());
        Ok(results)
    }

    /// Resolve route via RoutingService and spawn if needed.
    pub(crate) async fn resolve_route(
        &self,
        manifest: &[FileAffinity],
        target_agent: Option<&str>,
        task_capability_requirements: Option<&crate::contract::TaskCapabilityHints>,
    ) -> Result<AgentId, OrchestratorError> {
        if let Some(agent_name) = target_agent {
            // First check if an agent with this name exists
            let agents = crate::sync_lock::rw_read(&*self.agents);
            for (id, queue_lock) in agents.iter() {
                if crate::sync_lock::rw_read(&**queue_lock).name == agent_name {
                    return Ok(*id);
                }
            }
            drop(agents);
            // Otherwise, spawn an agent with this name
            return self.spawn_agent(agent_name);
        }

        let reputation_routing =
            crate::sync_lock::rw_read(&*self.config).socrates_reputation_routing;
        let reliability_map: Option<HashMap<AgentId, f64>> = if reputation_routing {
            self.db().map(|db| {
                db.block_on(async { db.list_agent_reliability().await })
                    .unwrap_or_default()
                    .into_iter()
                    .map(|(id, r): (String, f64)| {
                        let numeric_id = id.parse::<u64>().unwrap_or(0);
                        (AgentId(numeric_id), r)
                    })
                    .collect()
            })
        } else {
            None
        };

        let remote_hints = crate::sync_lock::rw_read(&*self.remote_populi_routing_hints);
        let remote = if remote_hints.is_empty() {
            None
        } else {
            Some(remote_hints.as_slice())
        };

        let result = {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            let groups = crate::sync_lock::rw_read(&*self.groups);
            let config = crate::sync_lock::rw_read(&*self.config);

            RoutingService::route(
                manifest,
                &self.affinity_map,
                &groups,
                &agents,
                &config,
                reliability_map.as_ref(),
                task_capability_requirements,
                remote,
                None, // Phase 15: attention_trust_scores (pass BudgetManager::trust_snapshot() when enabled)
            )
        };
        drop(remote_hints);

        match result {
            RouteResult::Existing(id) => Ok(id),
            RouteResult::SpawnAgent(name) => self.spawn_agent(&name),
        }
    }
}
