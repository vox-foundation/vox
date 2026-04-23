use axum::response::IntoResponse;
use axum::extract::Extension;
use vox_dashboard::assets::serve_asset;
use std::env;
use std::fs;

fn setup_dummy_assets() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let index_path = dir.path().join("index.html");
    fs::write(index_path, "<html><head></head><body></body></html>").unwrap();
    unsafe { env::set_var("VOX_DASHBOARD_ASSET_DIR", dir.path().to_str().unwrap()); }
    dir
}

#[tokio::test]
async fn test_asset_serving_headers() {
    let _dir = setup_dummy_assets();
    let response = serve_asset(None, axum::http::HeaderMap::new(), Extension(None)).await.into_response();
    let headers = response.headers();
    assert_eq!(headers.get("X-Frame-Options").unwrap(), "DENY");
    let csp = headers.get("Content-Security-Policy").unwrap().to_str().unwrap();
    assert!(csp.contains("frame-ancestors 'none'"));
    assert!(csp.contains("default-src 'self' 'unsafe-inline'"));
}

#[tokio::test]
async fn test_token_injection() {
    let _dir = setup_dummy_assets();
    let response = serve_asset(None, axum::http::HeaderMap::new(), Extension(Some("mock_token_123".to_string()))).await.into_response();
    let headers = response.headers();
    assert_eq!(headers.get("Cache-Control").unwrap(), "no-store");
}

