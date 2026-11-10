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
        // ── Gamify ───────────────────────────────────────────────────────────
        "vox_check_mood" | "vox_agent_status" | "vox_agent_continue" | "vox_agent_assess" => {
            parse_obj(
                r#"{"type":"object","additionalProperties":true,"description":"Pass agent_id and other fields per orchestrator tool docs."}"#,
            )
        }
        "vox_agent_handoff" => parse_obj(
            r#"{"type":"object","properties":{"from_agent_id":{"type":"integer","minimum":0},"to_agent_id":{"type":"integer","minimum":0},"plan_summary":{"type":"string","minLength":1},"unresolved_objectives":{"type":"array","items":{"type":"string"}},"verification_criteria":{"type":"array","items":{"type":"string"}},"context_envelope_json":{"type":"string","minLength":1},"harness_spec_json":{"type":"string","minLength":1}},"required":["from_agent_id","to_agent_id","plan_summary"],"additionalProperties":false}"#,
        ),
        "vox_gamify_notifications_list" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":100}},"additionalProperties":false}"#,
        ),
        "vox_gamify_progress_snapshot" => parse_obj(
            r#"{"type":"object","properties":{"notification_limit":{"type":"integer","minimum":1,"maximum":100},"policy_limit":{"type":"integer","minimum":1,"maximum":500},"policy_days":{"type":"integer","minimum":1,"maximum":3660}},"additionalProperties":false}"#,
        ),
        "vox_gamify_notification_ack" => parse_obj(
            r#"{"type":"object","properties":{"notification_id":{"type":"string","minLength":1}},"required":["notification_id"],"additionalProperties":false}"#,
        ),
        "vox_gamify_notifications_ack_all" => {
            parse_obj(r#"{"type":"object","additionalProperties":false}"#)
        }
        "vox_gamify_quest_list" => parse_obj(
            r#"{"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":200}},"additionalProperties":false}"#,
        ),
        "vox_gamify_shop_catalog" => parse_obj(r#"{"type":"object","additionalProperties":false}"#),
        "vox_gamify_shop_buy" => parse_obj(
            r#"{"type":"object","properties":{"item_index":{"type":"integer","minimum":1},"idempotency_key":{"type":"string"}},"required":["item_index"],"additionalProperties":false}"#,
        ),
        "vox_gamify_collegium_join" => parse_obj(
            r#"{"type":"object","properties":{"collegium_id":{"type":"string","minLength":1}},"required":["collegium_id"],"additionalProperties":false}"#,
        ),
        "vox_gamify_battle_start" => parse_obj(
            r#"{"type":"object","properties":{"companion_name":{"type":"string"},"rule_id":{"type":"string"},"message":{"type":"string"},"file_path":{"type":"string"},"line":{"type":"integer","minimum":1},"context":{"type":"string"}},"required":["companion_name","rule_id","message"],"additionalProperties":false}"#,
        ),
        "vox_gamify_battle_submit" => parse_obj(
            r#"{"type":"object","properties":{"companion_name":{"type":"string"},"code":{"type":"string"},"success":{"type":"boolean"}},"required":["companion_name","code"],"additionalProperties":false}"#,
        ),

        "vox_queue_status" | "vox_budget_status" | "vox_agent_events" | "vox_cost_history"
        | "vox_poll_events" => parse_obj(r#"{"type":"object","additionalProperties":true}"#),

        // ── Skills ───────────────────────────────────────────────────────────
        "vox_skill_add" => parse_obj(
            r#"{"type":"object","properties":{"source":{"type":"string"},"global":{"type":"boolean"},"skill":{"type":"string"}},"required":["source"],"additionalProperties":true}"#,
        ),
        "vox_skill_remove" => parse_obj(
            r#"{"type":"object","properties":{"id":{"type":"string"}},"required":["id"],"additionalProperties":true}"#,
        ),
        "vox_skill_uninstall"
        | "vox_skill_info"
        | "vox_skill_use"
        | "vox_skill_parse"
        | "vox_skill_run"
        | "vox_workspace_mcp_refresh" => {
            parse_obj(r#"{"type":"object","additionalProperties":true}"#)
        }
        "vox_skill_search" => parse_obj(
            r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":true}"#,
        ),
        "vox_skill_install" => parse_obj(
            r#"{"type":"object","additionalProperties":true,"description":"VoxSkillBundle JSON payload fields."}"#,
        ),

        // ── Vox Populi mesh (control-plane preferred, local fallback) ───────
        "vox_mesh_nodes" | "vox_mesh_queue_stats" => {
            parse_obj(r#"{"type":"object","additionalProperties":false}"#)
        }
        "vox_mesh_dispatch" => parse_obj(
            r#"{"type":"object","anyOf":[{"required":["source"]},{"required":["script"]}],"properties":{"source":{"type":"string","minLength":1,"description":".vox source to execute remotely"},"script":{"type":"string","minLength":1,"description":"Alias for `source`"},"node_id":{"type":"string","description":"Optional target node id"},"task_kind":{"type":"string"},"min_vram_mb":{"type":"integer","minimum":0}},"additionalProperties":false}"#,
        ),

        // ── Plugins (installed + catalog) ───────────────────────────────────
        "vox_plugin_list" | "vox_plugin_catalog" => {
            parse_obj(r#"{"type":"object","additionalProperties":false}"#)
        }
        "vox_plugin_info" | "vox_plugin_remove" => parse_obj(
            r#"{"type":"object","properties":{"id":{"type":"string","minLength":1,"description":"Plugin id"}},"required":["id"],"additionalProperties":false}"#,
        ),
        "vox_plugin_install" => parse_obj(
            r#"{"type":"object","oneOf":[{"required":["id"]},{"required":["path"]}],"properties":{"id":{"type":"string","minLength":1,"description":"Catalog plugin id to install"},"path":{"type":"string","minLength":1,"description":"Local plugin directory to install from (alternative to `id`)"}},"additionalProperties":false}"#,
        ),

        // ── gui_ops: models (plan 07) ──
        "vox_models_preference_set" => {
            derived_tool_schema!(crate::gui_ops::models::ModelsPreferenceSetParams)
        }
        // ── gui_ops: prefs (plan 07) ──
        "vox_gui_preference_get" => {
            derived_tool_schema!(crate::gui_ops::prefs::GuiPreferenceGetParams)
        }
        "vox_gui_preference_set" => {
            derived_tool_schema!(crate::gui_ops::prefs::GuiPreferenceSetParams)
        }
        // ── gui_ops: chat (plan 07) ──
        "vox_chat_session_create" => {
            derived_tool_schema!(crate::gui_ops::chat::ChatSessionCreateParams)
        }
        "vox_chat_session_append" => {
            derived_tool_schema!(crate::gui_ops::chat::ChatSessionAppendParams)
        }
        "vox_chat_session_rename" => {
            derived_tool_schema!(crate::gui_ops::chat::ChatSessionRenameParams)
        }
        "vox_chat_session_archive" => {
            derived_tool_schema!(crate::gui_ops::chat::ChatSessionArchiveParams)
        }
        // ── gui_ops: knowledge (plan 07) ──
        "vox_kb_node_delete" => {
            derived_tool_schema!(crate::gui_ops::knowledge::KbNodeDeleteParams)
        }
        "vox_kb_ingest_text" => {
            derived_tool_schema!(crate::gui_ops::knowledge::KbIngestTextParams)
        }
        "vox_kb_ingest_url" => {
            derived_tool_schema!(crate::gui_ops::knowledge::KbIngestUrlParams)
        }
        "vox_kb_import_research_session" => {
            derived_tool_schema!(crate::gui_ops::knowledge::KbImportResearchSessionParams)
        }
        "vox_memory_reindex" => {
            derived_tool_schema!(crate::gui_ops::knowledge::MemoryReindexParams)
        }
        // ── gui_ops: research (plan 07) ──
        "vox_research_flag_misleading" => {
            derived_tool_schema!(crate::gui_ops::research::ResearchFlagParams)
        }
        "vox_research_publish_doc" => {
            derived_tool_schema!(crate::gui_ops::research::ResearchPublishDocParams)
        }
        "vox_research_engine_config_set" => {
            derived_tool_schema!(crate::gui_ops::research::ResearchEngineConfigSetParams)
        }
        "vox_research_engine_probe" => {
            derived_tool_schema!(crate::gui_ops::research::ResearchEngineProbeParams)
        }
        "vox_scientia_publication_claim_review" => {
            derived_tool_schema!(crate::gui_ops::scientia::ClaimReviewParams)
        }
        "vox_scientia_publication_nanopublish" => {
            derived_tool_schema!(crate::gui_ops::scientia::NanopublishParams)
        }
        "vox_scientia_publication_autofill" => {
            derived_tool_schema!(crate::gui_ops::scientia::AutofillParams)
        }
        "vox_scientia_discovery_ack" => {
            derived_tool_schema!(crate::gui_ops::scientia::DiscoveryAckParams)
        }
        // ── gui_ops: orchestrator (plan 07) ──
        "vox_hopper_submit" => {
            derived_tool_schema!(crate::gui_ops::orchestrator::HopperSubmitParams)
        }
        "vox_hopper_reprioritize" => {
            derived_tool_schema!(crate::gui_ops::orchestrator::HopperReprioritizeParams)
        }
        "vox_hopper_cancel" => {
            derived_tool_schema!(crate::gui_ops::orchestrator::HopperItemParams)
        }
        "vox_hopper_mark_done" => {
            derived_tool_schema!(crate::gui_ops::orchestrator::HopperItemParams)
        }
        "vox_plan_node_upsert" => {
            derived_tool_schema!(crate::gui_ops::orchestrator::PlanNodeUpsertParams)
        }
        "vox_plan_nodes_approve" => {
            derived_tool_schema!(crate::gui_ops::orchestrator::PlanNodesApproveParams)
        }
        "vox_gui_run_start" => {
            derived_tool_schema!(crate::gui_ops::dto::orchestrator::StartGuiRunInput)
        }
        "vox_gui_run_finish" => {
            derived_tool_schema!(crate::gui_ops::dto::orchestrator::FinishGuiRunInput)
        }
        // ── gui_ops: harness (plan 07) ──
        "vox_harness_corpus_scan" => {
            derived_tool_schema!(crate::gui_ops::harness::CorpusScanParams)
        }
        "vox_harness_fix_propose" => {
            derived_tool_schema!(crate::gui_ops::harness::FixProposeParams)
        }
        "vox_harness_fix_resolve" => {
            derived_tool_schema!(crate::gui_ops::harness::FixResolveParams)
        }
        "vox_harness_issue_decide" => {
            derived_tool_schema!(crate::gui_ops::harness::IssueDecideParams)
        }
        // ── gui_ops: gamify (plan 07) ──
        "vox_gamify_profile" => {
            derived_tool_schema!(crate::gui_ops::gamify::GamifyProfileParams)
        }
        "vox_gamify_due_actions" => {
            derived_tool_schema!(crate::gui_ops::gamify::GamifyDueActionsParams)
        }
        "vox_gamify_record_event" => {
            derived_tool_schema!(crate::gui_ops::gamify::GamifyRecordEventParams)
        }
        "vox_gamify_leaderboard" => {
            derived_tool_schema!(crate::gui_ops::gamify::GamifyLeaderboardParams)
        }
        "vox_gamify_companions" => {
            derived_tool_schema!(crate::gui_ops::gamify::GamifyCompanionsParams)
        }
        "vox_gamify_discovery_record" => {
            derived_tool_schema!(crate::gui_ops::gamify::GamifyDiscoveryRecordParams)
        }
        _ => return None,
    };
    Some(schema)
}
