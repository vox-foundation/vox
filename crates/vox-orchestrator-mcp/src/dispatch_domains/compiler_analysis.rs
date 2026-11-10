#![allow(unused_imports)]
use crate::params::ToolResult;
use crate::server_state::ServerState;

#[cfg(feature = "heavy-browser")]
use crate::browser_tools;
#[cfg(feature = "gui-visual-review")]
use crate::visus_tools;
use crate::{
    agent_tools, benchmark_tools, chat_tools, code_validator, codex_tools, compiler_tools,
    db_tools, exec_time_tools, feedback_tools, git_tools, grammar_tools, introspection_tools,
    openclaw_tools, persistence_tools, populi_tools, project_init_tools, questioning_tools,
    rag_tools, receipt_tools, repo_catalog_tools, repo_index, secrets_tools, task_tools,
    toestub_tools, tool_aliases, training_tools, trust_tools, vcs_tools,
};
#[cfg(feature = "news-publish")]
use crate::{news_tools, scientia_tools};

#[cfg(feature = "oratio-rerank")]
use crate::{oratio_tools, speech_pipeline_tools};

pub fn handles(name: &str) -> bool {
    match name {
        "vox_validate_file" => true,
        "vox_check" => true,
        "vox_validate_mermaid" => true,
        "vox_validate_source" => true,
        "vox_run_tests" => true,
        "vox_check_workspace" => true,
        "vox_test_all" => true,
        "vox_agy_doctor" => true,
        "vox_agy_delegate" => true,
        "vox_agy_delegate_batch" => true,
        "vox_credentials_status" => true,
        "vox_agy_pipeline" => true,
        "vox_agy_review" => true,
        "vox_agy_ledger_digest" => true,
        "vox_search_status" => true,
        "vox_search_structural" => true,
        "vox_search_neighbors" => true,
        "vox_search_path" => true,
        "vox_search_callers" => true,
        "vox_search_callees" => true,
        "vox_search_compare" => true,
        "vox_search_atlas" => true,
        "vox_capability_explain" => true,
        "vox_capability_features" => true,
        "vox_search_history" => true,
        "vox_search_rebuild" => true,
        "vox_search_set_ttl" => true,
        "vox_language_surface" => true,
        "vox_capability_model_manifest" => true,
        "vox_compiler::ast_inspect" => true,
        "vox_pipeline_status" => true,
        "vox_decorator_registry" => true,
        "vox_builtin_registry" => true,
        "vox_workspace_modules" => true,
        "vox_export_grammar_ebnf" => true,
        "vox_db_schema" => true,
        "vox_db_relationships" => true,
        "vox_db_data_flow" => true,
        "vox_db_sample_data" => true,
        "vox_journey_canonical_steps" => true,
        "vox_db_explain_query" => true,
        "vox_db_suggest_query" => true,
        "vox_secrets_doctor" => true,
        "vox_db_research_session_upsert" => true,
        "vox_db_conversation_version_append" => true,
        "vox_db_conversation_edge_insert" => true,
        "vox_db_topic_evolution_append" => true,
        "vox_db_research_metric_linked" => true,
        "vox_db_trust_rollups" => true,
        "vox_db_trust_summary" => true,
        "vox_db_trust_drift" => true,
        "vox_db_trust_propagate" => true,
        "vox_generate_code" => true,
        #[cfg(feature = "oratio-rerank")]
        "vox_speech_to_code" => true,
        "vox_list_models" => true,
        "vox_suggest_model" => true,
        "vox_set_model" => true,
        "vox_set_active_model" => true,
        "vox_get_active_model" => true,
        "vox_build_crate" => true,
        "vox_lint_crate" => true,
        "vox_coverage_report" => true,
        "vox_apply_structured_edit" => true,
        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_transcribe" => true,
        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_listen" => true,
        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_status" => true,
        "vox_code_audit_findings_upsert" => true,
        "vox_consensus_run" => true,
        "vox_context_handoff" => true,
        "vox_checkpoint_capture" => true,
        "vox_checkpoint_list" => true,
        "vox_checkpoint_restore" => true,
        "vox_muse_pool_spawn" => true,
        "vox_muse_pool_adopt" => true,
        "vox_guest_share_mint" => true,
        "vox_guest_share_moderate" => true,
        "vox_session_import_external" => true,
        "vox_session_export_bundle" => true,
        "vox_session_import_bundle" => true,
        "vox_quarantine_set" => true,
        "vox_quarantine_list" => true,
        "vox_vcs_checkpoint_diff_summary" => true,
        "vox_file_diff_and_diagnostics" => true,
        "vox_stage_file" => true,
        "vox_revert_file" => true,
        "vox_list_mcp_servers" => true,
        "vox_list_mcp_tools" => true,
        "vox_call_mcp_lazy" => true,
        "vox_battery_status" => true,
        "vox_session_list_external" => true,
        "vox_guest_share_redeem" => true,
        "vox_guest_share_submit" => true,
        "vox_pr_review_guide" => true,
        _ => false,
    }
}

