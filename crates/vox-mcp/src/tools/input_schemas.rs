//! JSON Schema fragments for MCP tool `input_schema` (draft-07 subset).
//!
//! Keep shapes aligned with [`crate::params`], [`crate::memory`], [`crate::affinity`],
//! and [`super::chat_tools`] `Deserialize` structs. Unknown tools fall back to an empty map
//! (caller may treat as unconstrained JSON).

use schemars::schema_for;
use serde_json::{Map, Value, json};

fn parse_obj(s: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(s)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

/// `vox_submit_task` input schema; nested `capabilities` matches [`vox_orchestrator::TaskCapabilityHints`].
fn vox_submit_task_input_schema() -> Map<String, Value> {
    let caps = serde_json::to_value(schema_for!(vox_orchestrator::TaskCapabilityHints))
        .expect("TaskCapabilityHints JSON Schema (json-schema feature on vox-orchestrator)");

    let mut properties = Map::new();
    properties.insert(
        "description".into(),
        json!({
            "type": "string",
            "minLength": 1,
            "maxLength": 131072,
            "description": "Natural-language task description"
        }),
    );
    properties.insert(
        "files".into(),
        json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "minLength": 1, "maxLength": 4096 },
                    "access": { "type": "string", "enum": ["read", "write"] }
                },
                "required": ["path", "access"]
            }
        }),
    );
    properties.insert(
        "priority".into(),
        json!({
            "type": "string",
            "enum": ["urgent", "normal", "background"]
        }),
    );
    properties.insert(
        "agent_name".into(),
        json!({ "type": "string", "maxLength": 256 }),
    );
    properties.insert("capabilities".into(), caps);

    let mut root = Map::new();
    root.insert("type".into(), json!("object"));
    root.insert("properties".into(), Value::Object(properties));
    root.insert("required".into(), json!(["description", "files"]));
    root.insert("additionalProperties".into(), json!(false));
    root
}

