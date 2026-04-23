// origin_guard.rs - vox-dashboard integration test
// Tests for CSP and origin header construction in dashboard assets.

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
async fn test_asset_csp_origin_guard() {
    let _dir = setup_dummy_assets();
    let response = serve_asset(None, axum::http::HeaderMap::new(), Extension(None)).await.into_response();
    let csp = response.headers().get("Content-Security-Policy").unwrap().to_str().unwrap();
    
    // Ensure dashboard cannot be embedded or connect to wild origins
    assert!(csp.contains("connect-src 'self' ws: wss:"));
    assert!(csp.contains("frame-ancestors 'none'"));
}
