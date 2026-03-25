//! JJ-inspired VCS tool handlers for the Vox MCP server.
//!
//! Covers: snapshots, operation log (oplog), conflicts, workspaces, and change tracking.

use vox_orchestrator::{AgentId, ConflictId, ConflictResolution, OperationId, SnapshotId, TaskId};

use crate::params::ToolResult;
use crate::server::ServerState;

fn parse_snapshot_id_value(v: Option<&serde_json::Value>) -> Option<SnapshotId> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(SnapshotId(n));
    }
    let s = v.as_str()?;
    let raw = s.strip_prefix("S-").unwrap_or(s);
    raw.parse::<u64>().ok().map(SnapshotId)
}

fn parse_operation_id_value(v: Option<&serde_json::Value>) -> Option<OperationId> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(OperationId(n));
    }
    let s = v.as_str()?;
    let raw = s.strip_prefix("OP-").unwrap_or(s);
    raw.parse::<u64>().ok().map(OperationId)
}

fn parse_conflict_id_value(v: Option<&serde_json::Value>) -> Option<ConflictId> {
    let v = v?;
    if let Some(n) = v.as_u64() {
        return Some(ConflictId(n));
    }
    let s = v.as_str()?;
    let raw = s.strip_prefix("C-").unwrap_or(s);
    raw.parse::<u64>().ok().map(ConflictId)
}

// ---------------------------------------------------------------------------
// Snapshots
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Operation log
// ---------------------------------------------------------------------------

/// List recent operations from the operation log (async).
pub async fn oplog_list(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id_val = args.get("agent_id").and_then(|v| v.as_u64());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let orch = &state.orchestrator;
    let agent = agent_id_val.map(AgentId);
    let handle = orch.oplog_handle();
    let guard = handle.read().unwrap();
    let ops = guard.list(agent, limit);

    let items: Vec<serde_json::Value> = ops
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "agent_id": e.agent_id.0.to_string(),
                "timestamp_ms": e.timestamp_ms,
                "kind": format!("{:?}", e.kind),
                "description": e.description,
                "undone": e.undone,
            })
        })
        .collect();

    ToolResult::ok(serde_json::json!({ "operations": items })).to_json()
}

/// Undo an operation (async).
pub async fn oplog_undo(state: &ServerState, args: serde_json::Value) -> String {
    let Some(op_id) = parse_operation_id_value(args.get("operation_id")) else {
        return ToolResult::<String>::err(
            "Missing or invalid operation_id (number or OP-XXXXXX string)".to_string(),
        )
        .to_json();
    };

    let orch = &state.orchestrator;

    match orch.undo_operation(op_id).await {
        Ok(_) => ToolResult::ok(serde_json::json!({
            "undone": true,
            "operation_id": op_id.0,
        }))
        .to_json(),
        Err(e) => ToolResult::<String>::err(format!("Undo failed: {}", e)).to_json(),
    }
}

/// Redo an operation (async).
pub async fn oplog_redo(state: &ServerState, args: serde_json::Value) -> String {
    let Some(op_id) = parse_operation_id_value(args.get("operation_id")) else {
        return ToolResult::<String>::err(
            "Missing or invalid operation_id (number or OP-XXXXXX string)".to_string(),
        )
        .to_json();
    };

    let orch = &state.orchestrator;

    match orch.redo_operation(op_id).await {
        Ok(_) => ToolResult::ok(serde_json::json!({
            "redone": true,
            "operation_id": op_id.0,
        }))
        .to_json(),
        Err(e) => ToolResult::<String>::err(format!("Redo failed: {}", e)).to_json(),
    }
}

// ---------------------------------------------------------------------------
// Conflicts
// ---------------------------------------------------------------------------

/// List active conflicts (async).
pub async fn conflicts_list(state: &ServerState) -> String {
    let orch = &state.orchestrator;
    let cm_lock = orch.conflict_manager_handle();
    let mgr = cm_lock.read().unwrap();
    let conflicts = mgr.active_conflicts();

    let items: Vec<serde_json::Value> = conflicts
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id.to_string(),
                "path": c.path.display().to_string(),
                "sides": c.sides.len(),
                "created_ms": c.created_ms,
            })
        })
        .collect();

    ToolResult::ok(serde_json::json!({
        "active_conflicts": items,
        "total_active": items.len(),
    }))
    .to_json()
}

