use vox_orchestrator::AgentId;

use crate::params::ToolResult;
use crate::server::ServerState;

/// Create a workspace for an agent (async).
pub async fn workspace_create(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id = args.get("agent_id").and_then(|v| v.as_u64()).unwrap_or(0);

    let orch = &state.orchestrator;

    let orch = &state.orchestrator;
    let base_id = {
        let snapshot_store = orch.snapshot_store_handle();
        let mut store_guard = snapshot_store.write().unwrap();
        store_guard.take_snapshot(AgentId(agent_id), &[], "workspace base".to_string())
    };

    let mgr_handle = orch.workspace_manager_handle();
    let ws = mgr_handle
        .write()
        .unwrap()
        .create_workspace(AgentId(agent_id), base_id)
        .clone();

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
    let mgr = mgr_handle.read().unwrap();
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
    let mut mgr = mgr_handle.write().unwrap();
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
            ToolResult::<String>::err("No active workspace for this agent".to_string()).to_json()
        }
    }
}
