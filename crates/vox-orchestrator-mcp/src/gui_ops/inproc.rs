//! `orch.api_v2_get`: serve one `/api/v2` GET from inside the daemon so Axis local mode reaches
//! the exact handlers a remote Axis reaches over HTTP.

use std::net::SocketAddr;

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Method, Request};
use tower::ServiceExt;

use crate::http_gateway::GatewayState;
use crate::server_state::ServerState;

const MAX_BODY: usize = 16 * 1024 * 1024;

pub fn validate_api_v2_path(path_and_query: &str) -> Result<(), String> {
    let path = path_and_query.split('?').next().unwrap_or("");
    let ok = path.starts_with('/')
        && !path.starts_with("//")
        && !path.contains("..")
        && !path.contains('\\')
        && !path_and_query.contains('#')
        && !path_and_query.contains("://");
    if ok {
        Ok(())
    } else {
        Err(format!("invalid /api/v2 path: {path_and_query:?}"))
    }
}

pub async fn api_v2_get(
    state: &ServerState,
    path_and_query: &str,
) -> Result<serde_json::Value, String> {
    validate_api_v2_path(path_and_query)?;
    let app = axum::Router::new()
        .nest("/api/v2", crate::http_gateway::dashboard_api::router())
        .with_state(GatewayState::for_inproc_loopback(state.clone()));
    let mut req = Request::builder()
        .method(Method::GET)
        .uri(format!("/api/v2{path_and_query}"))
        .body(Body::empty())
        .map_err(|e| e.to_string())?;
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    let resp = app.oneshot(req).await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), MAX_BODY)
        .await
        .map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "GET /api/v2{path_and_query}: HTTP {}",
            status.as_u16()
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub async fn api_v2_post(
    state: &ServerState,
    path_and_query: &str,
    payload: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    validate_api_v2_path(path_and_query)?;
    let app = axum::Router::new()
        .nest("/api/v2", crate::http_gateway::dashboard_api::router())
        .with_state(GatewayState::for_inproc_loopback(state.clone()));
    let body_bytes = serde_json::to_vec(payload).map_err(|e| e.to_string())?;
    let mut req = Request::builder()
        .method(Method::POST)
        .uri(format!("/api/v2{path_and_query}"))
        .header("content-type", "application/json")
        .body(Body::from(body_bytes))
        .map_err(|e| e.to_string())?;
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    let resp = app.oneshot(req).await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), MAX_BODY)
        .await
        .map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "POST /api/v2{path_and_query}: HTTP {}",
            status.as_u16()
        ));
    }
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_rejects_escapes_and_absolute_urls() {
        assert!(validate_api_v2_path("/models/cards?limit=3").is_ok());
        for bad in [
            "models",
            "/../v1/tools/call",
            "//evil",
            "/x#frag",
            "http://h/x",
            "/a\\b",
        ] {
            assert!(validate_api_v2_path(bad).is_err(), "{bad} must be rejected");
        }
    }

    #[tokio::test]
    async fn inproc_get_serves_existing_dashboard_route() {
        let state = ServerState::new_test().await;
        let v = api_v2_get(&state, "/vcs/isolation").await.expect("served");
        assert_eq!(v["data"]["strategy_default"], "shared_branch");
    }

    #[tokio::test]
    async fn inproc_get_refuses_post_only_route() {
        // `/hopper/submit` is POST-only: a GET must not reach the mutation.
        let state = ServerState::new_test().await;
        let err = api_v2_get(&state, "/hopper/submit").await.unwrap_err();
        assert!(err.contains("405"), "{err}");
    }
}
