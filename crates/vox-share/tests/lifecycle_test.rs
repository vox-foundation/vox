use std::time::Duration;
use vox_share::lifecycle::format_duration;

#[test]
fn format_duration_2h30m() {
    assert_eq!(format_duration(Duration::from_secs(9000)), "2h 30m");
}

#[test]
fn format_duration_exact_hours() {
    assert_eq!(format_duration(Duration::from_secs(3600)), "1h");
}

#[test]
fn format_duration_minutes() {
    assert_eq!(format_duration(Duration::from_secs(300)), "5m");
}

#[test]
fn format_duration_seconds() {
    assert_eq!(format_duration(Duration::from_secs(5)), "5s");
}

#[tokio::test]
async fn countdown_fires_after_short_duration() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let handle = tokio::spawn(vox_share::lifecycle::run_countdown(
        Duration::from_millis(100),
        tx,
    ));
    // Should receive signal within a few seconds
    let result = tokio::time::timeout(Duration::from_secs(3), rx.recv()).await;
    assert!(result.is_ok(), "countdown should complete within timeout");
    assert!(result.unwrap().is_some(), "should receive done signal");
    handle.abort();
}

#[tokio::test]
async fn session_wait_exits_on_duration() {
    // Spin up a dummy upstream.
    let upstream = axum::Router::new().route("/", axum::routing::get(|| async { "ok" }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(listener, upstream).await.unwrap();
    });

    use vox_share::auth::AuthMode;
    use vox_share::{BackendKind, ShareConfig, ShareSession};
    let cfg = ShareConfig {
        backend: BackendKind::Lan,
        upstream_port: port,
        proxy_port: 0,
        duration: Some(Duration::from_millis(200)),
        app_binary: None,
        connect_timeout: Duration::from_secs(1),
        allow_fallback: false,
        auth_mode: AuthMode::None,
        allow_buffered_streaming: true,
    };
    let session = ShareSession::start(cfg).await.expect("session should start");
    // wait() should return after ~200ms (duration elapsed).
    let result = tokio::time::timeout(Duration::from_secs(5), session.wait()).await;
    assert!(
        result.is_ok(),
        "session.wait() should complete when duration elapses"
    );
}
