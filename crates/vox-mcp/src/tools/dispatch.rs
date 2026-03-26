//! `handle_tool_call` routing for all static MCP tools.

use crate::params::ToolResult;
use crate::server::ServerState;

use super::{
    benchmark_tools, chat_tools, codex_tools, compiler_tools, db_tools, git_tools,
    introspection_tools, news_tools, oratio_tools, populi_tools, repo_index, scientia_tools,
    task_tools, toestub_tools, tool_aliases, training_tools, vcs_tools,
};

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

    let result = handle_tool_call_inner(state, name_canonical, args.clone()).await;
    let duration_ms = start_time.elapsed().as_millis() as i64;

    // Ludus: canonical reward path when enabled; raw telemetry when gamification is off.
    if let Some(db) = &state.db {
        let aid = agent_id
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0u64);
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
            let agent_str = agent_id.unwrap_or("0");
            let _ = vox_ludus::db::insert_event(
                db,
                agent_str,
                "tool_call",
                Some(&payload.to_string()),
            )
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
        "vox_submit_task" => {
            Ok(task_tools::submit_task(state, serde_json::from_value(args)?).await)
        }
        "vox_task_status" => {
            Ok(task_tools::task_status(state, serde_json::from_value(args)?).await)
        }
        "vox_orchestrator_status" => crate::dei_tools::orchestrator_status(state).await,
        "vox_orchestrator_start" => Ok(crate::dei_tools::orchestrator_start(state).await),
        "vox_spawn_agent" => Ok(crate::dei_tools::spawn_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_retire_agent" => Ok(crate::dei_tools::retire_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_pause_agent" => Ok(crate::dei_tools::pause_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_resume_agent" => Ok(crate::dei_tools::resume_agent(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_complete_task" => {
            Ok(task_tools::complete_task(state, serde_json::from_value(args)?).await)
        }
        "vox_fail_task" => Ok(task_tools::fail_task(state, serde_json::from_value(args)?).await),
        "vox_check_file_owner" => Ok(crate::dei_tools::check_file_owner(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await),

        "vox_validate_file" => {
            Ok(compiler_tools::validate_file(serde_json::from_value(args)?).await)
        }
        "vox_run_tests" => {
            Ok(compiler_tools::run_tests(state, serde_json::from_value(args)?).await)
        }
        "vox_check_workspace" => Ok(compiler_tools::check_workspace(state).await),
        "vox_test_all" => Ok(compiler_tools::test_all(state).await),
        "vox_publish_message" => {
            Ok(task_tools::publish_message(state, serde_json::from_value(args)?).await)
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

        "vox_language_surface" => Ok(introspection_tools::language_surface().to_string()),
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
        "vox_vcs_status" => Ok(crate::dei_tools::vcs_status(state).await),

        "vox_db_schema" => Ok(db_tools::vox_db_schema(args)),
        "vox_db_relationships" => Ok(db_tools::vox_db_relationships(args)),
        "vox_db_data_flow" => Ok(db_tools::vox_db_data_flow(args)),
        "vox_db_sample_data" => Ok(db_tools::vox_db_sample_data(state, args).await),
        "vox_db_explain_query" => Ok(db_tools::vox_db_explain_query(state, args).await),
        "vox_db_suggest_query" => Ok(db_tools::vox_db_suggest_query(state, args).await),

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

        "vox_generate_code" => Ok(compiler_tools::generate_vox_code(state, args).await),
        "vox_list_models" => {
            Ok(crate::models::list_models(state, serde_json::from_value(args)?).await)
        }
        "vox_suggest_model" => {
            Ok(crate::models::suggest_model(state, serde_json::from_value(args)?).await)
        }
        "vox_set_model" => Ok(crate::models::set_model(state, serde_json::from_value(args)?).await),
        "vox_set_active_model" => Ok(crate::models::set_active_mcp_chat_model(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_get_active_model" => Ok(crate::models::get_active_mcp_chat_model(state).await),
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

        // ── Chat & Inline AI ──────────────────────────────────────────────
        "vox_chat_message" => {
            Ok(chat_tools::chat_message(state, serde_json::from_value(args)?).await)
        }
        "vox_chat_history" => {
            Ok(chat_tools::chat_history(state, serde_json::from_value(args)?).await)
        }
        "vox_inline_edit" => {
            Ok(chat_tools::inline_edit(state, serde_json::from_value(args)?).await)
        }
        "vox_plan" => Ok(chat_tools::plan_goal(state, serde_json::from_value(args)?).await),
        "vox_replan" => Ok(chat_tools::plan_replan(state, serde_json::from_value(args)?).await),
        "vox_plan_status" => {
            Ok(chat_tools::plan_status(state, serde_json::from_value(args)?).await)
        }

        "vox_schola_submit" => {
            Ok(training_tools::train_submit(state, serde_json::from_value(args)?).await)
        }

        "vox_news_test_syndicate" => {
            Ok(news_tools::vox_news_test_syndicate(state, serde_json::from_value(args)?).await)
        }

        "vox_news_draft_research" => {
            Ok(news_tools::vox_news_draft_research(state, serde_json::from_value(args)?).await)
        }
        "vox_news_approve" => {
            Ok(news_tools::vox_news_approve(state, serde_json::from_value(args)?).await)
        }
        "vox_news_approval_status" => {
            Ok(news_tools::vox_news_approval_status(state, serde_json::from_value(args)?).await)
        }
        "vox_news_simulate_publish_gate" => Ok(news_tools::vox_news_simulate_publish_gate(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_prepare" => Ok(scientia_tools::vox_scientia_publication_prepare(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_approve" => Ok(scientia_tools::vox_scientia_publication_approve(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_submit_local" => {
            Ok(scientia_tools::vox_scientia_publication_submit_local(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_scientia_publication_status" => Ok(scientia_tools::vox_scientia_publication_status(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_scholarly_remote_status" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_scholarly_remote_status_sync_all" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status_sync_all(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_scholarly_remote_status_sync_batch" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_remote_status_sync_batch(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_arxiv_handoff_record" => Ok(
            scientia_tools::vox_scientia_publication_arxiv_handoff_record(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_scholarly_staging_export" => Ok(
            scientia_tools::vox_scientia_publication_scholarly_staging_export(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_external_jobs_due" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_due(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_external_jobs_dead_letter" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_dead_letter(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_external_jobs_replay" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_replay(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_external_jobs_tick" => Ok(
            scientia_tools::vox_scientia_publication_external_jobs_tick(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_external_pipeline_metrics" => Ok(
            scientia_tools::vox_scientia_publication_external_pipeline_metrics(
                state,
                serde_json::from_value(args)?,
            )
            .await,
        ),
        "vox_scientia_publication_media_upsert" => {
            Ok(scientia_tools::vox_scientia_publication_media_upsert(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_scientia_publication_media_list" => {
            Ok(scientia_tools::vox_scientia_publication_media_list(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_scientia_publication_media_delete" => {
            Ok(scientia_tools::vox_scientia_publication_media_delete(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_scientia_publication_route_simulate" => {
            Ok(scientia_tools::vox_scientia_publication_route_simulate(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_scientia_publication_publish" => Ok(scientia_tools::vox_scientia_publication_publish(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_scientia_publication_retry_failed" => {
            Ok(scientia_tools::vox_scientia_publication_retry_failed(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_scientia_publication_preflight" => {
            Ok(scientia_tools::vox_scientia_publication_preflight(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_scientia_worthiness_evaluate" => Ok(scientia_tools::vox_scientia_worthiness_evaluate(
            state,
            serde_json::from_value(args)?,
        )
        .await),

        // Delegate others to existing modules
        "vox_my_files" => Ok(crate::affinity::my_files(state, serde_json::from_value(args)?).await),
        "vox_claim_file" => {
            Ok(crate::affinity::claim_file(state, serde_json::from_value(args)?).await)
        }
        "vox_transfer_file" => {
            Ok(crate::affinity::transfer_file(state, serde_json::from_value(args)?).await)
        }

        "vox_ask_agent" => Ok(crate::qa::ask_agent(state, serde_json::from_value(args)?).await),
        "vox_answer_question" => {
            Ok(crate::qa::answer_question(state, serde_json::from_value(args)?).await)
        }
        "vox_pending_questions" => {
            Ok(crate::qa::pending_questions(state, serde_json::from_value(args)?).await)
        }
        "vox_broadcast" => Ok(crate::qa::broadcast(state, serde_json::from_value(args)?).await),

        "vox_memory_store" => {
            Ok(crate::memory::memory_store(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_recall" => {
            Ok(crate::memory::memory_recall(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_search" => {
            Ok(crate::memory::memory_search(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_log" => {
            Ok(crate::memory::memory_daily_log(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_list_keys" => Ok(crate::memory::memory_list_keys(state).await),
        "vox_knowledge_query" => {
            Ok(crate::memory::knowledge_query(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_save_db" => {
            Ok(crate::memory::memory_save_db(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_recall_db" => {
            Ok(crate::memory::memory_recall_db(state, serde_json::from_value(args)?).await)
        }

        "vox_compaction_status" => {
            Ok(crate::memory::compaction_status(state, serde_json::from_value(args)?).await)
        }
        "vox_session_create" => {
            Ok(crate::memory::session_create(state, serde_json::from_value(args)?).await)
        }
        "vox_session_list" => Ok(crate::memory::session_list(state).await),
        "vox_session_reset" => {
            Ok(crate::memory::session_reset(state, serde_json::from_value(args)?).await)
        }
        "vox_session_compact" => {
            Ok(crate::memory::session_compact(state, serde_json::from_value(args)?).await)
        }
        "vox_session_info" => {
            Ok(crate::memory::session_info(state, serde_json::from_value(args)?).await)
        }
        "vox_session_cleanup" => Ok(crate::memory::session_cleanup(state).await),

        "vox_preference_get" => {
            Ok(crate::memory::preference_get(state, serde_json::from_value(args)?).await)
        }
        "vox_preference_set" => {
            Ok(crate::memory::preference_set(state, serde_json::from_value(args)?).await)
        }
        "vox_preference_list" => {
            Ok(crate::memory::preference_list(state, serde_json::from_value(args)?).await)
        }
        "vox_learn_pattern" => {
            Ok(crate::memory::learn_pattern(state, serde_json::from_value(args)?).await)
        }
        "vox_behavior_record" => {
            Ok(crate::memory::behavior_record(state, serde_json::from_value(args)?).await)
        }
        "vox_behavior_summary" => {
            Ok(crate::memory::behavior_summary(state, serde_json::from_value(args)?).await)
        }

        "vox_check_mood" => {
            Ok(crate::gamify::check_mood(state, serde_json::from_value(args)?).await)
        }
        "vox_ludus_notifications_list" => {
            Ok(crate::gamify::ludus_notifications_list(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_ludus_progress_snapshot" => {
            Ok(crate::gamify::ludus_progress_snapshot(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_ludus_notification_ack" => {
            Ok(crate::gamify::ludus_notification_ack(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_ludus_notifications_ack_all" => Ok(crate::gamify::ludus_notifications_ack_all(state).await),
        "vox_ludus_quest_list" => Ok(crate::gamify::ludus_quest_list(state, serde_json::from_value(args)?).await),
        "vox_ludus_shop_catalog" => Ok(crate::gamify::ludus_shop_catalog(state, serde_json::from_value(args)?).await),
        "vox_ludus_shop_buy" => Ok(crate::gamify::ludus_shop_buy(state, serde_json::from_value(args)?).await),
        "vox_ludus_collegium_join" => {
            Ok(crate::gamify::ludus_collegium_join(state, serde_json::from_value(args)?).await)
        }
        "vox_ludus_battle_start" => {
            Ok(crate::gamify::ludus_battle_start(state, serde_json::from_value(args)?).await)
        }
        "vox_ludus_battle_submit" => {
            Ok(crate::gamify::ludus_battle_submit(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_status" => {
            Ok(crate::gamify::agent_status(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_continue" => {
            Ok(crate::gamify::agent_continue(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_assess" => {
            Ok(crate::gamify::agent_assess(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_handoff" => {
            Ok(crate::gamify::agent_handoff(state, serde_json::from_value(args)?).await)
        }

        "vox_queue_status" => {
            Ok(crate::dei_tools::queue_status(state, serde_json::from_value(args)?).await)
        }
        "vox_lock_status" => Ok(crate::dei_tools::lock_status(state).await),
        "vox_budget_status" => Ok(crate::dei_tools::budget_status(state).await),
        "vox_cancel_task" => {
            Ok(crate::dei_tools::cancel_task(state, serde_json::from_value(args)?).await)
        }
        "vox_reorder_task" => {
            Ok(crate::dei_tools::reorder_task(state, serde_json::from_value(args)?).await)
        }
        "vox_drain_agent" => {
            Ok(crate::dei_tools::drain_agent(state, serde_json::from_value(args)?).await)
        }
        "vox_cost_history" => {
            Ok(crate::dei_tools::cost_history(state, serde_json::from_value(args)?).await)
        }
        "vox_file_graph" => Ok(crate::dei_tools::file_graph(state).await),
        "vox_config_get" => Ok(crate::dei_tools::config_get(state).await),
        "vox_config_set" => Ok(crate::dei_tools::config_set(state, args).await),
        "vox_map_agent_session" => {
            Ok(crate::dei_tools::map_agent_session(state, serde_json::from_value(args)?).await)
        }
        "vox_poll_events" => {
            Ok(crate::dei_tools::poll_events(state, serde_json::from_value(args)?).await)
        }
        "vox_heartbeat" => {
            Ok(crate::dei_tools::heartbeat(state, serde_json::from_value(args)?).await)
        }
        "vox_record_cost" => {
            Ok(crate::dei_tools::record_cost(state, serde_json::from_value(args)?).await)
        }
        "vox_rebalance" => Ok(crate::dei_tools::rebalance(state).await),
        "vox_agent_events" => {
            Ok(crate::dei_tools::agent_events(state, serde_json::from_value(args)?).await)
        }

        "vox_a2a_send" => Ok(crate::a2a::a2a_send(state, serde_json::from_value(args)?).await),
        "vox_a2a_inbox" => Ok(crate::a2a::a2a_inbox(state, serde_json::from_value(args)?).await),
        "vox_a2a_ack" => Ok(crate::a2a::a2a_ack(state, serde_json::from_value(args)?).await),
        "vox_a2a_broadcast" => {
            Ok(crate::a2a::a2a_broadcast(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_history" => {
            Ok(crate::a2a::a2a_history(state, serde_json::from_value(args)?).await)
        }

        "vox_skill_install" => {
            Ok(crate::skills::skill_install(state, serde_json::from_value(args)?).await)
        }
        "vox_skill_uninstall" => {
            Ok(crate::skills::skill_uninstall(state, serde_json::from_value(args)?).await)
        }
        "vox_skill_list" => Ok(crate::skills::skill_list(state)),
        "vox_skill_search" => Ok(crate::skills::skill_search(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_info" => Ok(crate::skills::skill_info(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_parse" => Ok(crate::skills::skill_parse(serde_json::from_value(args)?)),

        "vox_set_context" => {
            Ok(crate::context::set_context(state, serde_json::from_value(args)?).await)
        }
        "vox_get_context" => {
            Ok(crate::context::get_context(state, serde_json::from_value(args)?).await)
        }
        "vox_list_context" => {
            Ok(crate::context::list_context(state, serde_json::from_value(args)?).await)
        }
        "vox_context_budget" => {
            Ok(crate::context::context_budget(state, serde_json::from_value(args)?).await)
        }
        "vox_handoff_context" => {
            Ok(crate::context::handoff_context(state, serde_json::from_value(args)?).await)
        }

        "vox_oratio_transcribe" => Ok(oratio_tools::transcribe(state, args)?),
        "vox_oratio_listen" => Ok(oratio_tools::listen(state, args).await?),
        "vox_oratio_status" => Ok(oratio_tools::status()),

        "vox_populi_local_status" => Ok(populi_tools::mesh_local_status(args)?),

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
    use crate::server::ServerState;
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
        "vox_generate_code",
        "vox_oratio_transcribe",
    ];

    #[tokio::test]
    async fn tool_registry_names_are_unique() {
        let mut seen = HashSet::new();
        for (name, _) in TOOL_REGISTRY {
            assert!(seen.insert(*name), "duplicate TOOL_REGISTRY name: {name}");
        }
    }

    #[tokio::test]
    async fn every_registry_tool_has_static_dispatch() {
        let state = ServerState::new_test().await;
        for (name, _) in TOOL_REGISTRY {
            if SKIP_DISPATCH_PROBE.contains(name) {
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
