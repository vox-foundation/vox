//! Task completion / attestation path. CI-side completion-quality rules and tiers live in
//! `contracts/operations/completion-policy.v1.yaml` (`vox ci completion-audit|gates`); this module
//! enforces runtime harness expectations and [`crate::types::CompletionAttestation`] adequacy.

use std::path::PathBuf;

use crate::ApprovalTier;
use crate::oplog::OperationKind;
use crate::planning::PlanningTaskMeta;
use crate::services::MessageGateway;
use crate::types::{AgentId, AgentTask, CompletionAttestation, TaskId, TaskStatus};

use super::super::persistence_outbox::PERSISTENCE_OUTBOX_KEY;
use super::super::{Orchestrator, OrchestratorError, TaskTraceStep};

fn completion_attestation_satisfies_tier(
    tier: ApprovalTier,
    attestation: Option<&CompletionAttestation>,
) -> bool {
    match tier {
        ApprovalTier::AutoApprove => true,
        ApprovalTier::Confirm => attestation
            .map(|a| !a.checks_passed.is_empty() || a.declared_non_placeholder)
            .unwrap_or(false),
        ApprovalTier::Review => attestation
            .map(|a| {
                a.checks_passed.iter().any(|c| {
                    let normalized = c.trim().to_ascii_lowercase();
                    normalized == "human_review_approved"
                        || normalized == "peer_review_approved"
                        || normalized == "approval_reviewed"
                })
            })
            .unwrap_or(false),
        ApprovalTier::Blocked => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HarnessCompletionGuardMode {
    Off,
    Shadow,
    Enforce,
}

fn harness_completion_guard_mode() -> HarnessCompletionGuardMode {
    let raw = std::env::var("VOX_ORCHESTRATOR_HARNESS_COMPLETION_GUARD")
        .unwrap_or_else(|_| "off".to_string());
    match raw.trim().to_ascii_lowercase().as_str() {
        "shadow" | "warn" => HarnessCompletionGuardMode::Shadow,
        "enforce" | "strict" => HarnessCompletionGuardMode::Enforce,
        _ => HarnessCompletionGuardMode::Off,
    }
}

fn normalize_path_for_match(raw: &str) -> String {
    raw.trim().replace('\\', "/")
}

fn harness_completion_issues(
    task: &AgentTask,
    completion_attestation: Option<&CompletionAttestation>,
) -> Result<(Option<String>, Vec<String>), String> {
    let Some(raw_harness) = task
        .harness_spec_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok((None, Vec::new()));
    };
    let harness = serde_json::from_str::<crate::AgentHarnessSpec>(raw_harness)
        .map_err(|e| format!("task harness_spec_json parse failed: {e}"))?;
    let mut issues = Vec::new();
    if harness.contracts.requires_artifact_backed_completion {
        let provided_artifacts = completion_attestation
            .map(|a| {
                a.artifact_paths
                    .iter()
                    .map(|p| normalize_path_for_match(&p.to_string_lossy()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for output in harness
            .contracts
            .required_outputs
            .iter()
            .filter(|x| x.required)
        {
            let Some(path_hint) = output.path_hint.as_deref() else {
                continue;
            };
            let expected = normalize_path_for_match(path_hint);
            let present = provided_artifacts
                .iter()
                .any(|p| p == &expected || p.ends_with(&expected));
            if !present {
                issues.push(format!(
                    "missing required harness artifact path {:?} for output {:?}",
                    path_hint, output.artifact_id
                ));
            }
        }
    }
    let evidence_required = harness
        .contracts
        .completion_gates
        .iter()
        .any(|g| g.evidence_required);
    if evidence_required {
        let evidence_present = completion_attestation
            .map(|a| !a.evidence_citations.is_empty())
            .unwrap_or(false);
        if !evidence_present {
            issues.push("harness completion gate requires evidence_citations[] in completion attestation".to_string());
        }
    }
    Ok((Some(harness.harness_id), issues))
}

impl Orchestrator {
    pub async fn complete_task(&self, task_id: TaskId) -> Result<(), OrchestratorError> {
        self.complete_task_with_attestation(task_id, None).await
    }

    pub async fn complete_task_with_attestation(
        &self,
        task_id: TaskId,
        completion_attestation: Option<CompletionAttestation>,
    ) -> Result<(), OrchestratorError> {
        let agent_id = crate::sync_lock::rw_read(&*self.task_assignments)
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        self.record_activity();
        crate::sync_lock::rw_write(&self.monitor).record_progress(agent_id);

        #[cfg(feature = "toestub-gate")]
        let mut toestub_lineage_deferred: Option<(
            std::sync::Arc<vox_db::VoxDb>,
            String,
            i64,
            Option<i64>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        )> = None;

        let completion_data: Option<(
            Vec<PathBuf>,
            Option<String>,
            String,
            String,
            u8,
            Option<PlanningTaskMeta>,
            Option<String>,
            Option<crate::reconstruction::ReconstructionBenchmarkTier>,
        )> = {
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

            let mut auto_debug_requeue: Option<(AgentTask, String, usize, usize)> = None;
            let (max_debug_iterations, max_toestub_debug_iterations, max_socrates_debug_iterations) = {
                let cfg = crate::sync_lock::rw_read(&*self.config);
                (
                    cfg.max_debug_iterations,
                    cfg.max_toestub_debug_iterations,
                    cfg.max_socrates_debug_iterations,
                )
            };
            if auto_debug_requeue.is_none()
                && let Some(mut task_clone) = queue.current_task().cloned()
                && let Some(tier) = task_clone.approval_tier
            {
                let ok =
                    completion_attestation_satisfies_tier(tier, completion_attestation.as_ref());
                if !ok && task_clone.debug_iterations < max_debug_iterations {
                    task_clone.debug_iterations += 1;
                    let requirement = match tier {
                        ApprovalTier::Confirm => {
                            "completion attestation with non-empty checks_passed[] (or declared_non_placeholder=true)"
                        }
                        ApprovalTier::Review => {
                            "checks_passed[] containing one of: human_review_approved, peer_review_approved, approval_reviewed"
                        }
                        ApprovalTier::Blocked => {
                            "task is approval-blocked and cannot be completed without policy override"
                        }
                        ApprovalTier::AutoApprove => "no extra attestation required",
                    };
                    task_clone.description.push_str(&format!(
                        "\n\n[APPROVAL GATE]\nTask tier `{tier:?}` requires {requirement} before completion."
                    ));
                    task_clone.status = TaskStatus::Queued;
                    auto_debug_requeue = Some((
                        task_clone,
                        format!("approval gate blocked completion for tier {tier:?}"),
                        1usize,
                        0usize,
                    ));
                } else if !ok {
                    return Err(OrchestratorError::ApprovalAttestationRequired(format!(
                        "task {task_id} failed approval attestation for tier {tier:?}"
                    )));
                }
            }
            // Multidimensional trust completion gate:
            // when historical task-completion trust is below floor for this phase, require at least one
            // explicit completion check to reduce silent regressions from low-reliability agents.
            if auto_debug_requeue.is_none()
                && let Some(db) = self.db()
                && let Some(mut task_clone) = queue.current_task().cloned()
            {
                let phase = Self::extract_phase_label(&task_clone.description);
                let trust_score = db.block_on(async {
                    let rows = db
                        .list_trust_scores_for_dimension(
                            "agent",
                            "task_completion",
                            Some(phase.as_str()),
                            512,
                        )
                        .await
                        .unwrap_or_default();
                    rows.into_iter()
                        .find_map(|(id, score)| {
                            id.parse::<u64>()
                                .ok()
                                .filter(|aid| *aid == agent_id.0)
                                .map(|_| score)
                        })
                        .unwrap_or(0.5)
                });
                let floor = crate::sync_lock::rw_read(&*self.config).trust_task_completion_floor;
                let has_checks = completion_attestation
                    .as_ref()
                    .map(|a| !a.checks_passed.is_empty())
                    .unwrap_or(false);
                if trust_score < floor
                    && !has_checks
                    && task_clone.debug_iterations < max_toestub_debug_iterations
                {
                    task_clone.debug_iterations += 1;
                    task_clone.description.push_str(&format!(
                        "\n\n[TRUST GATE]\nAgent task-completion trust {:.2} is below floor {:.2} for phase `{}`. \
Provide completion attestation checks_passed[] before completing.",
                        trust_score, floor, phase
                    ));
                    task_clone.status = TaskStatus::Queued;
                    auto_debug_requeue = Some((
                        task_clone,
                        format!(
                            "trust gate blocked completion (score {:.2} < floor {:.2}, checks_passed missing)",
                            trust_score, floor
                        ),
                        1usize,
                        0usize,
                    ));
                }
            }
            if auto_debug_requeue.is_none()
                && let Some(mut task_clone) = queue.current_task().cloned()
            {
                let guard_mode = harness_completion_guard_mode();
                if guard_mode != HarnessCompletionGuardMode::Off {
                    match harness_completion_issues(&task_clone, completion_attestation.as_ref()) {
                        Ok((harness_id, issues)) if !issues.is_empty() => {
                            let detail = format!(
                                "{}{}",
                                harness_id
                                    .as_deref()
                                    .map(|h| format!("harness_id={h}; "))
                                    .unwrap_or_default(),
                                issues.join("; ")
                            );
                            tracing::warn!(
                                task_id = task_id.0,
                                mode = ?guard_mode,
                                "{}",
                                detail
                            );
                            if guard_mode == HarnessCompletionGuardMode::Enforce {
                                if task_clone.debug_iterations < max_debug_iterations {
                                    task_clone.debug_iterations += 1;
                                    task_clone.description.push_str(&format!(
                                        "\n\n[HARNESS COMPLETION GATE]\nCompletion blocked until harness requirements are satisfied:\n{}",
                                        detail
                                    ));
                                    task_clone.status = TaskStatus::Queued;
                                    auto_debug_requeue =
                                        Some((task_clone, detail.clone(), 1usize, 0usize));
                                } else {
                                    return Err(OrchestratorError::ScopeDenied(format!(
                                        "harness completion gate rejected completion for task {task_id}: {detail}"
                                    )));
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(err) => {
                            tracing::warn!(
                                task_id = task_id.0,
                                mode = ?guard_mode,
                                error = %err,
                                "harness completion gate evaluation failed"
                            );
                            if guard_mode == HarnessCompletionGuardMode::Enforce {
                                return Err(OrchestratorError::ScopeDenied(format!(
                                    "harness completion gate failed to evaluate task {task_id}: {err}"
                                )));
                            }
                        }
                    }
                }
            }

            #[cfg(feature = "toestub-gate")]
            {
                if crate::sync_lock::rw_read(&*self.config).toestub_gate {
                    if let Some(mut task_clone) = queue.current_task().cloned() {
                        match crate::services::PolicyEngine::check_completion_before_complete(
                            Some(&task_clone),
                            completion_attestation.as_ref(),
                        ) {
                            crate::services::PolicyCheckResult::Allowed => {}
                            crate::services::PolicyCheckResult::ScopeDenied(reason) => {
                                if task_clone.debug_iterations < max_toestub_debug_iterations {
                                    task_clone.debug_iterations += 1;
                                    task_clone.description.push_str(&format!(
                                        "\n\n[AUTO-DEBUG ITERATION {}]\nCompletion policy blocked completion. Fix and retry:\n{}",
                                        task_clone.debug_iterations, reason
                                    ));
                                    task_clone.status = TaskStatus::Queued;
                                    auto_debug_requeue =
                                        Some((task_clone.clone(), reason.clone(), 1usize, 0usize));
                                } else {
                                    return Err(OrchestratorError::ScopeDenied(reason));
                                }
                            }
                            crate::services::PolicyCheckResult::LockConflict(conflict) => {
                                return Err(OrchestratorError::LockConflict(conflict));
                            }
                        }
                        if auto_debug_requeue.is_some() {
                            // Requeue already set by completion policy branch.
                            // Skip TOESTUB validation for this iteration.
                            // (The requeued task will be validated again on next completion attempt.)
                            // intentionally empty
                        } else {
                            let vr = crate::validation::post_task_validate(
                                &task_clone,
                                completion_attestation.as_ref(),
                            );
                            if !crate::validation::quality_gate(&vr)
                                && task_clone.debug_iterations < max_toestub_debug_iterations
                            {
                                task_clone.debug_iterations += 1;
                                task_clone.description.push_str(&format!("\n\n[AUTO-DEBUG ITERATION {}]\nValidation failed with diagnostic issues. Please fix the following:\n{}", task_clone.debug_iterations, vr.report));
                                task_clone.status = TaskStatus::Queued;
                                auto_debug_requeue = Some((
                                    task_clone,
                                    vr.report.clone(),
                                    vr.error_count,
                                    vr.warning_count,
                                ));
                            }
                        }
                    }
                }
            }

            if auto_debug_requeue.is_none()
                && let Some(mut task_clone) = queue.current_task().cloned()
            {
                let md_files: Vec<_> = write_files.iter()
                    .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
                    .collect();
                
                if !md_files.is_empty() {
                    let mut broken_reports = Vec::new();
                    for md_file in md_files {
                        let out = std::process::Command::new("cargo")
                            .arg("run")
                            .arg("-p")
                            .arg("vox-cli")
                            .arg("--")
                            .arg("ci")
                            .arg("check-links")
                            .arg("--target")
                            .arg(md_file)
                            .output();
                        if let Ok(out) = out {
                            if !out.status.success() {
                                broken_reports.push(String::from_utf8_lossy(&out.stdout).to_string());
                            }
                        }
                    }
                    
                    if !broken_reports.is_empty() {
                        let detail = format!("Broken links detected. Please investigate if files need to be created or fix the links:\n{}", broken_reports.join("\n"));
                        if task_clone.debug_iterations < max_debug_iterations {
                            task_clone.debug_iterations += 1;
                            task_clone.description.push_str(&format!("\n\n[DOC INTEGRITY GATE]\n{}", detail));
                            task_clone.status = TaskStatus::Queued;
                            auto_debug_requeue = Some((task_clone, detail, 1, 0));
                        }
                    }
                }
            }

            if let Some((requeue_task, err_report, err_n, warn_n)) = auto_debug_requeue {
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

                #[cfg(feature = "toestub-gate")]
                {
                    if crate::lineage::orchestration_lineage_persist_enabled() {
                        if let Some(db) = self.db() {
                            let repo = crate::lineage::repository_id();
                            let mut payload = serde_json::json!({
                                "toestub_iteration": requeue_task.debug_iterations,
                                "error_count": err_n,
                                "warning_count": warn_n,
                                "report_preview": err_report.chars().take(800).collect::<String>(),
                            });
                            if let Some(cid) = crate::lineage::orchestration_campaign_id() {
                                payload["campaign_id"] = serde_json::Value::String(cid);
                            }
                            let payload_str = payload.to_string();
                            toestub_lineage_deferred = Some((
                                db,
                                repo,
                                task_id.0 as i64,
                                Some(agent_id.0 as i64),
                                session_id.clone(),
                                requeue_task.plan_session_id.clone(),
                                requeue_task.plan_node_id.clone(),
                                payload_str,
                            ));
                        }
                    }
                }

                // Requeue the modified task back to the *same* queue
                queue.enqueue(requeue_task);
                None
            } else {
                let mut socrates_requeue: Option<AgentTask> = None;
                {
                    let config = crate::sync_lock::rw_read(&*self.config);
                    let policy = config.effective_socrates_policy();
                    let trust_relax_gates = config.trust_gate_relax_enabled
                        && self
                            .lookup_agent_reliability_sync(agent_id)
                            .is_some_and(|r| r >= config.trust_gate_relax_min_reliability);
                    if let Some(task) = queue.current_task() {
                        if let Some(ref ctx) = task.socrates {
                            let envelope_raw = task.session_id.as_ref().and_then(|sid| {
                                let key = crate::socrates::session_context_envelope_key(sid);
                                crate::sync_lock::rw_read(&*self.context_store).get(&key)
                            });
                            if config.completion_grounding_shadow || config.completion_grounding_enforce
                            {
                                let declared =
                                    crate::grounding::declared_evidence_citations(
                                        completion_attestation.as_ref(),
                                    );
                                let grounding_msg = if !declared.is_empty() {
                                    crate::grounding::grounding_violation_declared_not_in_envelope(
                                        completion_attestation.as_ref(),
                                        envelope_raw.as_deref(),
                                    )
                                } else {
                                    crate::grounding::grounding_violation_factual_mode_without_declarations(
                                        completion_attestation.as_ref(),
                                        ctx,
                                    )
                                };
                                if let Some(msg) = grounding_msg {
                                    let violation_kind = if !declared.is_empty() {
                                        "declared_not_in_envelope"
                                    } else {
                                        "factual_without_declarations"
                                    };
                                    if config.completion_grounding_shadow {
                                        tracing::info!(
                                            target: "vox_orchestrator::grounding",
                                            task_id = task_id.0,
                                            agent_id = agent_id.0,
                                            violation_kind,
                                            "{msg}"
                                        );
                                    }
                                    if config.completion_grounding_enforce
                                        && !trust_relax_gates
                                        && task.debug_iterations < max_socrates_debug_iterations
                                    {
                                        tracing::warn!(
                                            target: "vox_orchestrator::grounding",
                                            task_id = task_id.0,
                                            agent_id = agent_id.0,
                                            violation_kind,
                                            requeued = true,
                                            "completion grounding enforce: task re-queued for more evidence",
                                        );
                                        let mut t = task.clone();
                                        t.debug_iterations += 1;
                                        t.description.push_str(&format!(
                                            "\n\n[GROUNDING GATE]\n{msg}\n",
                                        ));
                                        t.status = TaskStatus::Queued;
                                        socrates_requeue = Some(t);
                                    }
                                }
                            }
                            if socrates_requeue.is_none() {
                                let mut augmented =
                                    crate::grounding::merge_attestation_into_socrates_context(
                                        (*ctx).clone(),
                                        completion_attestation.as_ref(),
                                        envelope_raw.as_deref(),
                                    );
                                // Phase 2B: Force strict Socratic Lockout if developer is actively fatigued
                                if crate::sync_lock::rw_read(&*self.budget_manager).is_fatigued() {
                                    augmented.fatigue_active = true;
                                }
                                let outcome =
                                    crate::socrates::evaluate_socrates_gate(&augmented, &policy);
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
                                    && !trust_relax_gates
                                    && outcome.decision != vox_socrates_policy::RiskDecision::Answer
                                    && task.debug_iterations < max_socrates_debug_iterations
                                {
                                    let mut t = task.clone();
                                    if let Some(ref sid) = t.session_id {
                                        let context_key =
                                            crate::socrates::session_context_envelope_key(sid);
                                        let store = crate::sync_lock::rw_read(&*self.context_store);
                                        let context_raw = store.get(&context_key);
                                        drop(store);
                                        let parsed = context_raw
                                            .as_ref()
                                            .and_then(|raw| {
                                                serde_json::from_str::<crate::ContextEnvelope>(raw)
                                                    .ok()
                                                    .and_then(|env| {
                                                        crate::socrates::SessionRetrievalEnvelope::from_context_envelope(&env)
                                                    })
                                            });
                                        if let Some(env) = parsed {
                                            t.socrates = Some(env.merge_into(t.socrates.clone()));
                                        }
                                    }
                                    t.debug_iterations += 1;
                                    let next_action = t
                                        .socrates
                                        .as_ref()
                                        .and_then(|ctx| ctx.recommended_next_action.as_deref())
                                        .unwrap_or("gather_more_grounding");
                                    t.description.push_str(&format!(
                                        "\n\n[SOCRATES GATE]\nRisk decision {:?} (confidence {:.2}, contradiction {:.2}). Improve grounding (citations, evidence) or resolve contradictions before completing. Suggested next action: {}.\n",
                                        outcome.decision,
                                        outcome.confidence,
                                        outcome.contradiction_ratio,
                                        next_action,
                                    ));
                                    t.status = TaskStatus::Queued;
                                    socrates_requeue = Some(t);
                                }
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
                    None
                } else {
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
                                    campaign_id: t.campaign_id.clone(),
                                    benchmark_tier: t.benchmark_tier,
                                    execution_role: t.execution_role,
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
                    let debug_iterations =
                        current.as_ref().map(|t| t.debug_iterations).unwrap_or(0);
                    let campaign_id = current.as_ref().and_then(|t| t.campaign_id.clone());
                    let benchmark_tier = current.as_ref().and_then(|t| t.benchmark_tier);
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
                    Some((
                        write_files,
                        session_id,
                        desc,
                        phase_label,
                        debug_iterations,
                        plan_completion_meta,
                        campaign_id,
                        benchmark_tier,
                    ))
                }
            }
        };

        let (
            write_files,
            session_id,
            desc,
            phase_label,
            debug_iterations,
            plan_completion_meta,
            campaign_id,
            benchmark_tier,
        ) = match completion_data {
            Some(tuple) => tuple,
            None => {
                #[cfg(feature = "toestub-gate")]
                if let Some((db, repo, tid, aid, sid, ps, pn, payload)) = toestub_lineage_deferred {
                    if let Err(e) = db
                        .append_orchestration_lineage_event(
                            repo.as_str(),
                            "toestub_requeued",
                            tid,
                            aid,
                            sid.as_deref(),
                            None,
                            ps.as_deref(),
                            pn.as_deref(),
                            Some(payload.as_str()),
                        )
                        .await
                    {
                        self.record_persistence_degradation_with_meta(
                            "lineage/toestub_requeued",
                            &e.to_string(),
                            Some(serde_json::json!({
                                "op": "append_orchestration_lineage_event",
                                "repository_id": repo,
                                "kind": "toestub_requeued",
                                "task_id": tid,
                                "agent_id": aid,
                                "session_id": sid,
                                "workflow_id": serde_json::Value::Null,
                                "plan_session_id": ps,
                                "plan_node_id": pn,
                                "payload_json": payload,
                            })),
                        );
                    }
                }
                return Ok(());
            }
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
            let reliability_agent_id = agent_id.0.to_string();
            let reliability_replay = serde_json::json!({
                "op": "record_task_reliability_observation",
                "agent_id": reliability_agent_id,
                "success": true,
            });
            self.persist_with_retry_meta(
                "trust/reliability_observation",
                Some(reliability_replay),
                || async {
                    db.record_task_reliability_observation(&agent_id.0.to_string(), true)
                        .await
                },
            )
            .await;
            let agent_id_s = agent_id.0.to_string();
            let repo = crate::lineage::repository_id();
            let artifact_ref = task_id.0.to_string();
            let trust_replay = serde_json::json!({
                "op": "record_trust_observation",
                "entity_type": "agent",
                "entity_id": agent_id_s.clone(),
                "dimension": "task_completion",
                "domain": phase_label.clone(),
                "task_class": phase_label.clone(),
                "provider": serde_json::Value::Null,
                "model_id": serde_json::Value::Null,
                "repository_id": repo.clone(),
                "source_kind": "task_completed",
                "observation_value": 1.0,
                "confidence_weight": 1.0,
                "sample_size": 1,
                "artifact_ref": artifact_ref.clone(),
                "metadata_json": serde_json::Value::Null,
                "ewma_alpha": 0.10,
            });
            self.persist_with_retry_meta("trust/observation", Some(trust_replay), || async {
                db.record_trust_observation(vox_db::TrustObservationInput {
                    entity_type: "agent",
                    entity_id: agent_id_s.as_str(),
                    dimension: "task_completion",
                    domain: Some(&phase_label),
                    task_class: Some(&phase_label),
                    provider: None,
                    model_id: None,
                    repository_id: Some(repo.as_str()),
                    source_kind: Some("task_completed"),
                    observation_value: 1.0,
                    confidence_weight: 1.0,
                    sample_size: 1,
                    artifact_ref: Some(artifact_ref.as_str()),
                    metadata_json: None,
                    ewma_alpha: 0.10,
                })
                .await
                .map(|_| ())
            })
            .await;
            if let Some(meta) = plan_completion_meta.as_ref() {
                let plan_attempt_replay = serde_json::json!({
                    "op": "record_plan_node_attempt",
                    "plan_session_id": meta.plan_session_id.clone(),
                    "version": i64::from(meta.plan_version),
                    "node_id": meta.plan_node_id.clone(),
                    "attempt_no": 1i64,
                    "task_id": task_id.0.to_string(),
                    "outcome": "completed",
                    "error_text": serde_json::Value::Null,
                    "latency_ms": serde_json::Value::Null,
                });
                self.persist_with_retry_meta(
                    "plan/node_attempt",
                    Some(plan_attempt_replay),
                    || async {
                        db.record_plan_node_attempt(
                            &meta.plan_session_id,
                            i64::from(meta.plan_version),
                            &meta.plan_node_id,
                            1,
                            Some(&task_id.0.to_string()),
                            "completed",
                            None,
                            None,
                        )
                        .await
                    },
                )
                .await;
                let plan_status_replay = serde_json::json!({
                    "op": "set_plan_node_status",
                    "plan_session_id": meta.plan_session_id.clone(),
                    "version": meta.plan_version as i64,
                    "node_id": meta.plan_node_id.clone(),
                    "status": "completed",
                });
                self.persist_with_retry_meta(
                    "plan/node_status",
                    Some(plan_status_replay),
                    || async {
                        db.set_plan_node_status(
                            &meta.plan_session_id,
                            meta.plan_version as i64,
                            &meta.plan_node_id,
                            "completed",
                        )
                        .await
                    },
                )
                .await;
                if let Err(e) = crate::planning::schedule::enqueue_runnable_plan_nodes(
                    self,
                    &meta.plan_session_id,
                    meta.plan_version,
                    session_id.clone(),
                )
                .await
                {
                    self.record_persistence_degradation("plan/enqueue_runnable", &e.to_string());
                }
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
                let verification_layers = crate::reconstruction::VerificationLayerStatus {
                    structural_ok: phase_label.contains("verify") || phase_label.contains("build"),
                    behavioral_ok: phase_label.contains("test") || phase_label.contains("verify"),
                    contract_ok: phase_label.contains("contract"),
                    docs_ssot_ok: phase_label.contains("doc") || phase_label.contains("ssot"),
                    grounding_ok: debug_iterations == 0,
                };
                let mut payload = serde_json::json!({
                    "phase": phase_label,
                    "verification_layers_source": "heuristic_v1",
                    "verification_layers": verification_layers,
                });
                if let Some(ref cid) = campaign_id {
                    payload["campaign_id"] = serde_json::Value::String(cid.clone());
                }
                if let Some(tier) = benchmark_tier {
                    payload["benchmark_tier"] =
                        serde_json::Value::String(tier.as_str().to_string());
                }
                let payload_str = payload.to_string();
                let replay_meta = serde_json::json!({
                    "op": "append_orchestration_lineage_event",
                    "repository_id": repo,
                    "kind": "task_completed",
                    "task_id": task_id.0 as i64,
                    "agent_id": agent_id.0 as i64,
                    "session_id": session_id.clone(),
                    "workflow_id": serde_json::Value::Null,
                    "plan_session_id": ps,
                    "plan_node_id": pn,
                    "payload_json": payload_str,
                });
                self.persist_with_retry_meta(
                    "lineage/task_completed",
                    Some(replay_meta),
                    || async {
                        db.append_orchestration_lineage_event(
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
                        .await
                    },
                )
                .await;
            }
        }
        if let (Some(campaign_id), Some(tier)) = (campaign_id.clone(), benchmark_tier) {
            let desc_lower = desc.to_ascii_lowercase();
            let evidence = crate::reconstruction::ReconstructionEvidence {
                compile_ok: phase_label.contains("build") || desc_lower.contains("cargo check"),
                targeted_tests_ok: phase_label.contains("test")
                    || phase_label.contains("verify")
                    || desc_lower.contains("test"),
                contract_checks_ok: phase_label.contains("contract")
                    || desc_lower.contains("contract"),
                docs_ssot_ok: desc_lower.contains("ssot") || phase_label.contains("doc"),
                regression_checks_ok: phase_label.contains("regression"),
                failures: Vec::new(),
            };
            self.record_reconstruction_campaign_result(
                campaign_id,
                tier,
                evidence,
                "heuristic_task_complete",
                false,
                session_id.as_deref(),
                Some(task_id),
                Some(agent_id),
            )
            .await;
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

        let (
            session_id,
            planning_meta,
            failed_desc,
            campaign_id,
            benchmark_tier,
            failed_write_paths,
        ) = {
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
                        campaign_id: t.campaign_id.clone(),
                        benchmark_tier: t.benchmark_tier,
                        execution_role: t.execution_role,
                    })
                } else {
                    None
                }
            });
            let failed_desc = queue
                .current_task()
                .map(|t| t.description.clone())
                .unwrap_or_default();
            let failed_write_paths = queue
                .current_task()
                .map(|t| {
                    t.write_files()
                        .into_iter()
                        .cloned()
                        .collect::<Vec<std::path::PathBuf>>()
                })
                .unwrap_or_default();
            let campaign_id = queue.current_task().and_then(|t| t.campaign_id.clone());
            let benchmark_tier = queue.current_task().and_then(|t| t.benchmark_tier);
            if failed_desc.contains("[PHASE:SHARD_VALIDATE]") {
                queue.recent_shard_validation_failures =
                    queue.recent_shard_validation_failures.saturating_add(1);
            }
            queue.mark_failed(task_id, reason.clone());
            (
                session_id,
                planning_meta,
                failed_desc,
                campaign_id,
                benchmark_tier,
                failed_write_paths,
            )
        };

        if let Some(db) = self.db() {
            let reliability_agent_id = agent_id.0.to_string();
            let reliability_replay = serde_json::json!({
                "op": "record_task_reliability_observation",
                "agent_id": reliability_agent_id,
                "success": false,
            });
            self.persist_with_retry_meta(
                "trust/reliability_observation",
                Some(reliability_replay),
                || async {
                    db.record_task_reliability_observation(&agent_id.0.to_string(), false)
                        .await
                },
            )
            .await;
            let agent_id_s = agent_id.0.to_string();
            let domain = Self::extract_phase_label(&failed_desc);
            let artifact_ref = task_id.0.to_string();
            let repo = crate::lineage::repository_id();
            let trust_replay = serde_json::json!({
                "op": "record_trust_observation",
                "entity_type": "agent",
                "entity_id": agent_id_s.clone(),
                "dimension": "task_completion",
                "domain": domain.clone(),
                "task_class": domain.clone(),
                "provider": serde_json::Value::Null,
                "model_id": serde_json::Value::Null,
                "repository_id": repo.clone(),
                "source_kind": "task_failed",
                "observation_value": 0.0,
                "confidence_weight": 1.0,
                "sample_size": 1,
                "artifact_ref": artifact_ref.clone(),
                "metadata_json": serde_json::Value::Null,
                "ewma_alpha": 0.10,
            });
            self.persist_with_retry_meta("trust/observation", Some(trust_replay), || async {
                db.record_trust_observation(vox_db::TrustObservationInput {
                    entity_type: "agent",
                    entity_id: &agent_id_s,
                    dimension: "task_completion",
                    domain: Some(domain.as_str()),
                    task_class: Some(domain.as_str()),
                    provider: None,
                    model_id: None,
                    repository_id: Some(repo.as_str()),
                    source_kind: Some("task_failed"),
                    observation_value: 0.0,
                    confidence_weight: 1.0,
                    sample_size: 1,
                    artifact_ref: Some(artifact_ref.as_str()),
                    metadata_json: None,
                    ewma_alpha: 0.10,
                })
                .await
                .map(|_| ())
            })
            .await;
        }
        if let (Some(campaign_id), Some(tier)) = (campaign_id.clone(), benchmark_tier) {
            let reason_lower = reason.to_ascii_lowercase();
            let failures = Self::classify_verification_failures(&reason_lower);
            let evidence = crate::reconstruction::ReconstructionEvidence {
                compile_ok: !reason_lower.contains("compile"),
                targeted_tests_ok: !reason_lower.contains("test"),
                contract_checks_ok: !reason_lower.contains("contract"),
                docs_ssot_ok: !reason_lower.contains("doc"),
                regression_checks_ok: false,
                failures: failures.clone(),
            };
            self.record_reconstruction_campaign_result(
                campaign_id.clone(),
                tier,
                evidence,
                "heuristic_task_fail",
                false,
                session_id.as_deref(),
                Some(task_id),
                Some(agent_id),
            )
            .await;

            if let Some(primary_failure) = failures.first().copied() {
                let key = format!(
                    "{}repair_recommendation:{}",
                    crate::reconstruction::campaign_context_prefix(campaign_id.as_str()),
                    task_id.0
                );
                let payload = serde_json::json!({
                    "campaign_id": campaign_id,
                    "task_id": task_id.0,
                    "failure_kind": Self::failure_kind_tag(primary_failure),
                    "reason": reason,
                    "write_paths": failed_write_paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>(),
                });
                crate::sync_lock::rw_read(&*self.context_store).set(
                    AgentId(0),
                    key,
                    payload.to_string(),
                    0,
                );
            }
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

        // Release task-specific ownership first, then any residual locks.
        for path in &failed_write_paths {
            self.lock_manager.release(path, agent_id);
            self.affinity_map.release(path);
            crate::sync_lock::rw_write(&*self.scope_guard).revoke_file(agent_id, path);
        }
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
                if let Err(e) = db
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
                    .await
                {
                    self.record_persistence_degradation("plan/replan_node_attempt", &e.to_string());
                }
                let next_version = i64::from(meta.plan_version) + 1;
                if let Err(e) = db
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
                    .await
                {
                    self.record_persistence_degradation_with_meta(
                        "plan/replan_version",
                        &e.to_string(),
                        Some(serde_json::json!({
                            "op": "append_plan_version",
                            "plan_session_id": meta.plan_session_id.clone(),
                            "version": next_version,
                            "parent_version": i64::from(meta.plan_version),
                            "origin": "task_failed",
                            "metadata_json": serde_json::json!({
                                "task_id": task_id.0,
                                "reason": reason,
                            }).to_string(),
                        })),
                    );
                }
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
                if crate::lineage::orchestration_lineage_persist_enabled() {
                    let repo = crate::lineage::repository_id();
                    let mut payload = serde_json::json!({
                        "failed_task_id": task_id.0,
                        "next_version": next_version,
                        "reason_preview": reason.chars().take(500).collect::<String>(),
                    });
                    if let Some(cid) = crate::lineage::orchestration_campaign_id() {
                        payload["campaign_id"] = serde_json::Value::String(cid);
                    }
                    let payload_str = payload.to_string();
                    let replay_meta = serde_json::json!({
                        "op": "append_orchestration_lineage_event",
                        "repository_id": repo,
                        "kind": "plan_replan_triggered",
                        "task_id": task_id.0 as i64,
                        "agent_id": agent_id.0 as i64,
                        "session_id": session_id.clone(),
                        "workflow_id": serde_json::Value::Null,
                        "plan_session_id": meta.plan_session_id.clone(),
                        "plan_node_id": meta.plan_node_id.clone(),
                        "payload_json": payload_str,
                    });
                    self.persist_with_retry_meta(
                        "lineage/plan_replan_triggered",
                        Some(replay_meta),
                        || async {
                            db.append_orchestration_lineage_event(
                                &repo,
                                "plan_replan_triggered",
                                task_id.0 as i64,
                                Some(agent_id.0 as i64),
                                session_id.as_deref(),
                                None,
                                Some(meta.plan_session_id.as_str()),
                                Some(meta.plan_node_id.as_str()),
                                Some(payload_str.as_str()),
                            )
                            .await
                        },
                    )
                    .await;
                }
            }
            if let Err(e) = crate::planning::replan::enqueue_recovery_first_node(
                self,
                meta,
                &reason,
                &failed_desc,
                session_id.clone(),
            )
            .await
            {
                self.record_persistence_degradation("plan/replan_recovery_enqueue", &e.to_string());
            }
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
                let replay_meta = serde_json::json!({
                    "op": "append_orchestration_lineage_event",
                    "repository_id": repo,
                    "kind": "task_failed",
                    "task_id": task_id.0 as i64,
                    "agent_id": agent_id.0 as i64,
                    "session_id": session_id.clone(),
                    "workflow_id": serde_json::Value::Null,
                    "plan_session_id": ps,
                    "plan_node_id": pn,
                    "payload_json": payload_str,
                });
                self.persist_with_retry_meta("lineage/task_failed", Some(replay_meta), || async {
                    db.append_orchestration_lineage_event(
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
                    .await
                })
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

    fn classify_verification_failures(
        reason_lower: &str,
    ) -> Vec<crate::reconstruction::VerificationFailureKind> {
        let mut kinds = Vec::new();
        if reason_lower.contains("compile") || reason_lower.contains("cargo check") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Compile);
        }
        if reason_lower.contains("test") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Tests);
        }
        if reason_lower.contains("contract") || reason_lower.contains("schema") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Contract);
        }
        if reason_lower.contains("doc") || reason_lower.contains("ssot") {
            kinds.push(crate::reconstruction::VerificationFailureKind::DocsSsot);
        }
        if reason_lower.contains("regression") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Regression);
        }
        if reason_lower.contains("grounding") || reason_lower.contains("citation") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Grounding);
        }
        if reason_lower.contains("contradiction") {
            kinds.push(crate::reconstruction::VerificationFailureKind::Contradiction);
        }
        if kinds.is_empty() {
            kinds.push(crate::reconstruction::VerificationFailureKind::Unknown);
        }
        kinds
    }

    fn failure_kind_tag(kind: crate::reconstruction::VerificationFailureKind) -> &'static str {
        match kind {
            crate::reconstruction::VerificationFailureKind::Compile => "compile",
            crate::reconstruction::VerificationFailureKind::Tests => "tests",
            crate::reconstruction::VerificationFailureKind::Contract => "contract",
            crate::reconstruction::VerificationFailureKind::DocsSsot => "docs_ssot",
            crate::reconstruction::VerificationFailureKind::Regression => "regression",
            crate::reconstruction::VerificationFailureKind::Grounding => "grounding",
            crate::reconstruction::VerificationFailureKind::Contradiction => "contradiction",
            crate::reconstruction::VerificationFailureKind::Unknown => "unknown",
        }
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

    fn record_persistence_degradation(&self, lane: &str, error: &str) {
        self.record_persistence_degradation_with_meta(lane, error, None);
    }

    fn record_persistence_degradation_with_meta(
        &self,
        lane: &str,
        error: &str,
        replay_meta: Option<serde_json::Value>,
    ) {
        let key = format!("orchestrator/persistence_health/{lane}");
        let store = crate::sync_lock::rw_read(&*self.context_store);
        let previous = store
            .get(&key)
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok());
        let count = previous
            .as_ref()
            .and_then(|v| v.get("degraded_count"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        let payload = serde_json::json!({
            "lane": lane,
            "status": "degraded",
            "degraded_count": count,
            "last_error": error.chars().take(600).collect::<String>(),
            "last_error_unix_ms": crate::types::now_unix_ms(),
        });
        if let Ok(raw) = serde_json::to_string(&payload) {
            store.set(AgentId(0), key, raw, 0);
        }
        self.enqueue_persistence_outbox(lane, error, replay_meta);
        tracing::warn!(lane = lane, error = %error, "persistence lane degraded");
    }

    fn enqueue_persistence_outbox(
        &self,
        lane: &str,
        error: &str,
        replay_meta: Option<serde_json::Value>,
    ) {
        let key = PERSISTENCE_OUTBOX_KEY.to_string();
        let store = crate::sync_lock::rw_read(&*self.context_store);
        let mut queue = store
            .get(&key)
            .and_then(|raw| serde_json::from_str::<Vec<serde_json::Value>>(&raw).ok())
            .unwrap_or_default();
        let mut entry = serde_json::json!({
            "lane": lane,
            "error": error.chars().take(600).collect::<String>(),
            "first_seen_unix_ms": crate::types::now_unix_ms(),
            "retry_count": 0u64,
        });
        if let Some(meta) = replay_meta
            && let Some(obj) = entry.as_object_mut()
        {
            obj.insert("replay".to_string(), meta);
        }
        queue.push(entry);
        const MAX_OUTBOX_ITEMS: usize = 200;
        if queue.len() > MAX_OUTBOX_ITEMS {
            let drop_n = queue.len().saturating_sub(MAX_OUTBOX_ITEMS);
            queue.drain(0..drop_n);
        }
        if let Ok(raw) = serde_json::to_string(&queue) {
            store.set(AgentId(0), key, raw, 0);
        }
    }

    async fn persist_with_retry_meta<E, F, Fut>(
        &self,
        lane: &str,
        replay_meta: Option<serde_json::Value>,
        mut op: F,
    ) where
        E: std::fmt::Display,
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<(), E>>,
    {
        const MAX_ATTEMPTS: usize = 3;
        for attempt in 1..=MAX_ATTEMPTS {
            match op().await {
                Ok(()) => {
                    self.ack_persistence_lane_recovery(lane);
                    return;
                }
                Err(e) if attempt < MAX_ATTEMPTS => {
                    let delay_ms = 25u64 * attempt as u64;
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    tracing::debug!(lane = lane, attempt = attempt, error = %e, "persistence retry");
                }
                Err(e) => {
                    self.record_persistence_degradation_with_meta(
                        lane,
                        &e.to_string(),
                        replay_meta.clone(),
                    );
                }
            }
        }
    }

    fn ack_persistence_lane_recovery(&self, lane: &str) {
        let store = crate::sync_lock::rw_read(&*self.context_store);
        if crate::orchestrator::persistence_outbox::ack_persistence_outbox_lane(&store, lane) {
            tracing::debug!(
                lane = lane,
                "persistence lane recovered; acknowledged outbox item"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::harness_completion_issues;
    use crate::types::{AgentTask, CompletionAttestation, FileAffinity, TaskId, TaskPriority};

    fn task_with_harness(harness_json: &str) -> AgentTask {
        let mut task = AgentTask::new(
            TaskId(1),
            "harness completion test",
            TaskPriority::Normal,
            vec![FileAffinity::write("src/lib.rs")],
        );
        task.harness_spec_json = Some(harness_json.to_string());
        task
    }

    #[test]
    fn harness_completion_issues_empty_when_not_required() {
        let harness = crate::AgentHarnessSpec::minimal_contract_first(
            "repo-test",
            "no required outputs",
            None,
            None,
            &[],
        );
        let mut harness = harness;
        harness.contracts.requires_artifact_backed_completion = false;
        harness.contracts.required_outputs.clear();
        harness.contracts.completion_gates.clear();
        let raw = serde_json::to_string(&harness).expect("serialize harness");
        let task = task_with_harness(&raw);
        let (_hid, issues) = harness_completion_issues(&task, None).expect("evaluate");
        assert!(issues.is_empty(), "{issues:?}");
    }

    #[test]
    fn harness_completion_issues_reports_missing_required_artifact() {
        let harness = crate::AgentHarnessSpec::minimal_contract_first(
            "repo-test",
            "artifact required",
            None,
            None,
            &["artifacts/out.md".to_string()],
        );
        let raw = serde_json::to_string(&harness).expect("serialize harness");
        let task = task_with_harness(&raw);
        let att = CompletionAttestation::default();
        let (_hid, issues) = harness_completion_issues(&task, Some(&att)).expect("evaluate");
        assert!(
            issues.iter().any(|x| x.contains("missing required harness artifact")),
            "{issues:?}"
        );
    }
}
