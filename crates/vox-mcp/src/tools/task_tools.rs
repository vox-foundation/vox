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

/// Submit a new task to the orchestrator (async).
///
/// Routes the task to the best agent based on file affinity, acquires locks,
/// and enqueues it for processing.
pub async fn submit_task(state: &ServerState, params: SubmitTaskParams) -> String {
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
                        return ToolResult::<SubmitTaskResponse>::err(format!(
                            "Agent '{}' tried to edit outside its scope. File '{}' does not match scope {:?}",
                            agent_name, f.path, scopes
                        ))
                        .to_json();
                    }
                }
            }
        }
    }

    let mut orch = state.orchestrator.lock().await;

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
            return ToolResult::<SubmitTaskResponse>::err(format!("Prompt safety: {e}")).to_json();
        }
    };

    match orch
        .submit_task_with_agent(
            &description,
            manifest,
            priority,
            params.agent_name.clone(),
            params.capabilities.clone(),
            params.session_id.clone(),
        )
        .await
    {
        Ok(task_id) => {
            if let Some((_, Some(ref w), _)) = canonical_info {
                if !w.is_empty() {
                    orch.event_bus()
                        .emit(AgentEventKind::PromptConflictDetected {
                            task_id,
                            warnings: w.clone(),
                        });
                }
            }
            let agent_ids = orch.agent_ids();
            let agent_id = agent_ids.last().map(|a| a.0).unwrap_or(0);
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
        Err(e) => ToolResult::<SubmitTaskResponse>::err(format!("{e}")).to_json(),
    }
}

/// Get the current status of a specific task.
pub async fn task_status(state: &ServerState, params: TaskStatusParams) -> String {
    let orch = state.orchestrator.lock().await;

    let status = orch.status();
    let task_id = TaskId(params.task_id);
    for agent_summary in &status.agents {
        if let Some(queue) = orch.agent_queue(AgentId(agent_summary.id.0)) {
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
    ToolResult::<String>::err(format!("task {} not found", params.task_id)).to_json()
}

/// Mark a task as completed, releasing its file locks (async).
pub async fn complete_task(state: &ServerState, params: CompleteTaskParams) -> String {
    let res = {
        let mut orch = state.orchestrator.lock().await;
        orch.complete_task(TaskId(params.task_id)).await
    };

    match res {
        Ok(()) => {
            // Gamification: Update companion state
            if let Some(db) = &state.db {
                let id = "vox-orchestrator";
                let mut companion = match vox_ludus::db::list_companions(db, "user").await {
                    Ok(comps) => comps
                        .into_iter()
                        .find(|c: &vox_ludus::companion::Companion| c.id == id),
                    Err(_) => None,
                }
                .unwrap_or_else(|| {
                    vox_ludus::companion::Companion::new(id, "user", "Vox Orchestrator", "vox")
                });

                companion.interact(vox_ludus::companion::Interaction::TaskCompleted);
                let _ = vox_ludus::db::upsert_companion(db, &companion).await;
            }
            ToolResult::ok("task completed".to_string()).to_json()
        }
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// Mark a task as failed with a reason (async).
pub async fn fail_task(state: &ServerState, params: FailTaskParams) -> String {
    let res = {
        let mut orch = state.orchestrator.lock().await;
        orch.fail_task(TaskId(params.task_id), params.reason).await
    };

    match res {
        Ok(()) => {
            if let Some(db) = &state.db {
                let id = "vox-orchestrator";
                let mut companion = match vox_ludus::db::list_companions(db, "user").await {
                    Ok(comps) => comps
                        .into_iter()
                        .find(|c: &vox_ludus::companion::Companion| c.id == id),
                    Err(_) => None,
                }
                .unwrap_or_else(|| {
                    vox_ludus::companion::Companion::new(id, "user", "Vox Orchestrator", "vox")
                });

                companion.interact(vox_ludus::companion::Interaction::TaskFailed);
                let _ = vox_ludus::db::upsert_companion(db, &companion).await;
            }
            ToolResult::ok("task marked as failed".to_string()).to_json()
        }
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// Cancel a task by ID.
pub async fn cancel_task(state: &ServerState, params: crate::params::CancelTaskParams) -> String {
    let mut orch = state.orchestrator.lock().await;
    match orch.cancel_task(TaskId(params.task_id)) {
        Ok(()) => ToolResult::ok("Task cancelled successfully".to_string()).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// Change the priority of a queued task.
pub async fn reorder_task(state: &ServerState, params: crate::params::ReorderTaskParams) -> String {
    let mut orch = state.orchestrator.lock().await;

    let priority = match params.priority.as_str() {
        "urgent" => TaskPriority::Urgent,
        "background" => TaskPriority::Background,
        _ => TaskPriority::Normal,
    };

    match orch.reorder_task(TaskId(params.task_id), priority) {
        Ok(()) => ToolResult::ok("Task reordered successfully".to_string()).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// Remove all queued tasks from an agent without retiring it.
pub async fn drain_agent(state: &ServerState, params: DrainAgentParams) -> String {
    let mut orch = state.orchestrator.lock().await;
    match orch.drain_agent(AgentId(params.agent_id)) {
        Ok(tasks) => ToolResult::ok(format!("Agent drained {} tasks", tasks.len())).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}

/// Publish a message to the bulletin board.
pub async fn publish_message(state: &ServerState, _params: PublishMessageParams) -> String {
    let orch = state.orchestrator.lock().await;
    let board = orch.bulletin();
    board.publish(vox_orchestrator::AgentMessage::DependencyReady { task_id: TaskId(0) });
    ToolResult::ok("message published".to_string()).to_json()
}
