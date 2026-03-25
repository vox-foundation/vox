use std::collections::HashMap;
use std::path::PathBuf;

use crate::locks::LockKind;
use crate::oplog::OperationKind;
use crate::planning::{PlanningMode, PlanningStrategy, PlanningTaskMeta};
use crate::scope::ScopeEnforcement;
use crate::services::{
    MessageGateway, PolicyCheckResult, PolicyEngine, RouteResult, RoutingService,
};
use crate::types::{
    AccessKind, AgentId, AgentTask, FileAffinity, TaskId, TaskPriority, TaskStatus,
};

use super::{MAX_TASK_TRACES, Orchestrator, OrchestratorError, TaskTraceStep};

impl Orchestrator {
    /// If the context store holds a session-scoped retrieval envelope, attach it to the task.
    fn attach_session_retrieval_envelope_if_present(
        &self,
        task_id: TaskId,
        session_id: &Option<String>,
    ) {
        let Some(sid) = session_id.as_ref() else {
            return;
        };
        let key = crate::socrates::session_retrieval_envelope_key(sid);
        let raw_opt = crate::sync_lock::rw_read(&*self.context_store).get(&key);
        if let Some(raw) = raw_opt {
            if let Ok(env) =
                serde_json::from_str::<crate::socrates::SessionRetrievalEnvelope>(&raw)
            {
                if let Err(e) = self.attach_socrates_context(task_id, env.to_task_context()) {
                    tracing::debug!(
                        task_id = task_id.0,
                        error = %e,
                        "session retrieval envelope parse OK but Socrates attach failed"
                    );
                }
            }
        }
    }

