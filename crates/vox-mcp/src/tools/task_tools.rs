//! Task management tool handlers for the Vox MCP server.
//!
//! Covers: submit, status, complete, fail, cancel, reorder, drain, and publish.

use vox_orchestrator::{
    AgentEventKind, AgentId, FileAffinity, TaskCategory, TaskEnqueueHints, TaskId, TaskPriority,
};
use vox_repository::{load_agent_scopes, normalize_task_path};
use vox_runtime::prompt_canonical;

use crate::params::{
    CompleteTaskParams, DrainAgentParams, FailTaskParams, PublishMessageParams, ReorderTaskParams,
    SubmitTaskParams, SubmitTaskResponse, TaskStatusParams, ToolResult,
};
use crate::server::ServerState;

const REM_TASK_SCOPE: &str = "Limit `files` to paths under the agent scopes, or omit `agent_name` so routing picks a valid agent.";
const REM_QUESTIONING_PENDING: &str = "Call `vox_questioning_pending` for `question_id` / `question_options`, then `vox_questioning_submit_answer` with the same `session_id` as chat/plan (and optional `question_id` / `selected_option_id`), or continue until the open clarification is answered.";
const REM_PROMPT_SAFETY: &str =
    "Rewrite the task to remove injection patterns and disallowed content per Trust & Safety.";
const REM_TASK_SUBMIT: &str =
    "Check orchestrator health, queues, and that referenced files exist and are readable.";
const REM_TASK_ID: &str =
    "Confirm `task_id` with task/orchestrator status; it may be stale, completed, or cancelled.";
const REM_TASK_ORCH_OP: &str = "Verify task lifecycle state, file locks, and orchestrator health before complete/fail/cancel/reorder/drain.";

fn task_category_from_mcp_str(raw: &str) -> Option<TaskCategory> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "parsing" => Some(TaskCategory::Parsing),
        "type_checking" | "typechecking" => Some(TaskCategory::TypeChecking),
        "debugging" => Some(TaskCategory::Debugging),
        "research" => Some(TaskCategory::Research),
        "testing" => Some(TaskCategory::Testing),
        "codegen" | "code_gen" | "implementation" => Some(TaskCategory::CodeGen),
        "review" => Some(TaskCategory::Review),
        _ => {
            tracing::debug!(%raw, "submit_task: unknown task_category; ignoring");
            None
        }
    }
}

fn parse_campaign_from_description(
    description: &str,
) -> (
    Option<String>,
    Option<vox_orchestrator::ReconstructionBenchmarkTier>,
) {
    let mut campaign_id = None;
    let mut tier = None;
    for token in description.split_whitespace() {
        let t = token.trim_matches(|c: char| c == '[' || c == ']' || c == ',' || c == ';');
        let t_lower = t.to_ascii_lowercase();
        if t_lower.starts_with("campaign:") {
            if campaign_id.is_some() {
                tracing::debug!("submit_task: multiple campaign tags found; using first");
                continue;
            }
            let v = &t["campaign:".len()..];
            let vv = v.trim();
            if !vv.is_empty() {
                campaign_id = Some(vv.to_string());
            }
        }
        if t_lower.starts_with("tier:") {
            if tier.is_some() {
                tracing::debug!("submit_task: multiple tier tags found; using first");
                continue;
            }
            let v = &t_lower["tier:".len()..];
            tier = match v.trim() {
                "issue_repair" => Some(vox_orchestrator::ReconstructionBenchmarkTier::IssueRepair),
                "subsystem_regen" => {
                    Some(vox_orchestrator::ReconstructionBenchmarkTier::SubsystemRegen)
                }
                "crate_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen),
                "repo_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::RepoRegen),
                other => {
                    tracing::debug!(tier = %other, "submit_task: unknown reconstruction tier; ignoring");
                    None
                }
            };
        }
    }
    (campaign_id, tier)
}

fn parse_benchmark_tier(raw: &str) -> Option<vox_orchestrator::ReconstructionBenchmarkTier> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "issue_repair" => Some(vox_orchestrator::ReconstructionBenchmarkTier::IssueRepair),
        "subsystem_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::SubsystemRegen),
        "crate_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen),
        "repo_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::RepoRegen),
        _ => None,
    }
}

