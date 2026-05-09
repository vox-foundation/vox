//! Coordinator integration test using the LAN backend (zero infrastructure).

use std::time::Duration;
use vox_share::auth::AuthMode;
use vox_share::{BackendKind, ShareConfig};

#[tokio::test]
async fn coordinator_starts_lan_session_against_a_dummy_app() {
    // Spawn a tiny upstream so the proxy has something to forward to.
    let upstream = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));
    let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_port = upstream_listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream).await.unwrap();
    });

    let cfg = ShareConfig {
        backend: BackendKind::Lan,
        upstream_port,
        proxy_port: 0, // OS-pick
        duration: Some(Duration::from_secs(2)),
        app_binary: None, // already running externally
        connect_timeout: Duration::from_secs(2),
        allow_fallback: false,
        auth_mode: AuthMode::None,
        allow_buffered_streaming: false,
    };
    let session = vox_share::ShareSession::start(cfg)
        .await
        .expect("LAN session should start");

    assert_eq!(session.tunnel_handle.backend, BackendKind::Lan);
    assert!(session.public_url.starts_with("http://"));
    // The tunnel handle's public_url for LAN backend points at the LAN IP + proxy_port.
    // We verify the proxy works by hitting it via 127.0.0.1:<proxy_port>.
    assert!(session.proxy_port > 0);

    let resp = reqwest::get(format!("http://127.0.0.1:{}/", session.proxy_port))
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");

    session.shutdown().await;
}
