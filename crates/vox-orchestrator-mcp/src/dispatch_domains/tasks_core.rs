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
        "vox_visual_rag_query" => true,
        "vox_submit_task" => true,
        "vox_task_status" => true,
        "vox_test_decision" => true,
        "vox_tool_search" => true,
        "vox_pending_approvals" => true,
        "vox_resolve_approval" => true,
        "vox_add_approval_allowlist_entry" => true,
        "vox_list_approval_allowlist" => true,
        "vox_orchestrator_status" => true,
        "vox_orchestrator_persistence_outbox_lifecycle" => true,
        "vox_orchestrator_persistence_outbox_queue" => true,
        "vox_orchestrator_start" => true,
        "vox_spawn_agent" => true,
        "vox_retire_agent" => true,
        "vox_pause_agent" => true,
        "vox_resume_agent" => true,
        "vox_complete_task" => true,
        "vox_fail_task" => true,
        "vox_doubt_task" => true,
        "vox_verify_task_claims" => true,
        "vox_propose_skill" => true,
        "vox_ask_clarification" => true,
        "vox_resolve_feedback" => true,
        "vox_feedback_list" => true,
        "vox_check_file_owner" => true,
        "vox_publish_message" => true,
        "vox_a2a_tasks" => true,
        "vox_exec_time_query" => true,
        "vox_exec_time_record" => true,
        "vox_queue_status" => true,
        "vox_lock_status" => true,
        "vox_budget_status" => true,
        "vox_attention_summary" => true,
        "vox_attention_history" => true,
        "vox_attention_reset" => true,
        "vox_trust_override" => true,
        "vox_handoff_lineage" => true,
        "vox_cancel_task" => true,
        "vox_reorder_task" => true,
        "vox_drain_agent" => true,
        "vox_cost_history" => true,
        "vox_file_graph" => true,
        "vox_config_get" => true,
        "vox_config_set" => true,
        "vox_map_agent_session" => true,
        "vox_poll_events" => true,
        "vox_heartbeat" => true,
        "vox_record_cost" => true,
        "vox_rebalance" => true,
        "vox_agent_events" => true,
        "vox_a2a_send" => true,
        "vox_a2a_inbox" => true,
        "vox_a2a_ack" => true,
        "vox_a2a_broadcast" => true,
        "vox_a2a_history" => true,
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
        "vox_visual_rag_query" => {
            Ok(rag_tools::visual_rag_query(state, serde_json::from_value(args)?).await)
        }
        "vox_submit_task" => {
            Ok(task_tools::submit_task(state, serde_json::from_value(args)?).await)
        }
        "vox_task_status" => {
            Ok(task_tools::task_status(state, serde_json::from_value(args)?).await)
        }
        "vox_test_decision" => {
            Ok(task_tools::test_decision(state, serde_json::from_value(args)?).await)
        }
        "vox_tool_search" => Ok(crate::tool_search::vox_tool_search(serde_json::from_value(
            args,
        )?)),
        // B3 HITL: list / resolve approvals awaiting a human decision (the
        // dangerous-tool gate below parks on these).
        "vox_pending_approvals" => Ok(crate::params::ToolResult::ok(serde_json::json!({
            "approvals": state.pending_approvals.list(),
        }))
        .to_json_compact()),
        "vox_resolve_approval" => {
            let approval_id = args
                .get("approval_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let decision = args
                .get("outcome")
                .or_else(|| args.get("decision"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let outcome = crate::pending_approvals::outcome_from_decision(decision);
            if let Some(refusal) =
                crate::floor_gate::refuse_agent_floor_resolution(state, approval_id, outcome)
            {
                return Ok(refusal);
            }
            let resolved = state.pending_approvals.resolve(approval_id, outcome);
            Ok(crate::params::ToolResult::ok(serde_json::json!({
                "resolved": resolved,
                "approval_id": approval_id,
                "outcome": format!("{outcome:?}"),
            }))
            .to_json_compact())
        }
        // T0.3 Part D: persist a "always allow this tool in this repo"
        // allowlist entry (tier 3 of the dangerous-tool gate's precedence
        // order — see contracts/orchestration/permission-modes.v1.yaml). The
        // GUI's ApprovalsView calls this alongside resolving the current
        // approval when the "always allow" checkbox is set. `repo_id`
        // defaults to the current server's own repository when omitted, but
        // callers should pass it explicitly to avoid ambiguity.
        "vox_add_approval_allowlist_entry" => {
            let tool = args.get("tool").and_then(|v| v.as_str()).unwrap_or("");
            let repo_id = args
                .get("repo_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(state.repository.repository_id.as_str());
            if tool.is_empty() {
                return Ok(
                    crate::params::ToolResult::<()>::err("`tool` (string) is required")
                        .to_json_compact(),
                );
            }
            match crate::approval_allowlist::add_entry(repo_id, tool).await {
                Ok(()) => Ok(crate::params::ToolResult::ok(serde_json::json!({
                    "added": true,
                    "repo_id": repo_id,
                    "tool": tool,
                }))
                .to_json_compact()),
                Err(e) => Ok(crate::params::ToolResult::<()>::err(format!(
                    "failed to persist allowlist entry: {e}"
                ))
                .to_json_compact()),
            }
        }
        // T0.3 Part D: list allowlisted tools for a repo (GUI display of
        // current allowlist state). `repo_id` defaults to this server's own
        // repository when omitted.
        "vox_list_approval_allowlist" => {
            let repo_id = args
                .get("repo_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(state.repository.repository_id.as_str());
            let tools = crate::approval_allowlist::list_for_repo(repo_id).await;
            Ok(crate::params::ToolResult::ok(serde_json::json!({
                "repo_id": repo_id,
                "tools": tools,
            }))
            .to_json_compact())
        }
        "vox_orchestrator_status" => {
            let compact = args
                .get("compact")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if compact {
                let omit_idle_subagents = args
                    .get("omit_idle_subagents")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
                let val = crate::dei_tools::orchestrator_status_compact(state, omit_idle_subagents)
                    .await?;
                Ok(serde_json::to_string(&val)?)
            } else {
                crate::dei_tools::orchestrator_status(state).await
            }
        }
        "vox_orchestrator_persistence_outbox_lifecycle" => {
            Ok(persistence_tools::persistence_outbox_lifecycle(state, args).await)
        }
        "vox_orchestrator_persistence_outbox_queue" => {
            Ok(persistence_tools::persistence_outbox_queue(state, args).await)
        }
        "vox_orchestrator_start" => Ok(crate::dei_tools::orchestrator_start(state).await),
        "vox_spawn_agent" => {
            Ok(crate::dei_tools::spawn_agent(state, serde_json::from_value(args)?).await)
        }
        "vox_retire_agent" => {
            Ok(crate::dei_tools::retire_agent(state, serde_json::from_value(args)?).await)
        }
        "vox_pause_agent" => {
            Ok(crate::dei_tools::pause_agent(state, serde_json::from_value(args)?).await)
        }
        "vox_resume_agent" => {
            Ok(crate::dei_tools::resume_agent(state, serde_json::from_value(args)?).await)
        }
        "vox_complete_task" => {
            Ok(task_tools::complete_task(state, serde_json::from_value(args)?).await)
        }
        "vox_fail_task" => Ok(task_tools::fail_task(state, serde_json::from_value(args)?).await),
        "vox_doubt_task" => Ok(task_tools::doubt_task(state, serde_json::from_value(args)?).await),
        "vox_verify_task_claims" => {
            Ok(receipt_tools::verify_task_claims(state, serde_json::from_value(args)?).await)
        }
        "vox_propose_skill" => {
            Ok(feedback_tools::propose_skill(state, serde_json::from_value(args)?).await)
        }
        "vox_ask_clarification" => {
            Ok(feedback_tools::ask_clarification(state, serde_json::from_value(args)?).await)
        }
        "vox_resolve_feedback" => {
            Ok(feedback_tools::resolve_feedback(state, serde_json::from_value(args)?).await)
        }
        "vox_feedback_list" => {
            Ok(feedback_tools::feedback_list(state, serde_json::from_value(args)?).await)
        }
        "vox_check_file_owner" => Ok(crate::dei_tools::check_file_owner(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await),
        "vox_publish_message" => {
            Ok(task_tools::publish_message(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_tasks" => Ok(introspection_tools::a2a_tasks(state).await?.to_string()),

        // Execution Budget
        "vox_exec_time_query" => Ok(exec_time_tools::exec_time_query(state, args).await),
        "vox_exec_time_record" => Ok(exec_time_tools::exec_time_record(state, args).await),

        "vox_queue_status" => {
            Ok(crate::dei_tools::queue_status(state, serde_json::from_value(args)?).await)
        }
        "vox_lock_status" => Ok(crate::dei_tools::lock_status(state).await),
        "vox_budget_status" => Ok(crate::dei_tools::budget_status(state).await),
        "vox_attention_summary" => {
            Ok(crate::dei_tools::attention_summary(state, serde_json::from_value(args)?).await)
        }
        "vox_attention_history" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let bm = state.orchestrator.budget_manager_handle();
            let events =
                vox_orchestrator::sync_lock::rw_read(&*bm).attention_events_snapshot(limit);
            Ok(crate::params::ToolResult::ok(serde_json::to_value(&events)?).to_json())
        }
        "vox_attention_reset" => {
            let bm = state.orchestrator.budget_manager_handle();
            vox_orchestrator::sync_lock::rw_read(&*bm).reset_attention();
            // T-001: Also reset MCP-level Socrates attention tracking
            state.reset_all_questioning_attention();
            Ok(crate::params::ToolResult::ok(serde_json::json!({
                "reset": true,
                "message": "Attention budget spend and Socrates focus zeroed process-wide."
            }))
            .to_json())
        }
        "vox_trust_override" => {
            let agent_id = args
                .get("agent_id")
                .and_then(|v| v.as_u64())
                .map(|id| vox_orchestrator::types::AgentId(id as _))
                .unwrap_or(vox_orchestrator::types::AgentId(0));
            let trust_score = args
                .get("trust_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            let bm = state.orchestrator.budget_manager_handle();
            vox_orchestrator::sync_lock::rw_read(&*bm).force_trust_score(agent_id, trust_score);
            Ok(crate::params::ToolResult::ok(serde_json::json!({
                "agent_id": agent_id.0,
                "trust_score": trust_score,
                "message": "Trust score overridden."
            }))
            .to_json())
        }
        "vox_handoff_lineage" => {
            Ok(crate::dei_tools::handoff_lineage(state, serde_json::from_value(args)?).await)
        }
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

        "vox_a2a_send" => {
            Ok(crate::a2a_tools::a2a_send(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_inbox" => {
            Ok(crate::a2a_tools::a2a_inbox(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_ack" => Ok(crate::a2a_tools::a2a_ack(state, serde_json::from_value(args)?).await),
        "vox_a2a_broadcast" => {
            Ok(crate::a2a_tools::a2a_broadcast(state, serde_json::from_value(args)?).await)
        }
        "vox_a2a_history" => {
            Ok(crate::a2a_tools::a2a_history(state, serde_json::from_value(args)?).await)
        }
        _ => Err(anyhow::anyhow!("unhandled tool in tasks_core: {}", name)),
    }
}
