use crate::{AgentId, ConflictId, ConflictResolution, SnapshotId};

use super::parse::parse_conflict_id_value;
use crate::mcp_tools::params::ToolResult;
use crate::mcp_tools::server_state::ServerState;
use crate::mcp_tools::sync_poison::{poison_rw_read, poison_rw_write};

const REM_VCS_LOCK: &str = "Retry; persistent poisoned-lock errors usually need an MCP restart.";
const REM_CONFLICT_ID: &str =
    "List conflicts with `conflicts_list` and pass `conflict_id` as a number or `C-XXXXXX`.";
const REM_CONFLICT_MISSING: &str =
    "Refresh `conflicts_list`; the conflict may have been resolved by another agent.";
const REM_CONFLICT_RESOLVE: &str =
    "Confirm the conflict id is still active; stale ids cannot be resolved twice.";

fn lock_err(e: anyhow::Error) -> String {
    ToolResult::<String>::err_with_remediation(e.to_string(), REM_VCS_LOCK).to_json()
}

/// List active conflicts (async).
pub async fn conflicts_list(state: &ServerState) -> String {
    match conflicts_list_inner(state) {
        Ok(s) => s,
        Err(e) => lock_err(e),
    }
}

fn conflicts_list_inner(state: &ServerState) -> anyhow::Result<String> {
    let orch = &state.orchestrator;
    let cm_lock = orch.conflict_manager_handle();
    let mgr = poison_rw_read(cm_lock.read(), "read conflict manager for conflicts_list")?;
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

    Ok(ToolResult::ok(serde_json::json!({
        "active_conflicts": items,
        "total_active": items.len(),
    }))
    .to_json())
}

/// Show an N-way conflict diff for a specific conflict (async).
pub async fn conflict_diff(state: &ServerState, args: serde_json::Value) -> String {
    match conflict_diff_inner(state, args) {
        Ok(s) => s,
        Err(e) => lock_err(e),
    }
}

fn conflict_diff_inner(state: &ServerState, args: serde_json::Value) -> anyhow::Result<String> {
    let conflict_id = if let Some(raw) = args.get("conflict_id").and_then(|v| v.as_u64()) {
        ConflictId(raw)
    } else if let Some(raw) = args.get("conflict_id").and_then(|v| v.as_str()) {
        let Some(id) = raw
            .strip_prefix("C-")
            .and_then(|s| s.parse::<u64>().ok())
            .map(ConflictId)
        else {
            return Ok(ToolResult::<String>::err_with_remediation(
                "Invalid conflict_id format. Expected C-XXXXXX",
                REM_CONFLICT_ID,
            )
            .to_json());
        };
        id
    } else {
        return Ok(ToolResult::<String>::err_with_remediation(
            "Missing conflict_id (number or C-XXXXXX string)".to_string(),
            REM_CONFLICT_ID,
        )
        .to_json());
    };

    let orch = &state.orchestrator;
    let ss_lock = orch.snapshot_store_handle();
    let conflict = {
        let mgr_handle = orch.conflict_manager_handle();
        let mgr = poison_rw_read(mgr_handle.read(), "read conflict manager for conflict_diff")?;
        mgr.get(conflict_id).cloned()
    };

    let Some(c) = conflict else {
        return Ok(ToolResult::<String>::err_with_remediation(
            format!("Conflict {} not found", conflict_id),
            REM_CONFLICT_MISSING,
        )
        .to_json());
    };

    let store = poison_rw_read(ss_lock.read(), "read snapshot store for conflict_diff")?;
    let base = c.base_snapshot.and_then(|sid| store.get(sid).cloned());
    let mut unique_hashes = std::collections::BTreeSet::new();
    let mut sides = Vec::new();

    for (idx, side) in c.sides.iter().enumerate() {
        let side_snap = store.get(side.snapshot_id).cloned();
        let side_entry = side_snap.as_ref().and_then(|snap| snap.files.get(&c.path));

        let side_hash = side_entry
            .map(|e| e.content_hash.clone())
            .unwrap_or_default();
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

    Ok(ToolResult::ok(body).to_json())
}

/// Resolve a conflict (async).
pub async fn resolve_conflict(state: &ServerState, args: serde_json::Value) -> String {
    match resolve_conflict_inner(state, args) {
        Ok(s) => s,
        Err(e) => lock_err(e),
    }
}

fn resolve_conflict_inner(state: &ServerState, args: serde_json::Value) -> anyhow::Result<String> {
    let Some(conflict_id) = parse_conflict_id_value(args.get("conflict_id")) else {
        return Ok(ToolResult::<String>::err_with_remediation(
            "Missing or invalid conflict_id (number or C-XXXXXX string)".to_string(),
            REM_CONFLICT_ID,
        )
        .to_json());
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
    let mut mgr_guard = poison_rw_write(
        conflict_manager.write(),
        "write conflict manager for resolve_conflict",
    )?;
    let ok = mgr_guard.resolve(conflict_id, resolution);

    if ok {
        Ok(ToolResult::ok("Conflict resolved".to_string()).to_json())
    } else {
        Ok(ToolResult::<String>::err_with_remediation(
            "Conflict not found or already resolved".to_string(),
            REM_CONFLICT_RESOLVE,
        )
        .to_json())
    }
}

#[cfg(test)]
mod conflict_diff_contract_tests {
    use super::conflict_diff;
    use crate::mcp_tools::server_state::ServerState;
    use serde_json::json;
    #[tokio::test]
    async fn conflict_diff_success_payload_has_expected_keys() {
        let state = ServerState::new_test().await;
        let orch = &state.orchestrator;
        // Exercise `conflict_diff` without `complete_task`: post-task TOESTUB / snapshot / oplog work
        // can run nested `cargo check --workspace` when `toestub_gate` is on, which is inappropriate
        // for a fast shape contract test (minutes + target-dir lock contention on Windows).
        let agent_a = orch
            .spawn_agent("conflict-diff-contract")
            .expect("spawn agent");
        let snap_id = {
            let ss = orch.snapshot_store_handle();
            ss.write().unwrap().take_snapshot_in_memory(
                agent_a,
                vec![(
                    std::path::PathBuf::from("shared.rs"),
                    b"contract-bytes".to_vec(),
                )],
                "conflict-diff-contract",
            )
        };
        let conflict_id = {
            let cm = orch.conflict_manager_handle();
            cm.write().unwrap().record_conflict(
                "shared.rs",
                Some(snap_id),
                vec![(crate::AgentId(1), snap_id), (crate::AgentId(2), snap_id)],
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
