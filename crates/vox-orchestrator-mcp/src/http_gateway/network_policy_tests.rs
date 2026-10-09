//! Capability-profiles plan 04: bind policy, TLS transport, and per-caller exposure.

use super::bind_policy::{BindPolicyError, BindPolicyInput, GatewayTransport, TlsMaterial};
use super::*;

fn network_input_without_credentials() -> BindPolicyInput {
    BindPolicyInput {
        host: "0.0.0.0".to_string(),
        port: 0,
        has_write_token: false,
        has_read_token: false,
        tls: TlsMaterial::Absent,
        behind_tunnel: false,
        allow_unauthenticated: false,
    }
}

#[tokio::test]
async fn spawn_refuses_network_bind_without_tokens() {
    let state = ServerState::new_test().await;
    let err = spawn_http_gateway(state, network_input_without_credentials())
        .expect_err("a network bind without tokens or TLS must refuse to start");
    let policy_err = err
        .downcast_ref::<BindPolicyError>()
        .expect("the refusal must come from the bind policy, not a later check");
    assert!(
        matches!(policy_err, BindPolicyError::NetworkBindRequirements { .. }),
        "{policy_err:?}"
    );
}

#[tokio::test]
async fn tls_transport_serves_health_over_https() {
    let dir = tempfile::tempdir().unwrap();
    let (cert, key) = tls::write_self_signed(dir.path());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = build_app(GatewayState::for_test().await);
    let _server = serve_gateway(listener, app, &GatewayTransport::Tls { cert, key }).unwrap();

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let resp = client
        .get(format!("https://127.0.0.1:{port}/health"))
        .send()
        .await
        .expect("https request");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(resp.text().await.unwrap(), "ok");

    let plain = reqwest::get(format!("http://127.0.0.1:{port}/health")).await;
    assert!(
        plain.map(|r| !r.status().is_success()).unwrap_or(true),
        "plain HTTP must not be served on the TLS port"
    );
}

#[tokio::test]
async fn plain_transport_still_serves_health() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = build_app(GatewayState::for_test().await);
    let _server = serve_gateway(listener, app, &GatewayTransport::PlainHttp).unwrap();
    let resp = reqwest::get(format!("http://127.0.0.1:{port}/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
}
