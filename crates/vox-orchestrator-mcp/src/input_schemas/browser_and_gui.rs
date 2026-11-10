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
        // ── Browser (CDP / chromiumoxide) ───────────────────────────────────
        "vox_browser_open" => derived_tool_schema!(crate::params::BrowserOpenParams),
        "vox_browser_open_ex" => derived_tool_schema!(crate::params::BrowserOpenExParams),
        "vox_browser_close" => derived_tool_schema!(crate::params::BrowserPageParams),
        "vox_browser_goto" => derived_tool_schema!(crate::params::BrowserGotoParams),
        "vox_browser_click" | "vox_browser_text" => {
            derived_tool_schema!(crate::params::BrowserTargetParams)
        }
        "vox_browser_fill" => derived_tool_schema!(crate::params::BrowserFillParams),
        "vox_browser_snapshot" => {
            derived_tool_schema!(crate::params::BrowserSnapshotParams)
        }
        "vox_browser_click_ref" => derived_tool_schema!(crate::params::BrowserRefParams),
        "vox_browser_fill_ref" => {
            derived_tool_schema!(crate::params::BrowserFillRefParams)
        }
        "vox_browser_cookies_export" => {
            derived_tool_schema!(crate::params::BrowserPageParams)
        }
        "vox_browser_cookies_import" => {
            derived_tool_schema!(crate::params::BrowserCookiesImportParams)
        }
        "vox_browser_wait_for" => derived_tool_schema!(crate::params::BrowserWaitParams),
        "vox_browser_html" => derived_tool_schema!(crate::params::BrowserHtmlParams),
        "vox_browser_screenshot" => {
            derived_tool_schema!(crate::params::BrowserScreenshotParams)
        }
        "vox_browser_extract" => {
            derived_tool_schema!(crate::params::BrowserExtractParams)
        }
        "vox_browser_extract_json" => {
            derived_tool_schema!(crate::params::BrowserExtractJsonParams)
        }
        "vox_browser_act" => derived_tool_schema!(crate::params::BrowserActParams),
        "vox_browser_back"
        | "vox_browser_forward"
        | "vox_browser_reload"
        | "vox_browser_stop"
        | "vox_browser_page_info"
        | "vox_browser_screenshot_viewport"
        | "vox_browser_screencast_frame" => {
            derived_tool_schema!(crate::params::BrowserPageParams)
        }
        "vox_browser_click_xy" => {
            derived_tool_schema!(crate::params::BrowserClickPointParams)
        }
        "vox_browser_scroll" => derived_tool_schema!(crate::params::BrowserScrollParams),
        "vox_browser_press" => derived_tool_schema!(crate::params::BrowserKeyParams),
        "vox_browser_type" => derived_tool_schema!(crate::params::BrowserTypeParams),
        "vox_browser_set_viewport" => {
            derived_tool_schema!(crate::params::BrowserViewportParams)
        }
        "vox_browser_set_control_lock" => {
            derived_tool_schema!(crate::params::BrowserControlLockParams)
        }
        "vox_browser_list_pages" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),

        // ── T3 Wave 3 tools (Phase 255: T3-MCP-RPC-WIRE-01..03) ──
        "vox_consensus_run" => parse_obj(
            r#"{"type":"object","properties":{"prompt":{"type":"string","minLength":1},"model_ids":{"type":"array","items":{"type":"string"},"minItems":2,"maxItems":4},"candidate_responses":{"type":"array","items":{"type":"string"}}},"required":["prompt"],"additionalProperties":false}"#,
        ),
        "vox_context_handoff" => parse_obj(
            r#"{"type":"object","properties":{"source_thread_id":{"type":"string","minLength":1},"source_model_id":{"type":"string","minLength":1},"target_model_id":{"type":"string","minLength":1},"mode":{"type":"string","enum":["full_history","summarized","custom"]},"custom_instructions":{"type":"string"},"messages":{"type":"array","items":{"type":"string"}}},"required":["source_thread_id","source_model_id","target_model_id"],"additionalProperties":false}"#,
        ),
        "vox_checkpoint_capture" => parse_obj(
            r#"{"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"turn_count":{"type":"integer","minimum":1}},"required":["thread_id"],"additionalProperties":false}"#,
        ),
        "vox_checkpoint_list" => parse_obj(
            r#"{"type":"object","properties":{"thread_id":{"type":"string","minLength":1}},"required":["thread_id"],"additionalProperties":false}"#,
        ),
        "vox_checkpoint_restore" => parse_obj(
            r#"{"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"target_turn":{"type":"integer","minimum":1}},"required":["thread_id","target_turn"],"additionalProperties":false}"#,
        ),
        "vox_muse_pool_spawn" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"variant_count":{"type":"integer","minimum":2,"maximum":4}},"required":["session_id"],"additionalProperties":false}"#,
        ),
        "vox_muse_pool_adopt" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"winning_slot_index":{"type":"integer","minimum":0,"maximum":3}},"required":["session_id","winning_slot_index"],"additionalProperties":false}"#,
        ),
        "vox_guest_share_mint" => parse_obj(
            r#"{"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"owner_id":{"type":"string","minLength":1},"policy":{"type":"string","enum":["view_only","comment_with_approval","interactive_fork"]}},"required":["thread_id","owner_id"],"additionalProperties":false}"#,
        ),
        "vox_guest_share_moderate" => parse_obj(
            r#"{"type":"object","properties":{"token_id":{"type":"string","minLength":1},"submission_id":{"type":"string","minLength":1},"approve":{"type":"boolean"},"guest_author":{"type":"string"},"prompt":{"type":"string"}},"required":["token_id"],"additionalProperties":false}"#,
        ),
        "vox_session_import_external" => parse_obj(
            r#"{"type":"object","properties":{"source_harness":{"type":"string","enum":["claude","codex","cursor"]},"external_session_id":{"type":"string","minLength":1},"raw_jsonl":{"type":"string","minLength":1}},"required":["source_harness","external_session_id","raw_jsonl"],"additionalProperties":false}"#,
        ),
        "vox_session_export_bundle" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"title":{"type":"string","minLength":1},"messages":{"type":"array","items":{"type":"object","properties":{"role":{"type":"string"},"content":{"type":"string"},"timestamp_ms":{"type":"integer"}},"required":["role","content"]}}},"required":["session_id","title","messages"],"additionalProperties":false}"#,
        ),
        "vox_session_import_bundle" => parse_obj(
            r#"{"type":"object","properties":{"bundle_json":{"type":"string","minLength":2}},"required":["bundle_json"],"additionalProperties":false}"#,
        ),
        "vox_quarantine_set" => parse_obj(
            r#"{"type":"object","properties":{"path_prefix":{"type":"string","minLength":1},"quarantined":{"type":"boolean"}},"required":["path_prefix"],"additionalProperties":false}"#,
        ),
        "vox_quarantine_list" => {
            parse_obj(r#"{"type":"object","properties":{},"additionalProperties":false}"#)
        }
        "vox_vcs_checkpoint_diff_summary" => parse_obj(
            r#"{"type":"object","properties":{"thread_id":{"type":"string","minLength":1},"target_turn":{"type":"integer","minimum":1}},"required":["thread_id","target_turn"],"additionalProperties":false}"#,
        ),
        "vox_file_diff_and_diagnostics" => parse_obj(
            r#"{"type":"object","properties":{"file_path":{"type":"string","minLength":1}},"required":["file_path"],"additionalProperties":false}"#,
        ),
        "vox_stage_file" => parse_obj(
            r#"{"type":"object","properties":{"file_path":{"type":"string","minLength":1}},"required":["file_path"],"additionalProperties":false}"#,
        ),
        "vox_revert_file" => parse_obj(
            r#"{"type":"object","properties":{"file_path":{"type":"string","minLength":1}},"required":["file_path"],"additionalProperties":false}"#,
        ),
        "vox_chat_compact_up_to" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"message_id":{"type":"string","minLength":1},"index":{"type":"integer","minimum":0},"direction":{"type":"string","enum":["up_to","from"]},"summary":{"type":"string"}},"additionalProperties":false}"#,
        ),
        "vox_chat_edit_message_in_place" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"message_id":{"type":"string","minLength":1},"new_content":{"type":"string"}},"required":["message_id","new_content"],"additionalProperties":false}"#,
        ),
        "vox_chat_regenerate" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"from_message_id":{"type":"string","minLength":1},"message_id":{"type":"string","minLength":1},"prompt_override":{"type":"string"},"model_override":{"type":"string"},"execute":{"type":"boolean"}},"additionalProperties":false}"#,
        ),
        "vox_chat_stream_resume" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"char_offset":{"type":"integer","minimum":0},"after_seq":{"type":"integer","minimum":0}},"additionalProperties":false}"#,
        ),
        "vox_chat_stream_cancel" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1}},"additionalProperties":false}"#,
        ),
        "vox_chat_queue_steer" => parse_obj(
            r#"{"type":"object","properties":{"session_id":{"type":"string","minLength":1},"thread_id":{"type":"string","minLength":1},"entry_id":{"type":"string","minLength":1},"prompt":{"type":"string"},"promote_to_steer":{"type":"boolean"}},"additionalProperties":false}"#,
        ),
        "vox_list_mcp_servers" => parse_obj(
            r#"{"type":"object","properties":{"prefixed_tools":{"type":"array","items":{"type":"object"}}},"additionalProperties":false}"#,
        ),
        "vox_list_mcp_tools" => parse_obj(
            r#"{"type":"object","properties":{"mcp_name":{"type":"string","minLength":1},"server":{"type":"string","minLength":1}},"additionalProperties":false}"#,
        ),
        "vox_call_mcp_lazy" => parse_obj(
            r#"{"type":"object","properties":{"mcp_name":{"type":"string","minLength":1},"tool_name":{"type":"string","minLength":1},"arguments":{"type":"object"}},"required":["mcp_name","tool_name"],"additionalProperties":false}"#,
        ),
        "vox_battery_status" => {
            parse_obj(r#"{"type":"object","properties":{},"additionalProperties":false}"#)
        }
        "vox_session_list_external" => parse_obj(
            r#"{"type":"object","properties":{"source_harness":{"type":"string","enum":["claude","codex","cursor"]},"root_dir":{"type":"string"}},"additionalProperties":false}"#,
        ),
        "vox_guest_share_redeem" => parse_obj(
            r#"{"type":"object","properties":{"invite_id":{"type":"string","minLength":1},"signature_hex":{"type":"string","minLength":1},"guest_session_id":{"type":"string"},"now_ms":{"type":"integer","minimum":0}},"required":["invite_id","signature_hex"],"additionalProperties":false}"#,
        ),
        "vox_guest_share_submit" => parse_obj(
            r#"{"type":"object","properties":{"message_id":{"type":"string","minLength":1},"thread_id":{"type":"string","minLength":1},"guest_session_id":{"type":"string"},"guest_author":{"type":"string"},"prompt":{"type":"string","minLength":1}},"required":["thread_id","prompt"],"additionalProperties":false}"#,
        ),
        "vox_pr_review_guide" => parse_obj(
            r#"{"type":"object","properties":{"pr_number":{"type":"integer","minimum":1},"head_sha":{"type":"string","minLength":1},"changed_files":{"type":"array","items":{"type":"string"}}},"required":["pr_number","head_sha"],"additionalProperties":false}"#,
        ),
        _ => return None,
    };
    Some(schema)
}
