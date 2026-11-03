//! Console → agent messaging: send a free-form note from the console operator to
//! a running agent's durable A2A inbox through the daemon's `vox_a2a_send` (`route: "db"`).
//!
//! Sender identity: the console operator is the human at the keyboard, not an
//! orchestrator-spawned agent. Orchestrator agent ids come from an `AtomicU64`
//! generator starting at 1, so `AgentId(0)` is reserved as the console operator
//! sentinel (the daemon's `CONSOLE_OPERATOR_SENDER_ID`).

use std::sync::Arc;

use serde_json::{Value, json};
use tauri::State;
use vox_orchestrator::types::AgentId;

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, tool_data};

/// The reserved sender id for messages originating from the console operator.
pub const CONSOLE_OPERATOR_AGENT_ID: u64 = 0;

const SEND_TOOL: &str = "vox_a2a_send";

/// Parse a receiver agent id string into an `AgentId`. Accepts both the display
/// form ("A-05") and a bare number ("5"), matching `AgentId`'s `FromStr`.
pub fn parse_receiver(raw: &str) -> Result<AgentId, String> {
    let id = raw
        .trim()
        .parse::<AgentId>()
        .map_err(|_| format!("invalid agent id: {raw:?}"))?;
    if id == AgentId(CONSOLE_OPERATOR_AGENT_ID) {
        return Err("agent id 0 is reserved for the console operator".into());
    }
    Ok(id)
}

pub(crate) async fn send_to_agent_impl<B: BackendCalls>(
    b: &B,
    agent_id: String,
    body: String,
) -> Result<String, String> {
    if body.trim().is_empty() {
        return Err("message body is empty".into());
    }
    let receiver = parse_receiver(&agent_id)?;
    let args = json!({
        "sender_id": CONSOLE_OPERATOR_AGENT_ID,
        "receiver_id": receiver.0,
        "msg_type": "free_form",
        "payload": body,
        "route": "db",
    });
    let data: Value = tool_data(SEND_TOOL, b.call_tool(SEND_TOOL, args).await?)?;
    data.get("db_message_uuid")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{SEND_TOOL} returned no db_message_uuid"))
}

/// Send a free-form note to an agent's A2A inbox. Returns the new message id.
#[tauri::command]
pub async fn send_to_agent(
    backend: State<'_, Arc<BackendState>>,
    agent_id: String,
    body: String,
) -> Result<String, String> {
    send_to_agent_impl(&backend.current().await, agent_id, body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    #[test]
    fn parses_bare_number() {
        assert_eq!(parse_receiver("5").unwrap(), AgentId(5));
    }

    #[test]
    fn parses_display_form() {
        assert_eq!(parse_receiver("A-07").unwrap(), AgentId(7));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_receiver("not-an-id").is_err());
    }

    #[test]
    fn rejects_reserved_zero() {
        assert!(parse_receiver("0").is_err());
        assert!(parse_receiver("A-00").is_err());
    }

    #[tokio::test]
    async fn send_to_agent_uses_durable_a2a_route() {
        let b = FakeBackend::new().on_tool(
            SEND_TOOL,
            Ok(json!({"success": true, "data": {"message_id": null, "db_message_uuid": "m-1"}})),
        );
        let id = send_to_agent_impl(&b, "A-05".into(), "hello".into())
            .await
            .unwrap();
        assert_eq!(id, "m-1");
        let calls = b.calls();
        assert_eq!(calls[0].0, format!("tool:{SEND_TOOL}"));
        assert_eq!(
            calls[0].1,
            json!({"sender_id": 0, "receiver_id": 5, "msg_type": "free_form", "payload": "hello", "route": "db"})
        );
    }

    #[tokio::test]
    async fn empty_body_and_reserved_receiver_never_reach_the_daemon() {
        let b = FakeBackend::new();
        assert!(
            send_to_agent_impl(&b, "A-05".into(), "  ".into())
                .await
                .is_err()
        );
        assert!(
            send_to_agent_impl(&b, "0".into(), "hi".into())
                .await
                .is_err()
        );
        assert!(b.calls().is_empty());
    }

    #[tokio::test]
    async fn daemon_rejection_surfaces_as_error() {
        let b = FakeBackend::new().on_tool(
            SEND_TOOL,
            Ok(json!({"success": false, "error": "Receiver agent 5 is not registered"})),
        );
        let err = send_to_agent_impl(&b, "5".into(), "hi".into())
            .await
            .unwrap_err();
        assert!(err.contains("not registered"), "{err}");
    }
}
