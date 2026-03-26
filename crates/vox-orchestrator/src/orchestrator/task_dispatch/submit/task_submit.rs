use std::path::PathBuf;
use std::time::Duration;

use crate::locks::LockKind;
use crate::oplog::OperationKind;
use crate::planning::PlanningTaskMeta;
use crate::scope::ScopeEnforcement;
use crate::services::{PolicyCheckResult, PolicyEngine};
use crate::types::{AccessKind, AgentTask, FileAffinity, TaskId, TaskPriority};

use super::super::super::{MAX_TASK_TRACES, Orchestrator, OrchestratorError, TaskTraceStep};

impl Orchestrator {
    // ORCH-01 SPLIT TARGET:
    //   new() / with_groups() / init_db() → orchestrator/core.rs
    //   submit_task*() / submit_batch() / resolve_route() / spawn_agent*() → orchestrator/task_dispatch.rs
    //   map_agent_session() / retire_agent() / heartbeat() / pause/resume_agent() → orchestrator/agent_state.rs
    //   All construction, lifecycle, scaling, and VCS methods are in sub-modules:
    //   core.rs, agent_lifecycle.rs, scaling.rs, vcs_ops.rs

    /// Submit a new task to the orchestrator (async).
    ///
    /// The orchestrator will:
    /// 1. Analyze the file manifest against the affinity map
    /// 2. Route the task to an existing agent or spawn a new one
    /// 3. Acquire file locks
    /// 4. Enqueue the task
    pub async fn submit_task(
        &self,
        description: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        session_id: Option<String>,
    ) -> Result<TaskId, OrchestratorError> {
        self.submit_task_with_agent(description, file_manifest, priority, None, None, session_id)
            .await
    }

