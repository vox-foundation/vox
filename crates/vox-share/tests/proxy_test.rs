use axum::{routing::get, Router};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use vox_share::auth::AuthMode;
use vox_share::proxy::ProxyConfig;

#[tokio::test]
async fn proxy_forwards_get_request_body_unchanged() {
    // Spawn a tiny upstream server
    let upstream = Router::new().route("/hello", get(|| async { "hello-from-upstream" }));
    let upstream_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_port = upstream_listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        axum::serve(upstream_listener, upstream).await.unwrap();
    });

    // Build and spawn the proxy
    let cfg = ProxyConfig {
        upstream_addr: SocketAddr::from(([127, 0, 0, 1], upstream_port)),
        bind_addr: SocketAddr::from(([127, 0, 0, 1], 0)),
        auth_mode: AuthMode::None,
    };
    let proxy_listener = TcpListener::bind(cfg.bind_addr).await.unwrap();
    let proxy_port = proxy_listener.local_addr().unwrap().port();
    let proxy_app = vox_share::proxy::build_app(cfg.clone());
    tokio::spawn(async move {
        axum::serve(proxy_listener, proxy_app).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/hello", proxy_port))
        .send()
        .await
        .expect("proxy should accept the request");
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "hello-from-upstream");
}
