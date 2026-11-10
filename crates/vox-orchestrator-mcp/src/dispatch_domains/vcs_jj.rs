#![allow(unused_imports, clippy::match_like_matches_macro)]
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
        "vox_git_log" => true,
        "vox_git_diff" => true,
        "vox_git_status" => true,
        "vox_git_blame" => true,
        "vox_repo_index_status" => true,
        "vox_repo_index_refresh" => true,
        "vox_repo_status" => true,
        "vox_project_init" => true,
        "vox_repo_catalog_list" => true,
        "vox_repo_catalog_refresh" => true,
        "vox_repo_query_text" => true,
        "vox_repo_query_file" => true,
        "vox_repo_query_history" => true,
        "vox_snapshot_list" => true,
        "vox_snapshot_diff" => true,
        "vox_snapshot_restore" => true,
        "vox_oplog" => true,
        "vox_undo" => true,
        "vox_redo" => true,
        "vox_conflicts" => true,
        "vox_resolve_conflict" => true,
        "vox_conflict_diff" => true,
        "vox_workspace_create" => true,
        "vox_workspace_merge" => true,
        "vox_workspace_status" => true,
        "vox_change_create" => true,
        "vox_change_log" => true,
        "vox_vcs_status" => true,
        "vox_my_files" => true,
        "vox_claim_file" => true,
        "vox_transfer_file" => true,
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
        "vox_git_log" => Ok(git_tools::git_log(
            state,
            args.get("max_commits")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize),
        )
        .await),
        "vox_git_diff" => {
            Ok(git_tools::git_diff(state, args.get("path").and_then(|v| v.as_str())).await)
        }
        "vox_git_status" => Ok(git_tools::git_status(state).await),
        "vox_git_blame" => Ok(git_tools::git_blame(
            state,
            args.get("path").and_then(|v| v.as_str()).unwrap_or("."),
        )
        .await),
        "vox_repo_index_status" => Ok(repo_index::repo_index_status(state).await),
        "vox_repo_index_refresh" => Ok(repo_index::repo_index_refresh(state).await),
        "vox_repo_status" => Ok(repo_catalog_tools::repo_status(state).await),
        "vox_project_init" => Ok(project_init_tools::project_init(state, args).await),
        "vox_repo_catalog_list" => Ok(repo_catalog_tools::repo_catalog_list(state).await),
        "vox_repo_catalog_refresh" => Ok(repo_catalog_tools::repo_catalog_refresh(state).await),
        "vox_repo_query_text" => {
            Ok(repo_catalog_tools::repo_query_text(state, serde_json::from_value(args)?).await)
        }
        "vox_repo_query_file" => {
            Ok(repo_catalog_tools::repo_query_file(state, serde_json::from_value(args)?).await)
        }
        "vox_repo_query_history" => {
            Ok(repo_catalog_tools::repo_query_history(state, serde_json::from_value(args)?).await)
        }

        "vox_snapshot_list" => Ok(vcs_tools::snapshot_list(state, args).await),
        "vox_snapshot_diff" => Ok(vcs_tools::snapshot_diff(state, args).await),
        "vox_snapshot_restore" => Ok(vcs_tools::snapshot_restore(state, args).await),
        "vox_oplog" => Ok(vcs_tools::oplog_list(state, args).await),
        "vox_undo" => Ok(vcs_tools::oplog_undo(state, args).await),
        "vox_redo" => Ok(vcs_tools::oplog_redo(state, args).await),
        "vox_conflicts" => Ok(vcs_tools::conflicts_list(state).await),
        "vox_resolve_conflict" => Ok(vcs_tools::resolve_conflict(state, args).await),
        "vox_conflict_diff" => Ok(vcs_tools::conflict_diff(state, args).await),
        "vox_workspace_create" => Ok(vcs_tools::workspace_create(state, args).await),
        "vox_workspace_merge" => Ok(vcs_tools::workspace_merge(state, args).await),
        "vox_workspace_status" => Ok(vcs_tools::workspace_status(state, args).await),
        "vox_change_create" => Ok(vcs_tools::change_create(state, args).await),
        "vox_change_log" => Ok(vcs_tools::change_log(state, args).await),
        "vox_vcs_status" => Ok(crate::dei_tools::vcs_status(state).await),

        // Delegate others to existing modules
        "vox_my_files" => Ok(crate::affinity::my_files(state, serde_json::from_value(args)?).await),
        "vox_claim_file" => {
            Ok(crate::affinity::claim_file(state, serde_json::from_value(args)?).await)
        }
        "vox_transfer_file" => {
            Ok(crate::affinity::transfer_file(state, serde_json::from_value(args)?).await)
        }
        _ => Err(anyhow::anyhow!("unhandled tool in vcs_jj: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_recognizes_vcs_jj_tools() {
        assert!(handles("vox_git_log"));
        assert!(handles("vox_vcs_status"));
        assert!(!handles("vox_chat_message"));
    }
}
