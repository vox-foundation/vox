use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use tower::ServiceExt;
use vox_orchestrator::services::routes;

#[tokio::test]
async fn api_v2_health_returns_envelope() {
    let app = routes::router();
    let req = Request::builder()
        .uri("/api/v2/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 64 * 1024)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["v"], 1);
    assert_eq!(body["data"]["status"], "ok");
}
