#![allow(missing_docs)]
#![allow(unsafe_code)]

//! `publish_mesh_on_mcp_start` performs HTTP `join` when orchestrator mens URL points at a live control plane.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Mutex;

use vox_mcp::ServerState;
use vox_orchestrator::OrchestratorConfig;
use vox_populi::http_client::MeshHttpClient;
use vox_populi::transport::{MeshHttpAuth, MeshTransportState, mesh_http_app_with_auth};

static MESH_ENV_MUTEX: Mutex<()> = Mutex::new(());

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
    let _lock = MESH_ENV_MUTEX.lock().expect("mens env mutex poisoned");

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

    let state_reg = MeshTransportState::new();
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let bound = listener.local_addr().unwrap();
    let app = mesh_http_app_with_auth(state_reg, MeshHttpAuth::Open);
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

    let client = MeshHttpClient::new(&base);
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
