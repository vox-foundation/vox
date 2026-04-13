use serde_json::Value;

use super::params::AmbientStateParams;
use crate::mcp_tools::params::ToolResult;
use crate::mcp_tools::server_state::ServerState;

/// Handle the `vox_ambient_state` tool call.
///
/// Snapshots the current DEI orchestrator state (active locks, conflicts, task-to-file
/// assignments) and converts it to a list of `AmbientDecoration` records. The VS Code
/// extension polls this every 2-3 seconds and renders gutter stripes + file-explorer
/// badges without interrupting the user's flow.
pub async fn ambient_state(state: &ServerState, params: AmbientStateParams) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let prefix_filter = params.path_prefix.as_deref().unwrap_or("");
    let limit = params.limit.unwrap_or(100);

    fn is_file_lock_row(d: &Value) -> bool {
        d.get("decoration")
            .and_then(|x| x.get("type"))
            .and_then(|t| t.as_str())
            == Some("file_lock")
    }

    let orch = &state.orchestrator;
    let mut decorations: Vec<Value> = Vec::new();

    // 1. Active file locks → FileLock decorations
    for (path, holder, exclusive) in orch.lock_manager().list_locks() {
        let holder: crate::AgentId = holder;
        let path_str = path.to_string_lossy().to_string();
        if !prefix_filter.is_empty() && !path_str.contains(prefix_filter) {
            continue;
        }
        let (severity, tooltip) = if exclusive {
            (
                "error",
                format!("\u{1f512} Agent {holder} holding exclusive write lock"),
            )
        } else {
            (
                "warning",
                format!("\u{1f50d} Agent {holder} reading this file"),
            )
        };
        decorations.push(serde_json::json!({
            "path": path_str,
            "decoration": {
                "type": "file_lock",
                "agent_id": holder.0,
                "exclusive": exclusive,
            },
            "severity": severity,
            "timestamp_ms": now_ms,
            "tooltip": tooltip,
        }));
    }

    // 2. Active conflicts → Conflict decorations
    {
        let handle = orch.conflict_manager_handle();
        match crate::mcp_tools::sync_poison::poison_rw_read::<crate::conflicts::ConflictManager>(handle.read(), "conflict manager") {
            Ok(guard) => {
                for conflict in guard.active_conflicts() {
                    let path_str = conflict.path.to_string_lossy().to_string();
                    if !prefix_filter.is_empty() && !path_str.contains(prefix_filter) {
                        continue;
                    }
                    let agent_ids: Vec<u64> = conflict.sides.iter().map(|s| s.agent_id.0).collect();
                    decorations.push(serde_json::json!({
                        "path": path_str,
                        "decoration": {
                            "type": "conflict",
                            "conflict_id": conflict.id.to_string(),
                            "agent_ids": agent_ids,
                        },
                        "severity": "error",
                        "timestamp_ms": now_ms,
                        "tooltip": format!(
                            "\u{26a0} Conflict between {} agents — resolve before proceeding",
                            conflict.sides.len()
                        ),
                    }));
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "ambient_state: conflict manager poisoned");
            }
        }
    }

    // 3. Agent-to-file affinity (active tasks) → AgentActive decorations
    for agent_id in orch.agent_ids() {
        let Some(queue) = orch.agent_queue(agent_id) else {
            continue;
        };
        let guard = match crate::mcp_tools::sync_poison::poison_rw_read::<crate::queue::AgentQueue>(queue.read(), "agent queue") {
            Ok(g) => g,
            Err(e) => {
                tracing::warn!(error = %e, ?agent_id, "ambient_state: agent queue poisoned");
                continue;
            }
        };
        if let Some(task) = guard.current_task() {
            for fa in &task.file_manifest {
                let path_str = fa.path.to_string_lossy().to_string();
                if !prefix_filter.is_empty() && !path_str.contains(prefix_filter) {
                    continue;
                }
                if decorations.iter().any(|d| {
                    d.get("path").and_then(|p| p.as_str()) == Some(path_str.as_str())
                        && is_file_lock_row(d)
                }) {
                    continue;
                }
                decorations.push(serde_json::json!({
                    "path": path_str,
                    "decoration": {
                        "type": "agent_active",
                        "agent_id": agent_id.0,
                        "activity": format!("{:.60}", task.description),
                    },
                    "severity": "info",
                    "timestamp_ms": now_ms,
                    "tooltip": format!(
                        "\u{1f916} Agent {} working on: {:.80}",
                        agent_id, task.description
                    ),
                }));
            }
        }
    }

    let total = decorations.len().min(limit);
    decorations.truncate(limit);

    let active_conflicts = decorations
        .iter()
        .filter(|d| d.get("severity").and_then(|s| s.as_str()) == Some("error"))
        .count();

    let result = serde_json::json!({
        "decorations": decorations,
        "total": total,
        "active_conflicts": active_conflicts,
        "timestamp_ms": now_ms,
    });

    ToolResult::ok(result).to_json()
}
