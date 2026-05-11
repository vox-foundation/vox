//! E2E contract tests for the /api/v2/mesh/* route set (Phase 4 live-state).
//!
//! The pre-P4 fixture tests have been removed; see mesh_phase4_routes.rs for the
//! authoritative integration tests. This file retains the contract shape tests
//! updated for the live-state registry and the Phase 4 action protocol.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;

// ── helpers ───────────────────────────────────────────────────────────────────

fn app() -> axum::Router {
    vox_dashboard::test_support::build_router_with_empty_mesh()
}

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let resp = app()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn post_json(uri: &str, body: serde_json::Value) -> (StatusCode, Value) {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

// ── GET /api/v2/mesh/summary ──────────────────────────────────────────────────

#[tokio::test]
async fn mesh_summary_returns_six_kpi_fields() {
    let (status, body) = get_json("/api/v2/mesh/summary").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1);
    let data = &body["data"];
    for field in &["nodes", "active", "blocked", "errors", "tok_s", "cost_h"] {
        assert!(data[field].is_string(), "field '{field}' must be a string");
    }
}

#[tokio::test]
async fn mesh_summary_includes_build_state() {
    let (status, body) = get_json("/api/v2/mesh/summary").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["build_state"].is_string());
}

// ── GET /api/v2/mesh/nodes ────────────────────────────────────────────────────

#[tokio::test]
async fn mesh_nodes_returns_array() {
    let (status, body) = get_json("/api/v2/mesh/nodes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1);
    assert!(body["data"].is_array(), "data must be an array");
    // Live empty registry has 0 nodes (not the old fixture's 7).
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn mesh_nodes_each_have_required_fields() {
    let (_, body) = get_json("/api/v2/mesh/nodes").await;
    let nodes = body["data"].as_array().unwrap();

    for node in nodes {
        for field in &["id", "kind", "status", "model"] {
            assert!(
                node[field].is_string(),
                "node missing string field '{field}': {node}"
            );
        }
        assert!(
            node["uptime_ms"].is_number(),
            "uptime_ms must be numeric: {node}"
        );
        assert!(node["tokens"].is_number(), "tokens must be numeric: {node}");
    }
}

// ── GET /api/v2/mesh/edges ────────────────────────────────────────────────────

#[tokio::test]
async fn mesh_edges_returns_array() {
    let (status, body) = get_json("/api/v2/mesh/edges").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1);
    assert!(body["data"].is_array());
    assert_eq!(body["data"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn mesh_edges_each_have_required_fields() {
    let (_, body) = get_json("/api/v2/mesh/edges").await;
    let edges = body["data"].as_array().unwrap();

    for edge in edges {
        for field in &["from", "to", "kind", "status"] {
            assert!(
                edge[field].is_string(),
                "edge missing string field '{field}': {edge}"
            );
        }
    }
}

// ── POST /api/v2/mesh/nodes/{id}/kill|pause|replay ───────────────────────────
//
// Phase 4 (P4-T7): destructive actions require `confirm_token: "yes-i-mean-it"`.
// Without the token they return 400; with it they return 200 + signed audit_id.

#[tokio::test]
async fn mesh_node_kill_without_confirm_returns_400() {
    let (status, _) = post_json(
        "/api/v2/mesh/nodes/lex-2/kill",
        serde_json::json!({"reason": "test"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn mesh_node_kill_with_confirm_returns_200_with_audit_id() {
    let app = vox_dashboard::test_support::build_router_with_signing_keys();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/mesh/nodes/lex-2/kill")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"reason":"test","confirm_token":"yes-i-mean-it"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["v"], 1);
    assert!(body["data"]["audit_id"].is_string());
    assert_eq!(body["data"]["action"], "kill");
    assert_eq!(body["data"]["target"], "lex-2");
}

#[tokio::test]
async fn mesh_node_pause_with_confirm_returns_200() {
    let app = vox_dashboard::test_support::build_router_with_signing_keys();
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/mesh/nodes/parse-1/pause")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"reason":"test","confirm_token":"yes-i-mean-it"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["data"]["action"], "pause");
}
