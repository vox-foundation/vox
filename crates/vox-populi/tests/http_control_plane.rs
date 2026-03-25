#![allow(missing_docs)]
#![cfg(feature = "transport")]

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Mutex;
use std::time::Duration;

use vox_populi::http_client::PopuliHttpClient;
use vox_populi::transport::{PopuliHttpAuth, PopuliTransportState};
use vox_populi::{node_record_for_current_process, transport};

static ENV_MUTEX: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn join_list_heartbeat_roundtrip() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let empty = client.list_nodes().await.unwrap();
    assert!(empty.nodes.is_empty());

    let node = node_record_for_current_process("node-a".into(), Some(bound.to_string()));
    let joined = client.join(&node).await.unwrap();
    assert_eq!(joined.id, "node-a");

    let listed = client.list_nodes().await.unwrap();
    assert_eq!(listed.nodes.len(), 1);

    let beat = client.heartbeat(&node).await.unwrap();
    assert!(beat.last_seen_unix_ms >= node.last_seen_unix_ms);

    server.abort();
}

#[tokio::test]
async fn leave_removes_node() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let node = node_record_for_current_process("gone".into(), None);
    client.join(&node).await.unwrap();
    assert_eq!(client.list_nodes().await.unwrap().nodes.len(), 1);

    assert!(client.leave("gone").await.unwrap());
    assert!(client.list_nodes().await.unwrap().nodes.is_empty());
    assert!(!client.leave("gone").await.unwrap());

    server.abort();
}

#[tokio::test]
async fn mesh_token_requires_bearer() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(
        state,
        PopuliHttpAuth::Bearer("unit-test-populi-token".into()),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let no_auth = PopuliHttpClient::new(&base);
    assert!(no_auth.list_nodes().await.is_err());

    let authed = PopuliHttpClient::new(&base).with_bearer("unit-test-populi-token");
    assert!(authed.list_nodes().await.is_ok());

    server.abort();
}

#[tokio::test]
async fn health_ok_without_bearer_when_token_required() {
    let state = PopuliTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(
        state,
        transport::PopuliHttpAuth::Bearer("unit-test-populi-token".into()),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let url = format!("{base}/health");
    let r = reqwest::get(&url).await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::OK);
    assert_eq!(r.text().await.unwrap(), "ok\n");

    server.abort();
}

#[tokio::test]
async fn join_rejected_when_scope_mismatch() {
    let state = PopuliTransportState::with_required_scope(Some("cluster-a".into()));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let mut node = node_record_for_current_process("wrong-scope".into(), None);
    node.scope_id = Some("cluster-b".into());
    let err = client.join(&node).await.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("403") || msg.contains("Forbidden"),
        "unexpected error: {msg}"
    );

    server.abort();
}

#[tokio::test]
async fn join_ok_when_scope_matches() {
    let state = PopuliTransportState::with_required_scope(Some("cluster-a".into()));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = PopuliHttpClient::new(&base);
    let mut node = node_record_for_current_process("scoped-node".into(), None);
    node.scope_id = Some("cluster-a".into());
    client.join(&node).await.unwrap();
    assert_eq!(client.list_nodes().await.unwrap().nodes.len(), 1);

    server.abort();
}

#[tokio::test]
#[allow(unsafe_code)]
async fn bootstrap_exchange_works_once() {
    let _guard = ENV_MUTEX.lock().expect("env lock");
    let prev_bootstrap = std::env::var("VOX_MESH_BOOTSTRAP_TOKEN").ok();
    let prev_expires = std::env::var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS").ok();
    let prev_token = std::env::var("VOX_MESH_TOKEN").ok();
    let prev_scope = std::env::var("VOX_MESH_SCOPE_ID").ok();

    let bootstrap = "bootstrap-unit-test-token";
    // SAFETY: serialized by `ENV_MUTEX`, restored at test end.
    unsafe {
        std::env::set_var("VOX_MESH_BOOTSTRAP_TOKEN", bootstrap);
        std::env::set_var(
            "VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS",
            (vox_populi::wall_clock_unix_ms() + 120_000).to_string(),
        );
        std::env::set_var("VOX_MESH_TOKEN", "mesh-unit-token");
        std::env::set_var("VOX_MESH_SCOPE_ID", "scope-unit");
    }

    let state = PopuliTransportState::new_for_serve();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = transport::populi_http_app_with_auth(state, PopuliHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = reqwest::Client::new();
    let first = client
        .post(format!("{base}/v1/populi/bootstrap/exchange"))
        .json(&serde_json::json!({ "bootstrap_token": bootstrap }))
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = first.json().await.unwrap();
    assert_eq!(payload["mesh_token"], "mesh-unit-token");
    assert_eq!(payload["scope_id"], "scope-unit");

    let second = client
        .post(format!("{base}/v1/populi/bootstrap/exchange"))
        .json(&serde_json::json!({ "bootstrap_token": bootstrap }))
        .send()
        .await
        .unwrap();
    assert_eq!(second.status(), reqwest::StatusCode::GONE);

    server.abort();
    unsafe {
        match prev_bootstrap {
            Some(v) => std::env::set_var("VOX_MESH_BOOTSTRAP_TOKEN", v),
            None => std::env::remove_var("VOX_MESH_BOOTSTRAP_TOKEN"),
        }
        match prev_expires {
            Some(v) => std::env::set_var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS", v),
            None => std::env::remove_var("VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS"),
        }
        match prev_token {
            Some(v) => std::env::set_var("VOX_MESH_TOKEN", v),
            None => std::env::remove_var("VOX_MESH_TOKEN"),
        }
        match prev_scope {
            Some(v) => std::env::set_var("VOX_MESH_SCOPE_ID", v),
            None => std::env::remove_var("VOX_MESH_SCOPE_ID"),
        }
    }
}