fn enqueue_hints_from_submit_params(params: &SubmitTaskParams) -> Option<TaskEnqueueHints> {
    let category = params
        .task_category
        .as_deref()
        .and_then(task_category_from_mcp_str);
    let execution_role = match category {
        Some(TaskCategory::Parsing) => Some(vox_orchestrator::AgentExecutionRole::Builder),
        Some(TaskCategory::TypeChecking) => Some(vox_orchestrator::AgentExecutionRole::Builder),
        Some(TaskCategory::Research) => Some(vox_orchestrator::AgentExecutionRole::Researcher),
        Some(TaskCategory::Testing) => Some(vox_orchestrator::AgentExecutionRole::Verifier),
        Some(TaskCategory::Review) => Some(vox_orchestrator::AgentExecutionRole::Verifier),
        Some(TaskCategory::Debugging) => Some(vox_orchestrator::AgentExecutionRole::Reproducer),
        Some(TaskCategory::CodeGen) => Some(vox_orchestrator::AgentExecutionRole::Builder),
        _ => None,
    };
    let (campaign_from_desc, tier_from_desc) = parse_campaign_from_description(&params.description);
    let campaign_id = params
        .campaign_id
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or(campaign_from_desc);
    let benchmark_tier = params
        .benchmark_tier
        .as_deref()
        .and_then(parse_benchmark_tier)
        .or(tier_from_desc);
    if category.is_none()
        && params.complexity.is_none()
        && params.model_preference.is_none()
        && params.model_override.is_none()
        && campaign_id.is_none()
        && benchmark_tier.is_none()
        && execution_role.is_none()
    {
        return None;
    }
    Some(TaskEnqueueHints {
        task_category: category,
        complexity: params.complexity.map(|c| c.clamp(1, 10)),
        model_preference: params.model_preference.clone(),
        model_override: params.model_override.clone(),
        campaign_id,
        benchmark_tier,
        execution_role,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_params(description: &str) -> SubmitTaskParams {
        SubmitTaskParams {
            description: description.to_string(),
            files: vec![],
            priority: None,
            agent_name: None,
            capabilities: None,
            task_category: None,
            complexity: None,
            model_preference: None,
            model_override: None,
            session_id: None,
            planning_mode: None,
            goal_type: None,
            retrieval: None,
            goal_scope: None,
            max_plan_depth: None,
            campaign_id: None,
            benchmark_tier: None,
        }
    }

    #[test]
    fn parse_campaign_from_description_extracts_campaign_and_tier_tokens() {
        let (cid, tier) =
            parse_campaign_from_description("do work [campaign:alpha1] [tier:crate_regen]");
        assert_eq!(cid.as_deref(), Some("alpha1"));
        assert_eq!(
            tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen)
        );
    }

    #[test]
    fn parse_campaign_from_description_is_case_insensitive_for_prefixes() {
        let (cid, tier) =
            parse_campaign_from_description("do work [Campaign:Alpha] [TIER:repo_regen]");
        assert_eq!(cid.as_deref(), Some("Alpha"));
        assert_eq!(
            tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::RepoRegen)
        );
    }

    #[test]
    fn enqueue_hints_from_submit_params_returns_none_when_no_signals_present() {
        let params = base_params("plain task");
        assert!(enqueue_hints_from_submit_params(&params).is_none());
    }

    #[test]
    fn enqueue_hints_from_submit_params_maps_testing_category_to_verifier_role() {
        let mut params = base_params("run tests");
        params.task_category = Some("testing".to_string());
        let hints = enqueue_hints_from_submit_params(&params).expect("hints");
        assert_eq!(
            hints.execution_role,
            Some(vox_orchestrator::AgentExecutionRole::Verifier)
        );
    }

    #[test]
    fn enqueue_hints_from_submit_params_merges_campaign_tokens_from_description() {
        let mut params = base_params("fix bug campaign:campA tier:issue_repair");
        params.complexity = Some(8);
        let hints = enqueue_hints_from_submit_params(&params).expect("hints");
        assert_eq!(hints.campaign_id.as_deref(), Some("campA"));
        assert_eq!(
            hints.benchmark_tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::IssueRepair)
        );
        assert_eq!(hints.complexity, Some(8));
    }

    #[test]
    fn enqueue_hints_prefers_structured_campaign_and_tier_over_description_tags() {
        let mut params = base_params("campaign:desc tier:issue_repair");
        params.campaign_id = Some("structured".to_string());
        params.benchmark_tier = Some("crate_regen".to_string());
        let hints = enqueue_hints_from_submit_params(&params).expect("hints");
        assert_eq!(hints.campaign_id.as_deref(), Some("structured"));
        assert_eq!(
            hints.benchmark_tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen)
        );
    }
}

