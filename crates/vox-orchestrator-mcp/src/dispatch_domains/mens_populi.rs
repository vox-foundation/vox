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
        "vox_openclaw_list_remote" => true,
        "vox_openclaw_search_remote" => true,
        "vox_openclaw_import_skill" => true,
        "vox_openclaw_discover" => true,
        "vox_openclaw_health" => true,
        "vox_openclaw_gateway_call" => true,
        "vox_openclaw_subscriptions" => true,
        "vox_openclaw_subscribe" => true,
        "vox_openclaw_unsubscribe" => true,
        "vox_openclaw_notify" => true,
        "vox_agent_list_remote" => true,
        "vox_agent_gateway_call" => true,
        "vox_agent_subscriptions" => true,
        "vox_agent_subscribe" => true,
        "vox_agent_unsubscribe" => true,
        "vox_agent_notify" => true,
        "vox_schola_submit" => true,
        "vox_check_mood" => true,
        "vox_gamify_notifications_list" => true,
        "vox_gamify_progress_snapshot" => true,
        "vox_gamify_notification_ack" => true,
        "vox_gamify_notifications_ack_all" => true,
        "vox_gamify_quest_list" => true,
        "vox_gamify_shop_catalog" => true,
        "vox_gamify_shop_buy" => true,
        "vox_gamify_collegium_join" => true,
        "vox_gamify_battle_start" => true,
        "vox_gamify_battle_submit" => true,
        "vox_agent_status" => true,
        "vox_agent_continue" => true,
        "vox_agent_assess" => true,
        "vox_agent_handoff" => true,
        "vox_skill_install" => true,
        "vox_skill_uninstall" => true,
        "vox_skill_list" => true,
        "vox_skill_search" => true,
        "vox_skill_info" => true,
        "vox_skill_parse" => true,
        "vox_skill_use" => true,
        "vox_skill_run" => true,
        "vox_skill_discover" => true,
        "vox_skill_add" => true,
        "vox_skill_remove" => true,
        "vox_workspace_mcp_refresh" => true,
        "vox_plugin_list" => true,
        "vox_plugin_catalog" => true,
        "vox_plugin_info" => true,
        "vox_plugin_install" => true,
        "vox_plugin_remove" => true,
        "vox_populi_local_status" => true,
        "vox_mesh_nodes" => true,
        "vox_mesh_queue_stats" => true,
        "vox_mesh_dispatch" => true,
        "vox_benchmark_list" => true,
        "vox_benchmark_record" => true,
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
        "vox_openclaw_list_remote" => Ok(openclaw_tools::openclaw_list_remote(state).await),
        "vox_openclaw_search_remote" => {
            Ok(openclaw_tools::openclaw_search_remote(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_import_skill" => {
            Ok(openclaw_tools::openclaw_import_skill(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_discover" => Ok(openclaw_tools::openclaw_discover(state).await),
        "vox_openclaw_health" => Ok(openclaw_tools::openclaw_health(state).await),
        "vox_openclaw_gateway_call" => {
            Ok(openclaw_tools::openclaw_gateway_call(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_subscriptions" => Ok(openclaw_tools::openclaw_subscriptions(state).await),
        "vox_openclaw_subscribe" => {
            Ok(openclaw_tools::openclaw_subscribe(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_unsubscribe" => {
            Ok(openclaw_tools::openclaw_unsubscribe(state, serde_json::from_value(args)?).await)
        }
        "vox_openclaw_notify" => {
            Ok(openclaw_tools::openclaw_notify(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_list_remote" => Ok(agent_tools::agent_list_remote(state).await),
        "vox_agent_gateway_call" => {
            Ok(agent_tools::agent_gateway_call(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_subscriptions" => Ok(agent_tools::agent_subscriptions(state).await),
        "vox_agent_subscribe" => {
            Ok(agent_tools::agent_subscribe(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_unsubscribe" => {
            Ok(agent_tools::agent_unsubscribe(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_notify" => {
            Ok(agent_tools::agent_notify(state, serde_json::from_value(args)?).await)
        }

        "vox_schola_submit" => {
            Ok(training_tools::train_submit(state, serde_json::from_value(args)?).await)
        }

        "vox_check_mood" => {
            Ok(crate::gamify::check_mood(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_notifications_list" => {
            Ok(crate::gamify::ludus_notifications_list(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_progress_snapshot" => {
            Ok(crate::gamify::ludus_progress_snapshot(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_notification_ack" => {
            Ok(crate::gamify::ludus_notification_ack(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_notifications_ack_all" => {
            Ok(crate::gamify::ludus_notifications_ack_all(state).await)
        }
        "vox_gamify_quest_list" => {
            Ok(crate::gamify::ludus_quest_list(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_shop_catalog" => {
            Ok(crate::gamify::ludus_shop_catalog(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_shop_buy" => {
            Ok(crate::gamify::ludus_shop_buy(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_collegium_join" => {
            Ok(crate::gamify::ludus_collegium_join(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_battle_start" => {
            Ok(crate::gamify::ludus_battle_start(state, serde_json::from_value(args)?).await)
        }
        "vox_gamify_battle_submit" => {
            Ok(crate::gamify::ludus_battle_submit(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_status" => {
            Ok(crate::gamify::agent_status(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_continue" => {
            Ok(crate::gamify::agent_continue(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_assess" => {
            Ok(crate::gamify::agent_assess(state, serde_json::from_value(args)?).await)
        }
        "vox_agent_handoff" => {
            Ok(crate::gamify::agent_handoff(state, serde_json::from_value(args)?).await)
        }

        "vox_skill_install" => {
            Ok(crate::skills::skill_install(state, serde_json::from_value(args)?).await)
        }
        "vox_skill_uninstall" => {
            Ok(crate::skills::skill_uninstall(state, serde_json::from_value(args)?).await)
        }
        "vox_skill_list" => Ok(crate::skills::skill_list(state)),
        "vox_skill_search" => Ok(crate::skills::skill_search(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_info" => Ok(crate::skills::skill_info(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_parse" => Ok(crate::skills::skill_parse(serde_json::from_value(args)?)),
        "vox_skill_use" => Ok(crate::skills::skill_use(
            state,
            serde_json::from_value(args)?,
        )),
        "vox_skill_run" => Ok(crate::skills::skill_run(state, serde_json::from_value(args)?).await),
        "vox_skill_discover" => Ok(crate::skills::skill_discover(state)),
        "vox_skill_add" => Ok(crate::skills::skill_add(state, serde_json::from_value(args)?).await),
        "vox_skill_remove" => {
            Ok(crate::skills::skill_remove(state, serde_json::from_value(args)?).await)
        }

        "vox_workspace_mcp_refresh" => {
            let root = state
                .workspace_root
                .clone()
                .unwrap_or_else(|| state.repository.root.clone());
            let config = crate::workspace_mcp::load_scan_config(&root);
            let load = crate::workspace_mcp::WorkspaceMcpLoader::load_repo(&root, &config)
                .map_err(|e| anyhow::anyhow!(e))?;
            for err in &load.errors {
                tracing::warn!(
                    path = %err.path.display(),
                    error = %err.message,
                    "workspace MCP scan skipped file"
                );
            }
            if !load.surface.shadowed.is_empty() {
                tracing::warn!(
                    shadowed = ?load.surface.shadowed,
                    "workspace MCP tools shadowed by static catalog"
                );
            }
            *state.workspace_mcp.write() = load.surface.clone();
            let errors: Vec<serde_json::Value> = load
                .errors
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "path": e.path.display().to_string(),
                        "message": e.message,
                    })
                })
                .collect();
            Ok(ToolResult::ok(serde_json::json!({
                "tool_count": load.surface.tool_count(),
                "resource_count": load.surface.resource_count(),
                "shadowed": load.surface.shadowed,
                "duplicate_tools": load.surface.duplicate_tools,
                "duplicate_resources": load.surface.duplicate_resources,
                "errors": errors,
            }))
            .to_json())
        }

        "vox_plugin_list" => Ok(crate::plugins::plugin_list(state).await),
        "vox_plugin_catalog" => Ok(crate::plugins::plugin_catalog()),
        "vox_plugin_info" => {
            Ok(crate::plugins::plugin_info(state, serde_json::from_value(args)?).await)
        }
        "vox_plugin_install" => {
            Ok(crate::plugins::plugin_install(state, serde_json::from_value(args)?).await)
        }
        "vox_plugin_remove" => {
            Ok(crate::plugins::plugin_remove(state, serde_json::from_value(args)?).await)
        }

        "vox_populi_local_status" => Ok(populi_tools::mesh_local_status(args)?),
        "vox_mesh_nodes" => Ok(populi_tools::mesh_nodes(state, args).await?),
        "vox_mesh_queue_stats" => Ok(populi_tools::mesh_queue_stats(state, args).await?),
        "vox_mesh_dispatch" => Ok(populi_tools::mesh_dispatch(state, args).await?),

        "vox_benchmark_list" => {
            Ok(benchmark_tools::benchmark_list(state, serde_json::from_value(args)?).await)
        }
        "vox_benchmark_record" => {
            Ok(benchmark_tools::benchmark_record(state, serde_json::from_value(args)?).await)
        }
        _ => Err(anyhow::anyhow!("unhandled tool in mens_populi: {}", name)),
    }
}
