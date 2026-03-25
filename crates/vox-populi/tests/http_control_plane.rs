#![allow(missing_docs)]

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use vox_populi::http_client::MeshHttpClient;
use vox_populi::transport::{MeshHttpAuth, MeshTransportState};
use vox_populi::{node_record_for_current_process, transport};

#[tokio::test]
async fn join_list_heartbeat_roundtrip() {
    let state = MeshTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::mesh_http_app_with_auth(state, MeshHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = MeshHttpClient::new(&base);
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
    let state = MeshTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::mesh_http_app_with_auth(state, MeshHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = MeshHttpClient::new(&base);
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
    let state = MeshTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::mesh_http_app_with_auth(
        state,
        MeshHttpAuth::Bearer("unit-test-populi-token".into()),
    );
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let no_auth = MeshHttpClient::new(&base);
    assert!(no_auth.list_nodes().await.is_err());

    let authed = MeshHttpClient::new(&base).with_bearer("unit-test-populi-token");
    assert!(authed.list_nodes().await.is_ok());

    server.abort();
}

#[tokio::test]
async fn health_ok_without_bearer_when_token_required() {
    let state = MeshTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::mesh_http_app_with_auth(
        state,
        transport::MeshHttpAuth::Bearer("unit-test-populi-token".into()),
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
    let state = MeshTransportState::with_required_scope(Some("cluster-a".into()));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::mesh_http_app_with_auth(state, MeshHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = MeshHttpClient::new(&base);
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
    let state = MeshTransportState::with_required_scope(Some("cluster-a".into()));
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();

    let app = transport::mesh_http_app_with_auth(state, MeshHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = MeshHttpClient::new(&base);
    let mut node = node_record_for_current_process("scoped-node".into(), None);
    node.scope_id = Some("cluster-a".into());
    client.join(&node).await.unwrap();
    assert_eq!(client.list_nodes().await.unwrap().nodes.len(), 1);

    server.abort();
}
