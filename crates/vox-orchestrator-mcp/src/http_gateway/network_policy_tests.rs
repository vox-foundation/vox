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

use crate::profiles::{CallContext, CallerRole, Exposure};
use axum::body::to_bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderValue;
use vox_mcp_registry::McpToolRegistryEntry;

const LAN_PEER: &str = "192.168.1.20:50123";
const LOOPBACK_PEER: &str = "127.0.0.1:50123";

fn peer(s: &str) -> SocketAddr {
    s.parse().unwrap()
}

/// First registry tool matching `pred` that needs no subsystem, so the result does not depend
/// on which subsystems the test ServerState booted.
fn tool_where(pred: impl Fn(&McpToolRegistryEntry) -> bool) -> &'static str {
    TOOL_REGISTRY
        .iter()
        .find(|e| e.requires.is_empty() && pred(e))
        .map(|e| e.name)
        .expect("registry has a matching subsystem-free tool")
}

fn internal_tool() -> &'static str {
    tool_where(|e| e.exposure == Exposure::Internal)
}
fn operator_tool() -> &'static str {
    tool_where(|e| e.exposure == Exposure::Operator)
}
fn public_read_tool() -> &'static str {
    tool_where(|e| e.exposure == Exposure::Public && e.http_read_role_eligible)
}

async fn dev_state_all_tools() -> GatewayState {
    let all: HashSet<String> = TOOL_REGISTRY.iter().map(|e| e.name.to_string()).collect();
    GatewayState::for_test_with_state(ServerState::new_test().await, all).await
}

fn bearer(token: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    h
}

async fn body_json(resp: Response) -> (StatusCode, Value) {
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn loopback_comes_from_the_peer_not_headers() {
    let state = dev_state_all_tools().await;
    assert!(caller_is_loopback(&state, &peer(LOOPBACK_PEER)));
    assert!(caller_is_loopback(&state, &peer("[::1]:9")));
    assert!(!caller_is_loopback(&state, &peer(LAN_PEER)));
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    assert_eq!(
        ctx,
        CallContext {
            role: CallerRole::Write,
            loopback: false
        }
    );
}

#[tokio::test]
async fn tunnel_callers_are_never_loopback() {
    let state = dev_state_all_tools().await.with_behind_tunnel(true);
    assert!(!caller_is_loopback(&state, &peer(LOOPBACK_PEER)));
}

#[tokio::test]
async fn internal_tools_are_visible_only_to_loopback() {
    let state = dev_state_all_tools().await;
    let internal = internal_tool();
    let local = visible_tools_for(
        &state,
        call_context(&state, &peer(LOOPBACK_PEER), AccessRole::Write),
    );
    let remote = visible_tools_for(
        &state,
        call_context(&state, &peer(LAN_PEER), AccessRole::Write),
    );
    assert!(
        local.contains(internal),
        "dev profile on loopback serves {internal}"
    );
    assert!(
        !remote.contains(internal),
        "{internal} must not be served off loopback"
    );
}

#[tokio::test]
async fn non_loopback_peer_cannot_call_internal_tool() {
    let state = dev_state_all_tools().await;
    let internal = internal_tool();
    let req: ToolCallRequest =
        serde_json::from_value(serde_json::json!({ "name": internal, "args": {} })).unwrap();
    let resp = http_call_tool(
        State(state),
        ConnectInfo(peer(LAN_PEER)),
        HeaderMap::new(),
        Json(req),
    )
    .await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn read_token_off_loopback_cannot_call_operator_tool() {
    let state = dev_state_all_tools()
        .await
        .with_tokens(Some("w"), Some("r"));
    let op = operator_tool();
    let req: ToolCallRequest =
        serde_json::from_value(serde_json::json!({ "name": op, "args": {} })).unwrap();
    let resp = http_call_tool(
        State(state),
        ConnectInfo(peer(LAN_PEER)),
        bearer("r"),
        Json(req),
    )
    .await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn v1_tools_off_loopback_hides_internal_and_respects_role() {
    let state = dev_state_all_tools()
        .await
        .with_tokens(Some("w"), Some("r"));
    let names = |v: &Value| -> Vec<String> {
        v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    };
    let (_, read) = body_json(
        http_tools(
            State(state.clone()),
            ConnectInfo(peer(LAN_PEER)),
            bearer("r"),
        )
        .await,
    )
    .await;
    let (_, write) =
        body_json(http_tools(State(state), ConnectInfo(peer(LAN_PEER)), bearer("w")).await).await;
    let (read, write) = (names(&read), names(&write));
    assert!(read.iter().any(|n| n == public_read_tool()));
    assert!(!read.iter().any(|n| n == operator_tool()));
    assert!(write.iter().any(|n| n == operator_tool()));
    assert!(
        !read
            .iter()
            .chain(write.iter())
            .any(|n| n == internal_tool())
    );
}

#[tokio::test]
async fn ws_list_tools_hides_internal_off_loopback() {
    let state = dev_state_all_tools().await;
    let req: WsMessageIn =
        serde_json::from_value(serde_json::json!({ "type": "list_tools" })).unwrap();
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    let out = ws_handle_message(&state, req, peer(LAN_PEER), ctx).await;
    let tools: Vec<&str> = out.data["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert!(!tools.contains(&internal_tool()));
}

#[tokio::test]
async fn ws_call_internal_off_loopback_is_denied() {
    let state = dev_state_all_tools().await;
    let req: WsMessageIn = serde_json::from_value(
        serde_json::json!({ "type": "call_tool", "name": internal_tool(), "args": {} }),
    )
    .unwrap();
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    let out = ws_handle_message(&state, req, peer(LAN_PEER), ctx).await;
    assert!(out.is_error);
    assert_eq!(out.data["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn gated_tool_json_denies_without_executing() {
    let state = dev_state_all_tools().await;
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    let v = gated_tool_json(&state, ctx, internal_tool()).await;
    assert_eq!(v["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn dashboard_token_is_not_honoured_behind_a_tunnel() {
    let tunnelled = dev_state_all_tools()
        .await
        .with_tokens(Some("w"), Some("r"))
        .with_dashboard_token("dt")
        .with_behind_tunnel(true);
    assert!(resolve_access_role(&tunnelled, &bearer("dt"), Some(&peer(LOOPBACK_PEER))).is_err());
    let direct = dev_state_all_tools()
        .await
        .with_tokens(Some("w"), Some("r"))
        .with_dashboard_token("dt");
    assert_eq!(
        resolve_access_role(&direct, &bearer("dt"), Some(&peer(LOOPBACK_PEER))),
        Ok(AccessRole::Write)
    );
}
