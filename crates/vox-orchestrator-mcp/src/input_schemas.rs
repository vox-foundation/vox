//! JSON Schema fragments for MCP tool `input_schema` (draft-07 subset).
//!
//! Keep shapes aligned with [`crate::params`], [`crate::memory`], [`crate::affinity`],
//! and [`super::chat_tools`] `Deserialize` structs. Unknown tools fall back to an empty map
//! (caller may treat as unconstrained JSON).
//!
//! ## Schemars-first vs hand-tuned
//!
//! Most strict object shapes use [`schemars::schema_for`] on the same Rust types handlers
//! deserialize. Remaining `parse_obj(...)` literals are **intentional overrides**: `Value`-parsed
//! tools (Oratio), `anyOf`/discriminated union chat payloads, VCS pass-through maps, and other
//! shapes that are awkward or misleading if derived verbatim.

use serde_json::{Map, Value};

#[path = "input_schemas/browser_and_gui.rs"]
mod browser_and_gui;
#[path = "input_schemas/chat_and_memory.rs"]
mod chat_and_memory;
#[path = "input_schemas/core_and_tasks.rs"]
mod core_and_tasks;
#[path = "input_schemas/mens_and_ops.rs"]
mod mens_and_ops;
#[path = "input_schemas/scientia_and_kb.rs"]
mod scientia_and_kb;
#[path = "input_schemas/vcs_and_workspace.rs"]
mod vcs_and_workspace;

