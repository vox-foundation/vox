use super::*;
use crate::TaskId;
use crate::mcp_tools::params::{TaskStatusParams, ToolResult};

pub(super) const REM_TASK_ID: &str =
    "Confirm `task_id` with task/orchestrator status; it may be stale, completed, or cancelled.";

/// Get the current status of a specific task.
pub async fn task_status(state: &ServerState, params: TaskStatusParams) -> String {
    let task_id = TaskId(params.task_id);
    match state.orchestrator.task_lifecycle_status_label(task_id) {
        Some(label) => ToolResult::ok(label).to_json(),
        None => ToolResult::<String>::err_with_remediation(
            format!("task {} not found", params.task_id),
            REM_TASK_ID,
        )
        .to_json(),
    }
}

/// Retrieve the Testing Decision Engine output for a given task.
pub async fn test_decision(state: &ServerState, params: TaskStatusParams) -> String {
    let task_id_str = params.task_id.to_string();
    if let Some(db) = state.orchestrator.db() {
        match db.load_test_decision(&task_id_str).await {
            Ok(Some((decision, rationale))) => {
                let res = serde_json::json!({
                    "decision": decision,
                    "rationale": rationale
                });
                return ToolResult::ok(res).to_json();
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!("test_decision query failed: {e}");
            }
        }
    }

    // Fallback if not evaluated or DB unavailable
    ToolResult::ok(serde_json::json!({
        "decision": "Unknown",
        "rationale": "No test decision recorded for this task."
    }))
    .to_json()
}
