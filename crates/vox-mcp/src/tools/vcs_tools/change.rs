use vox_orchestrator::{AgentId, SnapshotId};

use crate::params::ToolResult;
use crate::server::ServerState;

/// Create a new logical change (async).
pub async fn change_create(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id = args.get("agent_id").and_then(|v| v.as_u64()).unwrap_or(0);
    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed change");

    let orch = &state.orchestrator;

    let mgr_handle = orch.workspace_manager_handle();
    let mut mgr = match crate::sync_poison::poison_rw_write(mgr_handle.write(), "workspace manager")
    {
        Ok(g) => g,
        Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
    };
    let change_id = mgr.create_change(AgentId(agent_id), description);

    ToolResult::ok(serde_json::json!({
        "change_id": change_id.to_string(),
        "description": description,
    }))
    .to_json()
}

/// Show history of a change (async).
pub async fn change_log(state: &ServerState, args: serde_json::Value) -> String {
    let change_id = args.get("change_id").and_then(|v| v.as_u64());
    let agent_id = args.get("agent_id").and_then(|v| v.as_u64());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let orch = &state.orchestrator;

    if let Some(cid) = change_id {
        let mgr_handle = orch.workspace_manager_handle();
        let mgr = match crate::sync_poison::poison_rw_read(mgr_handle.read(), "workspace manager") {
            Ok(g) => g,
            Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
        };
        match mgr.get_change(vox_orchestrator::workspace::ChangeId(cid))
        {
            Some(change) => ToolResult::ok(serde_json::json!({
                "change_id": change.id.to_string(),
                "description": change.description,
                "agent_id": change.agent_id.to_string(),
                "status": format!("{:?}", change.status),
                "snapshots": change.snapshots.iter().map(|s: &SnapshotId| s.to_string()).collect::<Vec<_>>(),
                "created_ms": change.created_ms,
            }))
            .to_json(),
            None => ToolResult::<String>::err("Change not found".to_string()).to_json(),
        }
    } else {
        let agent = agent_id.map(AgentId);
        let mgr_handle = orch.workspace_manager_handle();
        let mgr = match crate::sync_poison::poison_rw_read(mgr_handle.read(), "workspace manager") {
            Ok(g) => g,
            Err(e) => return ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
        };
        let changes = mgr.list_changes(agent, limit);
        let items: Vec<serde_json::Value> = changes
            .iter()
            .map(|c| {
                serde_json::json!({
                    "change_id": c.id.to_string(),
                    "description": c.description,
                    "agent_id": c.agent_id.to_string(),
                    "status": format!("{:?}", c.status),
                    "snapshot_count": c.snapshots.len(),
                })
            })
            .collect();
        ToolResult::ok(serde_json::json!({ "changes": items })).to_json()
    }
}
