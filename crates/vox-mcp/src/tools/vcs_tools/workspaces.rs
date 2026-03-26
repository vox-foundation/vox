use vox_orchestrator::AgentId;

use crate::params::ToolResult;
use crate::server::ServerState;

const REM_VCS_LOCK: &str =
    "Retry; persistent poisoned-lock errors usually need an MCP restart.";
const REM_WORKSPACE_NONE: &str =
    "Create a workspace with `workspace_create` or verify `agent_id` matches an agent with an active workspace.";

/// Create a workspace for an agent (async).
pub async fn workspace_create(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id = args.get("agent_id").and_then(|v| v.as_u64()).unwrap_or(0);

    let orch = &state.orchestrator;
    let base_id = {
        let snapshot_store = orch.snapshot_store_handle();
        let mut store_guard =
            match crate::sync_poison::poison_rw_write(snapshot_store.write(), "snapshot store") {
                Ok(g) => g,
                Err(e) => {
                    return ToolResult::<serde_json::Value>::err_with_remediation(e.to_string(), REM_VCS_LOCK)
                        .to_json();
                }
            };
        store_guard.take_snapshot(AgentId(agent_id), &[], "workspace base".to_string())
    };

    let mgr_handle = orch.workspace_manager_handle();
    let mut mgr = match crate::sync_poison::poison_rw_write(mgr_handle.write(), "workspace manager")
    {
        Ok(g) => g,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err_with_remediation(e.to_string(), REM_VCS_LOCK)
                .to_json();
        }
    };
    let ws = mgr.create_workspace(AgentId(agent_id), base_id).clone();

    ToolResult::ok(serde_json::json!({
        "workspace_created": true,
        "agent_id": ws.agent_id.to_string(),
        "base_snapshot": base_id.to_string(),
    }))
    .to_json()
}

/// Show workspace status (async).
pub async fn workspace_status(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id = args.get("agent_id").and_then(|v| v.as_u64()).unwrap_or(0);

    let orch = &state.orchestrator;

    let mgr_handle = orch.workspace_manager_handle();
    let mgr = match crate::sync_poison::poison_rw_read(mgr_handle.read(), "workspace manager") {
        Ok(g) => g,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err_with_remediation(e.to_string(), REM_VCS_LOCK)
                .to_json();
        }
    };
    match mgr.get_workspace(AgentId(agent_id)) {
        Some(ws) => {
            let paths: Vec<String> = ws
                .modified_paths()
                .iter()
                .map(|p| p.display().to_string())
                .collect();
            ToolResult::ok(serde_json::json!({
                "has_workspace": true,
                "modified_files": paths,
                "modified_count": ws.modified_count(),
                "base_snapshot": ws.base_snapshot.to_string(),
                "active_change": ws.active_change.map(|c: vox_orchestrator::workspace::ChangeId| c.to_string()),
            }))
            .to_json()
        }
        None => ToolResult::ok(serde_json::json!({ "has_workspace": false })).to_json(),
    }
}

/// Merge workspace back to main (async).
pub async fn workspace_merge(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id = args.get("agent_id").and_then(|v| v.as_u64()).unwrap_or(0);

    let orch = &state.orchestrator;

    let mgr_handle = orch.workspace_manager_handle();
    let mut mgr = match crate::sync_poison::poison_rw_write(mgr_handle.write(), "workspace manager")
    {
        Ok(g) => g,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err_with_remediation(e.to_string(), REM_VCS_LOCK)
                .to_json();
        }
    };
    match mgr.destroy_workspace(AgentId(agent_id)) {
        Some(ws) => {
            let count = ws.modified_count();
            ToolResult::ok(serde_json::json!({
                "merged": true,
                "files_merged": count,
            }))
            .to_json()
        }
        None => {
            ToolResult::<String>::err_with_remediation(
                "No active workspace for this agent".to_string(),
                REM_WORKSPACE_NONE,
            )
            .to_json()
        }
    }
}