fn socrates_context_from_retrieval(
    retrieval: &crate::memory::RetrievalEvidenceEnvelope,
) -> vox_orchestrator::SocratesTaskContext {
    vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: retrieval.retrieval_tier.clone(),
        memory_hit_count: retrieval.memory_hit_count,
        knowledge_hit_count: retrieval.knowledge_hit_count,
        chunk_hit_count: retrieval.chunk_hit_count,
        used_vector: retrieval.used_vector,
        used_bm25: retrieval.used_bm25,
        used_lexical_fallback: retrieval.used_lexical_fallback,
        contradiction_count: retrieval.contradiction_count,
    }
    .to_task_context()
}

/// Submit a new task to the orchestrator (async).
///
/// Routes the task to the best agent based on file affinity, acquires locks,
/// and enqueues it for processing.
pub async fn submit_task(state: &ServerState, params: SubmitTaskParams) -> String {
    // Session-scoped envelopes are attached inside `submit_task_with_agent`. MCP only overrides
    // when the client passes an explicit `retrieval` payload (may differ from the store).
    let explicit_retrieval = params.retrieval.as_ref();

    // Phase 7.3: Scope enforcement
    if let Some(agent_name) = &params.agent_name {
        if let Some(scopes) = load_agent_scopes(&state.repository.root, agent_name) {
            if !scopes.is_empty() && !scopes.iter().any(|s| s == "**" || s == "**/*") {
                for f in &params.files {
                    let mut ok = false;
                    let path_str = normalize_task_path(&state.repository.root, &f.path);
                    for s in &scopes {
                        if s.ends_with("/**") {
                            let prefix = s.trim_end_matches("/**");
                            if path_str.starts_with(prefix) {
                                ok = true;
                                break;
                            }
                        } else if path_str == *s {
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                            format!(
                                "Agent '{}' tried to edit outside its scope. File '{}' does not match scope {:?}",
                                agent_name, f.path, scopes
                            ),
                            REM_TASK_SCOPE,
                        )
                        .to_json();
                    }
                }
            }
        }
    }

    let bypass_questioning_gate = std::env::var("VOX_SUBMIT_TASK_BYPASS_QUESTIONING_GATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !bypass_questioning_gate {
        if let (Some(db), Some(sid)) = (&state.db, params.session_id.as_deref()) {
            match db
                .has_pending_clarification_for_mcp_session(sid, &state.repository.repository_id)
                .await
            {
                Ok(true) => {
                    return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                        "Socrates clarification pending for this MCP session; resolve before submitting tasks."
                            .to_string(),
                        REM_QUESTIONING_PENDING,
                    )
                    .to_json();
                }
                Err(e) => tracing::debug!(error = %e, "questioning gate: pending check failed"),
                Ok(false) => {}
            }
        }
    }

    let orch = &state.orchestrator;

    let manifest: Vec<FileAffinity> = params
        .files
        .iter()
        .map(|f| match f.access {
            crate::params::FileAccess::Write => FileAffinity::write(&f.path),
            crate::params::FileAccess::Read => FileAffinity::read(&f.path),
        })
        .collect();

    let priority = params.priority.as_deref().map(|p| match p {
        "background" => TaskPriority::Background,
        "urgent" => TaskPriority::Urgent,
        _ => TaskPriority::Normal,
    });
    let planning_mode = params.planning_mode.as_deref().and_then(|m| match m {
        "auto" => Some(vox_orchestrator::PlanningMode::Auto),
        "direct" => Some(vox_orchestrator::PlanningMode::Direct),
        "force_plan" => Some(vox_orchestrator::PlanningMode::ForcePlan),
        "workflow_only" => Some(vox_orchestrator::PlanningMode::WorkflowOnly),
        _ => None,
    });

    // Prompt canonicalization: normalize and order-invariant pack to reduce order bias
    let (description, canonical_info) = match prompt_canonical::canonicalize_prompt(
        &params.description,
        true, // order_invariant
        true, // run_safety_pass: reject injection attempts and surface in Trust & Safety
    ) {
        Ok(c) => {
            tracing::debug!(
                "prompt_canonical: task description hash {} -> {} conflict warnings",
                c.original_hash,
                c.conflict_warnings.len()
            );
            let warnings = if c.conflict_warnings.is_empty() {
                None
            } else {
                Some(c.conflict_warnings)
            };
            (c.text, Some((true, warnings, Some(c.original_hash))))
        }
        Err(e) => {
            orch.event_bus().emit(AgentEventKind::InjectionDetected {
                detail: e.to_string(),
            });
            return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                format!("Prompt safety: {e}"),
                REM_PROMPT_SAFETY,
            )
            .to_json();
        }
    };

    let enqueue_hints = enqueue_hints_from_submit_params(&params);
    let submit_result = if params.planning_mode.is_some() {
        orch.submit_goal(
            description.clone(),
            manifest,
            priority,
            planning_mode,
            params.session_id.clone(),
            enqueue_hints,
        )
        .await
    } else {
        orch.submit_task_with_agent(
            &description,
            manifest,
            priority,
            params.agent_name.clone(),
            params.capabilities.clone(),
            enqueue_hints,
            params.session_id.clone(),
        )
        .await
    };
    match submit_result {
        Ok(task_id) => {
            if let Some(retrieval) = explicit_retrieval {
                let soc = socrates_context_from_retrieval(retrieval);
                if let Err(e) = orch.attach_socrates_context(task_id, soc) {
                    tracing::warn!(
                        task_id = task_id.0,
                        error = %e,
                        "failed to attach Socrates retrieval context to submitted task"
                    );
                }
            }
            if let Some((_, Some(ref w), _)) = canonical_info {
                if !w.is_empty() {
                    orch.event_bus()
                        .emit(AgentEventKind::PromptConflictDetected {
                            task_id,
                            warnings: w.clone(),
                        });
                }
            }
            let agent_id = orch
                .task_assignments_copy()
                .get(&task_id)
                .map(|a| a.0)
                .unwrap_or(0);
            let (prompt_canonicalized, conflict_warnings, original_prompt_hash) =
                canonical_info.unwrap_or((false, None, None));
            let v2 = state
                .orchestrator_config
                .orchestration_migration
                .orchestration_v2_enabled;
            ToolResult::ok(SubmitTaskResponse {
                task_id: task_id.0,
                agent_id,
                prompt_canonicalized: Some(prompt_canonicalized),
                conflict_warnings,
                original_prompt_hash,
                orchestration_contract: if v2 { Some("v2".to_string()) } else { None },
            })
            .to_json()
        }
        Err(e) => {
            let msg = format!("{e}");
            let remediation =
                if msg.contains("scope") || msg.contains("Scope") || msg.contains("outside") {
                    REM_TASK_SCOPE
                } else {
                    REM_TASK_SUBMIT
                };
            ToolResult::<SubmitTaskResponse>::err_with_remediation(msg, remediation).to_json()
        }
    }
}

