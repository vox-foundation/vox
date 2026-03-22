#![allow(missing_docs)]

//! Background mesh federation poller updates [`vox_mcp::ServerState::mesh_remote_snapshot`].

use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use vox_mcp::ServerState;
use vox_mesh::http_client::MeshHttpClient;
use vox_mesh::transport::{MeshHttpAuth, MeshTransportState, mesh_http_app_with_auth};
use vox_orchestrator::OrchestratorConfig;

#[tokio::test]
async fn mesh_federation_poller_fills_snapshot() {
    let state_reg = MeshTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = mesh_http_app_with_auth(state_reg, MeshHttpAuth::Open);
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    tokio::time::sleep(Duration::from_millis(50)).await;

    let base = format!("http://{}", bound);
    let client = MeshHttpClient::new(&base);
    let node = vox_mesh::node_record_for_current_process("poll-test-node".into(), None);
    client.join(&node).await.unwrap();

    let mut cfg = OrchestratorConfig::default();
    cfg.mesh_control_url = Some(base);
    cfg.mesh_poll_interval_secs = 1;
    cfg.mesh_http_timeout_ms = 5000;

    let st = ServerState::new(cfg);
    tokio::time::sleep(Duration::from_millis(1600)).await;
    let snap = st.mesh_remote_snapshot.read().await.clone();
    assert!(snap.ok, "snap={snap:?}");
    assert_eq!(snap.node_count, 1);

    server.abort();
}