/// Schema object for RMCP / MCP tool registration.
pub(super) fn tool_input_schema(name: &str) -> Map<String, Value> {
    let name = super::tool_aliases::canonical_tool_name(name);
    if let Some(m) = core_and_tasks::schema_for_tool(name)
        .or_else(|| vcs_and_workspace::schema_for_tool(name))
        .or_else(|| chat_and_memory::schema_for_tool(name))
        .or_else(|| scientia_and_kb::schema_for_tool(name))
        .or_else(|| mens_and_ops::schema_for_tool(name))
        .or_else(|| browser_and_gui::schema_for_tool(name))
    {
        return m;
    }

    #[cfg(any())]
    match name {
        "vox_oratio_transcribe" => {}
        "vox_oratio_listen" => {}
        "vox_oratio_status" => {}
        "vox_speech_to_code" => {}
        "vox_submit_task" => {}
        "vox_tool_search" => {}
        "vox_task_status" => {}
        "vox_cancel_task" => {}
        "vox_test_decision" => {}
        "vox_complete_task" => {}
        "vox_fail_task" => {}
        "vox_doubt_task" => {}
        "vox_verify_task_claims" => {}
        "vox_propose_skill" => {}
        "vox_ask_clarification" => {}
        "vox_resolve_feedback" => {}
        "vox_feedback_list" => {}
        "vox_publish_message" => {}
        "vox_reorder_task" => {}
        "vox_drain_agent" => {}
        "vox_spawn_agent" => {}
        "vox_retire_agent" => {}
        "vox_pause_agent" => {}
        "vox_resume_agent" => {}
        "vox_orchestrator_status" => {}
        "vox_orchestrator_persistence_outbox_lifecycle" => {}
        "vox_rebalance" => {}
        "vox_lock_status" => {}
        "vox_file_graph" => {}
        "vox_config_get" => {}
        "vox_openclaw_list_remote" => {}
        "vox_openclaw_discover" => {}
        "vox_openclaw_health" => {}
        "vox_openclaw_subscriptions" => {}
        "vox_agent_list_remote" => {}
        "vox_agent_subscriptions" => {}
        "vox_session_list" => {}
        "vox_session_cleanup" => {}
        "vox_memory_list_keys" => {}
        "vox_skill_list" => {}
        "vox_skill_discover" => {}
        "vox_test_all" => {}
        "vox_check_workspace" => {}
        "vox_get_active_model" => {}
        "vox_language_surface" => {}
        "vox_capability_model_manifest" => {}
        "vox_pipeline_status" => {}
        "vox_decorator_registry" => {}
        "vox_builtin_registry" => {}
        "vox_workspace_modules" => {}
        "vox_export_grammar_ebnf" => {}
        "vox_a2a_tasks" => {}
        "vox_secrets_doctor" => {}
        "vox_orchestrator_start" => {}
        "vox_orchestrator_persistence_outbox_queue" => {}
        "vox_openclaw_gateway_call" => {}
        "vox_agent_gateway_call" => {}
        "vox_openclaw_search_remote" => {}
        "vox_openclaw_import_skill" => {}
        "vox_openclaw_subscribe" => {}
        "vox_openclaw_unsubscribe" => {}
        "vox_agent_subscribe" => {}
        "vox_agent_unsubscribe" => {}
        "vox_openclaw_notify" => {}
        "vox_agent_notify" => {}
        "vox_exec_time_query" => {}
        "vox_exec_time_record" => {}
        "vox_ask_agent" => {}
        "vox_answer_question" => {}
        "vox_pending_questions" => {}
        "vox_broadcast" => {}
        "vox_a2a_send" => {}
        "vox_a2a_inbox" => {}
        "vox_a2a_ack" => {}
        "vox_a2a_broadcast" => {}
        "vox_a2a_history" => {}
        "vox_pending_approvals" => {}
        "vox_resolve_approval" => {}
        "vox_add_approval_allowlist_entry" => {}
        "vox_list_approval_allowlist" => {}
        "vox_check" => {}
        "vox_validate_file" => {}
        "vox_compiler::ast_inspect" => {}
        "vox_validate_source" => {}
        "vox_validate_mermaid" => {}
        "vox_run_tests" => {}
        "vox_build_crate" => {}
        "vox_lint_crate" => {}
        "vox_coverage_report" => {}
        "vox_check_file_owner" => {}
        "vox_my_files" => {}
        "vox_claim_file" => {}
        "vox_transfer_file" => {}
        "vox_git_log" => {}
        "vox_git_diff" => {}
        "vox_git_status" => {}
        "vox_git_blame" => {}
        "vox_repo_index_status" => {}
        "vox_repo_index_refresh" => {}
        "vox_visus_audit" => {}
        "vox_visus_baseline" => {}
        "vox_visual_rag_query" => {}
        "vox_repo_status" => {}
        "vox_gui_components" => {}
        "vox_gui_tokens" => {}
        "vox_gui_rules" => {}
        "vox_agy_doctor" => {}
        "vox_credentials_status" => {}
        "vox_agy_ledger_digest" => {}
        "vox_agy_delegate" => {}
        "vox_agy_delegate_batch" => {}
        "vox_agy_pipeline" => {}
        "vox_agy_review" => {}
        "vox_validate_vuv" => {}
        "vox_validate_diagram" => {}
        "vox_search_status" => {}
        "vox_search_structural" => {}
        "vox_search_atlas" => {}
        "vox_search_neighbors" => {}
        "vox_search_path" => {}
        "vox_search_callers" => {}
        "vox_search_callees" => {}
        "vox_search_compare" => {}
        "vox_capability_explain" => {}
        "vox_capability_features" => {}
        "vox_search_history" => {}
        "vox_search_rebuild" => {}
        "vox_search_set_ttl" => {}
        "vox_project_init" => {}
        "vox_repo_catalog_list" => {}
        "vox_repo_catalog_refresh" => {}
        "vox_repo_query_text" => {}
        "vox_repo_query_file" => {}
        "vox_repo_query_history" => {}
        "vox_conflict_diff" => {}
        "vox_snapshot_diff" => {}
        "vox_snapshot_restore" => {}
        "vox_undo" => {}
        "vox_redo" => {}
        "vox_resolve_conflict" => {}
        "vox_snapshot_list" => {}
        "vox_oplog" => {}
        "vox_conflicts" => {}
        "vox_workspace_create" => {}
        "vox_workspace_merge" => {}
        "vox_workspace_status" => {}
        "vox_change_create" => {}
        "vox_change_log" => {}
        "vox_vcs_status" => {}
        "vox_db_schema" => {}
        "vox_db_relationships" => {}
        "vox_db_data_flow" => {}
        "vox_db_sample_data" => {}
        "vox_db_explain_query" => {}
        "vox_db_suggest_query" => {}
        "vox_journey_canonical_steps" => {}
        "vox_db_research_session_upsert" => {}
        "vox_db_conversation_version_append" => {}
        "vox_db_conversation_edge_insert" => {}
        "vox_db_topic_evolution_append" => {}
        "vox_db_research_metric_linked" => {}
        "vox_db_trust_rollups" => {}
        "vox_db_trust_summary" => {}
        "vox_db_trust_drift" => {}
        "vox_db_trust_propagate" => {}
        "vox_generate_code" => {}
        "vox_set_context" => {}
        "vox_get_context" => {}
        "vox_list_context" => {}
        "vox_context_budget" => {}
        "vox_set_agent_budget" => {}
        "vox_emergency_stop" => {}
        "vox_handoff_context" => {}
        "vox_memory_store" => {}
        "vox_memory_recall" => {}
        "vox_memory_search" => {}
        "vox_semantic_fs_discover" => {}
        "vox_memory_log" => {}
        "vox_knowledge_query" => {}
        "vox_research_run" => {}
        "vox_research_start" => {}
        "vox_research_status" => {}
        "vox_research_get" => {}
        "vox_research_search" => {}
        "vox_session_create" => {}
        "vox_session_reset" => {}
        "vox_session_info" => {}
        "vox_compaction_status" => {}
        "vox_session_compact" => {}
        "vox_preference_get" => {}
        "vox_preference_set" => {}
        "vox_preference_list" => {}
        "vox_learn_pattern" => {}
        "vox_behavior_record" => {}
        "vox_behavior_summary" => {}
        "vox_memory_save_db" => {}
        "vox_memory_recall_db" => {}
        "vox_list_models" => {}
        "vox_suggest_model" => {}
        "vox_set_model" => {}
        "vox_set_active_model" => {}
        "vox_config_set" => {}
        "vox_map_agent_session" => {}
        "vox_heartbeat" => {}
        "vox_record_cost" => {}
        "vox_chat_message" => {}
        "vox_chat_history" => {}
        "vox_questioning_submit_answer" => {}
        "vox_questioning_pending" => {}
        "vox_questioning_sync_ssot" => {}
        "vox_inline_edit" => {}
        "vox_apply_structured_edit" => {}
        "vox_plan" => {}
        "vox_ghost_text" => {}
        "vox_replan" => {}
        "vox_plan_status" => {}
        "vox_plan_list_sessions" => {}
        "vox_plan_resume" => {}
        "vox_attention_summary" => {}
        "vox_attention_history" => {}
        "vox_attention_reset" => {}
        "vox_trust_override" => {}
        "vox_handoff_lineage" => {}
        "vox_benchmark_list" => {}
        "vox_benchmark_record" => {}
        "vox_code_audit_findings_upsert" => {}
        "vox_schola_submit" => {}
        "vox_populi_local_status" => {}
        "vox_kb_search" => {}
        "vox_kb_get" => {}
        "vox_kb_cite" => {}
        "vox_kb_status" => {}
        "vox_kb_reindex" => {}
        "vox_news_test_syndicate" => {}
        "vox_news_draft_research" => {}
        "vox_news_approve" => {}
        "vox_news_approval_status" => {}
        "vox_news_simulate_publish_gate" => {}
        "vox_scientia_publication_prepare" => {}
        "vox_scientia_publication_preflight" => {}
        "vox_scientia_worthiness_evaluate" => {}
        "vox_scientia_publication_approve" => {}
        "vox_scientia_publication_submit_local" => {}
        "vox_scientia_publication_status" => {}
        "vox_scientia_publication_scholarly_remote_status" => {}
        "vox_scientia_publication_scholarly_remote_status_sync_all" => {}
        "vox_scientia_publication_scholarly_remote_status_sync_batch" => {}
        "vox_scientia_publication_arxiv_handoff_record" => {}
        "vox_scientia_publication_scholarly_staging_export" => {}
        "vox_scientia_publication_external_jobs_due" => {}
        "vox_scientia_publication_external_jobs_dead_letter" => {}
        "vox_scientia_publication_external_jobs_replay" => {}
        "vox_scientia_publication_external_jobs_tick" => {}
        "vox_scientia_publication_scholarly_pipeline_run" => {}
        "vox_scientia_publication_external_pipeline_metrics" => {}
        "vox_scientia_publication_media_list" => {}
        "vox_scientia_publication_media_delete" => {}
        "vox_scientia_publication_media_upsert" => {}
        "vox_scientia_publication_route_simulate" => {}
        "vox_scientia_publication_publish" => {}
        "vox_scientia_publication_retry_failed" => {}
        "vox_scientia_publication_discovery_scan" => {}
        "vox_scientia_publication_discovery_explain" => {}
        "vox_scientia_publication_discovery_refresh_evidence" => {}
        "vox_scientia_publication_novelty_fetch" => {}
        "vox_scientia_publication_decision_explain" => {}
        "vox_scientia_publication_novelty_happy_path" => {}
        "vox_scientia_assist_suggestions" => {}
        "vox_check_mood" => {}
        "vox_agent_status" => {}
        "vox_agent_continue" => {}
        "vox_agent_assess" => {}
        "vox_agent_handoff" => {}
        "vox_gamify_notifications_list" => {}
        "vox_gamify_progress_snapshot" => {}
        "vox_gamify_notification_ack" => {}
        "vox_gamify_notifications_ack_all" => {}
        "vox_gamify_quest_list" => {}
        "vox_gamify_shop_catalog" => {}
        "vox_gamify_shop_buy" => {}
        "vox_gamify_collegium_join" => {}
        "vox_gamify_battle_start" => {}
        "vox_gamify_battle_submit" => {}
        "vox_queue_status" => {}
        "vox_budget_status" => {}
        "vox_agent_events" => {}
        "vox_cost_history" => {}
        "vox_poll_events" => {}
        "vox_skill_add" => {}
        "vox_skill_remove" => {}
        "vox_skill_uninstall" => {}
        "vox_skill_info" => {}
        "vox_skill_use" => {}
        "vox_skill_parse" => {}
        "vox_skill_run" => {}
        "vox_workspace_mcp_refresh" => {}
        "vox_skill_search" => {}
        "vox_skill_install" => {}
        "vox_mesh_nodes" => {}
        "vox_mesh_queue_stats" => {}
        "vox_mesh_dispatch" => {}
        "vox_plugin_list" => {}
        "vox_plugin_catalog" => {}
        "vox_plugin_info" => {}
        "vox_plugin_remove" => {}
        "vox_plugin_install" => {}
        "vox_models_preference_set" => {}
        "vox_gui_preference_get" => {}
        "vox_gui_preference_set" => {}
        "vox_chat_session_create" => {}
        "vox_chat_session_append" => {}
        "vox_chat_session_rename" => {}
        "vox_chat_session_archive" => {}
        "vox_kb_node_delete" => {}
        "vox_kb_ingest_text" => {}
        "vox_kb_ingest_url" => {}
        "vox_kb_import_research_session" => {}
        "vox_memory_reindex" => {}
        "vox_research_flag_misleading" => {}
        "vox_research_publish_doc" => {}
        "vox_research_engine_config_set" => {}
        "vox_research_engine_probe" => {}
        "vox_scientia_publication_claim_review" => {}
        "vox_scientia_publication_nanopublish" => {}
        "vox_scientia_publication_autofill" => {}
        "vox_scientia_discovery_ack" => {}
        "vox_hopper_submit" => {}
        "vox_hopper_reprioritize" => {}
        "vox_hopper_cancel" => {}
        "vox_hopper_mark_done" => {}
        "vox_plan_node_upsert" => {}
        "vox_plan_nodes_approve" => {}
        "vox_gui_run_start" => {}
        "vox_gui_run_finish" => {}
        "vox_harness_corpus_scan" => {}
        "vox_harness_fix_propose" => {}
        "vox_harness_fix_resolve" => {}
        "vox_harness_issue_decide" => {}
        "vox_gamify_profile" => {}
        "vox_gamify_due_actions" => {}
        "vox_gamify_record_event" => {}
        "vox_gamify_leaderboard" => {}
        "vox_gamify_companions" => {}
        "vox_gamify_discovery_record" => {}
        "vox_browser_open" => {}
        "vox_browser_open_ex" => {}
        "vox_browser_close" => {}
        "vox_browser_goto" => {}
        "vox_browser_click" => {}
        "vox_browser_text" => {}
        "vox_browser_fill" => {}
        "vox_browser_snapshot" => {}
        "vox_browser_click_ref" => {}
        "vox_browser_fill_ref" => {}
        "vox_browser_cookies_export" => {}
        "vox_browser_cookies_import" => {}
        "vox_browser_wait_for" => {}
        "vox_browser_html" => {}
        "vox_browser_screenshot" => {}
        "vox_browser_extract" => {}
        "vox_browser_extract_json" => {}
        "vox_browser_act" => {}
        "vox_browser_back" => {}
        "vox_browser_forward" => {}
        "vox_browser_reload" => {}
        "vox_browser_stop" => {}
        "vox_browser_page_info" => {}
        "vox_browser_screenshot_viewport" => {}
        "vox_browser_screencast_frame" => {}
        "vox_browser_click_xy" => {}
        "vox_browser_scroll" => {}
        "vox_browser_press" => {}
        "vox_browser_type" => {}
        "vox_browser_set_viewport" => {}
        "vox_browser_set_control_lock" => {}
        "vox_browser_list_pages" => {}
        "vox_consensus_run" => {}
        "vox_context_handoff" => {}
        "vox_checkpoint_capture" => {}
        "vox_checkpoint_list" => {}
        "vox_checkpoint_restore" => {}
        "vox_muse_pool_spawn" => {}
        "vox_muse_pool_adopt" => {}
        "vox_guest_share_mint" => {}
        "vox_guest_share_moderate" => {}
        "vox_session_import_external" => {}
        "vox_session_export_bundle" => {}
        "vox_session_import_bundle" => {}
        "vox_quarantine_set" => {}
        "vox_quarantine_list" => {}
        "vox_vcs_checkpoint_diff_summary" => {}
        "vox_file_diff_and_diagnostics" => {}
        "vox_stage_file" => {}
        "vox_revert_file" => {}
        "vox_chat_compact_up_to" => {}
        "vox_chat_edit_message_in_place" => {}
        "vox_chat_regenerate" => {}
        "vox_chat_stream_resume" => {}
        "vox_chat_stream_cancel" => {}
        "vox_chat_queue_steer" => {}
        "vox_list_mcp_servers" => {}
        "vox_list_mcp_tools" => {}
        "vox_call_mcp_lazy" => {}
        "vox_battery_status" => {}
        "vox_session_list_external" => {}
        "vox_guest_share_redeem" => {}
        "vox_guest_share_submit" => {}
        "vox_pr_review_guide" => {}
        _ => {}
    }

    Map::new()
}
#[cfg(test)]
mod tests {
    use super::tool_input_schema;
    use crate::TOOL_REGISTRY;
    use serde_json::Value;

