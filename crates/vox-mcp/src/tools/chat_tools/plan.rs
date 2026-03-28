use serde::Deserialize;

use super::build_system_prompt;
use super::params::{PlanParams, PlanReplanParams, PlanResult, PlanStatusParams, PlanTask};
use super::plan_loop;
use crate::llm_bridge::{McpChatModelResolution, McpInferRouting, mcp_infer_completion};
use crate::params::ToolResult;
use crate::server::ServerState;
use crate::tools::chat_model_resolve::resolve_chat_llm_model;
use crate::tools::chat_socrates_meta::{
    clarification_turn_for_session, mcp_questioning_session_key, socrates_tool_meta,
    socrates_surface_tags, spawn_questioning_trace_from_socrates,
    spawn_socrates_telemetry_with_meta,
};

const REM_MCP_MODEL_RESOLVE: &str = "Run `list_models`, ensure Ollama/API routes work, and check `vox clavis doctor` for inference secrets.";
const REM_MCP_MODEL_LOCK: &str =
    "Retry; restart the MCP server if `mcp_chat_model_override` stays poisoned.";
const REM_LLM_COMPLETION: &str = "Check inference logs, rate limits, and backend health; verify API keys via `vox clavis doctor`.";
const REM_PLAN_JSON: &str = "Retry planning with a simpler goal or lower `max_tasks`; ensure the model returns valid JSON in a ```json block.";
const REM_DEI_DAEMON: &str =
    "Start `vox-dei-d` (DeI daemon) or verify IPC/socket configuration for this workspace.";

#[derive(Deserialize)]
struct PlanResponseSchema {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    tasks: Vec<PlanTask>,
}