/// Schema object for RMCP / MCP tool registration.
pub(super) fn tool_input_schema(name: &str) -> Map<String, Value> {
    let name = super::tool_aliases::canonical_tool_name(name);
    match name {
        // ── Oratio (already strict) ─────────────────────────────────────────
        "vox_oratio_transcribe" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string","description":"Workspace-relative or absolute path to an audio or transcript file"},"language_hint":{"type":"string"},"profile":{"type":"string","enum":["conservative","balanced","aggressive"]},"debug_parser_payload":{"type":"boolean"}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_oratio_listen" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"},"session_id":{"type":"string"},"timeout_ms":{"type":"integer","minimum":1},"max_duration_ms":{"type":"integer","minimum":1},"inference_deadline_ms":{"type":"integer","minimum":1},"heartbeat_ms":{"type":"integer","minimum":1},"language_hint":{"type":"string"},"profile":{"type":"string","enum":["conservative","balanced","aggressive"]},"route_mode":{"type":"string","enum":["none","tool","chat","orchestrator"]},"debug_parser_payload":{"type":"boolean"},"emit_asr_refine_path":{"type":"string"},"llm_refinement":{"type":"boolean"},"llm_min_det_confidence":{"type":"number"},"llm_max_output_tokens":{"type":"integer","minimum":1}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_oratio_status" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),

        // ── Tasks & bulletin ─────────────────────────────────────────────────
        "vox_submit_task" => vox_submit_task_input_schema(),
        "vox_task_status" | "vox_complete_task" | "vox_cancel_task" => parse_obj(
            r#"{"type":"object","properties":{"task_id":{"type":"integer","minimum":0}},"required":["task_id"],"additionalProperties":false}"#,
        ),
        "vox_fail_task" => parse_obj(
            r#"{"type":"object","properties":{"task_id":{"type":"integer","minimum":0},"reason":{"type":"string"}},"required":["task_id","reason"],"additionalProperties":false}"#,
        ),
        "vox_publish_message" => parse_obj(
            r#"{"type":"object","properties":{"message":{"type":"string"}},"required":["message"],"additionalProperties":false}"#,
        ),
        "vox_reorder_task" => parse_obj(
            r#"{"type":"object","properties":{"task_id":{"type":"integer","minimum":0},"priority":{"type":"string","enum":["urgent","normal","background"]}},"required":["task_id","priority"],"additionalProperties":false}"#,
        ),
        "vox_drain_agent" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0}},"required":["agent_id"],"additionalProperties":false}"#,
        ),

        // ── Orchestrator status / no-arg tools ───────────────────────────────
        "vox_orchestrator_status"
        | "vox_rebalance"
        | "vox_lock_status"
        | "vox_file_graph"
        | "vox_config_get"
        | "vox_session_list"
        | "vox_session_cleanup"
        | "vox_memory_list_keys"
        | "vox_skill_list"
        | "vox_test_all"
        | "vox_check_workspace"
        | "vox_get_active_model"
        | "vox_language_surface"
        | "vox_pipeline_status"
        | "vox_decorator_registry"
        | "vox_builtin_registry"
        | "vox_workspace_modules"
        | "vox_a2a_tasks" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),

        // Handler ignores args today; keep the schema strict so clients send `{}` only.
        "vox_orchestrator_start" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),

        // ── Compiler / workspace ─────────────────────────────────────────────
        "vox_validate_file" | "vox_compiler::ast_inspect" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string","description":"Path to a .vox file"}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_run_tests" => parse_obj(
            r#"{"type":"object","properties":{"crate_name":{"type":"string","description":"Cargo package name (-p)"},"test_filter":{"type":"string","description":"Optional substring after --"}},"required":["crate_name"],"additionalProperties":false}"#,
        ),
        "vox_build_crate" | "vox_lint_crate" | "vox_coverage_report" => parse_obj(
            r#"{"type":"object","properties":{"crate_name":{"type":"string","description":"Cargo package name or omit for workspace"}},"additionalProperties":false}"#,
        ),

        // ── File affinity ────────────────────────────────────────────────────
        "vox_check_file_owner" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_my_files" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0}},"required":["agent_id"],"additionalProperties":false}"#,
        ),
        "vox_claim_file" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0},"path":{"type":"string"}},"required":["agent_id","path"],"additionalProperties":false}"#,
        ),
        "vox_transfer_file" => parse_obj(
            r#"{"type":"object","properties":{"from_agent":{"type":"integer","minimum":0},"to_agent":{"type":"integer","minimum":0},"path":{"type":"string"}},"required":["from_agent","to_agent","path"],"additionalProperties":false}"#,
        ),

        // ── Context ──────────────────────────────────────────────────────────
        "vox_set_context" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0},"key":{"type":"string"},"value":{"type":"string"},"ttl_seconds":{"type":"integer","minimum":0}},"required":["agent_id","key","value"],"additionalProperties":false}"#,
        ),
        "vox_get_context" => parse_obj(
            r#"{"type":"object","properties":{"key":{"type":"string","minLength":1}},"required":["key"],"additionalProperties":false}"#,
        ),
        "vox_list_context" => parse_obj(
            r#"{"type":"object","properties":{"prefix":{"type":"string"}},"required":["prefix"],"additionalProperties":false}"#,
        ),
        "vox_context_budget" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0}},"required":["agent_id"],"additionalProperties":false}"#,
        ),
        "vox_handoff_context" => parse_obj(
            r#"{"type":"object","properties":{"from_agent":{"type":"integer","minimum":0},"to_agent":{"type":"integer","minimum":0}},"required":["from_agent","to_agent"],"additionalProperties":false}"#,
        ),

        // ── Gamify ───────────────────────────────────────────────────────────
        "vox_check_mood" | "vox_agent_status" | "vox_agent_continue" | "vox_agent_assess"
        | "vox_agent_handoff" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"Pass agent_id and other fields per orchestrator tool docs."}"#,
        ),

        "vox_queue_status" | "vox_budget_status" | "vox_agent_events" | "vox_cost_history"
        | "vox_poll_events" => parse_obj(r#"{"type":"object","additionalProperties":true}"#),

        // ── Memory (MEMORY.md / search) ─────────────────────────────────────
        "vox_memory_store" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0},"key":{"type":"string"},"value":{"type":"string"},"relations":{"type":"array","items":{"type":"string"}},"media_url":{"type":"string"},"media_type":{"type":"string"}},"required":["agent_id","key","value"],"additionalProperties":false}"#,
        ),
        "vox_memory_recall" => parse_obj(
            r#"{"type":"object","properties":{"key":{"type":"string"}},"required":["key"],"additionalProperties":false}"#,
        ),
        "vox_memory_search" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}"#,
        ),
        "vox_memory_log" => parse_obj(
            r#"{"type":"object","properties":{"entry":{"type":"string"}},"required":["entry"],"additionalProperties":false}"#,
        ),
        "vox_knowledge_query" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer"}},"required":["query"],"additionalProperties":false}"#,
        ),

        // ── Sessions & compaction ─────────────────────────────────────────────
        "vox_session_create" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0}},"required":["agent_id"],"additionalProperties":false}"#,
        ),
        "vox_session_reset" | "vox_session_info" | "vox_compaction_status" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string"}},"required":["session_id"],"additionalProperties":false}"#,
        ),
        "vox_session_compact" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string"},"summary":{"type":"string"}},"required":["session_id","summary"],"additionalProperties":false}"#,
        ),

        // ── Preferences & behavior ──────────────────────────────────────────
        "vox_preference_get" => parse_obj(
            r#"{"type":"object","properties":{"key":{"type":"string"}},"required":["key"],"additionalProperties":false}"#,
        ),
        "vox_preference_set" => parse_obj(
            r#"{"type":"object","properties":{"key":{"type":"string"},"value":{"type":"string"}},"required":["key","value"],"additionalProperties":false}"#,
        ),
        "vox_preference_list" => parse_obj(
            r#"{"type":"object","properties":{"prefix":{"type":"string"}},"additionalProperties":false}"#,
        ),
        "vox_learn_pattern" | "vox_behavior_record" | "vox_behavior_summary" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }

        // ── Codex memory DB ─────────────────────────────────────────────────
        "vox_memory_save_db" | "vox_memory_recall_db" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"Typed agent_memory payloads; see memory module handlers."}"#,
        ),

        // ── Models ──────────────────────────────────────────────────────────
        "vox_list_models" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_suggest_model" => parse_obj(
            r#"{"type":"object","properties":{"task_category":{"type":"string","description":"e.g. codegen, review, testing"}},"required":["task_category"],"additionalProperties":false}"#,
        ),
        "vox_set_model" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0},"model_id":{"type":"string"}},"required":["agent_id","model_id"],"additionalProperties":false}"#,
        ),
        "vox_set_active_model" => parse_obj(
            r#"{"type":"object","properties":{"model_id":{"type":"string","description":"Empty string clears sticky override"}},"required":["model_id"],"additionalProperties":false}"#,
        ),

        // ── Git & repo index ─────────────────────────────────────────────────
        "vox_git_log" => parse_obj(
            r#"{"type":"object","properties":{"max_commits":{"type":"integer"}},"additionalProperties":false}"#,
        ),
        "vox_git_diff" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"}},"additionalProperties":false}"#,
        ),
        "vox_git_status" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_git_blame" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_repo_index_status" | "vox_repo_index_refresh" => {
            parse_obj(r#"{"type":"object","additionalProperties":false}"#)
        }
        "vox_conflict_diff" => parse_obj(
            r#"{"type":"object","properties":{"conflict_id":{"description":"Conflict id as number or C-XXXXXX string","oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^C-[0-9]{6}$"}]}},"required":["conflict_id"],"additionalProperties":false}"#,
        ),
        "vox_snapshot_diff" => parse_obj(
            r#"{"type":"object","properties":{"before":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^S-[0-9]{6}$"}]},"after":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^S-[0-9]{6}$"}]}},"additionalProperties":false}"#,
        ),
        "vox_snapshot_restore" => parse_obj(
            r#"{"type":"object","properties":{"snapshot_id":{"type":"string","pattern":"^S-[0-9]{6}$"}},"required":["snapshot_id"],"additionalProperties":false}"#,
        ),
        "vox_undo" => parse_obj(
            r#"{"type":"object","properties":{"operation_id":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^OP-[0-9]{6}$"}]}},"required":["operation_id"],"additionalProperties":false}"#,
        ),
        "vox_redo" => parse_obj(
            r#"{"type":"object","properties":{"operation_id":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^OP-[0-9]{6}$"}]}},"required":["operation_id"],"additionalProperties":false}"#,
        ),
        "vox_resolve_conflict" => parse_obj(
            r#"{"type":"object","properties":{"conflict_id":{"oneOf":[{"type":"integer","minimum":0},{"type":"string","pattern":"^C-[0-9]{6}$"}]},"strategy":{"type":"string","enum":["take_left","take_right","defer"]},"defer_to_agent":{"type":"integer","minimum":0}},"required":["conflict_id"],"additionalProperties":false}"#,
        ),

        // ── VCS (pass-through args; shape varies by tool) ───────────────────
        "vox_snapshot_list"
        | "vox_oplog"
        | "vox_conflicts"
        | "vox_workspace_create"
        | "vox_workspace_merge"
        | "vox_workspace_status"
        | "vox_change_create"
        | "vox_change_log"
        | "vox_vcs_status" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"Tool-specific JSON; see vcs_tools handlers."}"#,
        ),

        // ── DB digest tools ──────────────────────────────────────────────────
        "vox_db_schema" | "vox_db_relationships" | "vox_db_data_flow" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }
        "vox_db_sample_data" | "vox_db_explain_query" | "vox_db_suggest_query" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }

        "vox_db_research_session_upsert" => parse_obj(
            r#"{"type":"object","properties":{"session_key":{"type":"string"},"title":{"type":"string"},"status":{"type":"string"},"repository_id":{"type":"string"},"config_json":{"type":"object"},"summary_json":{"type":"object"}},"required":["session_key"],"additionalProperties":false}"#,
        ),
        "vox_db_conversation_version_append" => parse_obj(
            r#"{"type":"object","properties":{"conversation_id":{"type":"integer"},"version_index":{"type":"integer"},"label":{"type":"string"},"snapshot_json":{"type":"object"}},"required":["conversation_id","version_index"],"additionalProperties":false}"#,
        ),
        "vox_db_conversation_edge_insert" => parse_obj(
            r#"{"type":"object","properties":{"from_conversation_id":{"type":"integer"},"to_conversation_id":{"type":"integer"},"edge_kind":{"type":"string"},"weight":{"type":"number"},"metadata_json":{"type":"object"}},"required":["from_conversation_id","to_conversation_id"],"additionalProperties":false}"#,
        ),
        "vox_db_topic_evolution_append" => parse_obj(
            r#"{"type":"object","properties":{"topic_id":{"type":"integer"},"event_kind":{"type":"string"},"prior_label":{"type":"string"},"new_label":{"type":"string"},"detail_json":{"type":"object"}},"required":["topic_id","event_kind"],"additionalProperties":false}"#,
        ),
        "vox_db_research_metric_linked" => parse_obj(
            r#"{"type":"object","properties":{"session_key":{"type":"string"},"metric_type":{"type":"string"},"metric_value":{"type":"number"},"metadata_json":{"type":["string","object","null"]},"repository_id":{"type":"string"}},"required":["session_key","metric_type"],"additionalProperties":false}"#,
        ),

        // ── Codegen ──────────────────────────────────────────────────────────
        "vox_generate_code" => parse_obj(
            r#"{"type":"object","properties":{"prompt":{"type":"string"}},"required":["prompt"],"additionalProperties":true}"#,
        ),

        // ── Q&A & A2A ───────────────────────────────────────────────────────
        "vox_ask_agent" | "vox_answer_question" | "vox_pending_questions" | "vox_broadcast" => {
            parse_obj(
                r#"{"type":"object","additionalProperties":true,"description":"See qa module for expected fields."}"#,
            )
        }
        "vox_a2a_send" | "vox_a2a_inbox" | "vox_a2a_ack" | "vox_a2a_broadcast"
        | "vox_a2a_history" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"See a2a module for expected fields."}"#,
        ),

        // ── Skills ───────────────────────────────────────────────────────────
        "vox_skill_uninstall" | "vox_skill_info" | "vox_skill_parse" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }
        "vox_skill_search" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":true}"#,
        ),
        "vox_skill_install" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"VoxSkillBundle JSON payload fields."}"#,
        ),

        // ── Config / session map / heartbeat / cost ────────────────────────
        "vox_config_set" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"Partial orchestrator config updates."}"#,
        ),
        "vox_map_agent_session" => parse_obj(
            r#"{"type":"object","properties":{"agent_id":{"type":"integer","minimum":0,"description":"Orchestrator agent id to bind"},"session_id":{"type":"string","minLength":1,"maxLength":2048,"description":"Opaque client session string"}},"required":["agent_id","session_id"],"additionalProperties":false}"#,
        ),
        "vox_heartbeat" | "vox_record_cost" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }

        // ── Chat & plan ──────────────────────────────────────────────────────
        "vox_chat_message" => parse_obj(
            r#"{"type":"object","anyOf":[{"required":["prompt"]},{"required":["message"]}],"properties":{"prompt":{"type":"string","minLength":1,"maxLength":262144},"message":{"type":"string","minLength":1,"maxLength":262144,"description":"Alias for prompt (serde maps to prompt)"},"context_files":{"type":"array","items":{"type":"string","maxLength":4096}},"open_files":{"type":"array","items":{"type":"string","maxLength":4096}},"active_file":{"type":"string","maxLength":4096},"active_line":{"type":"integer"},"selected_text":{"type":"string","maxLength":1048576},"diagnostics":{"type":"array"},"session_id":{"type":"string","maxLength":2048,"description":"Opaque session isolation key. Independent sessions maintain separate history transcripts. Omit or pass null to use the shared default session."},"cognitive_profile":{"type":"string","enum":["fast","reasoning","creative"],"description":"Optional routing hint: fast=lowest latency model, reasoning=high-tier model, creative=high temperature. Omit for standard automatic resolution."}},"additionalProperties":true}"#,
        ),
        "vox_chat_history" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","maxLength":2048,"description":"Session isolation key. Omit to retrieve the shared default session history."}},"additionalProperties":false}"#,
        ),
        "vox_inline_edit" => parse_obj(
            r#"{"type":"object","properties":{"prompt":{"type":"string"},"instruction":{"type":"string"},"file":{"type":"string"},"file_path":{"type":"string"},"start_line":{"type":"integer"},"end_line":{"type":"integer"},"current_text":{"type":"string"},"selection":{"type":"string"},"language":{"type":"string"},"context_before":{"type":"string"},"context_after":{"type":"string"}},"required":["start_line","end_line","current_text"],"additionalProperties":true}"#,
        ),
        "vox_plan" => parse_obj(
            r#"{"type":"object","properties":{"goal":{"type":"string","minLength":1,"maxLength":65536},"scope_files":{"type":"array","items":{"type":"string","maxLength":4096}},"write_to_disk":{"type":"boolean"},"max_tasks":{"type":"integer","minimum":1}},"required":["goal"],"additionalProperties":false}"#,
        ),
        "vox_replan" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1,"maxLength":2048},"delta_hint":{"type":"string","minLength":1,"maxLength":65536},"write_to_disk":{"type":"boolean"},"mode":{"type":"string","maxLength":64}},"required":["session_id","delta_hint"],"additionalProperties":false}"#,
        ),
        "vox_plan_status" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1,"maxLength":2048}},"required":["session_id"],"additionalProperties":false}"#,
        ),
        "vox_benchmark_list" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":500}},"additionalProperties":false}"#,
        ),
        "vox_benchmark_record" => parse_obj(
            r#"{"type":"object","properties":{"name":{"type":"string","minLength":1,"maxLength":512,"description":"Benchmark name (e.g. build_time, eval_p95)"},"value":{"type":"number","description":"Optional metric value"},"details":{"description":"Optional structured JSON details"}},"required":["name"],"additionalProperties":false}"#,
        ),
        "vox_toestub_findings_upsert" => parse_obj(
            r#"{"type":"object","properties":{"findings":{"type":"array","minItems":1,"items":{"type":"object","properties":{"rule_id":{"type":"string","minLength":1},"rule_name":{"type":"string","minLength":1},"severity":{"type":"string","enum":["Info","Warning","Error","Critical"]},"file":{"type":"string","minLength":1},"line":{"type":"integer","minimum":1},"column":{"type":"integer","minimum":0},"message":{"type":"string","minLength":1},"suggestion":{"type":"string"},"context":{"type":"string"}},"required":["rule_id","rule_name","severity","file","line","column","message"],"additionalProperties":false}},"session_id":{"type":"string","maxLength":2048}},"required":["findings"],"additionalProperties":false}"#,
        ),
        "vox_schola_submit" => parse_obj(
            r#"{"type":"object","properties":{"description":{"type":"string","minLength":1,"maxLength":65536},"require_cuda":{"type":"boolean"},"require_metal":{"type":"boolean"},"min_vram_mb":{"type":"integer","minimum":0},"pool_label":{"type":"string","maxLength":256},"trajectory_capture":{"type":"boolean"},"min_quality_score":{"type":"integer","minimum":1,"maximum":5}},"required":["description"],"additionalProperties":false}"#,
        ),
        "vox_populi_local_status" => parse_obj(
            r#"{"type":"object","properties":{"registry_path":{"type":"string","description":"Optional override for the mens registry JSON path"}},"additionalProperties":false}"#,
        ),

        // ── Unified news (syndication safety + templates) ───────────────────
        "vox_news_test_syndicate" => parse_obj(
            r#"{"type":"object","properties":{"content":{"type":"string","minLength":1,"description":"Markdown with YAML frontmatter"}},"required":["content"],"additionalProperties":false}"#,
        ),
        "vox_news_draft_research" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256,"description":"Filename stem for docs/news/drafts/{news_id}.md"},"title":{"type":"string","minLength":1},"author":{"type":"string","minLength":1},"abstract_text":{"type":"string"}},"required":["news_id","title","author","abstract_text"],"additionalProperties":false}"#,
        ),
        "vox_news_approve" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256},"approver":{"type":"string","minLength":1,"maxLength":256}},"required":["news_id","approver"],"additionalProperties":false}"#,
        ),
        "vox_news_approval_status" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256}},"required":["news_id"],"additionalProperties":false}"#,
        ),
        "vox_news_simulate_publish_gate" => parse_obj(
            r#"{"type":"object","properties":{"news_id":{"type":"string","minLength":1,"maxLength":256},"content":{"type":"string","minLength":1}},"required":["news_id","content"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_prepare" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"title":{"type":"string","minLength":1},"author":{"type":"string","minLength":1},"content":{"type":"string","minLength":1},"abstract_text":{"type":"string"},"citations_json":{"type":"object"},"scholarly_metadata":{"type":"object","description":"ScientificPublicationMetadata (authors, license_spdx, funding_statement, ...)","additionalProperties":true},"preflight":{"type":"boolean","description":"If true, run publication_preflight before upsert; fail on error-level findings."},"preflight_profile":{"type":"string","enum":["default","double_blind"]}},"required":["publication_id","title","author","content"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_preflight" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"profile":{"type":"string","enum":["default","double_blind"]},"with_worthiness":{"type":"boolean","description":"If true, attach conservative worthiness rubric output (repo default YAML)."}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_worthiness_evaluate" => parse_obj(
            r#"{"type":"object","properties":{"contract_yaml_relative":{"type":"string","minLength":1,"maxLength":512,"description":"Repo-relative path to worthiness YAML"},"metrics":{"type":"object","description":"WorthinessInputs","properties":{"red_line_violation_ids":{"type":"array","items":{"type":"string","minLength":1}},"repeated_unresolved_contradiction":{"type":"boolean"},"claim_evidence_coverage":{"type":"number"},"artifact_replayability":{"type":"number"},"before_after_pair_integrity":{"type":"number"},"metadata_completeness":{"type":"number"},"ai_disclosure_compliance":{"type":"number"},"epistemic":{"type":"number"},"reproducibility":{"type":"number"},"novelty":{"type":"number"},"reliability":{"type":"number"},"metadata_policy":{"type":"number"},"meaningful_advance":{"type":"boolean"}},"required":["claim_evidence_coverage","artifact_replayability","before_after_pair_integrity","metadata_completeness","ai_disclosure_compliance","epistemic","reproducibility","novelty","reliability","metadata_policy"],"additionalProperties":false}},"required":["metrics"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_approve" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"approver":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id","approver"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_submit_local" | "vox_scientia_publication_status" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_media_list" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_media_delete" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"asset_ref":{"type":"string","minLength":1,"maxLength":2048}},"required":["publication_id","asset_ref"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_media_upsert" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"asset_ref":{"type":"string","minLength":1,"maxLength":2048},"media_type":{"type":"string","minLength":1,"maxLength":64},"storage_uri":{"type":"string","maxLength":4096},"status":{"type":"string","minLength":1,"maxLength":64},"metadata_json":{"type":"object","additionalProperties":true}},"required":["publication_id","asset_ref","media_type","status"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_route_simulate" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_publish" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"channels":{"type":"array","items":{"type":"string","minLength":1,"maxLength":64}},"dry_run":{"type":"boolean"},"json":{"type":"boolean"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),
        "vox_scientia_publication_retry_failed" => parse_obj(
            r#"{"type":"object","properties":{"publication_id":{"type":"string","minLength":1,"maxLength":256},"channel":{"type":"string","minLength":1,"maxLength":64},"dry_run":{"type":"boolean"},"json":{"type":"boolean"}},"required":["publication_id"],"additionalProperties":false}"#,
        ),

        _ => Map::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::tool_input_schema;
    use crate::tools::TOOL_REGISTRY;

    #[test]
    fn registry_tools_have_input_schema_coverage() {
        let mut missing = Vec::new();
        for (name, _) in TOOL_REGISTRY {
            if tool_input_schema(name).is_empty() {
                missing.push(*name);
            }
        }
        assert!(
            missing.is_empty(),
            "TOOL_REGISTRY tools missing non-empty input_schema: {missing:?}"
        );
    }

    #[test]
    fn submit_task_schema_has_files_array() {
        let m = tool_input_schema("vox_submit_task");
        let props = m.get("properties").and_then(|p| p.as_object()).unwrap();
        assert!(props.contains_key("files"));
        assert!(props.contains_key("description"));
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
}
