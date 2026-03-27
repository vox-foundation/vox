//! Task management tool handlers for the Vox MCP server.
//!
//! Covers: submit, status, complete, fail, cancel, reorder, drain, and publish.

use vox_orchestrator::{AgentEventKind, AgentId, FileAffinity, TaskId, TaskPriority};
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
        .map(|f| match f.access.as_str() {
            "write" => FileAffinity::write(&f.path),
            _ => FileAffinity::read(&f.path),
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

    let submit_result = if params.planning_mode.is_some() {
        orch.submit_goal(
            description.clone(),
            manifest,
            priority,
            planning_mode,
            params.session_id.clone(),
        )
        .await
    } else {
        orch.submit_task_with_agent(
            &description,
            manifest,
            priority,
            params.agent_name.clone(),
            params.capabilities.clone(),
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
    let res = state.orchestrator.complete_task(task_id).await;

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
