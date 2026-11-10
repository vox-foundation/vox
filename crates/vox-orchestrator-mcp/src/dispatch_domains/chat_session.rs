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
        "vox_chat_message" => true,
        "vox_chat_history" => true,
        "vox_questioning_submit_answer" => true,
        "vox_questioning_pending" => true,
        "vox_questioning_sync_ssot" => true,
        "vox_inline_edit" => true,
        "vox_plan" => true,
        "vox_replan" => true,
        "vox_plan_status" => true,
        "vox_plan_list_sessions" => true,
        "vox_plan_resume" => true,
        "vox_ghost_text" => true,
        "vox_ask_agent" => true,
        "vox_answer_question" => true,
        "vox_pending_questions" => true,
        "vox_broadcast" => true,
        "vox_memory_store" => true,
        "vox_memory_recall" => true,
        "vox_memory_search" => true,
        "vox_semantic_fs_discover" => true,
        "vox_memory_log" => true,
        "vox_memory_list_keys" => true,
        "vox_knowledge_query" => true,
        "vox_research_run" => true,
        "vox_research_start" => true,
        "vox_research_status" => true,
        "vox_research_get" => true,
        "vox_research_search" => true,
        "vox_memory_save_db" => true,
        "vox_memory_recall_db" => true,
        "vox_compaction_status" => true,
        "vox_session_create" => true,
        "vox_session_list" => true,
        "vox_session_reset" => true,
        "vox_session_compact" => true,
        "vox_session_info" => true,
        "vox_session_cleanup" => true,
        "vox_preference_get" => true,
        "vox_preference_set" => true,
        "vox_preference_list" => true,
        "vox_learn_pattern" => true,
        "vox_behavior_record" => true,
        "vox_behavior_summary" => true,
        "vox_set_context" => true,
        "vox_get_context" => true,
        "vox_list_context" => true,
        "vox_context_budget" => true,
        "vox_set_agent_budget" => true,
        "vox_emergency_stop" => true,
        "vox_handoff_context" => true,
        "vox_chat_compact_up_to" => true,
        "vox_chat_edit_message_in_place" => true,
        "vox_chat_regenerate" => true,
        "vox_chat_stream_resume" => true,
        "vox_chat_stream_cancel" => true,
        "vox_chat_queue_steer" => true,
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
        "vox_semantic_fs_discover" => {
            Ok(crate::memory::semantic_fs_discover_mcp(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_log" => {
            Ok(crate::memory::memory_daily_log(state, serde_json::from_value(args)?).await)
        }
        "vox_memory_list_keys" => Ok(crate::memory::memory_list_keys(state).await),
        "vox_knowledge_query" => {
            Ok(crate::memory::knowledge_query(state, serde_json::from_value(args)?).await)
        }
        "vox_research_run" => {
            Ok(crate::memory::research_run(state, serde_json::from_value(args)?).await)
        }
        "vox_research_start" => {
            Ok(crate::memory::research_start(state, serde_json::from_value(args)?).await)
        }
        "vox_research_status" => {
            Ok(crate::memory::research_status(state, serde_json::from_value(args)?).await)
        }
        "vox_research_get" => {
            Ok(crate::memory::research_get(state, serde_json::from_value(args)?).await)
        }
        "vox_research_search" => {
            Ok(crate::memory::research_search(state, serde_json::from_value(args)?).await)
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

        "vox_set_context" => {
            Ok(crate::mcp_context::set_context(state, serde_json::from_value(args)?).await)
        }
        "vox_get_context" => {
            Ok(crate::mcp_context::get_context(state, serde_json::from_value(args)?).await)
        }
        "vox_list_context" => {
            Ok(crate::mcp_context::list_context(state, serde_json::from_value(args)?).await)
        }
        "vox_context_budget" => {
            Ok(crate::mcp_context::context_budget(state, serde_json::from_value(args)?).await)
        }
        "vox_set_agent_budget" => {
            Ok(crate::mcp_context::set_agent_budget(state, serde_json::from_value(args)?).await)
        }
        "vox_emergency_stop" => {
            Ok(crate::mcp_context::emergency_stop(state, serde_json::from_value(args)?).await)
        }
        "vox_handoff_context" => {
            Ok(crate::mcp_context::handoff_context(state, serde_json::from_value(args)?).await)
        }
        // ── T3 Wave 4 chat transcript & stream control tools (`T3-HIST-BATTERY-02`, `T3-LOOP-CHECKPOINT-03..04`, `T3-REGEN-WORKTREE-01`) ──
        "vox_chat_compact_up_to" => Ok(crate::chat_tools::chat_compact_up_to(state, args).await),
        "vox_chat_edit_message_in_place" => {
            Ok(crate::chat_tools::chat_edit_message_in_place(state, args).await)
        }
        "vox_chat_regenerate" => Ok(crate::chat_tools::chat_regenerate(state, args).await),
        "vox_chat_stream_resume" => Ok(crate::chat_tools::chat_stream_resume(state, args).await),
        "vox_chat_stream_cancel" => Ok(crate::chat_tools::chat_stream_cancel(state, args).await),
        "vox_chat_queue_steer" => Ok(crate::chat_tools::chat_queue_steer(state, args).await),
        _ => Err(anyhow::anyhow!("unhandled tool in chat_session: {}", name)),
    }
}
