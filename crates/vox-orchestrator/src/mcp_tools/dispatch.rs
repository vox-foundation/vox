//! `handle_tool_call` routing for all static MCP tools.
//!
//! ## Persisted tool args (Ludus / raw `tool_call` rows)
//! After each dispatch, when Codex is attached, stored payloads use
//! [`vox_ludus::mcp_privacy::prepare_mcp_tool_args_for_storage`] for **both** Ludus-routed `mcp_tool_called` events and the
//! fallback `insert_event` path. New DB persistence for MCP args must go through the same helper + env (`VOX_LUDUS_MCP_TOOL_ARGS`).

use crate::mcp_tools::params::ToolResult;
use crate::mcp_tools::server_state::ServerState;

use crate::mcp_tools::{
    benchmark_tools, browser_tools, chat_tools, clavis_tools, code_validator, codex_tools,
    compiler_tools, db_tools, exec_time_tools, git_tools, grammar_tools, introspection_tools,
    openclaw_tools, persistence_tools, populi_tools, project_init_tools,
    questioning_tools, rag_tools, repo_catalog_tools, repo_index, task_tools,
    toestub_tools, tool_aliases, training_tools, trust_tools, vcs_tools, visus_tools,
};
#[cfg(feature = "news-publish")]
use crate::mcp_tools::{news_tools, scientia_tools};

#[cfg(feature = "oratio-rerank")]
use crate::mcp_tools::{oratio_tools, speech_pipeline_tools};

