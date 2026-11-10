//! Shared client helpers for commands that call MCP tools through the `vox-orchestrator-d`
//! daemon (`orch.tool_call`): `vox dei …` and `vox kb status|reindex`.
//!
//! Routing through the daemon keeps a single writer for shared state (the Library's Tantivy
//! index, the orchestrator queues) instead of each CLI invocation opening its own.

use anyhow::Result;
use vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::OrchestratorDaemonEnsure;
use vox_foundation::protocol::orch_daemon_method;

/// T2.3: commands route through the shared `vox-orchestrator-d` TCP daemon (spawning it if
/// absent) instead of building a private, throwaway in-process `Orchestrator` per invocation —
/// see `crates/vox-cli-core/src/daemon_ipc/orchestrator_daemon_ensure.rs`. This closes the
/// split-brain gap T2.1 fixed for the GUI: a `vox dei doubt`/`vox dei submit` run from a
/// terminal is visible to the GUI's Approvals/DEI views (and vice versa), rather than mutating
/// a state the daemon never sees.
pub(crate) async fn daemon_client() -> Result<vox_orchestrator::orch_daemon::OrchDaemonClient> {
    let ensure = OrchestratorDaemonEnsure::default();
    ensure
        .client()
        .await
        .map_err(|e| anyhow::anyhow!("could not reach or spawn vox-orchestrator-d: {e}"))
}

/// Unwrap the `ToolResult<T>` envelope (`{"success": bool, "data": .., "error": ..}`) into a
/// plain `Result` carrying the inner `data`. A value without a `success` key is passed through
/// unchanged (tools that do not use the envelope).
pub(crate) fn unwrap_tool_envelope(envelope: serde_json::Value) -> Result<serde_json::Value> {
    match envelope.get("success").and_then(|s| s.as_bool()) {
        Some(true) => Ok(envelope
            .get("data")
            .cloned()
            .unwrap_or(serde_json::Value::Null)),
        Some(false) => {
            let msg = envelope
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("tool call failed");
            anyhow::bail!("{msg}")
        }
        None => Ok(envelope),
    }
}

/// Call an MCP tool through the daemon's `orch.tool_call` RPC and unwrap its envelope.
/// Mirrors `vox-orchestrator-mcp::daemon_route::call_tool_via_daemon` (T2.2's stdio-MCP
/// pattern) but returns the already-parsed `data` payload, since every call site wants the
/// inner value rather than the raw envelope.
pub(crate) async fn tool_call(
    client: &vox_orchestrator::orch_daemon::OrchDaemonClient,
    name: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value> {
    let envelope = client
        .call(
            orch_daemon_method::TOOL_CALL,
            serde_json::json!({ "name": name, "args": args }),
        )
        .await?;
    unwrap_tool_envelope(envelope)
}

#[cfg(test)]
mod tests {
    use super::unwrap_tool_envelope;
    use serde_json::json;

    #[test]
    fn unwrap_tool_envelope_success_returns_data() {
        let v = unwrap_tool_envelope(json!({"success": true, "data": {"a": 1}})).unwrap();
        assert_eq!(v, json!({"a": 1}));
    }

    #[test]
    fn unwrap_tool_envelope_failure_carries_error() {
        let e = unwrap_tool_envelope(json!({"success": false, "error": "boom"})).unwrap_err();
        assert!(e.to_string().contains("boom"), "{e}");
    }

    #[test]
    fn unwrap_tool_envelope_passes_through_without_success_key() {
        let raw = json!({"x": [1, 2]});
        assert_eq!(unwrap_tool_envelope(raw.clone()).unwrap(), raw);
    }
}