/// Get the current status of a specific task.
pub async fn task_status(state: &ServerState, params: TaskStatusParams) -> String {
    let orch = &state.orchestrator;

    let status = orch.status();
    let task_id = TaskId(params.task_id);
    for agent_summary in &status.agents {
        if let Some(queue_lock) = orch.agent_queue(AgentId(agent_summary.id.0)) {
            let queue = match crate::sync_poison::poison_rw_read(queue_lock.read(), "agent queue") {
                Ok(g) => g,
                Err(e) => {
                    tracing::warn!(error = %e, "task_status: agent queue poisoned");
                    continue;
                }
            };
            if queue.completed_ids().contains(&task_id) {
                return ToolResult::ok("Completed".to_string()).to_json();
            }
            if let Some(t) = queue.current_task() {
                if t.id == task_id {
                    return ToolResult::ok("InProgress".to_string()).to_json();
                }
            }
            if queue.is_blocked(task_id) {
                return ToolResult::ok("Blocked".to_string()).to_json();
            }
            let json = queue.to_json();
            if json.contains(&format!("\"id\": {trans}", trans = params.task_id))
                || json.contains(&format!("\"id\":{trans}", trans = params.task_id))
            {
                return ToolResult::ok("Queued".to_string()).to_json();
            }
        }
    }
    ToolResult::<String>::err_with_remediation(
        format!("task {} not found", params.task_id),
        REM_TASK_ID,
    )
    .to_json()
}

