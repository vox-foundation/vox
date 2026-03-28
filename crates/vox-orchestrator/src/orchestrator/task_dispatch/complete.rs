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
        crate::sync_lock::rw_write(&self.monitor).record_progress(agent_id);

        let (write_files, session_id, desc, phase_label, debug_iterations, plan_completion_meta) = {
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
            let (max_debug_iterations, max_toestub_debug_iterations, max_socrates_debug_iterations) = {
                let cfg = crate::sync_lock::rw_read(&*self.config);
                (
                    cfg.max_debug_iterations,
                    cfg.max_toestub_debug_iterations,
                    cfg.max_socrates_debug_iterations,
                )
            };

            #[cfg(feature = "toestub-gate")]
            {
                if crate::sync_lock::rw_read(&*self.config).toestub_gate {
                    if let Some(mut task_clone) = queue.current_task().cloned() {
                        let vr = crate::validation::post_task_validate(&task_clone);
                        if !crate::validation::quality_gate(&vr)
                            && task_clone.debug_iterations < max_toestub_debug_iterations
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
                self.record_task_loop_metric(
                    task_id,
                    &Self::extract_phase_label(&requeue_task.description),
                    "toestub_requeue",
                    requeue_task.debug_iterations,
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
                            && task.debug_iterations < max_socrates_debug_iterations
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
                self.record_task_loop_metric(
                    task_id,
                    &Self::extract_phase_label(&requeue_task.description),
                    "socrates_requeue",
                    requeue_task.debug_iterations,
                );
                queue.enqueue(requeue_task);
                return Ok(());
            }

            let current = queue.current_task().cloned();
            let plan_completion_meta = current.as_ref().and_then(|t| {
                match (
                    t.plan_session_id.clone(),
                    t.plan_node_id.clone(),
                    t.plan_version,
                ) {
                    (Some(plan_session_id), Some(plan_node_id), Some(plan_version)) => {
                        Some(PlanningTaskMeta {
                            plan_session_id,
                            plan_node_id,
                            plan_version,
                            execution_policy_json: t.execution_policy_json.clone(),
                        })
                    }
                    _ => None,
                }
            });
            let desc = current
                .as_ref()
                .map(|t| t.description.clone())
                .unwrap_or_default();
            let phase_label = Self::extract_phase_label(&desc);
            let debug_iterations = current.map(|t| t.debug_iterations).unwrap_or(0);
            if phase_label == "shard_validate" {
                queue.recent_shard_validation_failures =
                    queue.recent_shard_validation_failures.saturating_sub(1);
                queue
                    .active_skills
                    .insert("shard_validate".to_string(), 1.0_f64);
            } else if phase_label == "shard_gen" {
                queue.active_skills.insert("shard_gen".to_string(), 1.0_f64);
            } else if phase_label == "reduce" {
                queue.active_skills.insert("reduce".to_string(), 1.0_f64);
            }
            queue.mark_complete(task_id);
            (
                write_files,
                session_id,
                desc,
                phase_label,
                debug_iterations,
                plan_completion_meta,
            )
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
        let mut overlap_conflicts = 0_u32;
        for other_id in other_agents {
            let overlaps = crate::sync_lock::rw_read(&*self.workspace_manager)
                .overlapping_paths(agent_id, other_id);
            for overlap_path in overlaps {
                overlap_conflicts = overlap_conflicts.saturating_add(1);
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

        if phase_label == "reduce" && overlap_conflicts > 0 {
            let cooldown_ms =
                crate::sync_lock::rw_read(&*self.config).repo_reduce_conflict_cooldown_ms;
            let now_ms = crate::types::now_unix_ms();
            if let Some(queue_lock) = crate::sync_lock::rw_read(&*self.agents).get(&agent_id) {
                let mut queue = crate::sync_lock::rw_write(&**queue_lock);
                queue.reducer_cooldown_until_ms = Some(now_ms.saturating_add(cooldown_ms));
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
            session_id.clone(),
        );

        // Unblock dependent tasks across ALL agents
        {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            let _db_opt = self.db();
            for queue_lock in agents.values() {
                crate::sync_lock::rw_write(&**queue_lock).unblock(task_id);
            }
        }

        if let Some(db) = self.db().as_ref() {
            let _ = db
                .record_task_reliability_observation(&agent_id.0.to_string(), true)
                .await;
            if let Some(meta) = plan_completion_meta.as_ref() {
                let _ = db
                    .record_plan_node_attempt(
                        &meta.plan_session_id,
                        i64::from(meta.plan_version),
                        &meta.plan_node_id,
                        1,
                        Some(&task_id.0.to_string()),
                        "completed",
                        None,
                        None,
                    )
                    .await;
                let _ = db
                    .set_plan_node_status(
                        &meta.plan_session_id,
                        meta.plan_version as i64,
                        &meta.plan_node_id,
                        "completed",
                    )
                    .await;
                let _ = crate::planning::schedule::enqueue_runnable_plan_nodes(
                    self,
                    &meta.plan_session_id,
                    meta.plan_version,
                    session_id.clone(),
                )
                .await;
            }

            if crate::lineage::orchestration_lineage_persist_enabled() {
                let repo = crate::lineage::repository_id();
                let (ps, pn) = plan_completion_meta
                    .as_ref()
                    .map(|m| {
                        (
                            Some(m.plan_session_id.as_str()),
                            Some(m.plan_node_id.as_str()),
                        )
                    })
                    .unwrap_or((None, None));
                let payload = serde_json::json!({ "phase": phase_label });
                let payload_str = payload.to_string();
                let _ = db
                    .append_orchestration_lineage_event(
                        &repo,
                        "task_completed",
                        task_id.0 as i64,
                        Some(agent_id.0 as i64),
                        session_id.as_deref(),
                        None,
                        ps,
                        pn,
                        Some(payload_str.as_str()),
                    )
                    .await;
            }
        }
        {
            let cfg = crate::sync_lock::rw_read(&*self.config).clone();
            crate::sync_lock::rw_write(&*self.budget_manager).record_trust_outcome(
                agent_id,
                true,
                cfg.trust_ewma_alpha,
                cfg.trust_provisional_threshold,
                cfg.trust_trusted_threshold,
            );
        }

        tracing::info!("Task {} completed by agent {}", task_id, agent_id);
        self.record_task_loop_metric(task_id, &phase_label, "completed", debug_iterations);
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
            if failed_desc.contains("[PHASE:SHARD_VALIDATE]") {
                queue.recent_shard_validation_failures =
                    queue.recent_shard_validation_failures.saturating_add(1);
            }
            queue.mark_failed(task_id, reason.clone());
            (session_id, planning_meta, failed_desc)
        };

        if let Some(db) = self.db() {
            let _ =
                db.block_on(db.record_task_reliability_observation(&agent_id.0.to_string(), false));
        }
        {
            let cfg = crate::sync_lock::rw_read(&*self.config).clone();
            crate::sync_lock::rw_write(&*self.budget_manager).record_trust_outcome(
                agent_id,
                false,
                cfg.trust_ewma_alpha,
                cfg.trust_provisional_threshold,
                cfg.trust_trusted_threshold,
            );
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
            && let Some(ref meta) = planning_meta
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

        if crate::lineage::orchestration_lineage_persist_enabled() {
            if let Some(db) = self.db() {
                let repo = crate::lineage::repository_id();
                let (ps, pn) = planning_meta
                    .as_ref()
                    .map(|m| {
                        (
                            Some(m.plan_session_id.as_str()),
                            Some(m.plan_node_id.as_str()),
                        )
                    })
                    .unwrap_or((None, None));
                let preview: String = reason.chars().take(500).collect();
                let payload = serde_json::json!({ "reason_preview": preview });
                let payload_str = payload.to_string();
                let _ = db
                    .append_orchestration_lineage_event(
                        &repo,
                        "task_failed",
                        task_id.0 as i64,
                        Some(agent_id.0 as i64),
                        session_id.as_deref(),
                        None,
                        ps,
                        pn,
                        Some(payload_str.as_str()),
                    )
                    .await;
            }
        }

        tracing::warn!("Task {} failed: {}", task_id, reason);
        self.record_task_loop_metric(task_id, "unknown", "failed", 0);
        Ok(())
    }

    fn extract_phase_label(desc: &str) -> String {
        const PREFIX: &str = "[PHASE:";
        if let Some(start) = desc.find(PREFIX) {
            let suffix = &desc[start + PREFIX.len()..];
            if let Some(end) = suffix.find(']') {
                return suffix[..end].trim().to_ascii_lowercase();
            }
        }
        "single_shot".to_string()
    }

    fn record_task_loop_metric(
        &self,
        task_id: TaskId,
        phase: &str,
        outcome: &str,
        debug_iterations: u8,
    ) {
        let key = format!("task_loop_metrics/{}", task_id.0);
        let event = serde_json::json!({
            "task_id": task_id.0,
            "phase": phase,
            "outcome": outcome,
            "debug_iterations": debug_iterations,
            "ts_unix_ms": crate::types::now_unix_ms()
        });
        if let Ok(raw) = serde_json::to_string(&event) {
            crate::sync_lock::rw_read(&*self.context_store).set(AgentId(0), key, raw, 0);
        }
    }
}