/// Generate a structured plan for a goal. Optionally writes PLAN.md to the workspace root.
/// This backs the Cursor-style "Planning Mode" in the extension and in Vox agents.
pub async fn plan_goal(state: &ServerState, params: PlanParams) -> String {
    let max_tasks = params.max_tasks.unwrap_or(30);
    let scope_note = if params.scope_files.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nScope this plan to these files:\n{}",
            params.scope_files.join("\n")
        )
    };

    let user_prompt = format!(
        r#"You are an expert software architect and planner.

GOAL: {goal}{scope_note}

Generate a comprehensive, ordered task list to achieve this goal. You MUST output a valid JSON object matching this schema, embedded in a ```json codeblock.

{{
  "summary": "2-3 sentence executive summary of the approach",
  "tasks": [
    {{
      "id": 1,
      "description": "Short imperative description of what to implement.",
      "files": ["path/to/file.rs"],
      "estimated_complexity": 5,
      "depends_on": []
    }}
  ]
}}

Rules:
- Every task must be atomic and independently verifiable.
- "estimated_complexity" must be an integer from 1 (trivial edit) to 10 (full subsystem build).
- "depends_on" must be an array of prior task IDs that must complete first.
- If files are unknown, leave the array empty or use `["TBD"]`.
- Include test tasks explicitly.
- Maximum {max_tasks} tasks.
- Do NOT include filler tasks like 'Review and refactor'."#,
        goal = params.goal,
        max_tasks = max_tasks,
        scope_note = scope_note
    );

    let system_prompt = build_system_prompt(state).await;
    let resolution_template = McpChatModelResolution {
        complexity: match params.max_tasks {
            Some(n) if n > 10 => 9,
            _ => 7,
        },
        ..Default::default()
    };

    let (model, free_only) = match resolve_chat_llm_model(
        state,
        &user_prompt,
        resolution_template.clone(),
        params.session_id.as_deref(),
    )
    .await
    {
        Ok(pair) => pair,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("No model found for plan: {e}"),
                REM_MCP_MODEL_RESOLVE,
            )
            .to_json();
        }
    };

    let pref = match crate::sync_poison::poison_rw_read(
        state.mcp_chat_model_override.read(),
        "mcp_chat_model_override",
    ) {
        Ok(g) => g.clone(),
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e.to_string(), REM_MCP_MODEL_LOCK)
                .to_json();
        }
    };
    let routing = McpInferRouting {
        user_prompt: &user_prompt,
        sticky_model_pref: pref.as_deref(),
        resolution_template,
        free_only,
        allow_cloud_ollama_fallback: true,
        user_id: params.session_id.as_deref(),
    };

    let plan_llm_started = std::time::Instant::now();
    let (response_json, model_used, _tokens) = match mcp_infer_completion(
        state,
        model,
        "vox_plan",
        &system_prompt,
        &routing,
        4096,
        0.3,
        true, // Enforce strict JSON mode for planning
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("LLM error: {e}"),
                REM_LLM_COMPLETION,
            )
            .to_json();
        }
    };

    let plan_session_key =
        mcp_questioning_session_key(state, "vox_plan", params.session_id.as_deref());
    state.record_questioning_attention_spend(
        &plan_session_key,
        plan_llm_started.elapsed().as_millis() as u64,
    );

    // Strip any markdown fences if the model still included them despite JSON mode
    let block = response_json.trim();
    let cleaned = if block.starts_with("```json") {
        block
            .strip_prefix("```json")
            .unwrap_or(block)
            .strip_suffix("```")
            .unwrap_or(block)
            .trim()
    } else if block.starts_with("```") {
        block
            .strip_prefix("```")
            .unwrap_or(block)
            .strip_suffix("```")
            .unwrap_or(block)
            .trim()
    } else {
        block
    };

    let parsed: PlanResponseSchema = match serde_json::from_str(cleaned) {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, raw = cleaned, "plan_goal: JSON decode failed after cleanup");
            return ToolResult::<String>::err_with_remediation(
                format!("Failed to parse task list JSON: {e}"),
                REM_PLAN_JSON,
            )
            .to_json();
        }
    };

    let summary = if parsed.summary.is_empty() {
        "No summary provided.".to_string()
    } else {
        parsed.summary
    };
    let mut tasks = parsed.tasks;

    let loop_sess = mcp_questioning_session_key(state, "vox_plan", params.session_id.as_deref());
    let complexity_for_refine = match params.max_tasks {
        Some(n) if n > 10 => 9,
        _ => 7,
    };
    let (refined_tasks, refined_summary, loop_state) = plan_loop::maybe_refine_plan(
        state,
        &params,
        tasks,
        summary,
        complexity_for_refine,
        &loop_sess,
    )
    .await;
    tasks = refined_tasks;
    let summary = refined_summary;

    if let Some(db) = state.db.as_ref() {
        if let Some(pid) = params.plan_telemetry_session_id.as_deref() {
            let strat = format!("mcp_plan:{:?}", params.loop_mode.unwrap_or_default());
            let _ = db
                .create_plan_session(pid, params.session_id.as_deref(), &params.goal, &strat)
                .await;
            let meta = serde_json::json!({
                "refinement_rounds": loop_state.refinement_rounds,
                "loop_status": loop_state.loop_status,
                "stop_reason": loop_state.stop_reason,
                "telemetry": "vox_mcp_iterative_plan",
            });
            let _ = db
                .update_plan_session_iterative_fields(
                    pid,
                    params.question_link_session_id.as_deref(),
                    i64::from(loop_state.refinement_rounds),
                    loop_state.stop_reason.as_deref(),
                    Some(&meta.to_string()),
                )
                .await;
        }
    }

    let plan_total_tasks = tasks.len();
    let page_off = params.plan_page_offset.unwrap_or(0);
    let tasks_for_payload: Vec<PlanTask> = if let Some(lim) = params.plan_page_limit {
        tasks.iter().skip(page_off).take(lim).cloned().collect()
    } else {
        tasks.clone()
    };

    // Manual markdown generation for the on-disk/visual summary
    let mut base_plan_md = format!("## Plan\n\n**Overall Summary**: {summary}\n\n### Tasks\n\n");
    if tasks_for_payload.is_empty() {
        base_plan_md.push_str("*(No tasks generated)*\n");
    } else {
        for t in &tasks_for_payload {
            let deps = if t.depends_on.is_empty() {
                String::new()
            } else {
                let dep_strs: Vec<String> = t.depends_on.iter().map(|d| d.to_string()).collect();
                format!(" [depends: {}]", dep_strs.join(", "))
            };
            base_plan_md.push_str(&format!(
                "{}. **{}** — [files: {}] [complexity: {}/10]{}\n\n",
                t.id,
                t.description,
                t.files.join(", "),
                t.estimated_complexity,
                deps
            ));
        }
    }

    // Optionally write PLAN.md (always full refined task list when paginating the tool payload).
    let written_to_disk = if params.write_to_disk {
        let plan_path = state
            .workspace_root
            .as_deref()
            .unwrap_or(std::path::Path::new("."))
            .join("PLAN.md");
        let header = format!(
            "# Vox Plan\n\n**Goal**: {}\n**Generated**: {}\n**Model**: {}\n\n",
            params.goal,
            chrono::Local::now().format("%Y-%m-%d %H:%M"),
            model_used,
        );
        let body_md = if params.plan_page_limit.is_some() {
            let mut md = format!("## Plan\n\n**Overall Summary**: {summary}\n\n### Tasks\n\n");
            if tasks.is_empty() {
                md.push_str("*(No tasks generated)*\n");
            } else {
                for t in &tasks {
                    let deps = if t.depends_on.is_empty() {
                        String::new()
                    } else {
                        let dep_strs: Vec<String> =
                            t.depends_on.iter().map(|d| d.to_string()).collect();
                        format!(" [depends: {}]", dep_strs.join(", "))
                    };
                    md.push_str(&format!(
                        "{}. **{}** — [files: {}] [complexity: {}/10]{}\n\n",
                        t.id,
                        t.description,
                        t.files.join(", "),
                        t.estimated_complexity,
                        deps
                    ));
                }
            }
            md
        } else {
            base_plan_md.clone()
        };
        let full = header + &body_md;
        std::fs::write(&plan_path, &full).is_ok()
    } else {
        false
    };

    let gap_report_json = loop_state
        .last_gap_report
        .as_ref()
        .and_then(|g| serde_json::to_value(g).ok());
    let last_risk = loop_state
        .last_gap_report
        .as_ref()
        .map(|g| g.aggregate_unresolved_risk);
    let clarifying = if params.questioning_hints_enabled == Some(true) {
        loop_state
            .last_gap_report
            .as_ref()
            .map(|g| g.suggested_clarifying_questions.clone())
            .unwrap_or_default()
    } else {
        vec![]
    };

    let result = PlanResult {
        goal: params.goal.clone(),
        tasks: tasks_for_payload,
        summary,
        plan_md: base_plan_md,
        written_to_disk,
        plan_total_tasks,
        plan_page_offset: page_off,
        loop_mode_effective: format!("{:?}", params.loop_mode.unwrap_or_default())
            .to_ascii_lowercase(),
        refinement_rounds: loop_state.refinement_rounds,
        loop_stop_reason: loop_state.stop_reason,
        last_aggregate_gap_risk: last_risk,
        gap_report: gap_report_json,
        clarifying_questions: clarifying,
    };

    let grounding = if params.scope_files.is_empty() {
        0.56_f64
    } else {
        0.74_f64
    };
    let pol = state.orchestrator_config.effective_socrates_policy();
    let session_key = plan_session_key;
    let turn = clarification_turn_for_session(state, &session_key).await;
    let (spent_att, max_att) = state.questioning_attention_bounds(&session_key);
    let soc = socrates_tool_meta(&pol, grounding, false, turn, spent_att, max_att);
    spawn_socrates_telemetry_with_meta(
        state,
        "vox_plan",
        soc.clone(),
        Some(model_used.clone()),
        Some(socrates_surface_tags(
            "planning",
            &["planning", "decomposition"],
        )),
    );
    spawn_questioning_trace_from_socrates(
        state,
        "vox_plan",
        soc.clone(),
        Some(session_key.clone()),
        Some(params.goal.clone()),
    );
    let mut v = serde_json::to_value(&result).unwrap_or(serde_json::Value::Null);
    if let Some(obj) = v.as_object_mut() {
        obj.insert("socrates".to_string(), soc);
    }
    ToolResult::ok(v).to_json()
}

