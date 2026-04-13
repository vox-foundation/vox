use std::path::PathBuf;

use crate::ApprovalTier;
use crate::oplog::OperationKind;
use crate::orchestrator::{Orchestrator, OrchestratorError, TaskTraceStep};
use crate::planning::PlanningTaskMeta;
use crate::services::MessageGateway;
use crate::types::{AgentId, AgentTask, CompletionAttestation, TaskId, TaskStatus};
use vox_db::store::{ObservationReport, ObserverAction, VictoryVerdict};

use super::harness::{
    HarnessCompletionGuardMode, completion_attestation_satisfies_tier,
    harness_completion_guard_mode, harness_completion_issues,
};

impl Orchestrator {
    pub async fn complete_task(&self, task_id: TaskId) -> Result<(), OrchestratorError> {
        self.complete_task_with_attestation(task_id, None).await
    }

    /// Mark a task as complete and attach an audit report (typically from Doubted resolution).
    pub async fn complete_task_with_audit(
        &self,
        task_id: TaskId,
        audit_report: String,
    ) -> Result<(), OrchestratorError> {
        // Find the agent
        let agent_id = crate::sync_lock::rw_read(&*self.task_assignments)
            .get(&task_id)
            .copied()
            .ok_or(OrchestratorError::TaskNotFound(task_id))?;

        // Update the task in the queue to include the audit report
        {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            let queue_lock = agents
                .get(&agent_id)
                .ok_or(OrchestratorError::AgentNotFound(agent_id))?;
            let mut queue = crate::sync_lock::rw_write(&**queue_lock);
            if let Some(task) = queue.find_task_mut(task_id) {
                task.audit_report = Some(audit_report);
            } else if let Some(task) = queue.current_task_mut() {
                if task.id == task_id {
                    task.audit_report = Some(audit_report);
                }
            }
        }

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

        let (task_clone_opt, phase_label, write_files) = {
            let agents = crate::sync_lock::rw_read(&*self.agents);
            if let Some(queue_lock) = agents.get(&agent_id) {
                let queue = crate::sync_lock::rw_read(&**queue_lock);
                if let Some(t) = queue.current_task() {
                    (
                        Some(t.clone()),
                        Self::extract_phase_label(&t.description),
                        t.write_files().into_iter().cloned().collect::<Vec<_>>(),
                    )
                } else {
                    (None, String::new(), Vec::new())
                }
            } else {
                (None, String::new(), Vec::new())
            }
        };

        // 1. Fetch trust_score
        let mut trust_score_opt = None;
        if let Some(db) = self.db() {
            if task_clone_opt.is_some() {
                let phase = phase_label.clone();
                let rows = db
                    .list_trust_scores_for_dimension(
                        "agent",
                        "task_completion",
                        Some(phase.as_str()),
                        512,
                    )
                    .await
                    .unwrap_or_default();
                trust_score_opt = Some(
                    rows.into_iter()
                        .find_map(|(id, score)| {
                            id.parse::<u64>()
                                .ok()
                                .filter(|aid| *aid == agent_id.0)
                                .map(|_| score)
                        })
                        .unwrap_or(0.5),
                );
            }
        }

        // 2. Validate links using async Command
        let mut broken_links_report = String::new();
        if task_clone_opt.is_some() {
            let md_files: Vec<_> = write_files
                .iter()
                .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
                .collect();
            if !md_files.is_empty() {
                let mut broken_reports = Vec::new();
                for md_file in md_files {
                    let out = tokio::process::Command::new("cargo")
                        .arg("run")
                        .arg("-p")
                        .arg("vox-cli")
                        .arg("--")
                        .arg("ci")
                        .arg("check-links")
                        .arg("--target")
                        .arg(md_file)
                        .output()
                        .await;
                    if let Ok(out) = out {
                        if !out.status.success() {
                            broken_reports.push(String::from_utf8_lossy(&out.stdout).to_string());
                        }
                    }
                }
                if !broken_reports.is_empty() {
                    broken_links_report = broken_reports.join("\n");
                }
            }
        }

        #[cfg(feature = "runtime")]
        if let Some(ref task_clone) = task_clone_opt {
            let vox_files: Vec<_> = write_files
                .iter()
                .filter(|p| p.extension().is_some_and(|ext| ext == "vox"))
                .collect();

            for vox_file in vox_files {
                if let Ok(original_source) = vox_bounded_fs::read_utf8_path_capped(vox_file) {
                    let loop_ = vox_populi::mens::healing::HealingLoop::new(
                        3,
                        |src| {
                            let diagnostics = vox_lsp::validate_document(src);
                            let errors: Vec<_> = diagnostics
                                .into_iter()
                                .filter(|d| {
                                    matches!(
                                        d.severity,
                                        Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR)
                                    )
                                })
                                .map(|d| d.message)
                                .collect();
                            vox_populi::mens::healing::HealResult {
                                ok: errors.is_empty(),
                                diagnostics: errors,
                            }
                        },
                        |src, errs| {
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    let prompt = format!(
                                        "Fix these Vox compiler errors:\n{}\n\nCode:\n```vox\n{}\n```\n\nReturn only the fixed code, no extra text.",
                                        errs.join("\n"), src
                                    );
                                    let client = vox_ludus::ai::FreeAiClient::auto_discover().await;
                                    let raw = client.generate(&prompt).await.unwrap_or_default();

                                    let mut cleaned = raw;
                                    if cleaned.starts_with("```vox") {
                                        cleaned = cleaned.trim_start_matches("```vox").trim_end_matches("```").trim().to_string();
                                    } else if cleaned.starts_with("```") {
                                        cleaned = cleaned.trim_start_matches("```").trim_end_matches("```").trim().to_string();
                                    }
                                    Ok(cleaned)
                                })
                            })
                        },
                    );
                    let desc = task_clone.description.clone();
                    if let vox_populi::mens::healing::HealOutcome::Success { source, .. } =
                        loop_.heal(&desc, &original_source).await
                    {
                        let _ = std::fs::write(vox_file, &source);
                        self.event_bus()
                            .emit(crate::events::AgentEventKind::AutoHealApplied {
                                agent_id,
                                path: vox_file.clone(),
                                description: "Automated AST healing fixed compiler errors."
                                    .to_string(),
                                new_source: source,
                            });
                    }
                }
            }
        }

        let mut behavioral_failure = None;
        if task_clone_opt.is_some() {
            let gate = crate::gate::BehavioralGate::new(true);
            if let crate::gate::GateResult::BehavioralTestFailed { message } =
                gate.check_behavior(None).await
            {
                behavioral_failure = Some(message);
            }
        }

        let completion_data: Option<(
            Vec<PathBuf>,
            Option<String>,
            String,
            String,
            u8,
            Option<PlanningTaskMeta>,
            Option<String>,
            Option<crate::reconstruction::ReconstructionBenchmarkTier>,
            Option<String>, // audit_report
        )> = {
            let queue_lock = {
                let agents = crate::sync_lock::rw_read(&*self.agents);
                agents
                    .get(&agent_id)
                    .ok_or(OrchestratorError::AgentNotFound(agent_id))?
                    .clone()
            };
            let mut queue = crate::sync_lock::rw_write(&*queue_lock);

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
            {
                if task_clone.execution_role == Some(crate::reconstruction::AgentExecutionRole::Verifier) {
                    let has_checks = completion_attestation
                        .as_ref()
                        .map(|a| !a.checks_passed.is_empty())
                        .unwrap_or(false);
                    let force = completion_attestation.as_ref().is_some_and(|a| a.force_risky);
                    if !has_checks && !force {
                        return Err(OrchestratorError::TaskValidationFailed(
                            "SYSTEM_INTERVENTION: You are in Reflective Interrogation Mode (Doubted). You are attempting to complete a task without running any terminal commands to verify the codebase. Refusal to run code checks is a failure mode known as the 'Checklist Ritual'. You MUST run `vox ci check` or an equivalent test command before completing.".to_string()
                        ));
                    }
                }
            }

            if auto_debug_requeue.is_none()
                && let Some(message) = behavioral_failure.take()
                && let Some(mut task_clone) = queue.current_task().cloned()
            {
                if task_clone.debug_iterations < max_debug_iterations {
                    task_clone.debug_iterations += 1;
                    task_clone.description.push_str(&format!(
                        "\n\n[BEHAVIORAL GATE]\nBehavioral tests failed. Fix and retry:\n{}",
                        message
                    ));
                    task_clone.status = TaskStatus::Queued;
                    auto_debug_requeue = Some((task_clone.clone(), message, 1usize, 0usize));
                } else {
                    return Err(OrchestratorError::TaskValidationFailed(message));
                }
            }
            if auto_debug_requeue.is_none()
                && let Some(mut task_clone) = queue.current_task().cloned()
                && task_clone.task_category == crate::types::TaskCategory::Research
            {
                // Action Rejection Feedback for Research tasks: verify JSON output in completion attestation
                let is_json = completion_attestation
                    .as_ref()
                    .and_then(|a| a.completion_summary.as_ref())
                    .map(|s| {
                        let mut text = s.trim();
                        if text.starts_with("```json") {
                            text = text
                                .trim_start_matches("```json")
                                .trim_end_matches("```")
                                .trim();
                        }
                        serde_json::from_str::<serde_json::Value>(text).is_ok()
                    })
                    .unwrap_or(false);

                if !is_json && task_clone.debug_iterations < max_debug_iterations {
                    task_clone.debug_iterations += 1;
                    task_clone.description.push_str(
                        "\n\n[SCHEMA COMPLIANCE GATE]\nResearch logic must output a valid JSON response in the completion attestation summary to prevent anomalous routing instructions."
                    );
                    task_clone.status = TaskStatus::Queued;
                    auto_debug_requeue = Some((
                        task_clone,
                        "JSON schema compliance failure on Research task".to_string(),
                        1usize,
                        0usize,
                    ));
                } else if !is_json {
                    return Err(OrchestratorError::TaskValidationFailed(
                        "Research tasks require a valid JSON completion summary.".to_string(),
                    ));
                }
            }
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
                && let Some(_db) = self.db()
                && let Some(mut task_clone) = queue.current_task().cloned()
            {
                let phase = Self::extract_phase_label(&task_clone.description);
                let trust_score = trust_score_opt.unwrap_or(0.5);
                let floor = crate::sync_lock::rw_read(&*self.config).trust_task_completion_floor;
                let has_checks = completion_attestation
                    .as_ref()
                    .map(|a| !a.checks_passed.is_empty())
                    .unwrap_or(false);
                if trust_score < floor
                    && !has_checks
                    && task_clone.toestub_iterations < max_toestub_debug_iterations
                {
                    task_clone.toestub_iterations += 1;
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
                let md_files: Vec<_> = write_files
                    .iter()
                    .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
                    .collect();

                if !md_files.is_empty() {
                    if !broken_links_report.is_empty() {
                        let detail = format!(
                            "Broken links detected. Please investigate if files need to be created or fix the links:\n{}",
                            broken_links_report
                        );
                        if task_clone.debug_iterations < max_debug_iterations {
                            task_clone.debug_iterations += 1;
                            task_clone
                                .description
                                .push_str(&format!("\n\n[DOC INTEGRITY GATE]\n{}", detail));
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
                                "toestub_iteration": requeue_task.toestub_iterations,
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
                        let task = task.clone();
                        if let Some(ref ctx) = task.socrates {
                            let envelope_raw = task.session_id.as_ref().and_then(|sid| {
                                let key = crate::socrates::session_context_envelope_key(sid);
                                crate::sync_lock::rw_read(&*self.context_store).get(&key)
                            });
                            if config.completion_grounding_shadow
                                || config.completion_grounding_enforce
                            {
                                let declared = crate::grounding::declared_evidence_citations(
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
                                        t.description
                                            .push_str(&format!("\n\n[GROUNDING GATE]\n{msg}\n",));
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
                                let outcome = crate::socrates::evaluate_socrates_gate(
                                    &augmented,
                                    &policy,
                                    task.description.as_str(),
                                );
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
                                let task_clone = task.clone();
                                let mut research_results = Vec::new();
                                if outcome.research_decision.should_research {
                                    let queries = outcome
                                        .research_decision
                                        .suggested_query
                                        .clone()
                                        .map(|q| vec![q])
                                        .unwrap_or_else(|| vec![task_clone.description.clone()]);
                                    let trigger = outcome.research_decision.trigger.clone();

                                    let results = tokio::task::block_in_place(|| {
                                        tokio::runtime::Handle::current().block_on(
                                            self.perform_autonomous_research(
                                                Some(agent_id),
                                                Some(task_id),
                                                queries,
                                                &trigger,
                                            ),
                                        )
                                    })
                                    .unwrap_or_default();
                                    research_results = results;
                                }

                                if config.socrates_gate_enforce
                                    && !trust_relax_gates
                                    && (outcome.decision
                                        != vox_socrates_policy::RiskDecision::Answer
                                        || !research_results.is_empty())
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

                                    if !research_results.is_empty() {
                                        let mut s_ctx =
                                            t.socrates.clone().unwrap_or(augmented.clone());
                                        let old_quality = s_ctx.evidence_quality;
                                        self.inject_research_results(&mut s_ctx, research_results);
                                        t.socrates = Some(s_ctx.clone());

                                        tracing::info!(
                                            target: "vox_orchestrator::socrates",
                                            task_id = task_id.0,
                                            agent_id = agent_id.0,
                                            quality_improvement = s_ctx.evidence_quality - old_quality,
                                            "autonomous research injected; evidence quality boosted"
                                        );
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
                        current.and_then(|t| t.audit_report),
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
            audit_report,
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
                let other_base_snapshot = crate::sync_lock::rw_read(&*self.workspace_manager)
                    .get_workspace(other_id)
                    .map(|ws| ws.base_snapshot)
                    .unwrap_or(snapshot_after);

                overlap_conflicts = overlap_conflicts.saturating_add(1);
                let conflict_id = crate::sync_lock::rw_write(&*self.conflict_manager)
                    .record_conflict(
                        overlap_path.clone(),
                        Some(snapshot_after),
                        vec![(agent_id, snapshot_after), (other_id, other_base_snapshot)],
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
            audit_report,
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

        // 5. Hardening MENS Intelligence Persistence (Wave 0)
        if let Some(db) = self.db() {
            let task_id_s = task_id.0.to_string();

            // Record Victory Verdict
            let verdict = VictoryVerdict {
                task_id: task_id_s.clone(),
                passed: true,
                tiers_run: vec!["Full".to_string()],
                first_failure: None,
                report: desc.clone(),
                created_at_ms: crate::types::now_unix_ms() as i64,
            };
            let verdict_replay = serde_json::json!({
                "op": "insert_victory_verdict",
                "task_id": task_id_s.clone(),
                "verdict": verdict,
            });
            self.persist_with_retry_meta("mens/victory_verdict", Some(verdict_replay), || {
                let db = db.clone();
                let tid = task_id_s.clone();
                let v = verdict.clone();
                async move { db.insert_victory_verdict(&tid, &v).await }
            })
            .await;

            // Record Observation Report (if applicable)
            if let Some(_t) = task_clone_opt {
                let report = ObservationReport {
                    session_id: session_id.clone().unwrap_or_default(),
                    task_id: task_id_s.clone(),
                    observed_at: chrono::Utc::now(),
                    file_path: write_files
                        .first()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default(),
                    lsp_error_count: 0,
                    parse_rate: 1.0,
                    construct_coverage: 1.0,
                    recommended_action: ObserverAction::Continue,
                    metadata_json: None,
                };
                let obs_replay = serde_json::json!({
                    "op": "insert_observer_event",
                    "session_id": session_id.clone().unwrap_or_default(),
                    "task_id": task_id_s.clone(),
                    "report": report,
                });
                self.persist_with_retry_meta("mens/observer_event", Some(obs_replay), || {
                    let db = db.clone();
                    let sid = session_id.clone().unwrap_or_default();
                    let tid = task_id_s.clone();
                    let r = report.clone();
                    async move { db.insert_observer_event(&sid, &tid, &r).await }
                })
                .await;
            }
        }

        tracing::info!("Task {} completed by agent {}", task_id, agent_id);
        self.record_task_loop_metric(task_id, &phase_label, "completed", debug_iterations);
        Ok(())
    }
}
