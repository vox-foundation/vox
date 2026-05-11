//! Hopper panel smoke tests — Phase 4, P4-T13 (Hp-T6).
//!
//! Verifies the submit → list-inbox → reprioritize → list-history round-trip
//! with the audit trail intact and signed.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

// ── Helper ────────────────────────────────────────────────────────────────────

async fn body_json(body: axum::body::Body) -> Value {
    let bytes = axum::body::to_bytes(body, 64 * 1024).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn json_body(v: serde_json::Value) -> Body {
    Body::from(v.to_string())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn submit_returns_classified_priority() {
    let (app, _hopper) = vox_dashboard::test_support::build_router_with_hopper();
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v2/hopper/submit")
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({
                    "intent": "fix flaky test",
                    "session_id": "sess-abc"
                })))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res.into_body()).await;
    assert!(v["data"]["item_id"].as_str().unwrap().len() > 0);
    assert!(
        ["urgent", "normal", "background"]
            .contains(&v["data"]["classified_priority"].as_str().unwrap())
    );
    assert!(v["data"]["confidence"].as_f64().unwrap() > 0.0);
}

#[tokio::test]
async fn inbox_lists_submitted_items() {
    let (app, hopper) = vox_dashboard::test_support::build_router_with_hopper();

    // Submit two items directly via the model.
    use vox_orchestrator::hopper::{HopperIntake, IntakeSource, PriorityHint};
    hopper
        .submit(
            "task A".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;
    hopper
        .submit(
            "task B".into(),
            vec![],
            PriorityHint::Urgent,
            IntakeSource::Agent,
            None,
        )
        .await;

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/hopper/inbox")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res.into_body()).await;
    assert_eq!(v["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn reprioritize_round_trip_with_audit_trail() {
    let (app, hopper) = vox_dashboard::test_support::build_router_with_hopper();

    // Submit via the model.
    use vox_orchestrator::hopper::{HopperIntake, IntakeSource, PriorityHint};
    let item = hopper
        .submit(
            "improve perf".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;
    let item_id = item.item_id.0.clone();

    // Reprioritize via HTTP.
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/v2/hopper/items/{item_id}/reprioritize"))
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({
                    "new_priority":  "urgent",
                    "reason":        "customer escalation",
                    "confirm_token": "yes-i-mean-it"
                })))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res.into_body()).await;
    assert_eq!(v["data"]["priority"].as_str().unwrap(), "urgent");
    assert!(
        v["data"]["audit_id"]
            .as_str()
            .unwrap()
            .starts_with("audit-")
    );
    // signature is a 128-char hex string (Ed25519 64 bytes → hex)
    assert!(v["data"]["signature"].as_str().unwrap().len() >= 64);

    // Verify the override is in the item's history via the model.
    let inbox = hopper.inbox().await;
    let updated = inbox.iter().find(|i| i.item_id.0 == item_id).unwrap();
    assert_eq!(updated.override_history.len(), 1);
    assert_eq!(updated.override_history[0].reason, "customer escalation");
}

#[tokio::test]
async fn reprioritize_without_confirm_token_returns_400() {
    let (app, hopper) = vox_dashboard::test_support::build_router_with_hopper();

    use vox_orchestrator::hopper::{HopperIntake, IntakeSource, PriorityHint};
    let item = hopper
        .submit(
            "task".into(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!(
                    "/api/v2/hopper/items/{}/reprioritize",
                    item.item_id.0
                ))
                .header("content-type", "application/json")
                .body(json_body(serde_json::json!({
                    "new_priority": "urgent",
                    "reason": "forgot token"
                })))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