    /// Submit a higher-level goal that may be routed through planning.
    pub async fn submit_goal(
        &self,
        goal: impl Into<String>,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        planning_mode: Option<PlanningMode>,
        session_id: Option<String>,
    ) -> Result<TaskId, OrchestratorError> {
        let goal = goal.into();
        let cfg = crate::sync_lock::rw_read(&*self.config).clone();
        if planning_mode.is_none()
            && (!cfg.planning_auto_mode_enabled || cfg.planning_rollout_percent == 0)
        {
            return self
                .submit_task_with_agent(goal, file_manifest, priority, None, None, session_id)
                .await;
        }
        if planning_mode.is_none() {
            let selector = seahash::hash(goal.as_bytes()) % 100;
            if selector >= u64::from(cfg.planning_rollout_percent) {
                return self
                    .submit_task_with_agent(goal, file_manifest, priority, None, None, session_id)
                    .await;
            }
        }
        let eval = crate::planning::intake_router::evaluate_goal(&cfg, &goal, planning_mode);
        self.event_bus
            .emit(crate::events::AgentEventKind::PlanningRouted {
                strategy: format!("{:?}", eval.strategy),
                complexity: eval.complexity,
                confidence: eval.confidence,
                rationale: eval.rationale.clone(),
            });

        if cfg.planning_shadow_mode || eval.strategy == PlanningStrategy::ImmediateAct {
            return self
                .submit_task_with_agent(goal, file_manifest, priority, None, None, session_id)
                .await;
        }

        if eval.strategy == PlanningStrategy::WorkflowHandoff
            && cfg.planning_workflow_handoff_enabled
        {
            return self
                .submit_workflow_handoff_goal(goal, file_manifest, priority, session_id)
                .await;
        }

        let plan_session_id = format!("plan-{}", uuid::Uuid::new_v4());
        let plan_version = 1_u32;
        let nodes = crate::planning::synthesizer::synthesize_plan_nodes(&goal);
        if let Some(db) = self.db() {
            let strategy = format!("{:?}", eval.strategy);
            let _ = db
                .create_plan_session(&plan_session_id, session_id.as_deref(), &goal, &strategy)
                .await;
            let _ = db
                .append_plan_version(&plan_session_id, plan_version as i64, None, None, None)
                .await;
            for n in &nodes {
                let deps_json =
                    serde_json::to_string(&n.depends_on).unwrap_or_else(|_| "[]".to_string());
                let pol_json =
                    serde_json::to_string(&n.execution_policy).unwrap_or_else(|_| "{}".to_string());
                let _ = db
                    .upsert_plan_node(
                        &plan_session_id,
                        plan_version as i64,
                        &n.node_id,
                        &n.description,
                        &deps_json,
                        &pol_json,
                        "pending",
                        n.workflow_invocation.as_deref(),
                    )
                    .await;
            }
        }
        self.event_bus
            .emit(crate::events::AgentEventKind::PlanSessionCreated {
                plan_session_id: plan_session_id.clone(),
                strategy: format!("{:?}", eval.strategy),
                version: plan_version as i64,
            });

        let first = nodes
            .first()
            .cloned()
            .unwrap_or_else(|| crate::planning::PlanNode {
                node_id: "n1".to_string(),
                description: goal.clone(),
                depends_on: vec![],
                status: crate::planning::PlanStatus::Pending,
                execution_policy: crate::planning::ExecutionPolicy::default(),
                workflow_invocation: None,
            });
        crate::planning::executor_bridge::enqueue_plan_node(
            self,
            &first,
            &plan_session_id,
            plan_version,
            session_id,
        )
        .await
    }

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
            )
            .await?;

        // Pre-queue policy check (locks; scope when enforcement enabled)
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
            let _ = handle.send(env).await;
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

                // Acquire locks and assign scope
                for fa in &desc.file_manifest {
                    if fa.access == AccessKind::Write {
                        let _ =
                            self.lock_manager
                                .try_acquire(&fa.path, agent_id, LockKind::Exclusive);
                        self.affinity_map.assign(&fa.path, agent_id);
                        crate::sync_lock::rw_write(&*self.scope_guard)
                            .assign_file(agent_id, fa.path.clone());
                    }
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
                let _ = handle.send(env).await;
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
    async fn resolve_route(
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

    /// Mark a task as completed (async).
    pub async fn complete_task(&self, task_id: TaskId) -> Result<(), OrchestratorError> {
        let agent_id = crate::sync_lock::rw_read(&*self.task_assignments)
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        self.record_activity();

        let (write_files, session_id, desc) = {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            let queue_lock = agents
                .get(&agent_id)
                .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
            let mut queue = crate::sync_lock::rw_write(&**queue_lock);

            // Get the task's file manifest before completing
            let write_files: Vec<PathBuf> = queue
                .current_task()
                .map(|t| t.write_files().into_iter().cloned().collect())
                .unwrap_or_default();
            let session_id = queue.current_task().and_then(|t| t.session_id.clone());

            let mut auto_debug_requeue = None;
            let max_debug_iterations =
                crate::sync_lock::rw_read(&*self.config).max_debug_iterations;

            #[cfg(feature = "toestub-gate")]
            {
                if crate::sync_lock::rw_read(&*self.config).toestub_gate {
                    if let Some(mut task_clone) = queue.current_task().cloned() {
                        let vr = crate::validation::post_task_validate(&task_clone);
                        if !crate::validation::quality_gate(&vr)
                            && task_clone.debug_iterations < max_debug_iterations
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
                    max_debug_iterations
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
                let config = crate::sync_lock::rw_read(&*self.config);
                let policy = config.effective_socrates_policy();
                if let Some(task) = queue.current_task() {
                    if let Some(ref ctx) = task.socrates {
                        let outcome = crate::socrates::evaluate_socrates_gate(ctx, &policy);
                        if config.socrates_gate_shadow {
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
                        if config.socrates_gate_enforce
                            && outcome.decision != vox_socrates_policy::RiskDecision::Answer
                            && task.debug_iterations < config.max_debug_iterations
                        {
                            let mut t = task.clone();
                            if let Some(ref sid) = t.session_id {
                                let key = crate::socrates::session_retrieval_envelope_key(sid);
                                let raw_opt =
                                    crate::sync_lock::rw_read(&*self.context_store).get(&key);
                                if let Some(raw) = raw_opt {
                                    if let Ok(env) = serde_json::from_str::<
                                        crate::socrates::SessionRetrievalEnvelope,
                                    >(&raw)
                                    {
                                        t.socrates = Some(env.merge_into(t.socrates.clone()));
                                    }
                                }
                            }
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
            (write_files, session_id, desc)
        };

        // Find pre-task snapshots from the oplog to link this completion
        let (snap_before, db_snap_before) =
            crate::sync_lock::rw_read(&*self.oplog).find_task_snapshots(task_id.0);

        // Capture post-task snapshot and record in oplog (persisted to VoxDb)
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
        let other_agents: Vec<AgentId> = {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            let wm = crate::sync_lock::rw_read(&*self.workspace_manager);
            agents
                .keys()
                .filter(|&&id| id != agent_id && wm.has_workspace(id))
                .copied()
                .collect()
        };
        for other_id in other_agents {
            let overlaps = crate::sync_lock::rw_read(&*self.workspace_manager)
                .overlapping_paths(agent_id, other_id);
            for overlap_path in overlaps {
                let conflict_id = crate::sync_lock::rw_write(&*self.conflict_manager)
                    .record_conflict(
                        overlap_path.clone(),
                        Some(snapshot_after),
                        vec![(agent_id, snapshot_after), (other_id, snapshot_after)],
                    );
                self.event_bus
                    .emit(crate::events::AgentEventKind::ConflictDetected {
                        path: overlap_path.clone(),
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
        if let Some(steps) = crate::sync_lock::rw_write(&*self.task_traces).get_mut(&task_id) {
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
            &self.bulletin,
            &self.message_bus,
            &self.event_bus,
            task_id,
            agent_id,
            session_id,
        );

        // Unblock dependent tasks across ALL agents
        {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            let _db_opt = self.db();
            for queue_lock in agents.values() {
                crate::sync_lock::rw_write(&**queue_lock).unblock(task_id);
            }
        }

        if let Some(db) = self.db() {
            let _ =
                db.block_on(db.record_task_reliability_observation(&agent_id.0.to_string(), true));
            // Best-effort planning attempt persistence for plan-linked tasks.
            if let Some(queue_lock) = crate::sync_lock::rw_read(&*self.agents).get(&agent_id) {
                let queue = crate::sync_lock::rw_read(&**queue_lock);
                if let Some(task) = queue.current_task()
                    && let (Some(ps), Some(node), Some(ver)) = (
                        task.plan_session_id.as_deref(),
                        task.plan_node_id.as_deref(),
                        task.plan_version,
                    )
                {
                    let _ = db.block_on(db.record_plan_node_attempt(
                        ps,
                        i64::from(ver),
                        node,
                        1,
                        Some(&task_id.0.to_string()),
                        "completed",
                        None,
                        None,
                    ));
                }
            }
        }

        tracing::info!("Task {} completed by agent {}", task_id, agent_id);
        Ok(())
    }

    /// Mark a task as failed (async).
    pub async fn fail_task(
        &self,
        task_id: TaskId,
        reason: String,
    ) -> Result<(), OrchestratorError> {
        let agent_id = crate::sync_lock::rw_read(&*self.task_assignments)
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        let (session_id, planning_meta, failed_desc) = {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            let queue_lock = agents
                .get(&agent_id)
                .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
            let mut queue = crate::sync_lock::rw_write(&**queue_lock);

            let session_id = queue.current_task().and_then(|t| t.session_id.clone());
            let planning_meta = queue.current_task().and_then(|t| {
                if let (Some(plan_session_id), Some(plan_node_id), Some(plan_version)) = (
                    t.plan_session_id.clone(),
                    t.plan_node_id.clone(),
                    t.plan_version,
                ) {
                    Some(PlanningTaskMeta {
                        plan_session_id,
                        plan_node_id,
                        plan_version,
                        execution_policy_json: t.execution_policy_json.clone(),
                    })
                } else {
                    None
                }
            });
            let failed_desc = queue
                .current_task()
                .map(|t| t.description.clone())
                .unwrap_or_default();
            queue.mark_failed(task_id, reason.clone());
            (session_id, planning_meta, failed_desc)
        };

        if let Some(db) = self.db() {
            let _ =
                db.block_on(db.record_task_reliability_observation(&agent_id.0.to_string(), false));
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if let Some(steps) = crate::sync_lock::rw_write(&*self.task_traces).get_mut(&task_id) {
            steps.push(TaskTraceStep {
                stage: "outcome".to_string(),
                timestamp_ms: now_ms,
                detail: Some(format!("failed: {}", reason)),
            });
        }

        // Release locks
        self.lock_manager.release_all(agent_id);

        // Find pre-task snapshots to link this failure
        let (snap_before, db_snap_before) =
            crate::sync_lock::rw_read(&*self.oplog).find_task_snapshots(task_id.0);

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
            &self.bulletin,
            &self.event_bus,
            task_id,
            agent_id,
            reason.clone(),
            session_id.clone(),
        );

        let planning_cfg = crate::sync_lock::rw_read(&*self.config).clone();
        if planning_cfg.planning_enabled
            && planning_cfg.planning_replan_enabled
            && let Some(meta) = planning_meta
            && crate::planning::replan::trigger_matches(
                &reason,
                meta.execution_policy_json.as_deref(),
            )
        {
            if let Some(db) = self.db() {
                let _ = db
                    .record_plan_node_attempt(
                        &meta.plan_session_id,
                        i64::from(meta.plan_version),
                        &meta.plan_node_id,
                        1,
                        Some(&task_id.0.to_string()),
                        "failed",
                        Some(&reason),
                        None,
                    )
                    .await;
                let next_version = i64::from(meta.plan_version) + 1;
                let _ = db
                    .append_plan_version(
                        &meta.plan_session_id,
                        next_version,
                        Some(i64::from(meta.plan_version)),
                        Some("task_failed"),
                        Some(
                            &serde_json::json!({
                                "task_id": task_id.0,
                                "reason": reason,
                            })
                            .to_string(),
                        ),
                    )
                    .await;
                self.event_bus
                    .emit(crate::events::AgentEventKind::PlanVersionCreated {
                        plan_session_id: meta.plan_session_id.clone(),
                        version: next_version,
                        parent_version: Some(i64::from(meta.plan_version)),
                    });
                self.event_bus
                    .emit(crate::events::AgentEventKind::ReplanTriggered {
                        plan_session_id: meta.plan_session_id.clone(),
                        node_id: meta.plan_node_id.clone(),
                        reason: reason.clone(),
                        next_version,
                    });
            }
            let _ = crate::planning::replan::enqueue_recovery_first_node(
                self,
                &meta,
                &reason,
                &failed_desc,
                session_id.clone(),
            )
            .await;
        }

        tracing::warn!("Task {} failed: {}", task_id, reason);
        Ok(())
    }
}
