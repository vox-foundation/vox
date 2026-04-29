use axum::{body::Body, http::Request};
use tower::ServiceExt;
use vox_dashboard::dashboard_router;
use std::fs;
use std::env;

fn setup_dummy_assets() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let index_path = dir.path().join("index.html");
    fs::write(index_path, "<html><head></head><body></body></html>").unwrap();
    unsafe { env::set_var("VOX_DASHBOARD_ASSET_DIR", dir.path().to_str().unwrap()); }
    dir
}

#[tokio::test]
async fn test_dashboard_router_serves_asset() {
    let _dir = setup_dummy_assets();
    let app = dashboard_router::<()>(Some("mock_token_123".to_string()));

    let req = Request::builder()
        .uri("/dashboard")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    
    // Check security headers
    let headers = response.headers();
    assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
    assert_eq!(headers.get("Cache-Control").unwrap(), "no-store");
    assert!(headers.get("Content-Security-Policy").unwrap().to_str().unwrap().contains("frame-ancestors 'none'"));
}

#[tokio::test]
async fn test_dashboard_router_injects_token() {
    let _dir = setup_dummy_assets();
    let app = dashboard_router::<()>(Some("secret_token_abc".to_string()));

    let req = Request::builder()
        .uri("/dashboard/index.html")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    
    assert!(html.contains("<meta name=\"vox-bearer\" content=\"secret_token_abc\">"));
}

#[tokio::test]
async fn test_dashboard_router_no_token_injection() {
    let _dir = setup_dummy_assets();
    let app = dashboard_router::<()>(None);

    let req = Request::builder()
        .uri("/dashboard")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8(bytes.to_vec()).unwrap();
    
    assert!(!html.contains("vox-bearer"));
}
