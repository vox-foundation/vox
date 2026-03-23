#![allow(missing_docs)]

//! Background mesh federation poller updates [`vox_mcp::ServerState::mesh_remote_snapshot`].

use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::Duration;

use vox_mcp::ServerState;
use vox_mesh::http_client::MeshHttpClient;
use vox_mesh::transport::{MeshHttpAuth, MeshTransportState, mesh_http_app_with_auth};
use vox_orchestrator::OrchestratorConfig;

/// Poll TCP connectivity until the server accepts connections, bounded to avoid infinite loops.
async fn wait_for_tcp(addr: SocketAddr) {
    for _ in 0..100 {
        if TcpStream::connect_timeout(&addr, Duration::from_millis(10)).is_ok() {
            return;
        }
        tokio::task::yield_now().await;
    }
}

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
    // Wait for server to be ready without sleeping.
    wait_for_tcp(bound).await;

    let base = format!("http://{}", bound);
    let client = MeshHttpClient::new(&base);
    let node = vox_mesh::node_record_for_current_process("poll-test-node".into(), None);
    client.join(&node).await.unwrap();

    let mut cfg = OrchestratorConfig::default();
    cfg.mesh_control_url = Some(base);
    cfg.mesh_poll_interval_secs = 1; // Must be >= 1; 0 disables the poller (see server.rs).
    cfg.mesh_http_timeout_ms = 5000;

    let st = ServerState::new(cfg);

    // Poll until snapshot is populated; poller fires within ~1s. Max 3s total.
    let snap = {
        let mut result = st.mesh_remote_snapshot.read().await.clone();
        for _ in 0..60 {
            if result.ok {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
            result = st.mesh_remote_snapshot.read().await.clone();
        }
        result
    };
    assert!(snap.ok, "snap={snap:?}");
    assert_eq!(snap.node_count, 1);

    server.abort();
}