/// Show an N-way conflict diff for a specific conflict (async).
pub async fn conflict_diff(state: &ServerState, args: serde_json::Value) -> String {
    let conflict_id = if let Some(raw) = args.get("conflict_id").and_then(|v| v.as_u64()) {
        ConflictId(raw)
    } else if let Some(raw) = args.get("conflict_id").and_then(|v| v.as_str()) {
        let Some(id) = raw
            .strip_prefix("C-")
            .and_then(|s| s.parse::<u64>().ok())
            .map(ConflictId)
        else {
            return ToolResult::<String>::err("Invalid conflict_id format. Expected C-XXXXXX")
                .to_json();
        };
        id
    } else {
        return ToolResult::<String>::err(
            "Missing conflict_id (number or C-XXXXXX string)".to_string(),
        )
        .to_json();
    };

    let orch = &state.orchestrator;
    let ss_lock = orch.snapshot_store_handle();
    let conflict = {
        let mgr_handle = orch.conflict_manager_handle();
        let mgr = mgr_handle.read().unwrap();
        mgr.get(conflict_id).cloned()
    };

    let Some(c) = conflict else {
        return ToolResult::<String>::err(format!("Conflict {} not found", conflict_id)).to_json();
    };

    let store = ss_lock.read().unwrap();
    let base = c.base_snapshot.and_then(|sid| store.get(sid).cloned());
    let mut unique_hashes = std::collections::BTreeSet::new();
    let mut sides = Vec::new();

    for (idx, side) in c.sides.iter().enumerate() {
        let side_snap = store.get(side.snapshot_id).cloned();
        let side_entry = side_snap.as_ref().and_then(|snap| snap.files.get(&c.path));
        
        let side_hash = side_entry.map(|e| e.content_hash.clone()).unwrap_or_default();
        if !side_hash.is_empty() {
            unique_hashes.insert(side_hash.clone());
        }

        let base_entry = base.as_ref().and_then(|snap| snap.files.get(&c.path));
        let base_hash = base_entry.map(|e| e.content_hash.clone());

        let kind_vs_base = match (base_hash.as_deref(), side_entry) {
            (None, None) => "unchanged",
            (None, Some(_)) => "added",
            (Some(_), None) => "removed",
            (Some(b), Some(entry)) if b == entry.content_hash => "unchanged",
            (Some(_), Some(_)) => "modified",
        };

        let preview = if let Some(entry) = side_entry {
            if entry.content_hash.is_empty() {
                None
            } else {
                store.get_blob(&entry.content_hash).map(|blob| {
                    let text = String::from_utf8_lossy(blob);
                    text.chars().take(240).collect::<String>()
                })
            }
        } else {
            None
        };

        sides.push(serde_json::json!({
            "index": idx,
            "agent_id": side.agent_id.to_string(),
            "snapshot_id": side.snapshot_id.to_string(),
            "timestamp_ms": side.timestamp_ms,
            "present_in_snapshot": side_entry.is_some(),
            "content_hash": if side_hash.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(side_hash) },
            "size_bytes": side_entry.map(|e| e.size_bytes),
            "kind_vs_base": kind_vs_base,
            "preview": preview,
        }));
    }

    let base_entry = base.as_ref().and_then(|snap| snap.files.get(&c.path));
    let base_hash = base_entry.map(|e| e.content_hash.clone());

    let body = serde_json::json!({
        "conflict_id": c.id.to_string(),
        "path": c.path.display().to_string(),
        "base_snapshot": c.base_snapshot.map(|s: SnapshotId| s.to_string()),
        "base_hash": base_hash,
        "side_count": c.sides.len(),
        "unique_side_hashes": unique_hashes.len(),
        "all_sides_identical": unique_hashes.len() <= 1 && !c.sides.is_empty(),
        "resolved": c.resolved,
        "resolution": c.resolution,
        "sides": sides,
    });

    ToolResult::ok(body).to_json()
}

/// Resolve a conflict (async).
pub async fn resolve_conflict(state: &ServerState, args: serde_json::Value) -> String {
    let Some(conflict_id) = parse_conflict_id_value(args.get("conflict_id")) else {
        return ToolResult::<String>::err(
            "Missing or invalid conflict_id (number or C-XXXXXX string)".to_string(),
        )
        .to_json();
    };
    let strategy = args
        .get("strategy")
        .and_then(|v| v.as_str())
        .unwrap_or("take_left");

    let orch = &state.orchestrator;

    let resolution = match strategy {
        "take_right" => ConflictResolution::TakeRight,
        "defer" => {
            let agent_id = args
                .get("defer_to_agent")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            ConflictResolution::DeferToAgent(AgentId(agent_id))
        }
        _ => ConflictResolution::TakeLeft,
    };

    let conflict_manager = orch.conflict_manager_handle();
    let mut mgr_guard = conflict_manager.write().unwrap();
    let ok = mgr_guard.resolve(conflict_id, resolution);

    if ok {
        ToolResult::ok("Conflict resolved".to_string()).to_json()
    } else {
        ToolResult::<String>::err("Conflict not found or already resolved".to_string()).to_json()
    }
}