/// Replan an existing DeI plan session (`vox-dei-d` on PATH or next to the MCP binary).
pub async fn plan_replan(state: &ServerState, params: PlanReplanParams) -> String {
    let body = serde_json::json!({
        "session_id": params.session_id,
        "delta_hint": params.delta_hint,
        "write_to_disk": params.write_to_disk,
        "mode": params.mode,
    });
    match crate::dei_ipc::call_dei_daemon("ai.plan.replan", body).await {
        Ok(mut v) => {
            let pol = state.orchestrator_config.effective_socrates_policy();
            let session_key =
                mcp_questioning_session_key(state, "vox_replan", Some(params.session_id.as_str()));
            let turn = clarification_turn_for_session(state, &session_key).await;
            let (spent_att, max_att) = state.questioning_attention_bounds(&session_key);
            let soc = socrates_tool_meta(&pol, 0.62, false, turn, spent_att, max_att);
            spawn_socrates_telemetry_with_meta(
                state,
                "vox_replan",
                soc.clone(),
                None,
                Some(socrates_surface_tags("planning", &["planning", "replan"])),
            );
            spawn_questioning_trace_from_socrates(
                state,
                "vox_replan",
                soc.clone(),
                Some(session_key.clone()),
                Some(params.delta_hint.clone()),
            );
            if let Some(obj) = v.as_object_mut() {
                obj.insert("socrates".to_string(), soc);
            }
            ToolResult::ok(v).to_json()
        }
        Err(e) => {
            ToolResult::<serde_json::Value>::err_with_remediation(e.to_string(), REM_DEI_DAEMON)
                .to_json()
        }
    }
}

/// Read structured plan session status from `vox-dei-d`.
pub async fn plan_status(state: &ServerState, params: PlanStatusParams) -> String {
    let body = serde_json::json!({ "session_id": params.session_id });
    match crate::dei_ipc::call_dei_daemon("ai.plan.status", body).await {
        Ok(mut v) => {
            let pol = state.orchestrator_config.effective_socrates_policy();
            let session_key = mcp_questioning_session_key(
                state,
                "vox_plan_status",
                Some(params.session_id.as_str()),
            );
            let turn = clarification_turn_for_session(state, &session_key).await;
            let (spent_att, max_att) = state.questioning_attention_bounds(&session_key);
            let soc = socrates_tool_meta(&pol, 0.58, false, turn, spent_att, max_att);
            spawn_socrates_telemetry_with_meta(
                state,
                "vox_plan_status",
                soc.clone(),
                None,
                Some(socrates_surface_tags("planning_status", &["planning", "status"])),
            );
            spawn_questioning_trace_from_socrates(
                state,
                "vox_plan_status",
                soc.clone(),
                Some(session_key.clone()),
                None,
            );
            if let Some(obj) = v.as_object_mut() {
                obj.insert("socrates".to_string(), soc);
            }
            ToolResult::ok(v).to_json()
        }
        Err(e) => {
            ToolResult::<serde_json::Value>::err_with_remediation(e.to_string(), REM_DEI_DAEMON)
                .to_json()
        }
    }
}
