use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::{AccessKind, AgentId, AgentTask, FileAffinity, TaskId, TaskPriority, TaskStatus};
use crate::oplog::OperationKind;
use crate::scope::ScopeEnforcement;
use crate::services::{MessageGateway, PolicyCheckResult, PolicyEngine, RouteResult, RoutingService};
use crate::locks::LockKind;

use super::{Orchestrator, OrchestratorError, TaskTraceStep, MAX_TASK_TRACES};

impl Orchestrator {
    /// Create a new orchestrator with the given configuration.
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
        &mut self,
        description: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        session_id: Option<String>,
    ) -> Result<TaskId, OrchestratorError> {
        self.submit_task_with_agent(
            description,
            file_manifest,
            priority,
            None,
            None,
            session_id,
        )
        .await
    }

    /// Submit a new task to the orchestrator, potentially targeting a specific agent name (async).
    pub async fn submit_task_with_agent(
        &mut self,
        description: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        target_agent: Option<String>,
        capability_requirements: Option<crate::contract::TaskCapabilityHints>,
        session_id: Option<String>,
    ) -> Result<TaskId, OrchestratorError> {
        if !self.config.enabled {
            return Err(OrchestratorError::Disabled);
        }

        let task_id = self.task_id_gen.next();
        let priority = priority.unwrap_or(self.config.default_priority);

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
            )
            .await?;

        // Pre-queue policy check (locks; scope when enforcement enabled)
        let scope_guard = (self.config.scope_enforcement != ScopeEnforcement::Disabled)
            .then_some(&self.scope_guard);
        match PolicyEngine::check_before_queue(
            &self.lock_manager,
            scope_guard,
            &self.event_bus,
            &file_manifest,
            agent_id,
        ) {
            PolicyCheckResult::Allowed => {}
            PolicyCheckResult::LockConflict(e) => return Err(OrchestratorError::LockConflict(e)),
            PolicyCheckResult::ScopeDenied(msg) => return Err(OrchestratorError::ScopeDenied(msg)),
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
                self.scope_guard.assign_file(agent_id, fa.path.clone());
            }
        }

        // Capture pre-task snapshot for version control (persisted to CodeStore)
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
        // Enqueue the task
        if let Some(queue) = self.agents.get_mut(&agent_id) {
            self.event_bus.emit(crate::events::AgentEventKind::TaskSubmitted {
                task_id,
                agent_id,
                description: task.description.clone(),
                session_id: task.session_id.clone(),
            });
            queue.enqueue(task);
            self.task_assignments.insert(task_id, agent_id);

            // Notify the agent process to wake up and process
            if let Some(handle) = self.agent_handles.get(&agent_id) {
                let json = serde_json::to_string(&crate::runtime::AgentCommand::ProcessQueue)
                    .unwrap_or_else(|e| {
                        tracing::warn!("serialize ProcessQueue: {e}");
                        "{}".to_string()
                    });
                let env = vox_runtime::mailbox::Envelope::Message(vox_runtime::mailbox::Message {
                    from: vox_runtime::Pid::new(),
                    payload: vox_runtime::mailbox::MessagePayload::Json(json),
                });
                let _ = handle.send(env).await;
            }

            tracing::info!(
                "Task {} routed to agent {} (queue len: {})",
                task_id,
                agent_id,
                queue.len()
            );
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if self.task_traces.len() >= MAX_TASK_TRACES {
            if let Some(min_id) = self.task_traces.keys().min().copied() {
                self.task_traces.remove(&min_id);
            }
        }
        self.task_traces.insert(
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

        Ok(task_id)
    }

    /// Submit a batch of interdependent tasks (async).
    pub async fn submit_batch(
        &mut self,
        descriptors: Vec<crate::types::TaskDescriptor>,
    ) -> Result<Vec<TaskId>, OrchestratorError> {
        if !self.config.enabled {
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

            let priority = desc.priority.unwrap_or(self.config.default_priority);
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

            let scope_guard = (self.config.scope_enforcement != ScopeEnforcement::Disabled)
                .then_some(&self.scope_guard);
            match PolicyEngine::check_before_queue(
                &self.lock_manager,
                scope_guard,
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

            // Acquire locks and assign scope
            for fa in &desc.file_manifest {
                if fa.access == AccessKind::Write {
                    let _ = self
                        .lock_manager
                        .try_acquire(&fa.path, agent_id, LockKind::Exclusive);
                    self.affinity_map.assign(&fa.path, agent_id);
                    self.scope_guard.assign_file(agent_id, fa.path.clone());
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
            // Enqueue
            if let Some(queue) = self.agents.get_mut(&agent_id) {
                self.event_bus.emit(crate::events::AgentEventKind::TaskSubmitted {
                    task_id: my_id,
                    agent_id,
                    description: task.description.clone(),
                    session_id: task.session_id.clone(),
                });
                queue.enqueue(task);
                self.task_assignments.insert(my_id, agent_id);

                // Notify
                if let Some(handle) = self.agent_handles.get(&agent_id) {
                    let json = serde_json::to_string(&crate::runtime::AgentCommand::ProcessQueue)
                        .unwrap_or_else(|e| {
                            tracing::warn!("serialize ProcessQueue: {e}");
                            "{}".to_string()
                        });
                    let env =
                        vox_runtime::mailbox::Envelope::Message(vox_runtime::mailbox::Message {
                            from: vox_runtime::Pid::new(),
                            payload: vox_runtime::mailbox::MessagePayload::Json(json),
                        });
                    let _ = handle.send(env).await;
                }
            }

            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if self.task_traces.len() >= MAX_TASK_TRACES {
                if let Some(min_id) = self.task_traces.keys().min().copied() {
                    self.task_traces.remove(&min_id);
                }
            }
            self.task_traces.insert(
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

            results.push(my_id);
        }

        tracing::info!("Submitted batch of {} tasks", results.len());
        Ok(results)
    }

    /// Resolve route via RoutingService and spawn if needed.
    async fn resolve_route(
        &mut self,
        manifest: &[FileAffinity],
        target_agent: Option<&str>,
        task_capability_requirements: Option<&crate::contract::TaskCapabilityHints>,
    ) -> Result<AgentId, OrchestratorError> {
        if let Some(agent_name) = target_agent {
            // First check if an agent with this name exists
            for (id, queue) in &self.agents {
                if queue.name == agent_name {
                    return Ok(*id);
                }
            }
            // Otherwise, spawn an agent with this name
            return self.spawn_agent(agent_name);
        }

        let reliability_map: Option<HashMap<AgentId, f64>> =
            if self.config.socrates_reputation_routing {
                self.db.as_ref().map(|db| {
                    db.store()
                        .block_on(async { db.store().list_agent_reliability().await })
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(id, r)| {
                            let numeric_id = id.parse::<u64>().unwrap_or(0);
                            (AgentId(numeric_id), r)
                        })
                        .collect()
                })
            } else {
                None
            };

        let remote = if self.remote_mesh_routing_hints.is_empty() {
            None
        } else {
            Some(self.remote_mesh_routing_hints.as_slice())
        };
        let result = RoutingService::route(
            manifest,
            &self.affinity_map,
            &self.groups,
            &self.agents,
            &self.config,
            reliability_map.as_ref(),
            task_capability_requirements,
            remote,
        );
        match result {
            RouteResult::Existing(id) => Ok(id),
            RouteResult::SpawnAgent(name) => self.spawn_agent(&name),
        }
    }

    /// Mark a task as completed (async).
    pub async fn complete_task(&mut self, task_id: TaskId) -> Result<(), OrchestratorError> {
        let agent_id = self
            .task_assignments
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        self.record_activity();

        let queue = self
            .agents
            .get_mut(&agent_id)
            .ok_or(OrchestratorError::AgentNotFound(agent_id))?;

        // Get the task's file manifest before completing
        let write_files: Vec<PathBuf> = queue
            .current_task()
            .map(|t| t.write_files().into_iter().cloned().collect())
            .unwrap_or_default();
        let session_id = queue.current_task().and_then(|t| t.session_id.clone());

        let mut auto_debug_requeue = None;

        #[cfg(feature = "toestub-gate")]
        {
            if self.config.toestub_gate {
                if let Some(mut task_clone) = queue.current_task().cloned() {
                    let vr = crate::validation::post_task_validate(&task_clone);
                    if !crate::validation::quality_gate(&vr)
                        && task_clone.debug_iterations < self.config.max_debug_iterations
                    {
                        task_clone.debug_iterations += 1;
                        task_clone.description.push_str(&format!("\n\n[AUTO-DEBUG ITERATION {}]\nValidation failed with diagnostic issues. Please fix the following:\n{}", task_clone.debug_iterations, vr.report));
                        task_clone.status = TaskStatus::Queued;
                        auto_debug_requeue = Some((task_clone, vr.report.clone()));
                    }
                }
            }
        }

        if let Some((requeue_task, err_report)) = auto_debug_requeue {
            // Log it
            tracing::warn!(
                "Task {} failed validation. Auto-debugging (iteration {}/{})",
                task_id,
                requeue_task.debug_iterations,
                self.config.max_debug_iterations
            );
            queue.mark_failed(
                task_id,
                format!("Auto-debug validation failure:\n{}", err_report),
            );

            // Requeue the modified task back to the *same* queue
            queue.enqueue(requeue_task);
            return Ok(());
        }

        let mut socrates_requeue: Option<AgentTask> = None;
        {
            let policy = self.config.effective_socrates_policy();
            if let Some(task) = queue.current_task() {
                if let Some(ref ctx) = task.socrates {
                    let outcome = crate::socrates::evaluate_socrates_gate(ctx, &policy);
                    if self.config.socrates_gate_shadow {
                        tracing::info!(
                            target: "vox_orchestrator::socrates",
                            task_id = task_id.0,
                            agent_id = agent_id.0,
                            decision = ?outcome.decision,
                            confidence = outcome.confidence,
                            contradiction = outcome.contradiction_ratio,
                            "socrates gate (shadow)"
                        );
                    }
                    if self.config.socrates_gate_enforce
                        && outcome.decision != vox_socrates_policy::RiskDecision::Answer
                        && task.debug_iterations < self.config.max_debug_iterations
                    {
                        let mut t = task.clone();
                        t.debug_iterations += 1;
                        t.description.push_str(&format!(
                            "\n\n[SOCRATES GATE]\nRisk decision {:?} (confidence {:.2}, contradiction {:.2}). Improve grounding (citations, evidence) or resolve contradictions before completing.\n",
                            outcome.decision, outcome.confidence, outcome.contradiction_ratio
                        ));
                        t.status = TaskStatus::Queued;
                        socrates_requeue = Some(t);
                    }
                }
            }
        }

        if let Some(requeue_task) = socrates_requeue {
            tracing::warn!(
                task_id = task_id.0,
                "Socrates gate blocked completion; requeueing"
            );
            queue.mark_failed(task_id, "Socrates risk gate blocked completion".to_string());
            queue.enqueue(requeue_task);
            return Ok(());
        }

        let desc = queue
            .current_task()
            .map(|t| t.description.clone())
            .unwrap_or_default();
        queue.mark_complete(task_id);

        // Capture post-task snapshot and record in oplog (persisted to CodeStore)
        let snap_desc = format!("post-task complete: {:.50}", desc);
        let snapshot_after = self
            .capture_snapshot(agent_id, &write_files, snap_desc.clone())
            .await;

        self.event_bus
            .emit(crate::events::AgentEventKind::SnapshotCaptured {
                agent_id,
                snapshot_id: snapshot_after.to_string(),
                file_count: write_files.len(),
                description: snap_desc,
                session_id: session_id.clone(),
            });

        // Find pre-task snapshots from the oplog to link this completion
        let (snap_before, db_snap_before) = self.oplog.find_task_snapshots(task_id.0);
        let db_snap_after = self
            .take_db_snapshot(agent_id, format!("post-task-complete: {}", task_id))
            .await;

        self.record_operation(
            agent_id,
            OperationKind::TaskComplete { task_id: task_id.0 },
            format!("Completed task {}", task_id),
            snap_before,
            Some(snapshot_after),
            db_snap_before,
            db_snap_after,
        )
        .await;

        // Auto-detect conflicts: check if any other agent's workspace overlaps
        let other_agents: Vec<AgentId> = self
            .agents
            .keys()
            .filter(|&&id| id != agent_id && self.workspace_manager.has_workspace(id))
            .copied()
            .collect();
        for other_id in other_agents {
            let overlaps = self.workspace_manager.overlapping_paths(agent_id, other_id);
            for overlap_path in overlaps {
                let conflict_id = self.conflict_manager.record_conflict(
                    overlap_path.clone(),
                    Some(snapshot_after),
                    vec![(agent_id, snapshot_after), (other_id, snapshot_after)],
                );
                self.event_bus
                    .emit(crate::events::AgentEventKind::ConflictDetected {
                        path: overlap_path,
                        agent_ids: vec![agent_id, other_id],
                        conflict_id: conflict_id.to_string(),
                    });
                tracing::warn!(
                    "Conflict {} detected between {} and {}",
                    conflict_id,
                    agent_id,
                    other_id
                );
            }
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Some(steps) = self.task_traces.get_mut(&task_id) {
            steps.push(TaskTraceStep {
                stage: "outcome".to_string(),
                timestamp_ms: now_ms,
                detail: Some("completed".to_string()),
            });
        }

        // Release file locks for this task's write files
        for path in &write_files {
            self.lock_manager.release(path, agent_id);
        }

        MessageGateway::publish_task_completed(
            &mut self.bulletin,
            &mut self.message_bus,
            &self.event_bus,
            task_id,
            agent_id,
            session_id,
        );

        // Unblock dependent tasks across ALL agents
        for queue in self.agents.values_mut() {
            queue.unblock(task_id);
        }

        if let Some(db) = &self.db {
            let _ = db.store().block_on(db.store().record_task_reliability_observation(&agent_id.0.to_string(), true));
        }

        tracing::info!("Task {} completed by agent {}", task_id, agent_id);
        Ok(())
    }

    /// Mark a task as failed (async).
    pub async fn fail_task(
        &mut self,
        task_id: TaskId,
        reason: String,
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

        let session_id = queue.current_task().and_then(|t| t.session_id.clone());
        queue.mark_failed(task_id, reason.clone());

        if let Some(db) = &self.db {
            let _ = db.store().block_on(db.store().record_task_reliability_observation(&agent_id.0.to_string(), false));
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Some(steps) = self.task_traces.get_mut(&task_id) {
            steps.push(TaskTraceStep {
                stage: "outcome".to_string(),
                timestamp_ms: now_ms,
                detail: Some(format!("failed: {}", reason)),
            });
        }

        // Release locks
        self.lock_manager.release_all(agent_id);

        // Find pre-task snapshots to link this failure
        let (snap_before, db_snap_before) = self.oplog.find_task_snapshots(task_id.0);

        // Record failure in oplog (async to support DB snapshot)
        self.record_operation(
            agent_id,
            OperationKind::TaskFail {
                task_id: task_id.0,
                reason: reason.clone(),
            },
            format!("Failed task {}", task_id),
            snap_before,
            None,
            db_snap_before,
            None,
        )
        .await;

        MessageGateway::publish_task_failed(
            &mut self.bulletin,
            &self.event_bus,
            task_id,
            agent_id,
            reason.clone(),
            session_id,
        );

        tracing::warn!("Task {} failed: {}", task_id, reason);
        Ok(())
    }
}