#[inline(never)]
pub async fn dispatch(
    state: &ServerState,
    name: &str,
    args: serde_json::Value,
) -> Result<String, anyhow::Error> {
    match name {
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
                let resolved = crate::workspace_path::resolve_existing_path_in_repository(state, p)
                    .unwrap_or_else(|_| std::path::PathBuf::from(p));
                let report = if resolved.extension().and_then(|s| s.to_str()) == Some("rs")
                    || resolved.extension().and_then(|s| s.to_str()) == Some("vox")
                {
                    state.observer.observe_rust_file(s_id, t_id, &resolved)
                } else {
                    state.observer.observe_file(s_id, t_id, &resolved)
                };
                state.orchestrator.event_bus().emit(
                    vox_orchestrator::AgentEventKind::ObservationRecorded {
                        agent_id: vox_orchestrator::types::AgentId(t_id.parse().unwrap_or(0)),
                        task_id: vox_orchestrator::types::TaskId(t_id.parse().unwrap_or(0)),
                        file_path: resolved.clone(),
                        lsp_error_count: report.lsp_error_count,
                        parse_rate: report.parse_rate,
                        construct_coverage: report.construct_coverage,
                        recommended_action: format!("{:?}", report.recommended_action),
                    },
                );
            }

            Ok(code_validator::validate_file(state, serde_json::from_value(args)?).await)
        }
        "vox_check" => Ok(code_validator::vox_check(state, serde_json::from_value(args)?).await),
        "vox_validate_mermaid" => {
            Ok(crate::chat_tools::chat::mermaid_repair::vox_validate_mermaid(args).await)
        }
        "vox_validate_source" => {
            Ok(code_validator::validate_source(state, serde_json::from_value(args)?).await)
        }
        "vox_run_tests" => {
            Ok(compiler_tools::run_tests(state, serde_json::from_value(args)?).await)
        }
        "vox_check_workspace" => Ok(compiler_tools::check_workspace(state).await),
        "vox_test_all" => Ok(compiler_tools::test_all(state).await),
        "vox_agy_doctor" => Ok(crate::agy_tools::vox_agy_doctor(state, args).await),
        "vox_agy_delegate" => Ok(crate::agy_tools::vox_agy_delegate(state, args).await),
        "vox_agy_delegate_batch" => Ok(crate::agy_tools::vox_agy_delegate_batch(state, args).await),
        "vox_credentials_status" => Ok(crate::agy_tools::vox_credentials_status(state, args).await),
        "vox_agy_pipeline" => Ok(crate::agy_pipeline::vox_agy_pipeline(state, args).await),
        "vox_agy_review" => Ok(crate::agy_pipeline::vox_agy_review(state, args).await),
        "vox_agy_ledger_digest" => {
            Ok(crate::agy_pipeline::vox_agy_ledger_digest(state, args).await)
        }
        "vox_search_status" => {
            Ok(crate::graph_tools::graphify_status(state, serde_json::from_value(args)?).await)
        }
        "vox_search_structural" => {
            Ok(crate::graph_tools::graphify_search(state, serde_json::from_value(args)?).await)
        }
        "vox_search_neighbors" => {
            Ok(crate::graph_tools::graphify_query(state, serde_json::from_value(args)?).await)
        }
        "vox_search_path" => {
            Ok(crate::graph_tools::graphify_path(state, serde_json::from_value(args)?).await)
        }
        "vox_search_callers" => {
            Ok(crate::graph_tools::graphify_callers(state, serde_json::from_value(args)?).await)
        }
        "vox_search_callees" => {
            Ok(crate::graph_tools::graphify_callees(state, serde_json::from_value(args)?).await)
        }
        "vox_search_compare" => {
            Ok(crate::graph_tools::graphify_compare(state, serde_json::from_value(args)?).await)
        }
        "vox_search_atlas" => {
            Ok(crate::graph_tools::graphify_atlas(state, serde_json::from_value(args)?).await)
        }
        "vox_capability_explain" => Ok(crate::capability_tools::explain(serde_json::from_value(
            args,
        )?)),
        "vox_capability_features" => Ok(crate::capability_tools::features(serde_json::from_value(
            args,
        )?)),
        "vox_search_history" => {
            Ok(crate::history_tools::history(state, serde_json::from_value(args)?).await)
        }
        "vox_search_rebuild" => {
            Ok(crate::graph_tools::graphify_rebuild(state, serde_json::from_value(args)?).await)
        }
        "vox_search_set_ttl" => {
            Ok(crate::graph_tools::graphify_set_ttl(state, serde_json::from_value(args)?).await)
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
        "vox_export_grammar_ebnf" => Ok(grammar_tools::export_grammar_ebnf(state).await),

        "vox_db_schema" => Ok(db_tools::vox_db_schema(args)),
        "vox_db_relationships" => Ok(db_tools::vox_db_relationships(args)),
        "vox_db_data_flow" => Ok(db_tools::vox_db_data_flow(args)),
        "vox_db_sample_data" => Ok(db_tools::vox_db_sample_data(state, args).await),
        "vox_journey_canonical_steps" => {
            Ok(db_tools::vox_journey_canonical_steps(state, args).await)
        }
        "vox_db_explain_query" => Ok(db_tools::vox_db_explain_query(state, args).await),
        "vox_db_suggest_query" => Ok(db_tools::vox_db_suggest_query(state, args).await),
        "vox_secrets_doctor" => Ok(secrets_tools::secrets_doctor(state, args).await),

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
        "vox_apply_structured_edit" => Ok(compiler_tools::apply_structured_edit(state, args).await),

        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_transcribe" => Ok(oratio_tools::transcribe(state, args)?),
        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_listen" => Ok(oratio_tools::listen(state, args).await?),
        #[cfg(feature = "oratio-rerank")]
        "vox_oratio_status" => Ok(oratio_tools::status()),
        "vox_code_audit_findings_upsert" => {
            Ok(toestub_tools::toestub_findings_upsert(state, serde_json::from_value(args)?).await)
        }
        // ── T3 Wave 3 subsystems (`T3-MCP-RPC-WIRE-01..03`) ──
        "vox_consensus_run" => Ok(crate::t3_wave3_tools::handle_consensus_run(state, args).await),
        "vox_context_handoff" => {
            Ok(crate::t3_wave3_tools::handle_context_handoff(state, args).await)
        }
        "vox_checkpoint_capture" => {
            Ok(crate::t3_wave3_tools::handle_checkpoint_capture(state, args).await)
        }
        "vox_checkpoint_list" => {
            Ok(crate::t3_wave3_tools::handle_checkpoint_list(state, args).await)
        }
        "vox_checkpoint_restore" => {
            Ok(crate::t3_wave3_tools::handle_checkpoint_restore(state, args).await)
        }
        "vox_muse_pool_spawn" => {
            Ok(crate::t3_wave3_tools::handle_muse_pool_spawn(state, args).await)
        }
        "vox_muse_pool_adopt" => {
            Ok(crate::t3_wave3_tools::handle_muse_pool_adopt(state, args).await)
        }
        "vox_guest_share_mint" => {
            Ok(crate::t3_wave3_tools::handle_guest_share_mint(state, args).await)
        }
        "vox_guest_share_moderate" => {
            Ok(crate::t3_wave3_tools::handle_guest_share_moderate(state, args).await)
        }
        "vox_session_import_external" => {
            Ok(crate::t3_wave3_tools::handle_session_import_external(state, args).await)
        }
        "vox_session_export_bundle" => {
            Ok(crate::t3_wave3_tools::handle_session_export_bundle(state, args).await)
        }
        "vox_session_import_bundle" => {
            Ok(crate::t3_wave3_tools::handle_session_import_bundle(state, args).await)
        }
        "vox_quarantine_set" => Ok(crate::t3_wave3_tools::handle_quarantine_set(state, args).await),
        "vox_quarantine_list" => {
            Ok(crate::t3_wave3_tools::handle_quarantine_list(state, args).await)
        }
        "vox_vcs_checkpoint_diff_summary" => {
            Ok(crate::t3_wave3_tools::handle_vcs_checkpoint_diff_summary(state, args).await)
        }
        "vox_file_diff_and_diagnostics" => {
            Ok(crate::t3_wave3_tools::handle_file_diff_and_diagnostics(state, args).await)
        }
        "vox_stage_file" => Ok(crate::t3_wave3_tools::handle_stage_file(state, args).await),
        "vox_revert_file" => Ok(crate::t3_wave3_tools::handle_revert_file(state, args).await),
        // ── T3 Wave 5 tools (`T3-PROMPT-LOOP-04`, `T3-STREAM-QUEUE-03`, `T3-TOOLS-HARDENING-03..04`) ──
        "vox_list_mcp_servers" => {
            Ok(crate::t3_wave3_tools::handle_list_mcp_servers(state, args).await)
        }
        "vox_list_mcp_tools" => Ok(crate::t3_wave3_tools::handle_list_mcp_tools(state, args).await),
        "vox_call_mcp_lazy" => Ok(crate::t3_wave3_tools::handle_call_mcp_lazy(state, args).await),
        "vox_battery_status" => Ok(crate::t3_wave3_tools::handle_battery_status(state, args).await),
        "vox_session_list_external" => {
            Ok(crate::t3_wave3_tools::handle_session_list_external(state, args).await)
        }
        "vox_guest_share_redeem" => {
            Ok(crate::t3_wave3_tools::handle_guest_share_redeem(state, args).await)
        }
        "vox_guest_share_submit" => {
            Ok(crate::t3_wave3_tools::handle_guest_share_submit(state, args).await)
        }
        "vox_pr_review_guide" => {
            Ok(crate::t3_wave3_tools::handle_pr_review_guide(state, args).await)
        }
        _ => Err(anyhow::anyhow!(
            "unhandled tool in compiler_analysis: {}",
            name
        )),
    }
}
