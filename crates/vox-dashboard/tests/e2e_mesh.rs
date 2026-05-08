//! E2E contract tests for the /api/v2/mesh/* route set (Phase 2.7).
//!
//! Spins up the mesh sub-router in-process via `tower::ServiceExt::oneshot`
//! and asserts the shape / HTTP contract of every route.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use serde_json::Value;
use tower::ServiceExt;
use vox_dashboard::api::mesh_router;

// ── helpers ───────────────────────────────────────────────────────────────────

fn app() -> Router {
    mesh_router::<()>()
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
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    (status, val)
}

async fn post_json(uri: &str) -> (StatusCode, Value) {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    (status, val)
}

// ── GET /api/v2/mesh/summary ──────────────────────────────────────────────────

#[tokio::test]
async fn mesh_summary_returns_six_kpi_fields() {
    let (status, body) = get_json("/api/v2/mesh/summary").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1, "envelope version must be 1");

    let data = &body["data"];
    for field in &["nodes", "active", "blocked", "errors", "tok_s", "cost_h"] {
        assert!(
            data[field].is_string(),
            "field '{field}' must be a string, got: {}",
            data[field]
        );
    }
}

#[tokio::test]
async fn mesh_summary_includes_build_state() {
    let (status, body) = get_json("/api/v2/mesh/summary").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        body["data"]["build_state"].is_string(),
        "build_state must be present"
    );
    assert_eq!(body["data"]["build_state"], "idle");
}

// ── GET /api/v2/mesh/nodes ────────────────────────────────────────────────────

#[tokio::test]
async fn mesh_nodes_returns_seven_fixture_nodes() {
    let (status, body) = get_json("/api/v2/mesh/nodes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1);

    let nodes = body["data"].as_array().expect("data must be an array");
    assert_eq!(nodes.len(), 7, "fixture topology has 7 nodes");
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
        assert!(
            node["tokens"].is_number(),
            "tokens must be numeric: {node}"
        );
    }
}

#[tokio::test]
async fn mesh_nodes_contains_both_orchestrators() {
    let (_, body) = get_json("/api/v2/mesh/nodes").await;
    let nodes = body["data"].as_array().unwrap();
    let ids: Vec<&str> = nodes
        .iter()
        .map(|n| n["id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"orchestrator-7c2a"), "orchestrator-7c2a must be present");
    assert!(ids.contains(&"orchestrator-3f1b"), "orchestrator-3f1b must be present");
}

// ── GET /api/v2/mesh/edges ────────────────────────────────────────────────────

#[tokio::test]
async fn mesh_edges_returns_six_fixture_edges() {
    let (status, body) = get_json("/api/v2/mesh/edges").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1);

    let edges = body["data"].as_array().expect("data must be an array");
    assert_eq!(edges.len(), 6, "fixture topology has 6 edges");
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

#[tokio::test]
async fn mesh_edges_delegation_edge_present() {
    let (_, body) = get_json("/api/v2/mesh/edges").await;
    let edges = body["data"].as_array().unwrap();
    let delegation = edges.iter().find(|e| e["kind"] == "delegation");
    assert!(delegation.is_some(), "must have at least one delegation edge");
    let d = delegation.unwrap();
    assert_eq!(d["from"], "orchestrator-7c2a");
    assert_eq!(d["to"],   "orchestrator-3f1b");
}

// ── POST /api/v2/mesh/nodes/{id}/kill|pause|replay ───────────────────────────

#[tokio::test]
async fn mesh_node_kill_returns_ack() {
    let (status, body) = post_json("/api/v2/mesh/nodes/lex-2/kill").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1);
    assert_eq!(body["data"]["id"],     "lex-2");
    assert_eq!(body["data"]["action"], "kill");
}

#[tokio::test]
async fn mesh_node_pause_returns_ack() {
    let (status, body) = post_json("/api/v2/mesh/nodes/parse-1/pause").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"],     "parse-1");
    assert_eq!(body["data"]["action"], "pause");
}

#[tokio::test]
async fn mesh_node_replay_returns_ack() {
    let (status, body) = post_json("/api/v2/mesh/nodes/hir-3/replay").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"],     "hir-3");
    assert_eq!(body["data"]["action"], "replay");
}
