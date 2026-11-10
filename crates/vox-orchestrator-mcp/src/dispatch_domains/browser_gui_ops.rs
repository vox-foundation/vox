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
        #[cfg(feature = "gui-visual-review")]
        "vox_visus_audit" => true,
        #[cfg(feature = "gui-visual-review")]
        "vox_visus_baseline" => true,
        "vox_gui_components" => true,
        "vox_gui_tokens" => true,
        "vox_validate_vuv" => true,
        "vox_validate_diagram" => true,
        "vox_gui_rules" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_open" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_list_pages" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_page_info" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_close" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_back" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_forward" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_reload" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_stop" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_goto" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_click" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_click_xy" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_fill" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_scroll" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_press" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_type" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_set_viewport" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_set_control_lock" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_wait_for" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_text" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_html" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_screenshot" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_screenshot_viewport" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_screencast_frame" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_extract" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_extract_json" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_act" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_open_ex" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_snapshot" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_click_ref" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_fill_ref" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_cookies_export" => true,
        #[cfg(feature = "heavy-browser")]
        "vox_browser_cookies_import" => true,
        "vox_models_preference_set" => true,
        "vox_gui_preference_get" => true,
        "vox_gui_preference_set" => true,
        "vox_chat_session_create" => true,
        "vox_chat_session_append" => true,
        "vox_chat_session_rename" => true,
        "vox_chat_session_archive" => true,
        "vox_kb_node_delete" => true,
        "vox_kb_ingest_text" => true,
        "vox_kb_ingest_url" => true,
        "vox_kb_import_research_session" => true,
        "vox_memory_reindex" => true,
        "vox_research_flag_misleading" => true,
        "vox_research_publish_doc" => true,
        "vox_research_engine_config_set" => true,
        "vox_research_engine_probe" => true,
        "vox_scientia_publication_claim_review" => true,
        "vox_scientia_publication_nanopublish" => true,
        "vox_scientia_publication_autofill" => true,
        "vox_scientia_discovery_ack" => true,
        "vox_hopper_submit" => true,
        "vox_hopper_reprioritize" => true,
        "vox_hopper_cancel" => true,
        "vox_hopper_mark_done" => true,
        "vox_plan_node_upsert" => true,
        "vox_plan_nodes_approve" => true,
        "vox_gui_run_start" => true,
        "vox_gui_run_finish" => true,
        "vox_harness_corpus_scan" => true,
        "vox_harness_fix_propose" => true,
        "vox_harness_fix_resolve" => true,
        "vox_harness_issue_decide" => true,
        "vox_gamify_profile" => true,
        "vox_gamify_due_actions" => true,
        "vox_gamify_record_event" => true,
        "vox_gamify_leaderboard" => true,
        "vox_gamify_companions" => true,
        "vox_gamify_discovery_record" => true,
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
        #[cfg(feature = "gui-visual-review")]
        "vox_visus_audit" => {
            Ok(visus_tools::vox_visus_audit(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "gui-visual-review")]
        "vox_visus_baseline" => {
            Ok(visus_tools::vox_visus_baseline(state, serde_json::from_value(args)?).await)
        }
        "vox_gui_components" => {
            Ok(crate::gui_registry_tools::vox_gui_components(state, args).await)
        }
        "vox_gui_tokens" => Ok(crate::gui_registry_tools::vox_gui_tokens(state, args).await),
        "vox_validate_vuv" => Ok(crate::gui_registry_tools::vox_validate_vuv(state, args).await),
        "vox_validate_diagram" => Ok(crate::diagram_tools::vox_validate_diagram(state, args).await),
        "vox_gui_rules" => Ok(crate::gui_registry_tools::vox_gui_rules(state, args).await),

        #[cfg(feature = "heavy-browser")]
        "vox_browser_open" => {
            Ok(browser_tools::browser_open(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_list_pages" => Ok(browser_tools::browser_list_pages(state, args).await),
        #[cfg(feature = "heavy-browser")]
        "vox_browser_page_info" => {
            Ok(browser_tools::browser_page_info(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_close" => {
            Ok(browser_tools::browser_close(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_back" => {
            Ok(browser_tools::browser_back(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_forward" => {
            Ok(browser_tools::browser_forward(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_reload" => {
            Ok(browser_tools::browser_reload(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_stop" => {
            Ok(browser_tools::browser_stop(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_goto" => {
            Ok(browser_tools::browser_goto(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_click" => {
            Ok(browser_tools::browser_click(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_click_xy" => {
            Ok(browser_tools::browser_click_xy(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_fill" => {
            Ok(browser_tools::browser_fill(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_scroll" => {
            Ok(browser_tools::browser_scroll(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_press" => {
            Ok(browser_tools::browser_press(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_type" => {
            Ok(browser_tools::browser_type(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_set_viewport" => {
            Ok(browser_tools::browser_set_viewport(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_set_control_lock" => {
            Ok(browser_tools::browser_set_control_lock(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_wait_for" => {
            Ok(browser_tools::browser_wait_for(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_text" => {
            Ok(browser_tools::browser_text(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_html" => {
            Ok(browser_tools::browser_html(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_screenshot" => {
            Ok(browser_tools::browser_screenshot(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_screenshot_viewport" => Ok(browser_tools::browser_screenshot_viewport(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        #[cfg(feature = "heavy-browser")]
        "vox_browser_screencast_frame" => {
            Ok(browser_tools::browser_screencast_frame(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_extract" => {
            Ok(browser_tools::browser_extract(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_extract_json" => {
            Ok(browser_tools::browser_extract_json(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_act" => {
            Ok(browser_tools::browser_act(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_open_ex" => {
            Ok(browser_tools::browser_open_ex(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_snapshot" => {
            Ok(browser_tools::browser_snapshot(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_click_ref" => {
            Ok(browser_tools::browser_click_ref(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_fill_ref" => {
            Ok(browser_tools::browser_fill_ref(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_cookies_export" => {
            Ok(browser_tools::browser_cookies_export(state, serde_json::from_value(args)?).await)
        }
        #[cfg(feature = "heavy-browser")]
        "vox_browser_cookies_import" => {
            Ok(browser_tools::browser_cookies_import(state, serde_json::from_value(args)?).await)
        }
        // ── gui_ops: models (plan 07) ──
        "vox_models_preference_set" => Ok(crate::gui_ops::models::models_preference_set(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        // ── gui_ops: prefs (plan 07) ──
        "vox_gui_preference_get" => Ok(crate::gui_ops::prefs::gui_preference_get(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_gui_preference_set" => Ok(crate::gui_ops::prefs::gui_preference_set(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        // ── gui_ops: chat (plan 07) ──
        "vox_chat_session_create" => Ok(crate::gui_ops::chat::chat_session_create(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_chat_session_append" => Ok(crate::gui_ops::chat::chat_session_append(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_chat_session_rename" => Ok(crate::gui_ops::chat::chat_session_rename(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_chat_session_archive" => Ok(crate::gui_ops::chat::chat_session_archive(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        // ── gui_ops: knowledge (plan 07) ──
        "vox_kb_node_delete" => Ok(crate::gui_ops::knowledge::kb_node_delete(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_kb_ingest_text" => Ok(crate::gui_ops::knowledge::kb_ingest_text(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_kb_ingest_url" => Ok(crate::gui_ops::knowledge::kb_ingest_url(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_kb_import_research_session" => {
            Ok(crate::gui_ops::knowledge::kb_import_research_session(
                state,
                serde_json::from_value(args)?,
            )
            .await)
        }
        "vox_memory_reindex" => Ok(crate::gui_ops::knowledge::memory_reindex(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        // ── gui_ops: research (plan 07) ──
        "vox_research_flag_misleading" => Ok(crate::gui_ops::research::flag_misleading(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_research_publish_doc" => {
            Ok(crate::gui_ops::research::publish_doc(state, serde_json::from_value(args)?).await)
        }
        "vox_research_engine_config_set" => Ok(crate::gui_ops::research::engine_config_set(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_research_engine_probe" => {
            Ok(crate::gui_ops::research::engine_probe(state, serde_json::from_value(args)?).await)
        }
        "vox_scientia_publication_claim_review" => {
            Ok(crate::gui_ops::scientia::claim_review(state, serde_json::from_value(args)?).await)
        }
        "vox_scientia_publication_nanopublish" => {
            Ok(crate::gui_ops::scientia::nanopublish(state, serde_json::from_value(args)?).await)
        }
        "vox_scientia_publication_autofill" => {
            Ok(crate::gui_ops::scientia::autofill(state, serde_json::from_value(args)?).await)
        }
        "vox_scientia_discovery_ack" => {
            Ok(crate::gui_ops::scientia::discovery_ack(state, serde_json::from_value(args)?).await)
        }
        // ── gui_ops: orchestrator (plan 07) ──
        "vox_hopper_submit" => Ok(crate::gui_ops::orchestrator::hopper_submit(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_hopper_reprioritize" => Ok(crate::gui_ops::orchestrator::hopper_reprioritize(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_hopper_cancel" => Ok(crate::gui_ops::orchestrator::hopper_cancel(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_hopper_mark_done" => Ok(crate::gui_ops::orchestrator::hopper_mark_done(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_plan_node_upsert" => Ok(crate::gui_ops::orchestrator::plan_node_upsert(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_plan_nodes_approve" => Ok(crate::gui_ops::orchestrator::plan_nodes_approve(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_gui_run_start" => Ok(crate::gui_ops::orchestrator::gui_run_start(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_gui_run_finish" => Ok(crate::gui_ops::orchestrator::gui_run_finish(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        // ── gui_ops: harness (plan 07) ──
        "vox_harness_corpus_scan" => {
            Ok(crate::gui_ops::harness::corpus_scan(state, serde_json::from_value(args)?).await)
        }
        "vox_harness_fix_propose" => {
            Ok(crate::gui_ops::harness::fix_propose(state, serde_json::from_value(args)?).await)
        }
        "vox_harness_fix_resolve" => {
            Ok(crate::gui_ops::harness::fix_resolve(state, serde_json::from_value(args)?).await)
        }
        "vox_harness_issue_decide" => {
            Ok(crate::gui_ops::harness::issue_decide(state, serde_json::from_value(args)?).await)
        }
        // ── gui_ops: gamify (plan 07) ──
        "vox_gamify_profile" => {
            Ok(crate::gui_ops::gamify::gamify_profile(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_due_actions" => Ok(crate::gui_ops::gamify::gamify_due_actions(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_gamify_record_event" => Ok(crate::gui_ops::gamify::gamify_record_event(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_gamify_leaderboard" => Ok(crate::gui_ops::gamify::gamify_leaderboard(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_gamify_companions" => Ok(crate::gui_ops::gamify::gamify_companions(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        "vox_gamify_discovery_record" => Ok(crate::gui_ops::gamify::gamify_discovery_record(
            state,
            serde_json::from_value(args)?,
        )
        .await),
        _ => Err(anyhow::anyhow!(
            "unhandled tool in browser_gui_ops: {}",
            name
        )),
    }
}