/// Dispatch `name` to the matching submodule handler and record skill telemetry if DB is available.
pub async fn handle_tool_call(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
) -> Result<String, anyhow::Error> {
    let start_time = std::time::Instant::now();
    let name_canonical = tool_aliases::canonical_tool_name(name);

    // Check if the agent ID or session ID is included in meta arguments
    let agent_id = args.get("agent_id").and_then(|v| v.as_str());
    let session_id = args.get("session_id").and_then(|v| v.as_str());
    let trace_for_telemetry = args
        .get("trace_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            args.get("correlation_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
        });

    // Check Budget limits for explicit Tool interception (Agent Self-Correction)
    let b_signal = {
        let aid = agent_id.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
        let bm = state.orchestrator.budget_manager_handle();
        crate::sync_lock::rw_read(&*bm).agent_budget_signal(crate::types::AgentId(aid))
    };

    if matches!(
        b_signal,
        crate::budget::BudgetSignal::CostExceeded { .. }
            | crate::budget::BudgetSignal::Critical { .. }
    ) {
        return Ok(crate::mcp_tools::params::ToolResult::<()>::err("SYSTEM_INTERVENTION: You have exceeded your global task budget. Proceed to finalize and abort immediately.").to_json_compact());
    }

    // Unenforced LLM "Laziness" Ingestion Gate
    if matches!(
        name_canonical,
        "vox_write_file"
            | "vox_patch_file"
            | "vox_inline_edit_file"
            | "vox_multi_replace"
            | "vox_multi_replace_file"
    ) {
        let args_str = args.to_string();
        if args_str.contains("todo!()")
            || args_str.contains("unimplemented!()")
            || args_str.contains("// TODO")
        {
            return Ok(crate::mcp_tools::params::ToolResult::<()>::err("LAZY_GENERATION_DETECTED: The system intercepted a TOESTUB pattern (e.g. todo!(), unimplemented!(), or // TODO) in your code output. You must emit the complete, fully-implemented code. Re-run your action with the actual logic.").to_json_compact());
        }
    }

    if let Some(rejection) =
        crate::mcp_tools::scope_guard::check_scope(state, name_canonical, agent_id, &args)
    {
        return Ok(crate::mcp_tools::params::ToolResult::<()>::err(rejection).to_json_compact());
    }

    // Trust-Tier RBAC for dangerous operations
    if matches!(
        name_canonical,
        "vox_run_shell"
            | "vox_deploy"
            | "vox_multi_replace"
            | "vox_multi_replace_file"
            | "vox_write_file"
            | "vox_delete_file"
    ) {
        // Enforce explicit UserApproval requirement
        let approved = args
            .get("user_approval")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !approved {
            return Ok(crate::mcp_tools::params::ToolResult::<()>::err("RBAC_VIOLATION: This operation requires explicit UserApproval mode. Please set `user_approval: true` or seek explicit confirmation.").to_json_compact());
        }
    }

    let db_opt = state.db.as_ref().map(|db| (**db).clone());
    let te = vox_db::TimedExecution::new(
        format!("mcp:{}", name_canonical),
        &state.repository.repository_id,
        None,
        db_opt,
    )
    .with_costs(None, None, None);

    let result = te
        .run(|| async { handle_tool_call_inner(state, name_canonical, args.clone()).await })
        .await;

    let duration_ms = start_time.elapsed().as_millis() as i64;

    if let Some(ref tid) = trace_for_telemetry {
        tracing::info!(
            target: "vox_mcp::trace",
            trace_id = %tid,
            tool = name_canonical,
            duration_ms,
            success = result.is_ok(),
            "mcp_tool_call"
        );
    }

    // Ludus: canonical reward path when enabled; raw telemetry when gamification is off.
    if let Some(db) = &state.db {
        let aid = agent_id.and_then(|s| s.parse::<u64>().ok()).unwrap_or(0u64);
        let args_stored = vox_ludus::mcp_privacy::prepare_mcp_tool_args_for_storage(&args);
        let mut route_ev = serde_json::json!({
            "type": "mcp_tool_called",
            "agent_id": aid,
            "tool": name_canonical,
            "args": args_stored,
            "duration_ms": duration_ms,
            "success": result.is_ok(),
            "repository_id": state.repository.repository_id,
        });
        if let Some(sid) = session_id {
            route_ev["session_id"] = serde_json::Value::String(sid.to_string());
        }
        if let Some(ref tid) = trace_for_telemetry {
            route_ev["trace_id"] = serde_json::Value::String(tid.clone());
        }
        if vox_ludus::config_gate::is_enabled() {
            let _ = vox_ludus::event_router::route_event_auto_user(db, &route_ev).await;
        } else {
            let mut payload = serde_json::json!({
                "type": "tool_call",
                "tool": name_canonical,
                "args": args_stored,
                "duration_ms": duration_ms,
                "success": result.is_ok(),
                "repository_id": state.repository.repository_id,
            });
            if let Some(sid) = session_id {
                payload["session_id"] = serde_json::Value::String(sid.to_string());
            }
            if let Some(ref tid) = trace_for_telemetry {
                payload["trace_id"] = serde_json::Value::String(tid.clone());
            }
            let agent_str = agent_id.unwrap_or("0");
            let _ =
                vox_ludus::db::insert_event(db, agent_str, "tool_call", Some(&payload.to_string()))
                    .await;
        }
    }

    result
}
async fn handle_tool_call_inner(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
) -> Result<String, anyhow::Error> {
    match name {
        "vox_visual_rag_query" => {
            Ok(rag_tools::visual_rag_query(state, serde_json::from_value(args)?).await)
        }
        "vox_submit_task" => {
            Ok(task_tools::submit_task(state, serde_json::from_value(args)?).await)
        }
        "vox_task_status" => {
            Ok(task_tools::task_status(state, serde_json::from_value(args)?).await)
        }
        "vox_test_decision" => {
            Ok(task_tools::test_decision(state, serde_json::from_value(args)?).await)
        }
        "vox_orchestrator_status" => crate::mcp_tools::dei_tools::orchestrator_status(state).await,
        "vox_orchestrator_persistence_outbox_lifecycle" => {
            Ok(persistence_tools::persistence_outbox_lifecycle(state, args).await)
        }
        "vox_orchestrator_persistence_outbox_queue" => {
            Ok(persistence_tools::persistence_outbox_queue(state, args).await)
        }
        "vox_orchestrator_start" => {
            Ok(crate::mcp_tools::dei_tools::orchestrator_start(state).await)
        }
        "vox_spawn_agent" => Ok(crate::mcp_tools::dei_tools::spawn_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_retire_agent" => Ok(crate::mcp_tools::dei_tools::retire_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_pause_agent" => Ok(crate::mcp_tools::dei_tools::pause_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_resume_agent" => Ok(crate::mcp_tools::dei_tools::resume_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_complete_task" => {
            Ok(task_tools::complete_task(state, serde_json::from_value(args)?).await)
        }
        "vox_fail_task" => Ok(task_tools::fail_task(state, serde_json::from_value(args)?).await),
        "vox_doubt_task" => Ok(task_tools::doubt_task(state, serde_json::from_value(args)?).await),
        "vox_check_file_owner" => Ok(crate::mcp_tools::dei_tools::check_file_owner(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await),

        "vox_validate_file" => {
            let path_opt = args.get("path").and_then(|v| v.as_str());
            let s_id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let t_id = args
                .get("task_id")
                .and_then(|v| v.as_str())
                .or_else(|| args.get("agent_id").and_then(|v| v.as_str()))
                .unwrap_or(s_id);

            // Intercept path and run observer
            if let Some(p) = path_opt {
                let resolved =
                    crate::mcp_tools::workspace_path::resolve_existing_path_in_repository(state, p)
                        .unwrap_or_else(|_| std::path::PathBuf::from(p));
                let report = if resolved.extension().and_then(|s| s.to_str()) == Some("rs")
                    || resolved.extension().and_then(|s| s.to_str()) == Some("vox")
                {
                    state.observer.observe_rust_file(s_id, t_id, &resolved)
                } else {
                    state.observer.observe_file(s_id, t_id, &resolved)
                };
                state
                    .orchestrator
                    .event_bus()
                    .emit(crate::AgentEventKind::ObservationRecorded {
                        agent_id: crate::types::AgentId(t_id.parse().unwrap_or(0)),
                        task_id: crate::types::TaskId(t_id.parse().unwrap_or(0)),
                        file_path: resolved.clone(),
                        lsp_error_count: report.lsp_error_count,
                        parse_rate: report.parse_rate,
                        construct_coverage: report.construct_coverage,
                        recommended_action: format!("{:?}", report.recommended_action),
                    });
            }

            Ok(code_validator::validate_file(state, serde_json::from_value(args)?).await)
        }
        "vox_check" => Ok(code_validator::vox_check(state, serde_json::from_value(args)?).await),
        "vox_validate_source" => {
            Ok(code_validator::validate_source(state, serde_json::from_value(args)?).await)
        }
        "vox_run_tests" => {
            Ok(compiler_tools::run_tests(state, serde_json::from_value(args)?).await)
        }
        "vox_check_workspace" => Ok(compiler_tools::check_workspace(state).await),
        "vox_test_all" => Ok(compiler_tools::test_all(state).await),
        "vox_publish_message" => {
            Ok(task_tools::publish_message(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_list_remote" => Ok(openclaw_tools::openclaw_list_remote(state).await),
        "vox_openclaw_search_remote" => {
            Ok(openclaw_tools::openclaw_search_remote(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_import_skill" => {
            Ok(openclaw_tools::openclaw_import_skill(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_discover" => Ok(openclaw_tools::openclaw_discover(state).await),
        "vox_openclaw_health" => Ok(openclaw_tools::openclaw_health(state).await),
        "vox_openclaw_gateway_call" => {
            Ok(openclaw_tools::openclaw_gateway_call(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_subscriptions" => Ok(openclaw_tools::openclaw_subscriptions(state).await),
        "vox_openclaw_subscribe" => {
            Ok(openclaw_tools::openclaw_subscribe(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_unsubscribe" => {
            Ok(openclaw_tools::openclaw_unsubscribe(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_notify" => {
            Ok(openclaw_tools::openclaw_notify(state, serde_json::from_value(args)?).await)
        }

        "vox_git_log" => Ok(git_tools::git_log(
            state,
            args.get("max_commits")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
        )
        .await),
        "vox_git_diff" => {
            Ok(git_tools::git_diff(state, args.get("path").and_then(|v| v.as_str())).await)
        }
        "vox_git_status" => Ok(git_tools::git_status(state).await),
        "vox_git_blame" => Ok(git_tools::git_blame(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await),
        "vox_repo_index_status" => Ok(repo_index::repo_index_status(state).await),
        "vox_repo_index_refresh" => Ok(repo_index::repo_index_refresh(state).await),
        "vox_visus_audit" => {
            Ok(visus_tools::vox_visus_audit(state, serde_json::from_value(args)?).await)
        }
        "vox_visus_baseline" => {
            Ok(visus_tools::vox_visus_baseline(state, serde_json::from_value(args)?).await)
        }
        "vox_repo_status" => Ok(repo_catalog_tools::repo_status(state).await),
        "vox_project_init" => Ok(project_init_tools::project_init(state, args).await),
        "vox_repo_catalog_list" => Ok(repo_catalog_tools::repo_catalog_list(state).await),
        "vox_repo_catalog_refresh" => Ok(repo_catalog_tools::repo_catalog_refresh(state).await),
        "vox_repo_query_text" => {
            Ok(repo_catalog_tools::repo_query_text(state, serde_json::from_value(args)?).await)
        }
        "vox_repo_query_file" => {
            Ok(repo_catalog_tools::repo_query_file(state, serde_json::from_value(args)?).await)
        }
        "vox_repo_query_history" => {
            Ok(repo_catalog_tools::repo_query_history(state, serde_json::from_value(args)?).await)
        }

        "vox_language_surface" => Ok(introspection_tools::language_surface().to_string()),
        "vox_capability_model_manifest" => {
            Ok(introspection_tools::capability_model_manifest(state)?.to_string())
        }
        "vox_compiler::ast_inspect" => Ok(introspection_tools::ast_inspect(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await?
        .to_string()),
        "vox_pipeline_status" => Ok(introspection_tools::pipeline_status().await.to_string()),
        "vox_decorator_registry" => Ok(introspection_tools::decorator_registry().to_string()),
        "vox_builtin_registry" => Ok(introspection_tools::builtin_registry().to_string()),
        "vox_workspace_modules" => Ok(introspection_tools::workspace_modules(state)
            .await?
            .to_string()),
        "vox_a2a_tasks" => Ok(introspection_tools::a2a_tasks(state).await?.to_string()),
        "vox_export_grammar_ebnf" => Ok(grammar_tools::export_grammar_ebnf(state).await),

        "vox_snapshot_list" => Ok(vcs_tools::snapshot_list(state, args).await),
        "vox_snapshot_diff" => Ok(vcs_tools::snapshot_diff(state, args).await),
        "vox_snapshot_restore" => Ok(vcs_tools::snapshot_restore(state, args).await),
        "vox_oplog" => Ok(vcs_tools::oplog_list(state, args).await),
        "vox_undo" => Ok(vcs_tools::oplog_undo(state, args).await),
        "vox_redo" => Ok(vcs_tools::oplog_redo(state, args).await),
        "vox_conflicts" => Ok(vcs_tools::conflicts_list(state).await),
        "vox_resolve_conflict" => Ok(vcs_tools::resolve_conflict(state, args).await),
        "vox_conflict_diff" => Ok(vcs_tools::conflict_diff(state, args).await),
        "vox_workspace_create" => Ok(vcs_tools::workspace_create(state, args).await),
        "vox_workspace_merge" => Ok(vcs_tools::workspace_merge(state, args).await),
        "vox_workspace_status" => Ok(vcs_tools::workspace_status(state, args).await),
        "vox_change_create" => Ok(vcs_tools::change_create(state, args).await),
        "vox_change_log" => Ok(vcs_tools::change_log(state, args).await),
        "vox_vcs_status" => Ok(crate::mcp_tools::dei_tools::vcs_status(state).await),

        "vox_db_schema" => Ok(db_tools::vox_db_schema(args)),
        "vox_db_relationships" => Ok(db_tools::vox_db_relationships(args)),
        "vox_db_data_flow" => Ok(db_tools::vox_db_data_flow(args)),
        "vox_db_sample_data" => Ok(db_tools::vox_db_sample_data(state, args).await),
        "vox_journey_canonical_steps" => {
            Ok(db_tools::vox_journey_canonical_steps(state, args).await)
        }
        "vox_db_explain_query" => Ok(db_tools::vox_db_explain_query(state, args).await),
        "vox_db_suggest_query" => Ok(db_tools::vox_db_suggest_query(state, args).await),
        "vox_clavis_doctor" => Ok(clavis_tools::clavis_doctor(state, args).await),

        "vox_db_research_session_upsert" => {
            Ok(codex_tools::codex_research_session_upsert(state, args).await)
        }
        "vox_db_conversation_version_append" => {
            Ok(codex_tools::codex_conversation_version_append(state, args).await)
        }
        "vox_db_conversation_edge_insert" => {
            Ok(codex_tools::codex_conversation_edge_insert(state, args).await)
        }
        "vox_db_topic_evolution_append" => {
            Ok(codex_tools::codex_topic_evolution_append(state, args).await)
        }
        "vox_db_research_metric_linked" => {
            Ok(codex_tools::codex_research_metric_linked(state, args).await)
        }
        "vox_db_trust_rollups" => Ok(trust_tools::trust_rollups_list(state, args).await),
        "vox_db_trust_summary" => Ok(trust_tools::trust_rollups_summary(state, args).await),
        "vox_db_trust_drift" => Ok(trust_tools::trust_observation_drift(state, args).await),
        "vox_db_trust_propagate" => Ok(trust_tools::trust_propagate(state, args).await),

        "vox_generate_code" => Ok(compiler_tools::generate_vox_code(state, args).await),
        #[cfg(feature = "oratio-rerank")]
        "vox_speech_to_code" => Ok(speech_pipeline_tools::speech_to_code(state, args).await?),
        "vox_list_models" => {
            Ok(crate::mcp_tools::models::list_models(state, serde_json::from_value(args)?).await)
        }
        "vox_suggest_model" => {
            Ok(crate::mcp_tools::models::suggest_model(state, serde_json::from_value(args)?).await)
        }
        "vox_set_model" => {
            Ok(crate::mcp_tools::models::set_model(state, serde_json::from_value(args)?).await)
        }
        "vox_set_active_model" => Ok(crate::mcp_tools::models::set_active_mcp_chat_model(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_get_active_model" => {
            Ok(crate::mcp_tools::models::get_active_mcp_chat_model(state).await)
        }
        "vox_build_crate" => Ok(compiler_tools::build_crate(
            state,
            args.get("crate_name").and_then(|v| v.as_str()),
        )
        .await),
        "vox_lint_crate" => Ok(compiler_tools::lint_crate(
            state,
            args.get("crate_name").and_then(|v| v.as_str()),
        )
        .await),
        "vox_coverage_report" => Ok(compiler_tools::coverage_report(
            state,
            args.get("crate_name").and_then(|v| v.as_str()),
        )
        .await),

        // Execution Budget
        "vox_exec_time_query" => Ok(exec_time_tools::exec_time_query(state, args).await),
        "vox_exec_time_record" => Ok(exec_time_tools::exec_time_record(state, args).await),

        // ── Chat & Inline AI ──────────────────────────────────────────────
        "vox_chat_message" => {
            Ok(chat_tools::chat_message(state, serde_json::from_value(args)?).await)
        }
        "vox_chat_history" => {
            Ok(chat_tools::chat_history(state, serde_json::from_value(args)?).await)
        }
        "vox_questioning_submit_answer" => Ok(questioning_tools::questioning_submit_answer(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_questioning_pending" => {
            Ok(questioning_tools::questioning_pending(state, serde_json::from_value(args)?).await)
        }
        "vox_questioning_sync_ssot" => Ok(questioning_tools::questioning_sync_ssot(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_inline_edit" => {
            Ok(chat_tools::inline_edit(state, serde_json::from_value(args)?).await)
        }
        "vox_apply_structured_edit" => Ok(compiler_tools::apply_structured_edit(state, args).await),
        "vox_plan" => Ok(chat_tools::plan_goal(state, serde_json::from_value(args)?).await),
        "vox_replan" => Ok(chat_tools::plan_replan(state, serde_json::from_value(args)?).await),
        "vox_plan_status" => {
            Ok(chat_tools::plan_status(state, serde_json::from_value(args)?).await)
        }
        "vox_plan_list_sessions" => {
            Ok(chat_tools::plan_list_sessions(state, serde_json::from_value(args)?).await)
        }
        "vox_plan_resume" => {
            Ok(chat_tools::plan_resume(state, serde_json::from_value(args)?).await)
        }
        "vox_ghost_text" => Ok(chat_tools::ghost_text(state, serde_json::from_value(args)?).await),

        "vox_schola_submit" => {
            Ok(training_tools::train_submit(state, serde_json::from_value(args)?).await)
        }

        #[cfg(feature = "news-publish")]
        "vox_news_test_syndicate" => {
            Ok(news_tools::vox_news_test_syndicate(state, serde_json::from_value(args)?).await)
        }

        #[cfg(feature = "news-publish")]
        "vox_news_draft_research" => {
            Ok(news_tools::vox_news_draft_research(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "news-publish")]
        "vox_news_approve" => {
            Ok(news_tools::vox_news_approve(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "news-publish")]
        "vox_news_approval_status" => {
            Ok(news_tools::vox_news_approval_status(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "news-publish")]
        "vox_news_simulate_publish_gate" => Ok(news_tools::vox_news_simulate_publish_gate(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_prepare" => Ok(scientia_tools::vox_scientia_publication_prepare(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_approve" => Ok(scientia_tools::vox_scientia_publication_approve(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_submit_local" => {
            Ok(scientia_tools::vox_scientia_publication_submit_local(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_status" => Ok(scientia_tools::vox_scientia_publication_status(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status_sync_all" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status_sync_all(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_remote_status_sync_batch" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status_sync_batch(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_arxiv_handoff_record" => Ok(
            scientia_tools::vox_scientia_publication_arxiv_handoff_record(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_staging_export" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_staging_export(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_due" => {
            Ok(scientia_tools::vox_scientia_publication_external_jobs_due(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_dead_letter" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_dead_letter(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_replay" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_replay(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_jobs_tick" => {
            Ok(scientia_tools::vox_scientia_publication_external_jobs_tick(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_external_pipeline_metrics" => Ok(
            scientia_tools::vox_scientia_publication_external_pipeline_metrics(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_scholarly_pipeline_run" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_pipeline_run(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_upsert" => {
            Ok(scientia_tools::vox_scientia_publication_media_upsert(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_list" => {
            Ok(scientia_tools::vox_scientia_publication_media_list(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_media_delete" => {
            Ok(scientia_tools::vox_scientia_publication_media_delete(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_route_simulate" => {
            Ok(scientia_tools::vox_scientia_publication_route_simulate(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_publish" => Ok(scientia_tools::vox_scientia_publication_publish(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_retry_failed" => {
            Ok(scientia_tools::vox_scientia_publication_retry_failed(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_preflight" => {
            Ok(scientia_tools::vox_scientia_publication_preflight(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_worthiness_evaluate" => Ok(scientia_tools::vox_scientia_worthiness_evaluate(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_scan" => {
            Ok(scientia_tools::vox_scientia_publication_discovery_scan(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_explain" => {
            Ok(scientia_tools::vox_scientia_publication_discovery_explain(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_discovery_refresh_evidence" => Ok(
            scientia_tools::vox_scientia_publication_discovery_refresh_evidence(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_novelty_fetch" => {
            Ok(scientia_tools::vox_scientia_publication_novelty_fetch(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_decision_explain" => {
            Ok(scientia_tools::vox_scientia_publication_decision_explain(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_publication_novelty_happy_path" => {
            Ok(scientia_tools::vox_scientia_publication_novelty_happy_path(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        #[cfg(feature = "news-publish")]
        "vox_scientia_assist_suggestions" => Ok(scientia_tools::vox_scientia_assist_suggestions(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        // Delegate others to existing modules
        "vox_my_files" => {
            Ok(crate::mcp_tools::affinity::my_files(state, serde_json::from_value(args)?).await)
        }
        "vox_claim_file" => {
            Ok(crate::mcp_tools::affinity::claim_file(state, serde_json::from_value(args)?).await)
        }
        "vox_transfer_file" => Ok(crate::mcp_tools::affinity::transfer_file(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        "vox_ask_agent" => {
            Ok(crate::mcp_tools::qa::ask_agent(state, serde_json::from_value(args)?).await)
        }
        "vox_answer_question" => {
            Ok(crate::mcp_tools::qa::answer_question(state, serde_json::from_value(args)?).await)
        }
        "vox_pending_questions" => {
            Ok(crate::mcp_tools::qa::pending_questions(state, serde_json::from_value(args)?).await)
        }
        "vox_broadcast" => {
            Ok(crate::mcp_tools::qa::broadcast(state, serde_json::from_value(args)?).await)
        }

        "vox_memory_store" => {
            Ok(crate::mcp_tools::memory::memory_store(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_recall" => {
            Ok(crate::mcp_tools::memory::memory_recall(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_search" => {
            Ok(crate::mcp_tools::memory::memory_search(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_log" => Ok(crate::mcp_tools::memory::memory_daily_log(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_memory_list_keys" => Ok(crate::mcp_tools::memory::memory_list_keys(state).await),
        "vox_knowledge_query" => Ok(crate::mcp_tools::memory::knowledge_query(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_memory_save_db" => Ok(crate::mcp_tools::memory::memory_save_db(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_memory_recall_db" => Ok(crate::mcp_tools::memory::memory_recall_db(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        "vox_compaction_status" => Ok(crate::mcp_tools::memory::compaction_status(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_session_create" => Ok(crate::mcp_tools::memory::session_create(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_session_list" => Ok(crate::mcp_tools::memory::session_list(state).await),
        "vox_session_reset" => {
            Ok(crate::mcp_tools::memory::session_reset(state, serde_json::from_value(args)?).await)
        }
        "vox_session_compact" => Ok(crate::mcp_tools::memory::session_compact(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_session_info" => {
            Ok(crate::mcp_tools::memory::session_info(state, serde_json::from_value(args)?).await)
        }
        "vox_session_cleanup" => Ok(crate::mcp_tools::memory::session_cleanup(state).await),

        "vox_preference_get" => Ok(crate::mcp_tools::memory::preference_get(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_preference_set" => Ok(crate::mcp_tools::memory::preference_set(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_preference_list" => Ok(crate::mcp_tools::memory::preference_list(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_learn_pattern" => {
            Ok(crate::mcp_tools::memory::learn_pattern(state, serde_json::from_value(args)?).await)
        }
        "vox_behavior_record" => Ok(crate::mcp_tools::memory::behavior_record(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_behavior_summary" => Ok(crate::mcp_tools::memory::behavior_summary(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        "vox_check_mood" => {
            Ok(crate::mcp_tools::gamify::check_mood(state, serde_json::from_value(args)?).await)
        }
        "vox_ludus_notifications_list" => Ok(crate::mcp_tools::gamify::ludus_notifications_list(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_progress_snapshot" => Ok(crate::mcp_tools::gamify::ludus_progress_snapshot(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_notification_ack" => Ok(crate::mcp_tools::gamify::ludus_notification_ack(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_notifications_ack_all" => {
            Ok(crate::mcp_tools::gamify::ludus_notifications_ack_all(state).await)
        }
        "vox_ludus_quest_list" => Ok(crate::mcp_tools::gamify::ludus_quest_list(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_shop_catalog" => Ok(crate::mcp_tools::gamify::ludus_shop_catalog(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_shop_buy" => Ok(crate::mcp_tools::gamify::ludus_shop_buy(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_collegium_join" => Ok(crate::mcp_tools::gamify::ludus_collegium_join(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_battle_start" => Ok(crate::mcp_tools::gamify::ludus_battle_start(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_ludus_battle_submit" => Ok(crate::mcp_tools::gamify::ludus_battle_submit(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_agent_status" => {
            Ok(crate::mcp_tools::gamify::agent_status(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_continue" => Ok(crate::mcp_tools::gamify::agent_continue(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_agent_assess" => {
            Ok(crate::mcp_tools::gamify::agent_assess(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_handoff" => {
            Ok(crate::mcp_tools::gamify::agent_handoff(state, serde_json::from_value(args)?).await)
        }

        "vox_queue_status" => Ok(crate::mcp_tools::dei_tools::queue_status(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_lock_status" => Ok(crate::mcp_tools::dei_tools::lock_status(state).await),
        "vox_budget_status" => Ok(crate::mcp_tools::dei_tools::budget_status(state).await),
        "vox_attention_summary" => Ok(crate::mcp_tools::dei_tools::attention_summary(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_attention_history" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let bm = state.orchestrator.budget_manager_handle();
            let events = crate::sync_lock::rw_read(&*bm).attention_events_snapshot(limit);
            Ok(crate::mcp_tools::params::ToolResult::ok(serde_json::to_value(&events)?).to_json())
        }
        "vox_attention_reset" => {
            let bm = state.orchestrator.budget_manager_handle();
            crate::sync_lock::rw_read(&*bm).reset_attention();
            // T-001: Also reset MCP-level Socrates attention tracking
            state.reset_all_questioning_attention();
            Ok(crate::mcp_tools::params::ToolResult::ok(serde_json::json!({
                "reset": true,
                "message": "Attention budget spend and Socrates focus zeroed process-wide."
            }))
            .to_json())
        }
        "vox_trust_override" => {
            let agent_id = args
                .get("agent_id")
                .and_then(|v| v.as_u64())
                .map(|id| crate::types::AgentId(id as _))
                .unwrap_or(crate::types::AgentId(0));
            let trust_score = args
                .get("trust_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            let bm = state.orchestrator.budget_manager_handle();
            crate::sync_lock::rw_read(&*bm).force_trust_score(agent_id, trust_score);
            Ok(crate::mcp_tools::params::ToolResult::ok(serde_json::json!({
                "agent_id": agent_id.0,
                "trust_score": trust_score,
                "message": "Trust score overridden."
            }))
            .to_json())
        }
        "vox_handoff_lineage" => Ok(crate::mcp_tools::dei_tools::handoff_lineage(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_cancel_task" => Ok(crate::mcp_tools::dei_tools::cancel_task(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_reorder_task" => Ok(crate::mcp_tools::dei_tools::reorder_task(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_drain_agent" => Ok(crate::mcp_tools::dei_tools::drain_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_cost_history" => Ok(crate::mcp_tools::dei_tools::cost_history(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_file_graph" => Ok(crate::mcp_tools::dei_tools::file_graph(state).await),
        "vox_config_get" => Ok(crate::mcp_tools::dei_tools::config_get(state).await),
        "vox_config_set" => Ok(crate::mcp_tools::dei_tools::config_set(state, args).await),
        "vox_map_agent_session" => Ok(crate::mcp_tools::dei_tools::map_agent_session(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_poll_events" => Ok(crate::mcp_tools::dei_tools::poll_events(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_heartbeat" => {
            Ok(crate::mcp_tools::dei_tools::heartbeat(state, serde_json::from_value(args)?).await)
        }
        "vox_record_cost" => Ok(crate::mcp_tools::dei_tools::record_cost(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_rebalance" => Ok(crate::mcp_tools::dei_tools::rebalance(state).await),
        "vox_agent_events" => Ok(crate::mcp_tools::dei_tools::agent_events(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        "vox_a2a_send" => {
            Ok(crate::mcp_tools::a2a_tools::a2a_send(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_inbox" => {
            Ok(crate::mcp_tools::a2a_tools::a2a_inbox(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_ack" => {
            Ok(crate::mcp_tools::a2a_tools::a2a_ack(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_broadcast" => Ok(crate::mcp_tools::a2a_tools::a2a_broadcast(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_a2a_history" => Ok(crate::mcp_tools::a2a_tools::a2a_history(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        "vox_skill_install" => {
            Ok(crate::mcp_tools::skills::skill_install(state, serde_json::from_value(args)?).await)
        }
        "vox_skill_uninstall" => Ok(crate::mcp_tools::skills::skill_uninstall(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_skill_list" => Ok(crate::mcp_tools::skills::skill_list(state)),
        "vox_skill_search" => Ok(crate::mcp_tools::skills::skill_search(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_info" => Ok(crate::mcp_tools::skills::skill_info(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_parse" => Ok(crate::mcp_tools::skills::skill_parse(
            serde_json::from_value(args)?,
        )),

        "vox_set_context" => Ok(crate::mcp_tools::mcp_context::set_context(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_get_context" => Ok(crate::mcp_tools::mcp_context::get_context(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_list_context" => Ok(crate::mcp_tools::mcp_context::list_context(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_context_budget" => Ok(crate::mcp_tools::mcp_context::context_budget(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_set_agent_budget" => Ok(crate::mcp_tools::mcp_context::set_agent_budget(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_emergency_stop" => Ok(crate::mcp_tools::mcp_context::emergency_stop(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_handoff_context" => Ok(crate::mcp_tools::mcp_context::handoff_context(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_transcribe" => Ok(oratio_tools::transcribe(state, args)?),
        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_listen" => Ok(oratio_tools::listen(state, args).await?),
        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_status" => Ok(oratio_tools::status()),

        "vox_populi_local_status" => Ok(populi_tools::mesh_local_status(args)?),

        "vox_browser_open" => {
            Ok(browser_tools::browser_open(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_close" => {
            Ok(browser_tools::browser_close(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_goto" => {
            Ok(browser_tools::browser_goto(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_click" => {
            Ok(browser_tools::browser_click(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_fill" => {
            Ok(browser_tools::browser_fill(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_wait_for" => {
            Ok(browser_tools::browser_wait_for(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_text" => {
            Ok(browser_tools::browser_text(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_html" => {
            Ok(browser_tools::browser_html(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_screenshot" => {
            Ok(browser_tools::browser_screenshot(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_extract" => {
            Ok(browser_tools::browser_extract(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_extract_json" => {
            Ok(browser_tools::browser_extract_json(state, serde_json::from_value(args)?).await)
        }
        "vox_browser_act" => {
            Ok(browser_tools::browser_act(state, serde_json::from_value(args)?).await)
        }

        "vox_benchmark_list" => {
            Ok(benchmark_tools::benchmark_list(state, serde_json::from_value(args)?).await)
        }
        "vox_benchmark_record" => {
            Ok(benchmark_tools::benchmark_record(state, serde_json::from_value(args)?).await)
        }
        "vox_toestub_findings_upsert" => {
            Ok(toestub_tools::toestub_findings_upsert(state, serde_json::from_value(args)?).await)
        }
        _ => {
            // Check skill macro tools
            let skills = state.skill_registry.list(None);
            if let Some(skill) = skills.iter().find(|s| s.tools.contains(&name.to_string())) {
                if let Some(db) = &state.db {
                    if let Ok(Some(entry)) = db.get_skill_manifest(&skill.id).await {
                        let msg = format!(
                            "This tool is an instructional macro from skill '{}'.\n\nPlease read these instructions and perform the requested actions yourself:\n\n{}",
                            skill.name, entry.skill_md
                        );
                        return Ok(ToolResult::ok(msg).to_json());
                    }
                }
            }
            Err(anyhow::anyhow!("Unknown tool: {}", name))
        }
    }
}

#[cfg(test)]
mod registry_dispatch_tests {
    use super::super::{TOOL_REGISTRY, handle_tool_call};
    use crate::mcp_tools::server_state::ServerState;
    use serde_json::json;
    use std::collections::HashSet;

    /// Subprocess / full-workspace tools — do not invoke from this guard (CI time + host deps).
    const SKIP_DISPATCH_PROBE: &[&str] = &[
        "vox_check_workspace",
        "vox_test_all",
        "vox_run_tests",
        "vox_build_crate",
        "vox_lint_crate",
        "vox_coverage_report",
        "vox_validate_file",
        "vox_validate_source",
        "vox_generate_code",
        "vox_project_init",
        "vox_oratio_transcribe",
        "vox_oratio_listen",
        "vox_oratio_status",
        "vox_speech_to_code",
        "vox_openclaw_list_remote",
        "vox_openclaw_search_remote",
        "vox_openclaw_import_skill",
        "vox_openclaw_discover",
        "vox_openclaw_health",
        "vox_openclaw_gateway_call",
        "vox_openclaw_subscriptions",
        "vox_openclaw_subscribe",
        "vox_openclaw_unsubscribe",
        "vox_openclaw_notify",
        "vox_browser_open",
        "vox_browser_close",
        "vox_browser_goto",
        "vox_browser_click",
        "vox_browser_fill",
        "vox_browser_wait_for",
        "vox_browser_text",
        "vox_browser_html",
        "vox_browser_screenshot",
        "vox_browser_extract",
        "vox_browser_extract_json",
        "vox_browser_act",
    ];

    #[tokio::test]
    async fn tool_registry_names_are_unique() {
        let mut seen = HashSet::new();
        for e in TOOL_REGISTRY {
            let name = e.name;
            assert!(seen.insert(name), "duplicate TOOL_REGISTRY name: {name}");
        }
    }

    #[test]
    fn yaml_registry_tools_have_dispatch_match_arms() {
        let src = include_str!("dispatch.rs");
        for e in TOOL_REGISTRY {
            let needle = format!("\"{}\" =>", e.name);
            assert!(
                src.contains(&needle),
                "TOOL_REGISTRY entry `{}` must have a `match` arm in dispatch.rs (SSOT: contracts/mcp/tool-registry.canonical.yaml)",
                e.name
            );
        }
    }

    #[tokio::test]
    async fn every_registry_tool_has_static_dispatch() {
        let state = ServerState::new_test().await;
        for e in TOOL_REGISTRY {
            let name = e.name;
            if SKIP_DISPATCH_PROBE.contains(&name) {
                continue;
            }
            let res = handle_tool_call(&state, name, json!({})).await;
            if let Err(e) = res {
                assert!(
                    !e.to_string().contains("Unknown tool"),
                    "missing dispatch for {name}: {e}"
                );
            }
        }
    }
}
