use serde_json::{Map, Value};

macro_rules! derived_tool_schema {
    ($t:ty) => {{
        let settings = schemars::generate::SchemaSettings::draft07().with(|s| {
            s.inline_subschemas = true;
        });
        let schema_generator = settings.into_generator();
        let root = schema_generator.into_root_schema_for::<$t>();
        let serde_json::Value::Object(mut map) =
            serde_json::to_value(&root).expect(concat!("schema_for ", stringify!($t)))
        else {
            panic!(concat!(
                "JsonSchema root must be an object: ",
                stringify!($t)
            ));
        };
        map.remove("$schema");
        map
    }};
}

fn parse_obj(s: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(s)
        .ok()
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_default()
}

#[inline(never)]
pub(super) fn schema_for_tool(name: &str) -> Option<Map<String, Value>> {
    let schema = match name {
        // ── Oratio (already strict) ─────────────────────────────────────────
        // Schemas are pure JSON — no oratio-rerank types referenced; always available
        // so that TOOL_REGISTRY coverage test passes in all feature configurations.
        "vox_oratio_transcribe" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string","description":"Workspace-relative or absolute path to an audio or transcript file"},"language_hint":{"type":"string"},"profile":{"type":"string","enum":["conservative","balanced","aggressive"]},"debug_parser_payload":{"type":"boolean"}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_oratio_listen" => parse_obj(
            r#"{"type":"object","properties":{"path":{"type":"string"},"session_id":{"type":"string"},"timeout_ms":{"type":"integer","minimum":1},"max_duration_ms":{"type":"integer","minimum":1},"inference_deadline_ms":{"type":"integer","minimum":1},"heartbeat_ms":{"type":"integer","minimum":1},"language_hint":{"type":"string"},"profile":{"type":"string","enum":["conservative","balanced","aggressive"]},"route_mode":{"type":"string","enum":["none","tool","chat","orchestrator"]},"debug_parser_payload":{"type":"boolean"},"emit_asr_refine_path":{"type":"string"},"llm_refinement":{"type":"boolean"},"llm_min_det_confidence":{"type":"number"},"llm_max_output_tokens":{"type":"integer","minimum":1}},"required":["path"],"additionalProperties":false}"#,
        ),
        "vox_oratio_status" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_speech_to_code" => parse_obj(
            r#"{"type":"object","description":"Exactly one of `path` (audio/transcript file under workspace) or `prompt` (text only). Chains Oratio STT when path is set, then vox_generate_code.","properties":{"path":{"type":"string","description":"Workspace-relative audio file for Candle Whisper + refine"},"prompt":{"type":"string","description":"Skip STT; use as codegen prompt"},"language_hint":{"type":"string"},"profile":{"type":"string","enum":["conservative","balanced","aggressive"]},"debug_parser_payload":{"type":"boolean"},"route_mode":{"type":"string","enum":["none","tool","chat","orchestrator"]},"include_route":{"type":"boolean","description":"When true (default), run deterministic intent routing on refined transcript (path mode only)"},"validate":{"type":"boolean"},"max_retries":{"type":"integer","minimum":0,"maximum":5},"session_id":{"type":"string","description":"Shared with generate; defaults to new correlation if omitted"},"output_surface_mode":{"type":"string"},"emit_trace_path":{"type":"string","description":"Append one JSON line (speech_trace.schema.json fields) under this workspace-relative path"}},"additionalProperties":false}"#,
        ),

        // ── Tasks & bulletin ─────────────────────────────────────────────────
        "vox_submit_task" => derived_tool_schema!(crate::params::SubmitTaskParams),
        "vox_tool_search" => derived_tool_schema!(crate::params::ToolSearchParams),
        "vox_task_status" | "vox_cancel_task" | "vox_test_decision" => {
            derived_tool_schema!(crate::params::TaskStatusParams)
        }
        "vox_complete_task" => derived_tool_schema!(crate::params::CompleteTaskParams),
        "vox_fail_task" => derived_tool_schema!(crate::params::FailTaskParams),
        "vox_doubt_task" => derived_tool_schema!(crate::params::DoubtTaskParams),
        "vox_verify_task_claims" => {
            derived_tool_schema!(crate::receipt_tools::VerifyTaskClaimsParams)
        }
        "vox_propose_skill" => derived_tool_schema!(crate::params::ProposeSkillParams),
        "vox_ask_clarification" => derived_tool_schema!(crate::params::AskClarificationParams),
        "vox_resolve_feedback" => derived_tool_schema!(crate::params::ResolveFeedbackParams),
        "vox_feedback_list" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_publish_message" => {
            derived_tool_schema!(crate::params::PublishMessageParams)
        }
        "vox_reorder_task" => derived_tool_schema!(crate::params::ReorderTaskParams),
        "vox_drain_agent" => derived_tool_schema!(crate::params::DrainAgentParams),
        "vox_spawn_agent" => derived_tool_schema!(crate::params::SpawnAgentParams),
        "vox_retire_agent" | "vox_pause_agent" | "vox_resume_agent" => {
            derived_tool_schema!(crate::params::AgentIdToolParams)
        }

        // ── Orchestrator status / no-arg tools ───────────────────────────────
        "vox_orchestrator_status"
        | "vox_orchestrator_persistence_outbox_lifecycle"
        | "vox_rebalance"
        | "vox_lock_status"
        | "vox_file_graph"
        | "vox_config_get"
        | "vox_openclaw_list_remote"
        | "vox_openclaw_discover"
        | "vox_openclaw_health"
        | "vox_openclaw_subscriptions"
        | "vox_agent_list_remote"
        | "vox_agent_subscriptions"
        | "vox_session_list"
        | "vox_session_cleanup"
        | "vox_memory_list_keys"
        | "vox_skill_list"
        | "vox_skill_discover"
        | "vox_test_all"
        | "vox_check_workspace"
        | "vox_get_active_model"
        | "vox_language_surface"
        | "vox_capability_model_manifest"
        | "vox_pipeline_status"
        | "vox_decorator_registry"
        | "vox_builtin_registry"
        | "vox_workspace_modules"
        | "vox_export_grammar_ebnf"
        | "vox_a2a_tasks" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_secrets_doctor" => parse_obj(
            r#"{
            "type": "object",
            "properties": {
                "workflow": { "type": "string", "enum": ["Chat", "Mcp", "Publish", "Review", "DbRemote", "MensMesh"] },
                "profile": { "type": "string", "enum": ["Dev", "Ci", "Prod", "Mobile"] }
            },
            "additionalProperties": false
        }"#,
        ),

        // Handler ignores args today; keep the schema strict so clients send `{}` only.
        "vox_orchestrator_start" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_orchestrator_persistence_outbox_queue" => parse_obj(
            r#"{"type":"object","properties":{"lane":{"type":"string","description":"Optional lane filter (e.g. lineage/task_failed)"},"limit":{"type":"integer","minimum":1,"maximum":1000,"description":"Maximum rows returned from the tail of filtered queue"},"include_replay":{"type":"boolean","description":"Include replay payload blobs in returned rows (default true)"}},"additionalProperties":false}"#,
        ),
        // `params` is `serde_json::Value` — derive would need a custom `schema_with`; keep explicit.
        "vox_openclaw_gateway_call" | "vox_agent_gateway_call" => parse_obj(
            r#"{"type":"object","properties":{"method":{"type":"string","minLength":1},"params":{"description":"OpenClaw gateway params JSON object"}},"required":["method"],"additionalProperties":false}"#,
        ),
        "vox_openclaw_search_remote" => {
            derived_tool_schema!(crate::params::OpenClawSearchParams)
        }
        "vox_openclaw_import_skill" => {
            derived_tool_schema!(crate::params::OpenClawImportParams)
        }
        "vox_openclaw_subscribe"
        | "vox_openclaw_unsubscribe"
        | "vox_agent_subscribe"
        | "vox_agent_unsubscribe" => derived_tool_schema!(crate::params::OpenClawDomainParams),
        "vox_openclaw_notify" | "vox_agent_notify" => {
            derived_tool_schema!(crate::params::OpenClawNotifyParams)
        }

        // ── Execution Budget ─────────────────────────────────────────────────
        "vox_exec_time_query" => parse_obj(
            r#"{"type":"object","properties":{"tool_key":{"type":"string","minLength":1},"repository_id":{"type":"string"},"window_days":{"type":"integer","minimum":1}},"required":["tool_key"],"additionalProperties":false}"#,
        ),
        "vox_exec_time_record" => parse_obj(
            r#"{"type":"object","properties":{"tool_key":{"type":"string","minLength":1},"repository_id":{"type":"string"},"duration_ms":{"type":"integer","minimum":0},"timeout_budget_ms":{"type":"integer","minimum":0},"outcome":{"type":"string","enum":["success","timeout","error"]}},"required":["tool_key","duration_ms"],"additionalProperties":false}"#,
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

        // ── HITL approvals ──────────────────────────────────────────────────
        "vox_pending_approvals" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_resolve_approval" => parse_obj(
            r#"{"type":"object","properties":{"approval_id":{"type":"string","minLength":1},"outcome":{"type":"string","enum":["approve","approved","modify","modified","reject","rejected"]},"decision":{"type":"string","description":"Alias for `outcome`"}},"required":["approval_id"],"anyOf":[{"required":["outcome"]},{"required":["decision"]}],"additionalProperties":false}"#,
        ),
        // T0.3: persisted per-repo "always allow this tool" allowlist.
        "vox_add_approval_allowlist_entry" => parse_obj(
            r#"{"type":"object","properties":{"tool":{"type":"string","minLength":1,"description":"Canonical MCP tool name to always allow"},"repo_id":{"type":"string","description":"Repository id to scope the allowlist entry to; defaults to the current server's repository"}},"required":["tool"],"additionalProperties":false}"#,
        ),
        "vox_list_approval_allowlist" => parse_obj(
            r#"{"type":"object","properties":{"repo_id":{"type":"string","description":"Repository id to list allowlisted tools for; defaults to the current server's repository"}},"additionalProperties":false}"#,
        ),

        _ => return None,
    };
    Some(schema)
}
