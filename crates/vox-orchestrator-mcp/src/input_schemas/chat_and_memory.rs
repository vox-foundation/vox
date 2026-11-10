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
        "vox_set_agent_budget" => {
            derived_tool_schema!(crate::mcp_context::SetAgentBudgetParams)
        }
        "vox_emergency_stop" => {
            derived_tool_schema!(crate::mcp_context::EmergencyStopParams)
        }
        "vox_handoff_context" => parse_obj(
            r#"{"type":"object","properties":{"from_agent":{"type":"integer","minimum":0},"to_agent":{"type":"integer","minimum":0}},"required":["from_agent","to_agent"],"additionalProperties":false}"#,
        ),

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
        "vox_semantic_fs_discover" => parse_obj(
            r#"{"type":"object","properties":{"intent":{"type":"string","minLength":1},"limit":{"type":"integer","minimum":1,"maximum":256}},"required":["intent"],"additionalProperties":false}"#,
        ),
        "vox_memory_log" => parse_obj(
            r#"{"type":"object","properties":{"entry":{"type":"string"}},"required":["entry"],"additionalProperties":false}"#,
        ),
        "vox_knowledge_query" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer"}},"required":["query"],"additionalProperties":false}"#,
        ),
        "vox_research_run" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string","minLength":1},"scope":{"type":"string"},"max_sources":{"type":"integer","minimum":1,"maximum":50},"verify_claims":{"type":"boolean"},"site_scope":{"type":"string"},"json":{"type":"boolean"},"waves":{"type":"integer","minimum":1,"description":"Research waves (default 1)"},"domain_mode":{"type":"string","description":"general (default), shopping, or codegen; case-insensitive"},"lane":{"type":"string","enum":["fast","deep"],"description":"fast (default; no LLM planner) or deep (LLM query decomposition). Canonical values are lowercase; the server also accepts any case and surrounding whitespace."}},"required":["query"],"additionalProperties":false}"#,
        ),
        "vox_research_start" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string","minLength":1},"scope":{"type":"string"},"max_sources":{"type":"integer","minimum":1,"maximum":50},"verify_claims":{"type":"boolean"},"site_scope":{"type":"string"},"waves":{"type":"integer","minimum":1,"description":"Research waves (default 1)"},"domain_mode":{"type":"string","description":"general (default), shopping, or codegen; case-insensitive"},"lane":{"type":"string","enum":["fast","deep"],"description":"fast (default; no LLM planner) or deep (LLM query decomposition). Canonical values are lowercase; the server also accepts any case and surrounding whitespace."}},"required":["query"],"additionalProperties":false}"#,
        ),
        "vox_research_status" | "vox_research_get" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"integer","minimum":1}},"required":["session_id"],"additionalProperties":false}"#,
        ),
        "vox_research_search" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string","minLength":1},"limit":{"type":"integer","minimum":1,"maximum":50},"domain":{"type":"string"},"min_confidence":{"type":"number","minimum":0.0,"maximum":1.0},"verified_only":{"type":"boolean"}},"required":["query"],"additionalProperties":false}"#,
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

        // ── Config / session map / heartbeat / cost ────────────────────────
        "vox_config_set" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"Partial orchestrator config updates."}"#,
        ),
        "vox_map_agent_session" => {
            derived_tool_schema!(crate::params::MapAgentSessionParams)
        }
        "vox_heartbeat" | "vox_record_cost" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }

        // ── Chat & plan ──────────────────────────────────────────────────────
        "vox_chat_message" => parse_obj(
            r#"{"type":"object","anyOf":[{"required":["prompt"]},{"required":["message"]}],"properties":{"prompt":{"type":"string","minLength":1,"maxLength":262144},"message":{"type":"string","minLength":1,"maxLength":262144,"description":"Alias for prompt (serde maps to prompt)"},"context_files":{"type":"array","items":{"type":"string","maxLength":4096}},"open_files":{"type":"array","items":{"type":"string","maxLength":4096}},"active_file":{"type":"string","maxLength":4096},"active_line":{"type":"integer"},"selected_text":{"type":"string","maxLength":1048576},"diagnostics":{"type":"array"},"session_id":{"type":"string","maxLength":2048,"description":"Opaque session isolation key. Independent sessions maintain separate history transcripts. Omit or pass null to use the shared default session."},"thread_id":{"type":"string","maxLength":2048,"description":"Editor thread id; included in structured transcript journey envelope"},"journey_id":{"type":"string","maxLength":2048,"description":"Stable request id across routing and storage (generated if omitted)"},"cognitive_profile":{"type":"string","enum":["fast","reasoning","creative"],"description":"Optional routing hint: fast=lowest latency model, reasoning=high-tier model, creative=high temperature. Omit for standard automatic resolution."},"temperature":{"type":"number","minimum":0.0,"maximum":2.0},"top_p":{"type":"number","minimum":0.0,"maximum":1.0},"force_research":{"type":"boolean","description":"Optional override to force trigger autonomous research"},"research_scope":{"type":"string","enum":["local","web","both"],"description":"Optional research scope override"},"model_override":{"type":"string","maxLength":512,"description":"Per-request model id pick. Takes precedence over the daemon's process-global override; blank is treated as absent."},"tier":{"type":"string","enum":["local","mesh","cloud","auto"],"description":"Composer runtime tier. Feeds model-resolution constraints (NOT cognitive_profile)."},"clutch":{"type":"string","enum":["free","efficiency","balanced","genius"],"description":"Composer clutch profile (how much gas)."},"risk":{"type":"string","enum":["high","moderate","low"],"description":"Composer risk posture."},"skill_exclusions":{"type":"array","items":{"type":"string","maxLength":512},"description":"Session-scoped skills the user rejected in the transcript."}},"additionalProperties":true}"#,
        ),
        "vox_chat_history" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","maxLength":2048,"description":"Session isolation key. Omit to retrieve the shared default session history."},"trace_id":{"type":"string","maxLength":256,"description":"Optional trace id; logged server-side for correlation with chat_message"}},"additionalProperties":false}"#,
        ),
        "vox_questioning_submit_answer" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1,"maxLength":2048},"answer_text":{"type":"string","minLength":1,"maxLength":131072},"answer_type":{"type":"string","maxLength":64},"question_id":{"type":"string","maxLength":512},"selected_option_id":{"type":"string","maxLength":256},"information_contribution_bits":{"type":"number"}},"required":["session_id","answer_text"],"additionalProperties":false}"#,
        ),
        "vox_questioning_pending" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1,"maxLength":2048,"description":"Same MCP session_id as chat / plan"}},"required":["session_id"],"additionalProperties":false}"#,
        ),
        "vox_questioning_sync_ssot" => parse_obj(
            r#"{"type":"object","properties":{"relative_path":{"type":"string","maxLength":4096,"description":"Workspace-relative path to questioning markdown; default is docs/src/reference/information-theoretic-questioning.md"}},"additionalProperties":false}"#,
        ),
        "vox_inline_edit" => parse_obj(
            r#"{"type":"object","properties":{"prompt":{"type":"string"},"instruction":{"type":"string"},"file":{"type":"string"},"file_path":{"type":"string"},"start_line":{"type":"integer"},"end_line":{"type":"integer"},"current_text":{"type":"string"},"selection":{"type":"string"},"language":{"type":"string"},"context_before":{"type":"string"},"context_after":{"type":"string"},"temperature":{"type":"number","minimum":0.0,"maximum":2.0},"top_p":{"type":"number","minimum":0.0,"maximum":1.0}},"required":["start_line","end_line","current_text"],"additionalProperties":true}"#,
        ),
        "vox_apply_structured_edit" => parse_obj(
            r#"{"type":"object","properties":{"file_path":{"type":"string"},"start_line":{"type":"integer","minimum":1},"end_line":{"type":"integer","minimum":1},"target_content":{"type":"string"},"replacement_code":{"type":"string"}},"required":["file_path","target_content","replacement_code"],"additionalProperties":false}"#,
        ),
        "vox_plan" => parse_obj(
            r#"{"type":"object","properties":{"goal":{"type":"string","minLength":1,"maxLength":65536},"scope_files":{"type":"array","items":{"type":"string","maxLength":4096}},"write_to_disk":{"type":"boolean"},"max_tasks":{"type":"integer","minimum":1,"maximum":2000},"session_id":{"type":"string","maxLength":2048},"plan_depth":{"type":"string","enum":["minimal","standard","deep"]},"auto_expand_thin_plan":{"type":"boolean"},"loop_mode":{"type":"string","enum":["off","auto","force"]},"max_refine_rounds":{"type":"integer","minimum":0,"maximum":8},"refine_budget_tokens":{"type":"integer","minimum":0,"maximum":200000},"gap_risk_threshold":{"type":"number","minimum":0.05,"maximum":0.95},"plan_page_offset":{"type":"integer","minimum":0,"maximum":500000},"plan_page_limit":{"type":"integer","minimum":1,"maximum":2000},"plan_telemetry_session_id":{"type":"string","maxLength":2048},"require_approval":{"type":"boolean"},"question_link_session_id":{"type":"string","maxLength":2048},"questioning_hints_enabled":{"type":"boolean"},"answerer_profile":{"type":"string","enum":["local_first","cloud_first","balanced"]},"temperature":{"type":"number","minimum":0.0,"maximum":2.0},"top_p":{"type":"number","minimum":0.0,"maximum":1.0},"model_override":{"type":"string","maxLength":256},"trace_id":{"type":"string","maxLength":256,"description":"End-to-end ChatHop correlation id"},"turn_id":{"type":"string","maxLength":256,"description":"Per-submit turn id for Drive / ChatHop JSONL"}},"required":["goal"],"additionalProperties":false}"#,
        ),
        "vox_ghost_text" => parse_obj(
            r#"{"type":"object","properties":{"prefix":{"type":"string"},"suffix":{"type":"string"},"language":{"type":"string"},"file_path":{"type":"string"},"max_tokens":{"type":"integer"},"session_id":{"type":"string"},"temperature":{"type":"number","minimum":0.0,"maximum":2.0},"top_p":{"type":"number","minimum":0.0,"maximum":1.0}},"required":["prefix","suffix"],"additionalProperties":true}"#,
        ),
        "vox_replan" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1,"maxLength":2048},"delta_hint":{"type":"string","minLength":1,"maxLength":65536},"write_to_disk":{"type":"boolean"},"mode":{"type":"string","maxLength":64}},"required":["session_id","delta_hint"],"additionalProperties":false}"#,
        ),
        "vox_plan_status" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1,"maxLength":2048}},"required":["session_id"],"additionalProperties":false}"#,
        ),
        "vox_plan_list_sessions" => {
            derived_tool_schema!(crate::chat_tools::params::PlanListSessionsParams)
        }
        "vox_plan_resume" => {
            derived_tool_schema!(crate::chat_tools::params::PlanResumeParams)
        }
        "vox_attention_summary" => {
            derived_tool_schema!(crate::dei_tools::params::AttentionSummaryParams)
        }
        "vox_attention_history" | "vox_attention_reset" | "vox_trust_override" => {
            parse_obj(r#"{"type":"object","additionalProperties":false}"#)
        }
        "vox_handoff_lineage" => {
            derived_tool_schema!(crate::dei_tools::params::HandoffLineageParams)
        }
        "vox_benchmark_list" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":500},"metric_type":{"type":"string","enum":["benchmark_event","syntax_k_event"]},"source":{"type":"string","enum":["research_metrics","build_health","build_regressions","build_warnings","dependency_shape"]},"run_id":{"type":"integer","minimum":1}},"additionalProperties":false}"#,
        ),
        "vox_benchmark_record" => parse_obj(
            r#"{"type":"object","properties":{"name":{"type":"string","minLength":1,"maxLength":512,"description":"Benchmark name (e.g. build_time, eval_p95)"},"fixture_id":{"type":"string","maxLength":512},"metric_type":{"type":"string","enum":["benchmark_event","syntax_k_event"]},"value":{"type":"number","description":"Optional metric value"},"details":{"description":"Optional structured JSON details"}},"required":["name"],"additionalProperties":false}"#,
        ),
        "vox_code_audit_findings_upsert" => parse_obj(
            r#"{"type":"object","properties":{"findings":{"type":"array","minItems":1,"items":{"type":"object","properties":{"rule_id":{"type":"string","minLength":1},"rule_name":{"type":"string","minLength":1},"severity":{"type":"string","enum":["Info","Warning","Error","Critical"]},"file":{"type":"string","minLength":1},"line":{"type":"integer","minimum":1},"column":{"type":"integer","minimum":0},"message":{"type":"string","minLength":1},"suggestion":{"type":"string"},"context":{"type":"string"}},"required":["rule_id","rule_name","severity","file","line","column","message"],"additionalProperties":false}},"session_id":{"type":"string","maxLength":2048}},"required":["findings"],"additionalProperties":false}"#,
        ),
        "vox_schola_submit" => parse_obj(
            r#"{"type":"object","properties":{"description":{"type":"string","minLength":1,"maxLength":65536},"require_cuda":{"type":"boolean"},"require_metal":{"type":"boolean"},"min_vram_mb":{"type":"integer","minimum":0},"pool_label":{"type":"string","maxLength":256},"trajectory_capture":{"type":"boolean"},"min_quality_score":{"type":"integer","minimum":1,"maximum":5}},"required":["description"],"additionalProperties":false}"#,
        ),
        "vox_populi_local_status" => parse_obj(
            r#"{"type":"object","properties":{"registry_path":{"type":"string","description":"Optional override for the mens registry JSON path"}},"additionalProperties":false}"#,
        ),

        _ => return None,
    };
    Some(schema)
}
