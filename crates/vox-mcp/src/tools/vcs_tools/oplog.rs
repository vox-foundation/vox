use vox_orchestrator::AgentId;

use super::parse::parse_operation_id_value;
use crate::params::ToolResult;
use crate::server::ServerState;

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
