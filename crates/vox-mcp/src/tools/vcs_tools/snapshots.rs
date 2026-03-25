use vox_orchestrator::{AgentId, SnapshotId};

use crate::params::ToolResult;
use crate::server::ServerState;

/// List recent snapshots for an agent (async).
pub async fn snapshot_list(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id_val = args.get("agent_id").and_then(|v| v.as_u64());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let orch = &state.orchestrator;

    let agent = agent_id_val.map(AgentId);
    let handle = orch.snapshot_store_handle();
    let guard = handle.read().unwrap();
    let snaps = guard.list(agent, limit);

    let items: Vec<serde_json::Value> = snaps
        .iter()
        .map(|s| {
            serde_json::json!({
                "id": s.id.to_string(),
                "agent_id": s.agent_id.0.to_string(),
                "timestamp_ms": s.timestamp_ms,
                "description": s.description,
                "file_count": s.files.len(),
            })
        })
        .collect();

    ToolResult::ok(serde_json::json!({ "snapshots": items })).to_json()
}

/// Show diff between two snapshots (async).
pub async fn snapshot_diff(state: &ServerState, args: serde_json::Value) -> String {
    let before_id = args.get("before").and_then(|v| v.as_u64()).unwrap_or(0);
    let after_id = args.get("after").and_then(|v| v.as_u64()).unwrap_or(0);

    let orch = &state.orchestrator;

    let store_handle = orch.snapshot_store_handle();
    let store = store_handle.read().unwrap();
    let before = store.get(SnapshotId(before_id)).cloned();
    let after = store.get(SnapshotId(after_id)).cloned();

    match (before, after) {
        (Some(b), Some(a)) => {
            let diffs = vox_orchestrator::snapshot::SnapshotStore::diff(&b, &a);
            let items: Vec<serde_json::Value> = diffs
                .iter()
                .map(|d| {
                    serde_json::json!({
                        "path": d.path.display().to_string(),
                        "kind": format!("{:?}", d.kind),
                    })
                })
                .collect();
            ToolResult::ok(serde_json::json!({ "diffs": items })).to_json()
        }
        _ => ToolResult::<String>::err("One or both snapshot IDs not found".to_string()).to_json(),
    }
}

/// Restore the workspace to a specific snapshot (async).
pub async fn snapshot_restore(state: &ServerState, args: serde_json::Value) -> String {
    let snapshot_id_str = args
        .get("snapshot_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let snapshot_id = snapshot_id_str
        .strip_prefix("S-")
        .and_then(|s| s.parse::<u64>().ok())
        .map(vox_orchestrator::snapshot::SnapshotId);

    let Some(sid) = snapshot_id else {
        return ToolResult::<String>::err("Invalid snapshot_id format. Expected S-XXXXXX")
            .to_json();
    };

    let orch = &state.orchestrator;

    match orch.restore_fs_snapshot(sid).await {
        Ok(_) => ToolResult::ok(format!("Workspace restored to snapshot {}", sid)).to_json(),
        Err(e) => ToolResult::<String>::err(format!("Restore failed: {}", e)).to_json(),
    }
}
