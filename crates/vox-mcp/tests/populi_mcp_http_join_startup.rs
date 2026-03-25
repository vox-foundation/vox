#![allow(missing_docs)]
#![allow(unsafe_code)]

//! `publish_mesh_on_mcp_start` performs HTTP `join` when orchestrator mens URL points at a live control plane.
//! `a2a_inbox_merges_remote_mesh_control_plane_and_ack` covers MCP A2A inbox merge + remote ack against the same Axum router.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::LazyLock;

use serde_json::json;

use vox_mcp::ServerState;
use vox_mcp::tools;
use vox_orchestrator::OrchestratorConfig;
use vox_populi::http_client::PopuliHttpClient;
use vox_populi::transport::{
    A2ADeliverRequest, PopuliHttpAuth, PopuliTransportState, populi_http_app_with_auth,
};

static MESH_ENV_MUTEX: LazyLock<tokio::sync::Mutex<()>> =
    LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Poll TCP connectivity until Axum is ready to serve requests, bounded to avoid hangs.
async fn wait_for_tcp(addr: std::net::SocketAddr) {
    use std::net::TcpStream;
    use std::time::Duration;
    for _ in 0..100 {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(10)).is_ok() {
            return;
        }
        tokio::task::yield_now().await;
    }
}

/// Restore an env key after the test finishes; call only while [`MESH_ENV_MUTEX`] is held.
fn restore_env(key: &str, previous: Option<String>) {
    // SAFETY: called only while MESH_ENV_MUTEX is held and the test thread is the sole writer.
    unsafe {
        match previous {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }
}

#[tokio::test]
async fn populi_startup_registers_on_http_control_plane() {
    let _lock = MESH_ENV_MUTEX.lock().await;

    const KEYS: &[&str] = &[
        "VOX_MESH_ENABLED",
        "VOX_ORCHESTRATOR_MESH_CONTROL_URL",
        "VOX_MESH_CONTROL_ADDR",
        "VOX_MESH_NODE_ID",
        "VOX_MESH_REGISTRY_PATH",
        "VOX_MESH_HTTP_HEARTBEAT_SECS",
        "VOX_MESH_HTTP_JOIN",
    ];

    let mut saved: HashMap<String, Option<String>> = HashMap::new();
    for k in KEYS {
        saved.insert((*k).to_string(), std::env::var(k).ok());
    }

    let tmp = std::env::temp_dir().join(format!(
        "vox-mcp-mens-join-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let registry_path = tmp.join("local-registry.json");

    let state_reg = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = populi_http_app_with_auth(state_reg, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    // Deterministic server readiness — no sleep.
    wait_for_tcp(bound).await;

    let base = format!("http://{}", bound);

    unsafe {
        std::env::set_var("VOX_MESH_ENABLED", "1");
        std::env::remove_var("VOX_MESH_CONTROL_ADDR");
        std::env::set_var("VOX_ORCHESTRATOR_MESH_CONTROL_URL", &base);
        std::env::set_var("VOX_MESH_NODE_ID", "mcp-join-integration");
        std::env::set_var("VOX_MESH_REGISTRY_PATH", registry_path.to_str().unwrap());
        std::env::set_var("VOX_MESH_HTTP_HEARTBEAT_SECS", "0");
        std::env::remove_var("VOX_MESH_HTTP_JOIN");
    }

    let state = ServerState::new(OrchestratorConfig::default());
    vox_mcp::populi_startup::publish_mesh_on_mcp_start(&state).await;

    let client = PopuliHttpClient::new(&base);
    let file = client.list_nodes().await.expect("list_nodes after join");
    assert!(
        file.nodes.iter().any(|n| n.id == "mcp-join-integration"),
        "expected mcp-join-integration in {:?}",
        file.nodes
    );

    server.abort();
    for k in KEYS {
        if let Some(prev) = saved.remove(*k) {
            restore_env(k, prev);
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
}

/// Exercises [`vox_mcp::a2a::a2a_inbox`] / [`vox_mcp::a2a::a2a_ack`] against a real in-process Populi
/// HTTP control plane (remote poll + ack), not only the in-memory orchestrator bus.
#[tokio::test]
async fn a2a_inbox_merges_remote_mesh_control_plane_and_ack() {
    let _lock = MESH_ENV_MUTEX.lock().await;

    const KEYS: &[&str] = &[
        "VOX_ORCHESTRATOR_MESH_CONTROL_URL",
        "VOX_MESH_CONTROL_ADDR",
        "VOX_MESH_TOKEN",
    ];

    let mut saved: HashMap<String, Option<String>> = HashMap::new();
    for k in KEYS {
        saved.insert((*k).to_string(), std::env::var(k).ok());
    }

    let state_reg = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = populi_http_app_with_auth(state_reg, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    wait_for_tcp(bound).await;

    let base = format!("http://{}", bound);

    unsafe {
        std::env::remove_var("VOX_MESH_CONTROL_ADDR");
        std::env::set_var("VOX_ORCHESTRATOR_MESH_CONTROL_URL", &base);
        std::env::remove_var("VOX_MESH_TOKEN");
    }

    let mesh = PopuliHttpClient::new(&base);
    const AGENT: u64 = 42;
    mesh.relay_a2a(&A2ADeliverRequest {
        sender_agent_id: "7".into(),
        receiver_agent_id: AGENT.to_string(),
        message_type: "free_form".into(),
        payload: "mesh-early".into(),
    })
    .await
    .expect("relay first A2A to mock control plane");
    mesh.relay_a2a(&A2ADeliverRequest {
        sender_agent_id: "8".into(),
        receiver_agent_id: AGENT.to_string(),
        message_type: "free_form".into(),
        payload: "mesh-late".into(),
    })
    .await
    .expect("relay second A2A to mock control plane");

    let mcp = ServerState::new_test().await;

    tools::handle_tool_call(
        &mcp,
        "vox_a2a_send",
        json!({
            "sender_id": 1,
            "receiver_id": AGENT,
            "msg_type": "free_form",
            "payload": "local-only",
        }),
    )
    .await
    .expect("local A2A send");

    let inbox_raw = tools::handle_tool_call(&mcp, "vox_a2a_inbox", json!({ "agent_id": AGENT }))
        .await
        .expect("vox_a2a_inbox");
    let inbox: serde_json::Value = serde_json::from_str(&inbox_raw).expect("inbox JSON");
    assert_eq!(inbox["success"], true, "{inbox_raw}");
    assert_eq!(
        inbox["data"]["remote_ok"], true,
        "expected remote mesh inbox poll to succeed: {inbox_raw}"
    );

    let messages = inbox["data"]["messages"]
        .as_array()
        .expect("messages array");
    let payload_strs: Vec<&str> = messages
        .iter()
        .filter_map(|m| m["payload"].as_str())
        .collect();
    assert!(
        payload_strs.iter().any(|p| p.contains("local-only")),
        "expected local bus entry in {:?}",
        payload_strs
    );
    assert!(
        payload_strs.contains(&"mesh-late"),
        "mesh id 2 should merge after local id 1 collides with mesh id 1: {:?}",
        payload_strs
    );
    assert!(
        !payload_strs.contains(&"mesh-early"),
        "duplicate message ids across planes should not duplicate rows: {:?}",
        payload_strs
    );

    let late_id = messages
        .iter()
        .find(|m| m["payload"].as_str() == Some("mesh-late"))
        .and_then(|m| m["id"].as_u64())
        .expect("mesh-late row id");

    let ack_raw = tools::handle_tool_call(
        &mcp,
        "vox_a2a_ack",
        json!({ "agent_id": AGENT, "message_id": late_id }),
    )
    .await
    .expect("vox_a2a_ack");
    let ack: serde_json::Value = serde_json::from_str(&ack_raw).expect("ack JSON");
    assert_eq!(ack["success"], true, "{ack_raw}");
    assert_eq!(ack["data"]["remote_acknowledged"], true, "{ack_raw}");

    let inbox2_raw = tools::handle_tool_call(&mcp, "vox_a2a_inbox", json!({ "agent_id": AGENT }))
        .await
        .expect("second inbox");
    let inbox2: serde_json::Value = serde_json::from_str(&inbox2_raw).expect("inbox2 JSON");
    let after = inbox2["data"]["messages"].as_array().expect("messages");
    assert!(
        !after
            .iter()
            .any(|m| m["payload"].as_str() == Some("mesh-late")),
        "acked mesh row should disappear from remote poll: {inbox2_raw}"
    );

    server.abort();
    for k in KEYS {
        if let Some(prev) = saved.remove(*k) {
            restore_env(k, prev);
        }
    }
}
