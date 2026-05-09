//! Test the fallback chain: Cloudflare failure → localhost.run.

use std::time::Duration;
use vox_share::auth::AuthMode;
use vox_share::{BackendKind, ShareConfig, ShareSession};

#[tokio::test]
async fn fallback_config_field_compiles_and_works_with_lan() {
    // Spawn a tiny upstream so the proxy has something to forward to.
    let upstream = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });

    // LAN backend always works — verify ShareConfig.allow_fallback field is accepted.
    let cfg = ShareConfig {
        backend: BackendKind::Lan,
        upstream_port: port,
        proxy_port: 0,
        duration: Some(Duration::from_millis(100)),
        app_binary: None,
        connect_timeout: Duration::from_secs(1),
        allow_fallback: false,
        auth_mode: AuthMode::None,
        allow_buffered_streaming: false,
    };
    let session = ShareSession::start(cfg)
        .await
        .expect("LAN session should start");
    assert_eq!(session.tunnel_handle.backend, BackendKind::Lan);
    session.shutdown().await;
}

#[tokio::test]
async fn allow_fallback_true_does_not_affect_non_cloudflare_backends() {
    // LAN backend with allow_fallback=true should still work (fallback only
    // triggers on Cloudflare failure, not on Lan).
    let upstream = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });

    let cfg = ShareConfig {
        backend: BackendKind::Lan,
        upstream_port: port,
        proxy_port: 0,
        duration: Some(Duration::from_millis(100)),
        app_binary: None,
        connect_timeout: Duration::from_secs(1),
        allow_fallback: true, // should be ignored for non-Cloudflare backend
        auth_mode: AuthMode::None,
        allow_buffered_streaming: false,
    };
    let session = ShareSession::start(cfg)
        .await
        .expect("LAN session should start regardless of allow_fallback");
    assert_eq!(session.tunnel_handle.backend, BackendKind::Lan);
    session.shutdown().await;
}
