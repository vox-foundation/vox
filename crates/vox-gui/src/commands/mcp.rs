//! B5: MCP tool execution for the Vox GUI.
//!
//! Tool calls are dispatched against the single persistent orchestrator daemon
//! (see [`crate::commands::daemon::PersistentDaemon`]) via its `orch.tool_call`
//! method. Routing every call through one shared daemon keeps tool execution,
//! HITL approval gates, and their later resolves inside the same `ServerState`
//! process — previously each one-shot stdio daemon was a distinct process, so a
//! parked dangerous-tool approval could not be resolved.

use std::sync::Arc;

use serde_json::Value;
use vox_foundation::protocol::orch_daemon_method;

use crate::remote::calls::deadline_for_tool;

/// Execute an MCP tool against the persistent daemon and return the parsed
/// result envelope.
///
/// The returned object carries an `is_error` flag (true when the daemon's
/// payload reports `success: false`) alongside the tool's own `result` value.
/// Dispatch-level failures surface as the `Err` arm (a human-readable string).
///
/// `permission_mode` (T0.3, `"ask" | "accept_edits" | "accept_all" | "plan"`)
/// is the GUI's selected `PermissionMode`, set on the outgoing
/// `DispatchRequest`'s own top-level field via
/// [`vox_orchestrator::orch_daemon::OrchDaemonClient::with_permission_mode`] — NEVER folded into `args`,
/// which is caller/tool-composed JSON the LLM agent can influence. Omitted
/// or `None` leaves the client's mode unset, which the daemon's dispatch
/// gate treats as the fail-safe `ask` default (today's always-park
/// behavior for dangerous tools).
#[tauri::command]
pub async fn invoke_mcp_tool(
    tool: String,
    args: Value,
    permission_mode: Option<String>,
    daemon: tauri::State<'_, Arc<crate::remote::backend::BackendState>>,
) -> Result<Value, String> {
    let value = daemon
        .call_with_deadline(
            orch_daemon_method::TOOL_CALL,
            serde_json::json!({ "name": tool, "args": args }),
            deadline_for_tool(&tool),
            permission_mode,
        )
        .await
        .map_err(|e| tool_error_message(&tool, e))?;
    Ok(tool_result_envelope(&tool, value))
}

fn tool_error_message(tool: &str, e: crate::remote::error::AxisError) -> String {
    match e {
        crate::remote::error::AxisError::Daemon(msg) => format!("MCP tool '{tool}' failed: {msg}"),
        other => String::from(other),
    }
}

fn tool_result_envelope(tool: &str, value: Value) -> Value {
    let is_error = value.get("success") == Some(&Value::Bool(false));
    serde_json::json!({
        "tool": tool,
        "is_error": is_error,
        "result": value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_flags_only_explicit_failures() {
        let failed = tool_result_envelope("t", serde_json::json!({"success": false}));
        assert_eq!(failed["is_error"], true);
        assert_eq!(failed["tool"], "t");
        let ok = tool_result_envelope("t", serde_json::json!({"success": true, "data": 1}));
        assert_eq!(ok["is_error"], false);
        assert_eq!(ok["result"]["data"], 1);
        let bare = tool_result_envelope("t", serde_json::json!({"data": 1}));
        assert_eq!(bare["is_error"], false);
    }

    #[test]
    fn daemon_errors_name_the_tool() {
        let msg = tool_error_message(
            "vox_x",
            crate::remote::error::AxisError::Daemon("boom".into()),
        );
        assert_eq!(msg, "MCP tool 'vox_x' failed: boom");
    }
}
