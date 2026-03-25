use std::path::PathBuf;

use crate::oplog::OperationKind;
use crate::planning::PlanningTaskMeta;
use crate::services::MessageGateway;
use crate::types::{AgentId, AgentTask, TaskId, TaskStatus};

use super::super::{Orchestrator, OrchestratorError, TaskTraceStep};

impl Orchestrator {
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
