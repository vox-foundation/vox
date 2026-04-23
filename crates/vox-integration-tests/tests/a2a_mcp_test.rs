#![allow(missing_docs)]

use vox_orchestrator::OrchestratorConfig;
use vox_orchestrator::mcp_tools::{ServerState, handle_tool_call as tools};

#[tokio::test]
async fn test_a2a_mcp_roundtrip() {
    let config = OrchestratorConfig::for_testing();
    let state = ServerState::new_full(config);
    let sender = state
        .orchestrator
        .spawn_agent("a2a-mcp-snd")
        .expect("spawn sender");
    let receiver = state
        .orchestrator
        .spawn_agent("a2a-mcp-rcv")
        .expect("spawn receiver");

    let send_req = serde_json::json!({
        "sender_id": sender.0,
        "receiver_id": receiver.0,
        "msg_type": "help_request",
        "payload": "I need help with parser."
    });

    let send_resp: String = tools(&state, "vox_a2a_send", send_req).await.unwrap();
    assert!(
        send_resp.contains("\"success\": true") || send_resp.contains("\"success\":true"),
        "Send failed: {}",
        send_resp
    );

    let inbox_req = serde_json::json!({
        "agent_id": receiver.0,
        "source": "local"
    });

    let inbox_resp: String = tools(&state, "vox_a2a_inbox", inbox_req).await.unwrap();
    assert!(inbox_resp.contains("\"success\": true") || inbox_resp.contains("\"success\":true"));
    assert!(inbox_resp.contains("I need help with parser."));

    let inbox_val: serde_json::Value = serde_json::from_str(&inbox_resp).expect("inbox json");
    let msg_id = inbox_val["data"]["messages"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|m| m["id"].as_u64())
        .expect("message id in inbox");

    let ack_req = serde_json::json!({
        "agent_id": receiver.0,
        "message_id": msg_id
    });

    let ack_resp: String = tools(&state, "vox_a2a_ack", ack_req).await.unwrap();
    assert!(ack_resp.contains("\"success\": true") || ack_resp.contains("\"success\":true"));

    let inbox_req2 = serde_json::json!({
        "agent_id": receiver.0,
        "source": "local"
    });
    let inbox_resp2: String = tools(&state, "vox_a2a_inbox", inbox_req2).await.unwrap();
    let inbox2: serde_json::Value = serde_json::from_str(&inbox_resp2).expect("inbox2 json");
    let pending = inbox2["data"]["messages"]
        .as_array()
        .map(Vec::len)
        .unwrap_or(0);
    assert_eq!(pending, 0, "inbox should be empty after ack: {inbox_resp2}");

    let history_req = serde_json::json!({
        "since_ms": 0,
        "limit": 10
    });

    let history_resp: String = tools(&state, "vox_a2a_history", history_req).await.unwrap();
    assert!(
        history_resp.contains("\"success\": true") || history_resp.contains("\"success\":true")
    );
    assert!(history_resp.contains("I need help with parser."));
}