    /// Submit a new task to the orchestrator, potentially targeting a specific agent name (async).
    pub async fn submit_task_with_agent(
        &self,
        description: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        target_agent: Option<String>,
        capability_requirements: Option<crate::contract::TaskCapabilityHints>,
        session_id: Option<String>,
    ) -> Result<TaskId, OrchestratorError> {
        let (default_priority, scope_enforcement) = {
            let config_guard = crate::sync_lock::rw_read(&*self.config);
            if !config_guard.enabled {
                return Err(OrchestratorError::Disabled);
            }
            (
                config_guard.default_priority,
                config_guard.scope_enforcement,
            )
        };

        let task_id = self.task_id_gen.next();
        let priority = priority.unwrap_or(default_priority);

        let mut task = AgentTask::new(task_id, description, priority, file_manifest.clone());
        task.capability_requirements = capability_requirements.clone();
        task.session_id = session_id.clone();
        task.start(); // ensure started_at_ms is populated for orchestrator-submitted tasks

        // Route to the right agent via RoutingService
        let agent_id = self
            .resolve_route(
                &file_manifest,
                target_agent.as_deref(),
                capability_requirements.as_ref(),
                Some(task.description.as_str()),
            )
            .await?;

        // Pre-queue policy check (locks; scope when enforcement enabled).
        // The scope READ guard must not overlap `assign_file`, which takes a WRITE lock on the
        // same `RwLock` — that self-deadlocks on typical OS RwLock implementations.
        {
            let scope_guard_lock = (scope_enforcement != ScopeEnforcement::Disabled)
                .then_some(crate::sync_lock::rw_read(&*self.scope_guard));
            let scope_guard_ref = scope_guard_lock.as_deref();
            match PolicyEngine::check_before_queue(
                &self.lock_manager,
                scope_guard_ref,
                &self.event_bus,
                &file_manifest,
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

        // Try to acquire locks for write files
        for fa in &file_manifest {
            if fa.access == AccessKind::Write {
                let lock_kind = LockKind::Exclusive;
                // If lock fails, we still enqueue (the agent will retry when it picks up the task)
                let _ = self.lock_manager.try_acquire(&fa.path, agent_id, lock_kind);
            }
        }

        // Assign files to the agent in the affinity map and scope guard
        for fa in &file_manifest {
            if fa.access == AccessKind::Write {
                self.affinity_map.assign(&fa.path, agent_id);
                crate::sync_lock::rw_write(&*self.scope_guard)
                    .assign_file(agent_id, fa.path.clone());
            }
        }

        // Capture pre-task snapshot for version control (persisted to VoxDb)
        let snapshot_before = {
            let paths: Vec<PathBuf> = file_manifest.iter().map(|f| f.path.clone()).collect();
            let desc_str = task.description.clone();
            let snap_desc = format!("pre-task: {:.50}", desc_str);
            let snap_id = self
                .capture_snapshot(agent_id, &paths, snap_desc.clone())
                .await;
            self.event_bus
                .emit(crate::events::AgentEventKind::SnapshotCaptured {
                    agent_id,
                    snapshot_id: snap_id.to_string(),
                    file_count: paths.len(),
                    description: snap_desc,
                    session_id: task.session_id.clone(),
                });
            snap_id
        };

        self.record_operation(
            agent_id,
            OperationKind::TaskSubmit { task_id: task_id.0 },
            format!("Submitted task {}", task_id),
            Some(snapshot_before),
            None,
            None,
            None,
        )
        .await;

        self.record_activity();
        crate::sync_lock::rw_write(&self.monitor).record_progress(agent_id);

        let remote_relay_desc = task.description.clone();
        // Enqueue the task
        let handle = {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            if let Some(queue_lock) = agents.get(&agent_id) {
                let mut queue = crate::sync_lock::rw_write(&**queue_lock);
                self.event_bus
                    .emit(crate::events::AgentEventKind::TaskSubmitted {
                        task_id,
                        agent_id,
                        description: task.description.clone(),
                        session_id: task.session_id.clone(),
                    });
                let q_len = queue.len();
                queue.enqueue(task);
                crate::sync_lock::rw_write(&*self.task_assignments).insert(task_id, agent_id);

                tracing::info!(
                    "Task {} routed to agent {} (queue len: {})",
                    task_id,
                    agent_id,
                    q_len + 1
                );
            }
            // Capture handle while we have the lock, to use it outside
            crate::sync_lock::rw_read(&*self.agent_handles)
                .get(&agent_id)
                .cloned()
        };

        // Notify the agent process to wake up and process (outside the locks)
        if let Some(handle) = handle {
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
                        tracing::warn!("submit_task: agent notify send failed: {e:?}");
                    }
                }
                Err(_) => tracing::warn!(
                    "submit_task: agent notify timed out after {:?}",
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
                task_id,
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

        self.attach_session_retrieval_envelope_if_present(task_id, &session_id);

        let remote_params = {
            let c = crate::sync_lock::rw_read(&*self.config);
            if !c.populi_remote_execute_experimental {
                None
            } else {
                match (
                    c.populi_control_url
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                    c.populi_remote_execute_receiver_agent
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty()),
                ) {
                    (Some(b), Some(r)) => Some((
                        b.to_string(),
                        r.to_string(),
                        c.populi_http_timeout_ms,
                        c.populi_scope_id.clone(),
                        c.populi_remote_execute_sender_agent.clone(),
                    )),
                    _ => None,
                }
            }
        };

        if let Some((base, recv_s, timeout_ms, scope, send_opt)) = remote_params {
            let task_id_u = task_id.0;
            let agent_u = agent_id.0;
            let desc = remote_relay_desc;
            let caps = capability_requirements.clone();
            let send_s = send_opt.unwrap_or_default();
            tokio::spawn(async move {
                use std::time::Duration;

                let Ok(recv_id) = recv_s.parse::<u64>() else {
                    tracing::warn!(
                        "populi remote relay: receiver agent id must be a u64 (got {:?})",
                        recv_s
                    );
                    return;
                };
                let send_id = send_s.trim().parse::<u64>().unwrap_or(1);
                let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(
                    &base,
                    Duration::from_millis(timeout_ms.max(1000)),
                )
                .with_env_deliver_token();
                let now = crate::types::now_unix_ms();
                let cap_json = caps
                    .as_ref()
                    .and_then(|c| serde_json::to_string(c).ok())
                    .unwrap_or_else(|| "{}".to_string());
                let idempotency_key = format!("orch-remote-{task_id_u}-{now}");
                let payload = serde_json::json!({
                    "task_description": desc,
                    "assigned_agent_id": agent_u,
                })
                .to_string();
                let repository_id = scope
                    .clone()
                    .unwrap_or_else(|| "orchestrator-local".to_string());
                let envelope = crate::a2a::RemoteTaskEnvelope {
                    idempotency_key,
                    task_id: task_id_u,
                    repository_id,
                    capability_requirements_json: cap_json,
                    payload,
                    privacy_class: None,
                    populi_scope_id: scope.clone(),
                    submitted_unix_ms: Some(now),
                };
                if let Err(err) = crate::a2a::relay_remote_task_envelope(
                    &client,
                    crate::types::AgentId(send_id),
                    crate::types::AgentId(recv_id),
                    &envelope,
                )
                .await
                {
                    tracing::debug!(
                        error = %err,
                        task_id = task_id_u,
                        "populi experimental remote relay failed (local queue still owns execution)"
                    );
                }
            });
        }

        Ok(task_id)
    }

    /// Submit a task with planning metadata attached.
    pub async fn submit_task_with_agent_planned(
        &self,
        description: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        target_agent: Option<String>,
        capability_requirements: Option<crate::contract::TaskCapabilityHints>,
        session_id: Option<String>,
        planning_meta: Option<PlanningTaskMeta>,
    ) -> Result<TaskId, OrchestratorError> {
        let task_id = self
            .submit_task_with_agent(
                description,
                file_manifest,
                priority,
                target_agent,
                capability_requirements,
                session_id,
            )
            .await?;
        if let Some(meta) = planning_meta
            && let Some(agent_id) = crate::sync_lock::rw_read(&*self.task_assignments)
                .get(&task_id)
                .copied()
            && let Some(q_lock) = crate::sync_lock::rw_read(&*self.agents).get(&agent_id)
        {
            let _ = crate::sync_lock::rw_write(&**q_lock).attach_planning_meta(task_id, &meta);
        }
        Ok(task_id)
    }

    /// Attach Socrates evidence context to an already submitted task.
    pub fn attach_socrates_context(
        &self,
        task_id: TaskId,
        ctx: crate::socrates::SocratesTaskContext,
    ) -> Result<(), OrchestratorError> {
        let agent_id = crate::sync_lock::rw_read(&*self.task_assignments)
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;
        let agents = crate::sync_lock::rw_read(&*self.agents);
        let queue_lock = agents
            .get(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
        let attached =
            crate::sync_lock::rw_write(&**queue_lock).attach_socrates_context(task_id, ctx);
        if attached {
            Ok(())
        } else {
            Err(OrchestratorError::TaskNotFound(task_id))
        }
    }
}
