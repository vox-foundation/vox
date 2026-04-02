#![allow(missing_docs)]
#![allow(unsafe_code)]

//! `publish_mesh_on_mcp_start` performs HTTP `join` when orchestrator mens URL points at a live control plane.
//! `a2a_inbox_merges_remote_mesh_control_plane_and_ack` covers MCP A2A inbox merge + remote ack against the same Axum router.
//! `a2a_inbox_source_modes_local_mesh_merged` asserts `source` selection, remote poll flags, counts, and merge dedupe.
//! `a2a_inbox_mesh_paging_forwards_limit_and_cursor` verifies MCP forwards mesh paging args (`max_messages`, `before_message_id`).
//! `mcp_federation_poller_auto_revokes_exec_lease_when_holder_left_mesh` covers **`VOX_ORCHESTRATOR_MESH_EXEC_LEASE_AUTO_REVOKE`** on orphan leases after `leave`.
//! `mcp_federation_poller_keeps_orphan_exec_lease_without_auto_revoke` asserts reconcile-only does not call admin revoke.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::LazyLock;

use serde_json::json;

use vox_mcp::ServerState;
use vox_mcp::tools;
use vox_orchestrator::OrchestratorConfig;
use vox_populi::http_client::PopuliHttpClient;
use vox_populi::node_record_for_current_process;
use vox_populi::transport::{
    A2ADeliverRequest, PopuliHttpAuth, PopuliTransportState, RemoteExecLeaseGrantRequest,
    populi_http_app_with_auth,
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

    let mcp = ServerState::new_test().await;
    let sender = mcp
        .orchestrator
        .spawn_agent("a2a-mesh-merge-sender")
        .expect("spawn sender");
    let receiver = mcp
        .orchestrator
        .spawn_agent("a2a-mesh-merge-receiver")
        .expect("spawn receiver");
    let agent = receiver.0;

    mesh.relay_a2a(&A2ADeliverRequest {
        sender_agent_id: "7".into(),
        receiver_agent_id: agent.to_string(),
        message_type: "free_form".into(),
        payload: "mesh-early".into(),
        idempotency_key: None,
        privacy_class: None,
        payload_blake3_hex: None,
        worker_ed25519_sig_b64: None,
    })
    .await
    .expect("relay first A2A to mock control plane");
    mesh.relay_a2a(&A2ADeliverRequest {
        sender_agent_id: "8".into(),
        receiver_agent_id: agent.to_string(),
        message_type: "free_form".into(),
        payload: "mesh-late".into(),
        idempotency_key: None,
        privacy_class: None,
        payload_blake3_hex: None,
        worker_ed25519_sig_b64: None,
    })
    .await
    .expect("relay second A2A to mock control plane");

    tools::handle_tool_call(
        &mcp,
        "vox_a2a_send",
        json!({
            "sender_id": sender.0,
            "receiver_id": agent,
            "msg_type": "free_form",
            "payload": "local-only",
        }),
    )
    .await
    .expect("local A2A send");

    let inbox_raw = tools::handle_tool_call(&mcp, "vox_a2a_inbox", json!({ "agent_id": agent }))
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
        json!({ "agent_id": agent, "message_id": late_id }),
    )
    .await
    .expect("vox_a2a_ack");
    let ack: serde_json::Value = serde_json::from_str(&ack_raw).expect("ack JSON");
    assert_eq!(ack["success"], true, "{ack_raw}");
    assert_eq!(ack["data"]["remote_acknowledged"], true, "{ack_raw}");

    let inbox2_raw = tools::handle_tool_call(&mcp, "vox_a2a_inbox", json!({ "agent_id": agent }))
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

