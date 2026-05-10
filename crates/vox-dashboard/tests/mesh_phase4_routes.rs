//! Phase 4 route integration tests (P4-T1, P4-T2, …).
//!
//! Each test builds an isolated router against an empty MeshRegistry and
//! verifies that the live routes return the correct shape.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

// ── P4-T1: Live mesh routes return live state, not fixtures ───────────────────

#[tokio::test]
async fn nodes_route_returns_live_state_not_fixture() {
    // The old fixture had exactly 7 entries. Live state on an empty registry is 0.
    let app = vox_dashboard::test_support::build_router_with_empty_mesh();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/mesh/nodes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 8 * 1024).await.unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let arr = v["data"].as_array().expect("data should be an array");
    assert_eq!(
        arr.len(),
        0,
        "live empty mesh should have 0 nodes, got fixture instead"
    );
}

#[tokio::test]
async fn summary_route_returns_live_zeroes() {
    let app = vox_dashboard::test_support::build_router_with_empty_mesh();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/mesh/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 4 * 1024).await.unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    let data = &v["data"];
    assert_eq!(data["nodes"].as_str().unwrap(), "0");
    assert_eq!(data["active"].as_str().unwrap(), "0");
}

#[tokio::test]
async fn edges_route_returns_empty_array_on_fresh_registry() {
    let app = vox_dashboard::test_support::build_router_with_empty_mesh();
    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/mesh/edges")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(res.into_body(), 4 * 1024).await.unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["data"].as_array().unwrap().len(), 0);
}

// ── P4-T1c: WS event bus subscription round-trip ─────────────────────────────

#[tokio::test]
async fn topology_changed_event_reaches_ws_subscriber() {
    use vox_orchestrator::events::AgentEventKind;

    let (registry, bus) = vox_dashboard::test_support::build_mesh_state();
    let _ = registry; // registry not needed for this test

    let mut rx = bus.subscribe();
    bus.emit(AgentEventKind::MeshTopologyChanged {
        added_nodes: vec!["alice-gpu".into()],
        removed_nodes: vec![],
        changed_edges: 0,
    });
    let evt = rx.recv().await.unwrap();
    match evt.kind {
        AgentEventKind::MeshTopologyChanged { added_nodes, .. } => {
            assert_eq!(added_nodes, vec!["alice-gpu".to_string()]);
        }
        other => panic!("expected MeshTopologyChanged, got {other:?}"),
    }
}

#[tokio::test]
async fn budget_event_reaches_subscriber() {
    use vox_orchestrator::events::AgentEventKind;

    let (_registry, bus) = vox_dashboard::test_support::build_mesh_state();
    let mut rx = bus.subscribe();
    bus.emit(AgentEventKind::MeshNodeBudget {
        node_id: "node-1".into(),
        cost_usd_24h: 1.23,
        cost_cap_usd: 10.0,
        token_count_24h: 50_000,
    });
    let evt = rx.recv().await.unwrap();
    match evt.kind {
        AgentEventKind::MeshNodeBudget { node_id, cost_usd_24h, .. } => {
            assert_eq!(node_id, "node-1");
            assert!((cost_usd_24h - 1.23).abs() < 1e-9);
        }
        other => panic!("expected MeshNodeBudget, got {other:?}"),
    }
}