// ---------------------------------------------------------------------------
// Workspaces
// ---------------------------------------------------------------------------

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
    let ws = mgr_handle.write().unwrap().create_workspace(AgentId(agent_id), base_id).clone();

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

// ---------------------------------------------------------------------------
// Change tracking
// ---------------------------------------------------------------------------

/// Create a new logical change (async).
pub async fn change_create(state: &ServerState, args: serde_json::Value) -> String {
    let agent_id = args.get("agent_id").and_then(|v| v.as_u64()).unwrap_or(0);
    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed change");

    let orch = &state.orchestrator;

    let mgr_handle = orch.workspace_manager_handle();
    let change_id = mgr_handle.write().unwrap()
        .create_change(AgentId(agent_id), description);

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
        let mgr = mgr_handle.read().unwrap();
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
        let mgr = mgr_handle.read().unwrap();
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

#[cfg(test)]
mod conflict_diff_contract_tests {
    use super::conflict_diff;
    use crate::server::ServerState;
    use serde_json::json;
    use vox_orchestrator::FileAffinity;

    #[tokio::test]
    async fn conflict_diff_success_payload_has_expected_keys() {
        let state = ServerState::new_test().await;
        let conflict_id = {
            let orch = &state.orchestrator;
            let task_id = orch
                .submit_task("setup", vec![FileAffinity::write("src/lib.rs")], None, None)
                .await
                .expect("submit");
            let agent_a = *orch.agent_ids().first().expect("agent");
            orch.complete_task(task_id).await.expect("complete");
            let ss_lock = orch.snapshot_store_handle();
            let snap_id = ss_lock.write().unwrap().take_snapshot(
                agent_a,
                &[std::path::PathBuf::from("src/lib.rs")],
                "initial".to_string(),
            );
            let cm_lock = orch.conflict_manager_handle();
            cm_lock.write().unwrap().record_conflict(
                "shared.rs",
                Some(snap_id),
                vec![
                    (vox_orchestrator::AgentId(1), snap_id),
                    (vox_orchestrator::AgentId(2), snap_id),
                ],
            )
        };

        let raw = conflict_diff(&state, json!({ "conflict_id": conflict_id.0 })).await;
        let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
        assert_eq!(v.get("success"), Some(&json!(true)));
        let data = v.get("data").expect("data");
        for key in [
            "conflict_id",
            "path",
            "side_count",
            "unique_side_hashes",
            "all_sides_identical",
            "resolved",
            "sides",
        ] {
            assert!(
                data.get(key).is_some(),
                "missing key {key} in conflict_diff payload: {data}"
            );
        }
        let sides = data
            .get("sides")
            .and_then(|x| x.as_array())
            .expect("sides array");
        assert_eq!(sides.len(), 2);
        let s0 = sides[0].as_object().expect("side object");
        for key in [
            "index",
            "agent_id",
            "snapshot_id",
            "kind_vs_base",
            "present_in_snapshot",
        ] {
            assert!(s0.contains_key(key), "missing side key {key}: {s0:?}");
        }
    }
}

#[cfg(test)]
mod id_parse_tests {
    use super::{parse_conflict_id_value, parse_operation_id_value, parse_snapshot_id_value};
    use serde_json::json;
    use vox_orchestrator::{ConflictId, OperationId, SnapshotId};

    #[test]
    fn snapshot_id_accepts_numeric_and_s_prefix() {
        assert_eq!(
            parse_snapshot_id_value(Some(&json!(3))),
            Some(SnapshotId(3))
        );
        assert_eq!(
            parse_snapshot_id_value(Some(&json!("S-000003"))),
            Some(SnapshotId(3))
        );
        assert_eq!(
            parse_snapshot_id_value(Some(&json!("3"))),
            Some(SnapshotId(3))
        );
    }

    #[test]
    fn operation_id_accepts_numeric_and_op_prefix() {
        assert_eq!(
            parse_operation_id_value(Some(&json!(7))),
            Some(OperationId(7))
        );
        assert_eq!(
            parse_operation_id_value(Some(&json!("OP-000007"))),
            Some(OperationId(7))
        );
    }

    #[test]
    fn conflict_id_accepts_numeric_and_c_prefix() {
        assert_eq!(
            parse_conflict_id_value(Some(&json!(9))),
            Some(ConflictId(9))
        );
        assert_eq!(
            parse_conflict_id_value(Some(&json!("C-000009"))),
            Some(ConflictId(9))
        );
    }
}
