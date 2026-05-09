//! E2E contract tests for the /api/v2/runs/* route set (Phase 3).
//!
//! Spins up the runs sub-router in-process via `tower::ServiceExt::oneshot`
//! and asserts the HTTP contract + response shape of every route.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use vox_dashboard::api::runs_router;

// ── helpers ───────────────────────────────────────────────────────────────────

fn app() -> Router {
    runs_router::<()>()
}

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let resp = app()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 256 * 1024)
        .await
        .unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    (status, val)
}

// ── GET /api/v2/runs ──────────────────────────────────────────────────────────

#[tokio::test]
async fn runs_list_returns_five_fixture_runs() {
    let (status, body) = get_json("/api/v2/runs").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1, "envelope version must be 1");

    let runs = body["data"].as_array().expect("data must be an array");
    assert_eq!(runs.len(), 5, "fixture dataset has exactly 5 runs");
}

#[tokio::test]
async fn runs_list_each_run_has_required_fields() {
    let (_, body) = get_json("/api/v2/runs").await;
    let runs = body["data"].as_array().unwrap();

    for run in runs {
        for field in &["id", "started", "model", "status"] {
            assert!(
                run[field].is_string(),
                "run missing string field '{field}': {run}"
            );
        }
        for field in &["duration_ms", "tokens"] {
            assert!(
                run[field].is_number(),
                "run missing numeric field '{field}': {run}"
            );
        }
        assert!(
            run["cost_usd"].is_number(),
            "run must have numeric cost_usd: {run}"
        );
        assert!(
            run["events"].is_array(),
            "run must have events array: {run}"
        );
    }
}

#[tokio::test]
async fn runs_list_contains_mixed_statuses() {
    let (_, body) = get_json("/api/v2/runs").await;
    let runs = body["data"].as_array().unwrap();
    let statuses: Vec<&str> = runs.iter().map(|r| r["status"].as_str().unwrap()).collect();

    assert!(statuses.contains(&"ok"), "must have at least one ok run");
    assert!(
        statuses.contains(&"error"),
        "must have at least one error run"
    );
}

#[tokio::test]
async fn runs_list_events_have_required_fields() {
    let (_, body) = get_json("/api/v2/runs").await;
    let runs = body["data"].as_array().unwrap();

    for run in runs {
        let events = run["events"].as_array().expect("events must be array");
        for event in events {
            for field in &["id", "kind", "label"] {
                assert!(
                    event[field].is_string(),
                    "event missing string field '{field}': {event}"
                );
            }
            assert!(
                event["ts_ms"].is_number(),
                "event must have numeric ts_ms: {event}"
            );
        }
    }
}

// ── GET /api/v2/runs/{id} ─────────────────────────────────────────────────────

#[tokio::test]
async fn get_run_by_id_returns_correct_run() {
    let (status, body) = get_json("/api/v2/runs/run-a1b2").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["v"], 1);
    assert_eq!(body["data"]["id"], "run-a1b2");
    assert_eq!(body["data"]["model"], "sonnet-4.6");
}

#[tokio::test]
async fn get_run_by_id_includes_events() {
    let (status, body) = get_json("/api/v2/runs/run-a1b2").await;
    assert_eq!(status, StatusCode::OK);

    let events = body["data"]["events"]
        .as_array()
        .expect("events must be array");
    assert!(!events.is_empty(), "run-a1b2 must have at least one event");

    // First event should be task.started
    assert_eq!(events[0]["kind"], "task.started");
    assert_eq!(events[0]["ts_ms"], 0);
}

#[tokio::test]
async fn get_run_error_run_has_agent_error_event() {
    let (status, body) = get_json("/api/v2/runs/run-e5f6").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], "error");

    let events = body["data"]["events"].as_array().unwrap();
    let error_event = events.iter().find(|e| e["kind"] == "agent.error");
    assert!(
        error_event.is_some(),
        "error run must contain an agent.error event"
    );
}

#[tokio::test]
async fn get_run_unknown_id_returns_404() {
    let (status, body) = get_json("/api/v2/runs/run-doesnotexist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["v"], 1);
    assert!(
        body["error"]["code"].is_string(),
        "404 must include error.code"
    );
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn get_all_five_fixture_runs_by_id() {
    for id in &["run-a1b2", "run-c3d4", "run-e5f6", "run-g7h8", "run-i9j0"] {
        let (status, body) = get_json(&format!("/api/v2/runs/{id}")).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "GET /api/v2/runs/{id} must return 200"
        );
        assert_eq!(
            body["data"]["id"], *id,
            "returned run id must match requested id"
        );
    }
}