    fn resolve_local_ref<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
        let ptr = reference.strip_prefix('#')?;
        root.pointer(ptr)
    }

    fn schema_has_concrete_shape(schema: &Value, root: &Value) -> bool {
        let Some(obj) = schema.as_object() else {
            return false;
        };
        if let Some(reference) = obj.get("$ref").and_then(Value::as_str) {
            return resolve_local_ref(root, reference).is_some();
        }
        obj.contains_key("type")
            || obj.contains_key("anyOf")
            || obj.contains_key("oneOf")
            || obj.contains_key("allOf")
    }

    fn schema_min_length_at_least_one_when_present(schema: &Value, root: &Value) -> bool {
        let Some(obj) = schema.as_object() else {
            return true;
        };
        if let Some(reference) = obj.get("$ref").and_then(Value::as_str) {
            return resolve_local_ref(root, reference)
                .map(|resolved| schema_min_length_at_least_one_when_present(resolved, root))
                .unwrap_or(true);
        }
        if let Some(min_len) = obj.get("minLength").and_then(Value::as_u64) {
            return min_len >= 1;
        }
        ["anyOf", "oneOf", "allOf"].iter().all(|k| {
            obj.get(*k)
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .all(|v| schema_min_length_at_least_one_when_present(v, root))
                })
                .unwrap_or(true)
        })
    }

    #[test]
    fn registry_tools_have_input_schema_coverage() {
        let mut missing = Vec::new();
        for e in TOOL_REGISTRY {
            let name = e.name;
            if !crate::registry::dispatchable_under_features(name) {
                continue;
            }
            if tool_input_schema(name).is_empty() {
                missing.push(name);
            }
        }
        assert!(
            missing.is_empty(),
            "TOOL_REGISTRY tools missing non-empty input_schema: {missing:?}"
        );
    }

    /// The research schemas set `additionalProperties:false`, so every field the params
    /// struct accepts must be advertised or strict clients cannot send it.
    #[test]
    fn research_run_start_schemas_advertise_every_param() {
        fn keys(v: &Value) -> std::collections::BTreeSet<String> {
            v["properties"]
                .as_object()
                .expect("properties")
                .keys()
                .cloned()
                .collect()
        }
        for (tool, derived) in [
            (
                "vox_research_run",
                serde_json::to_value(schemars::schema_for!(
                    crate::memory_tools::ResearchRunParams
                ))
                .unwrap(),
            ),
            (
                "vox_research_start",
                serde_json::to_value(schemars::schema_for!(
                    crate::memory_tools::ResearchStartParams
                ))
                .unwrap(),
            ),
        ] {
            let schema = Value::Object(tool_input_schema(tool));
            assert_eq!(keys(&schema), keys(&derived), "{tool}");
            assert_eq!(
                schema["properties"]["lane"]["enum"],
                serde_json::json!(["fast", "deep"]),
                "{tool}"
            );
        }
    }

    #[test]
    fn submit_task_schema_has_files_array() {
        let m = tool_input_schema("vox_submit_task");
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        assert!(props.contains_key("files"));
        assert!(props.contains_key("description"));
    }

    #[test]
    fn submit_task_schema_exposes_context_envelope_json() {
        let m = tool_input_schema("vox_submit_task");
        let root = Value::Object(m.clone());
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        assert!(
            props.contains_key("context_envelope_json"),
            "missing context_envelope_json field"
        );
        let context_prop = props
            .get("context_envelope_json")
            .expect("context_envelope_json");
        assert!(
            schema_has_concrete_shape(context_prop, &root),
            "context_envelope_json must expose a concrete schema shape or resolvable $ref"
        );
        assert!(
            schema_min_length_at_least_one_when_present(context_prop, &root),
            "if minLength is present for context_envelope_json, it must be >= 1"
        );
        let required = m
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required array");
        assert!(
            !required
                .iter()
                .any(|v| v.as_str() == Some("context_envelope_json")),
            "context_envelope_json should be optional"
        );
        assert_eq!(
            m.get("additionalProperties").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn submit_task_schema_exposes_harness_spec_json() {
        let m = tool_input_schema("vox_submit_task");
        let root = Value::Object(m.clone());
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        assert!(
            props.contains_key("harness_spec_json"),
            "missing harness_spec_json field"
        );
        let harness_prop = props.get("harness_spec_json").expect("harness_spec_json");
        assert!(
            schema_has_concrete_shape(harness_prop, &root),
            "harness_spec_json must expose a concrete schema shape or resolvable $ref"
        );
        assert!(
            schema_min_length_at_least_one_when_present(harness_prop, &root),
            "if minLength is present for harness_spec_json, it must be >= 1"
        );
    }

    #[test]
    fn map_session_deprecated_aliases_match_canonical_schema() {
        let canonical = tool_input_schema("vox_map_agent_session");
        assert!(!canonical.is_empty());
        assert_eq!(tool_input_schema("vox_map_opencode_session"), canonical);
        assert_eq!(tool_input_schema("vox_map_vscode_session"), canonical);
    }

    #[test]
    fn config_deprecated_aliases_match_canonical_schema() {
        let canonical = tool_input_schema("vox_config_get");
        assert!(!canonical.is_empty());
        assert_eq!(tool_input_schema("vox_get_config"), canonical);
        let set_canon = tool_input_schema("vox_config_set");
        assert!(!set_canon.is_empty());
        assert_eq!(tool_input_schema("vox_set_config"), set_canon);
    }
    #[test]
    fn chat_message_schema_requires_prompt_or_message() {
        let m = tool_input_schema("vox_chat_message");
        assert!(
            m.get("anyOf").is_some(),
            "expected anyOf for prompt|message"
        );
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        let p = props.get("prompt").and_then(|v| v.as_object()).unwrap();
        assert_eq!(p.get("minLength").and_then(|x| x.as_u64()), Some(1));
    }

    #[test]
    fn submit_task_description_has_length_bounds() {
        let m = tool_input_schema("vox_submit_task");
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        let d = props
            .get("description")
            .and_then(|v| v.as_object())
            .unwrap();
        assert_eq!(d.get("minLength").and_then(|x| x.as_u64()), Some(1));
        assert_eq!(d.get("maxLength").and_then(|x| x.as_u64()), Some(131072));
    }

    #[test]
    fn submit_task_capabilities_schema_covers_extended_hints() {
        let m = tool_input_schema("vox_submit_task");
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        let caps = props
            .get("capabilities")
            .and_then(|v| v.as_object())
            .unwrap();
        let cap_props = caps.get("properties").and_then(|p| p.as_object()).unwrap();
        for key in [
            "gpu_cuda",
            "gpu_metal",
            "min_vram_mb",
            "cpu_cores",
            "arch",
            "hostname",
            "labels",
            "min_cpu_cores",
            "prefer_gpu_compute",
        ] {
            assert!(
                cap_props.contains_key(key),
                "capabilities.properties missing {key:?}"
            );
        }
    }

    #[test]
    fn map_session_session_id_has_bounds() {
        let m = tool_input_schema("vox_map_agent_session");
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        let s = props.get("session_id").and_then(|v| v.as_object()).unwrap();
        assert_eq!(s.get("minLength").and_then(|x| x.as_u64()), Some(1));
        assert_eq!(s.get("maxLength").and_then(|x| x.as_u64()), Some(2048));
    }

    #[test]
    fn generate_code_schema_matches_implemented_args() {
        let m = tool_input_schema("vox_generate_code");
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        for key in [
            "prompt",
            "validate",
            "max_retries",
            "session_id",
            "output_surface_mode",
        ] {
            assert!(props.contains_key(key), "missing property {key}");
        }
        assert_eq!(
            m.get("additionalProperties").and_then(|x| x.as_bool()),
            Some(false)
        );
        let req = m
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required array");
        assert!(
            req.iter().any(|x| x.as_str() == Some("prompt")),
            "prompt must be required"
        );
    }

    #[test]
    fn agent_handoff_schema_exposes_context_envelope_json() {
        let m = tool_input_schema("vox_agent_handoff");
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        for key in [
            "from_agent_id",
            "to_agent_id",
            "plan_summary",
            "unresolved_objectives",
            "verification_criteria",
            "context_envelope_json",
        ] {
            assert!(props.contains_key(key), "missing property {key}");
        }
        let context_prop = props
            .get("context_envelope_json")
            .and_then(|v| v.as_object())
            .expect("context_envelope_json object");
        assert_eq!(
            context_prop.get("type").and_then(|v| v.as_str()),
            Some("string")
        );
        assert_eq!(
            context_prop.get("minLength").and_then(|v| v.as_u64()),
            Some(1)
        );
        let required = m
            .get("required")
            .and_then(|r| r.as_array())
            .expect("required array");
        assert!(required.iter().any(|v| v.as_str() == Some("from_agent_id")));
        assert!(required.iter().any(|v| v.as_str() == Some("to_agent_id")));
        assert!(required.iter().any(|v| v.as_str() == Some("plan_summary")));
        assert!(
            !required
                .iter()
                .any(|v| v.as_str() == Some("context_envelope_json"))
        );
        assert_eq!(
            m.get("additionalProperties").and_then(|v| v.as_bool()),
            Some(false)
        );
    }
    #[test]
    fn all_parse_obj_schemas_are_valid_jsonschema() {
        // Draft-07 meta-schema
        let meta: serde_json::Value = serde_json::from_str(include_str!(
            "../../../contracts/capability/json-schema-draft-07-meta.json"
        ))
        .expect("meta schema");
        let compiled = jsonschema::validator_for(&meta).expect("compile meta-schema");
        for entry in TOOL_REGISTRY {
            // Check only parse_obj generated tools (these are the ones missing structure derivations).
            // (Testing all tools ensures full schema validity)
            let schema = serde_json::Value::Object(tool_input_schema(entry.name));
            if schema.as_object().map(|m| m.is_empty()).unwrap_or(true) {
                continue;
            }
            if let Err(error) = compiled.validate(&schema) {
                panic!("Tool '{}' has invalid JSON Schema: {}", entry.name, error);
            }
        }
    }

    #[test]
    fn vox_plan_schema_defines_model_override() {
        let schema = tool_input_schema("vox_plan");
        let props = schema
            .get("properties")
            .and_then(|v| v.as_object())
            .expect("properties");
        assert!(props.contains_key("model_override"));
        let mo = props.get("model_override").unwrap();
        assert_eq!(mo["type"], "string");
        assert_eq!(mo["maxLength"], 256);
    }
}