/// `a2a_inbox` [`source`](vox_mcp::a2a::A2AInboxParams::source): `local` skips mesh poll even when the control URL is set;
/// `mesh` returns only relay rows; `merged` unions with id dedupe (same harness as
/// [`a2a_inbox_merges_remote_mesh_control_plane_and_ack`]).
#[tokio::test]
async fn a2a_inbox_source_modes_local_mesh_merged() {
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

    let mcp = ServerState::new_test().await;
    let bus_sender = mcp
        .orchestrator
        .spawn_agent("a2a-source-modes-sender")
        .expect("spawn sender");
    let bus_receiver = mcp
        .orchestrator
        .spawn_agent("a2a-source-modes-receiver")
        .expect("spawn receiver");
    let agent = bus_receiver.0;

    mesh.relay_a2a(&A2ADeliverRequest {
        sender_agent_id: "7".into(),
        receiver_agent_id: agent.to_string(),
        message_type: "free_form".into(),
        payload: "mesh-early".into(),
        idempotency_key: None,
        privacy_class: None,
        payload_blake3_hex: None,
        worker_ed25519_sig_b64: None,
    })
    .await
    .expect("relay mesh-early");
    mesh.relay_a2a(&A2ADeliverRequest {
        sender_agent_id: "8".into(),
        receiver_agent_id: agent.to_string(),
        message_type: "free_form".into(),
        payload: "mesh-late".into(),
        idempotency_key: None,
        privacy_class: None,
        payload_blake3_hex: None,
        worker_ed25519_sig_b64: None,
    })
    .await
    .expect("relay mesh-late");

    tools::handle_tool_call(
        &mcp,
        "vox_a2a_send",
        json!({
            "sender_id": bus_sender.0,
            "receiver_id": agent,
            "msg_type": "free_form",
            "payload": "local-only",
        }),
    )
    .await
    .expect("local A2A send");

    async fn a2a_inbox_parse(
        mcp: &ServerState,
        agent_id: u64,
        source: Option<&str>,
    ) -> serde_json::Value {
        let args = if let Some(s) = source {
            json!({ "agent_id": agent_id, "source": s })
        } else {
            json!({ "agent_id": agent_id })
        };
        let raw = tools::handle_tool_call(mcp, "vox_a2a_inbox", args)
            .await
            .expect("vox_a2a_inbox");
        serde_json::from_str(&raw).expect("inbox JSON")
    }

    let merged = a2a_inbox_parse(&mcp, agent, Some("merged")).await;
    assert_eq!(merged["success"], true, "{merged}");
    assert_eq!(merged["data"]["source"], "merged");
    assert_eq!(
        merged["data"]["remote_attempted"], true,
        "merged should poll mesh when URL is set: {merged}"
    );
    assert_eq!(merged["data"]["remote_ok"], true, "{merged}");
    assert_eq!(merged["data"]["unread_count"], 2, "{merged}");
    let merged_msgs = merged["data"]["messages"].as_array().expect("messages");
    let merged_payloads: Vec<&str> = merged_msgs
        .iter()
        .filter_map(|m| m["payload"].as_str())
        .collect();
    assert!(
        merged_payloads.iter().any(|p| p.contains("local-only")),
        "merged should include local bus: {merged_payloads:?}"
    );
    assert!(
        merged_payloads.contains(&"mesh-late"),
        "merged should include non-colliding mesh id: {merged_payloads:?}"
    );
    assert!(
        !merged_payloads.contains(&"mesh-early"),
        "shared id 1 should dedupe mesh-early vs local: {merged_payloads:?}"
    );

    let local = a2a_inbox_parse(&mcp, agent, Some("local")).await;
    assert_eq!(local["success"], true, "{local}");
    assert_eq!(local["data"]["source"], "local");
    assert_eq!(
        local["data"]["remote_attempted"], false,
        "local must not hit mesh: {local}"
    );
    assert_eq!(local["data"]["remote_ok"], false, "{local}");
    assert_eq!(local["data"]["unread_count"], 1, "{local}");
    let local_msgs = local["data"]["messages"].as_array().expect("messages");
    assert_eq!(local_msgs.len(), 1);
    assert!(
        local_msgs[0]["payload"]
            .as_str()
            .is_some_and(|p| p.contains("local-only")),
        "{local}"
    );

    let mesh_only = a2a_inbox_parse(&mcp, agent, Some("mesh")).await;
    assert_eq!(mesh_only["success"], true, "{mesh_only}");
    assert_eq!(mesh_only["data"]["source"], "mesh");
    assert_eq!(mesh_only["data"]["remote_attempted"], true, "{mesh_only}");
    assert_eq!(mesh_only["data"]["remote_ok"], true, "{mesh_only}");
    assert_eq!(mesh_only["data"]["unread_count"], 2, "{mesh_only}");
    let mesh_payloads: Vec<&str> = mesh_only["data"]["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .filter_map(|m| m["payload"].as_str())
        .collect();
    assert!(
        mesh_payloads.contains(&"mesh-early") && mesh_payloads.contains(&"mesh-late"),
        "mesh source should surface both relay rows: {mesh_payloads:?}"
    );
    assert!(
        !mesh_payloads.iter().any(|p| p.contains("local-only")),
        "mesh source must exclude in-process bus: {mesh_payloads:?}"
    );

    server.abort();
    for k in KEYS {
        if let Some(prev) = saved.remove(*k) {
            restore_env(k, prev);
        }
    }
}

/// `a2a_inbox` forwards mesh paging args to the Populi HTTP control-plane inbox API.
#[tokio::test]
async fn a2a_inbox_mesh_paging_forwards_limit_and_cursor() {
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

    // SAFETY: guarded by MESH_ENV_MUTEX for this test process.
    unsafe {
        std::env::remove_var("VOX_MESH_CONTROL_ADDR");
        std::env::set_var("VOX_ORCHESTRATOR_MESH_CONTROL_URL", &base);
        std::env::remove_var("VOX_MESH_TOKEN");
    }

    let mesh = PopuliHttpClient::new(&base);
    let mcp = ServerState::new_test().await;
    let receiver = mcp
        .orchestrator
        .spawn_agent("a2a-mesh-paging-receiver")
        .expect("spawn receiver");
    let agent = receiver.0;

    for i in 1..=4u64 {
        mesh.relay_a2a(&A2ADeliverRequest {
            sender_agent_id: (100 + i).to_string(),
            receiver_agent_id: agent.to_string(),
            message_type: "free_form".into(),
            payload: format!("mesh-{i}"),
            idempotency_key: None,
            privacy_class: None,
            payload_blake3_hex: None,
            worker_ed25519_sig_b64: None,
        })
        .await
        .expect("relay paged mesh message");
    }

    let page1_raw = tools::handle_tool_call(
        &mcp,
        "vox_a2a_inbox",
        json!({
            "agent_id": agent,
            "source": "mesh",
            "max_messages": 2
        }),
    )
    .await
    .expect("vox_a2a_inbox page1");
    let page1: serde_json::Value = serde_json::from_str(&page1_raw).expect("page1 JSON");
    assert_eq!(page1["success"], true, "{page1_raw}");
    assert_eq!(page1["data"]["remote_ok"], true, "{page1_raw}");
    let page1_msgs = page1["data"]["messages"]
        .as_array()
        .expect("page1 messages");
    assert_eq!(page1_msgs.len(), 2, "page1 should honor max_messages=2");
    let page1_ids: Vec<u64> = page1_msgs
        .iter()
        .map(|m| m["id"].as_u64().expect("page1 id"))
        .collect();
    let cursor = *page1_ids.iter().min().expect("page1 min id");

    let page2_raw = tools::handle_tool_call(
        &mcp,
        "vox_a2a_inbox",
        json!({
            "agent_id": agent,
            "source": "mesh",
            "max_messages": 2,
            "before_message_id": cursor
        }),
    )
    .await
    .expect("vox_a2a_inbox page2");
    let page2: serde_json::Value = serde_json::from_str(&page2_raw).expect("page2 JSON");
    assert_eq!(page2["success"], true, "{page2_raw}");
    assert_eq!(page2["data"]["remote_ok"], true, "{page2_raw}");
    let page2_msgs = page2["data"]["messages"]
        .as_array()
        .expect("page2 messages");
    assert_eq!(page2_msgs.len(), 2, "page2 should honor max_messages=2");
    let page2_ids: Vec<u64> = page2_msgs
        .iter()
        .map(|m| m["id"].as_u64().expect("page2 id"))
        .collect();
    assert!(
        page2_ids.iter().all(|id| *id < cursor),
        "cursor must enforce strict id window: cursor={cursor}, page2_ids={page2_ids:?}"
    );

    let mut all_ids = page1_ids.clone();
    all_ids.extend(page2_ids.iter().copied());
    all_ids.sort_unstable();
    assert_eq!(
        all_ids,
        vec![1, 2, 3, 4],
        "should read full 4-row inbox in 2 pages"
    );

    server.abort();
    for k in KEYS {
        if let Some(prev) = saved.remove(*k) {
            restore_env(k, prev);
        }
    }
}

/// Orphan exec lease (holder left via [`PopuliHttpClient::leave`]) is cleared when MCP federation
/// polling runs with **`VOX_ORCHESTRATOR_MESH_EXEC_LEASE_RECONCILE`** and
/// **`VOX_ORCHESTRATOR_MESH_EXEC_LEASE_AUTO_REVOKE`**.
#[tokio::test]
async fn mcp_federation_poller_auto_revokes_exec_lease_when_holder_left_mesh() {
    let _lock = MESH_ENV_MUTEX.lock().await;

    const KEYS: &[&str] = &[
        "VOX_ORCHESTRATOR_MESH_EXEC_LEASE_RECONCILE",
        "VOX_ORCHESTRATOR_MESH_EXEC_LEASE_AUTO_REVOKE",
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
    let setup = PopuliHttpClient::new(&base);
    let node = node_record_for_current_process("mcp-auto-revoke-holder".into(), None);
    setup.join(&node).await.unwrap();
    setup
        .exec_lease_grant(&RemoteExecLeaseGrantRequest {
            claimer_node_id: "mcp-auto-revoke-holder".into(),
            scope_key: "task:mcp-federation-auto-revoke".into(),
        })
        .await
        .unwrap();
    assert_eq!(setup.list_exec_leases().await.unwrap().leases.len(), 1);
    assert!(setup.leave("mcp-auto-revoke-holder").await.unwrap());
    assert!(setup.list_nodes().await.unwrap().nodes.is_empty());

    unsafe {
        std::env::set_var("VOX_ORCHESTRATOR_MESH_EXEC_LEASE_RECONCILE", "1");
        std::env::set_var("VOX_ORCHESTRATOR_MESH_EXEC_LEASE_AUTO_REVOKE", "1");
    }

    let mut cfg = OrchestratorConfig::for_testing();
    cfg.populi_control_url = Some(base.clone());
    cfg.populi_poll_interval_secs = 1;

    let _mcp = ServerState::new(cfg);

    let verify = PopuliHttpClient::new(&base);
    let mut cleared = false;
    for _ in 0..100 {
        if verify
            .list_exec_leases()
            .await
            .expect("list_exec_leases")
            .leases
            .is_empty()
        {
            cleared = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    assert!(
        cleared,
        "exec lease row should be admin-revoked by MCP reconcile poller"
    );

    server.abort();
    for k in KEYS {
        if let Some(prev) = saved.remove(*k) {
            restore_env(k, prev);
        }
    }
}

/// With reconcile enabled but **`VOX_ORCHESTRATOR_MESH_EXEC_LEASE_AUTO_REVOKE`** unset/false, orphan leases stay
/// until an operator revokes them or the holder returns.
#[tokio::test]
async fn mcp_federation_poller_keeps_orphan_exec_lease_without_auto_revoke() {
    let _lock = MESH_ENV_MUTEX.lock().await;

    const KEYS: &[&str] = &[
        "VOX_ORCHESTRATOR_MESH_EXEC_LEASE_RECONCILE",
        "VOX_ORCHESTRATOR_MESH_EXEC_LEASE_AUTO_REVOKE",
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
    let setup = PopuliHttpClient::new(&base);
    let node = node_record_for_current_process("mcp-reconcile-only-holder".into(), None);
    setup.join(&node).await.unwrap();
    setup
        .exec_lease_grant(&RemoteExecLeaseGrantRequest {
            claimer_node_id: "mcp-reconcile-only-holder".into(),
            scope_key: "task:mcp-federation-reconcile-only".into(),
        })
        .await
        .unwrap();
    let lease_id = setup.list_exec_leases().await.unwrap().leases[0]
        .lease_id
        .clone();
    assert!(setup.leave("mcp-reconcile-only-holder").await.unwrap());

    unsafe {
        std::env::set_var("VOX_ORCHESTRATOR_MESH_EXEC_LEASE_RECONCILE", "1");
        std::env::remove_var("VOX_ORCHESTRATOR_MESH_EXEC_LEASE_AUTO_REVOKE");
    }

    let mut cfg = OrchestratorConfig::for_testing();
    cfg.populi_control_url = Some(base.clone());
    cfg.populi_poll_interval_secs = 1;

    let _mcp = ServerState::new(cfg);

    // Interval is 1s; wait past multiple ticks so a mistaken auto-revoke would have run.
    tokio::time::sleep(std::time::Duration::from_millis(2600)).await;

    let verify = PopuliHttpClient::new(&base);
    let list = verify.list_exec_leases().await.expect("list_exec_leases");
    assert_eq!(
        list.leases.len(),
        1,
        "reconcile-only must retain orphan lease: {list:?}"
    );
    assert_eq!(list.leases[0].lease_id, lease_id);

    server.abort();
    for k in KEYS {
        if let Some(prev) = saved.remove(*k) {
            restore_env(k, prev);
        }
    }
}