/// Mark a task as completed, releasing its file locks (async).
pub async fn complete_task(state: &ServerState, params: CompleteTaskParams) -> String {
    let task_id = TaskId(params.task_id);
    let assigned = state.orchestrator.agent_assigned_to_task(task_id);
    let attestation = vox_orchestrator::CompletionAttestation {
        completion_summary: params.completion_summary,
        checks_passed: params.checks_passed,
        artifact_paths: params.artifact_paths.into_iter().map(Into::into).collect(),
        declared_non_placeholder: params.declared_non_placeholder,
        force_risky: params.force_risky,
        force_risky_reason: params.force_risky_reason,
    };
    let res = state
        .orchestrator
        .complete_task_with_attestation(task_id, Some(attestation))
        .await;

    match res {
        Ok(()) => {
            // Gamification: update the agent-scoped companion (matches event_router / HUD).
            if let (Some(db), Some(aid)) = (&state.db, assigned) {
                let uid = vox_ludus::db::canonical_user_id();
                let id = format!("agent-{}", aid.0);
                let mut companion = match vox_ludus::db::list_companions(db, &uid).await {
                    Ok(comps) => comps
                        .into_iter()
                        .find(|c: &vox_ludus::companion::Companion| c.id == id),
                    Err(_) => None,
                }
                .unwrap_or_else(|| {
                    vox_ludus::companion::Companion::new(
                        &id,
                        &uid,
                        format!("Agent {}", aid.0),
                        "vox",
                    )
                });

                companion.interact(vox_ludus::companion::Interaction::TaskCompleted);
                let _ = vox_ludus::db::upsert_companion(db, &companion).await;
            }
            ToolResult::ok("task completed".to_string()).to_json()
        }
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_TASK_ORCH_OP).to_json()
        }
    }
}

/// Mark a task as failed with a reason (async).
pub async fn fail_task(state: &ServerState, params: FailTaskParams) -> String {
    let task_id = TaskId(params.task_id);
    let assigned = state.orchestrator.agent_assigned_to_task(task_id);
    let res = state.orchestrator.fail_task(task_id, params.reason).await;

    match res {
        Ok(()) => {
            if let (Some(db), Some(aid)) = (&state.db, assigned) {
                let uid = vox_ludus::db::canonical_user_id();
                let id = format!("agent-{}", aid.0);
                let mut companion = match vox_ludus::db::list_companions(db, &uid).await {
                    Ok(comps) => comps
                        .into_iter()
                        .find(|c: &vox_ludus::companion::Companion| c.id == id),
                    Err(_) => None,
                }
                .unwrap_or_else(|| {
                    vox_ludus::companion::Companion::new(
                        &id,
                        &uid,
                        format!("Agent {}", aid.0),
                        "vox",
                    )
                });

                companion.interact(vox_ludus::companion::Interaction::TaskFailed);
                let _ = vox_ludus::db::upsert_companion(db, &companion).await;
            }
            ToolResult::ok("task marked as failed".to_string()).to_json()
        }
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_TASK_ORCH_OP).to_json()
        }
    }
}

/// Cancel a task by ID.
pub async fn cancel_task(state: &ServerState, params: crate::params::CancelTaskParams) -> String {
    let orch = &state.orchestrator;
    match orch.cancel_task(TaskId(params.task_id)) {
        Ok(()) => ToolResult::ok("Task cancelled successfully".to_string()).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_TASK_ORCH_OP).to_json()
        }
    }
}

/// Change the priority of a queued task.
pub async fn reorder_task(state: &ServerState, params: crate::params::ReorderTaskParams) -> String {
    let orch = &state.orchestrator;

    let priority = match params.priority.as_str() {
        "urgent" => TaskPriority::Urgent,
        "background" => TaskPriority::Background,
        _ => TaskPriority::Normal,
    };

    match orch.reorder_task(TaskId(params.task_id), priority) {
        Ok(()) => ToolResult::ok("Task reordered successfully".to_string()).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_TASK_ORCH_OP).to_json()
        }
    }
}

/// Remove all queued tasks from an agent without retiring it.
pub async fn drain_agent(state: &ServerState, params: DrainAgentParams) -> String {
    let orch = &state.orchestrator;
    match orch.drain_agent(AgentId(params.agent_id)) {
        Ok(tasks) => ToolResult::ok(format!("Agent drained {} tasks", tasks.len())).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_TASK_ORCH_OP).to_json()
        }
    }
}

/// Publish a message to the bulletin board.
pub async fn publish_message(state: &ServerState, _params: PublishMessageParams) -> String {
    let orch = &state.orchestrator;
    let board = orch.bulletin();
    board.publish(vox_orchestrator::AgentMessage::DependencyReady { task_id: TaskId(0) });
    ToolResult::ok("message published".to_string()).to_json()
}
