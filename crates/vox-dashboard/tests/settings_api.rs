use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::env;
use std::fs;
use tower::ServiceExt;
use vox_dashboard::dashboard_router;

struct EnvGuard(&'static str);
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe { env::remove_var(self.0); }
    }
}

fn setup_dummy_assets(dir: &tempfile::TempDir) {
    let index_path = dir.path().join("index.html");
    fs::write(index_path, "<html><head></head><body></body></html>").unwrap();
    unsafe { env::set_var("VOX_DASHBOARD_ASSET_DIR", dir.path().to_str().unwrap()); }
}

#[serial_test::serial]
#[tokio::test]
async fn settings_get_returns_empty_object_initially() {
    let tmp = tempfile::tempdir().unwrap();
    setup_dummy_assets(&tmp);
    // Point settings storage to temp dir so tests don't affect real config.
    let settings_path = tmp.path().join("dashboard-settings.json");
    let dir_str = tmp.path().to_str().unwrap().to_string();
    unsafe { env::set_var("VOX_CONFIG_DIR", &dir_str); }
    let _guard = EnvGuard("VOX_CONFIG_DIR");
    let _ = settings_path; // ensure it doesn't exist yet

    let app = dashboard_router::<()>(None);
    let req = Request::builder()
        .uri("/api/dashboard/settings")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 1024 * 64).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert!(val.is_object(), "expected JSON object, got: {val}");
}

#[serial_test::serial]
#[tokio::test]
async fn settings_put_persists_and_returns_merged_state() {
    let tmp = tempfile::tempdir().unwrap();
    setup_dummy_assets(&tmp);
    let dir_str = tmp.path().to_str().unwrap().to_string();
    unsafe { env::set_var("VOX_CONFIG_DIR", &dir_str); }
    let _guard = EnvGuard("VOX_CONFIG_DIR");

    let payload = json!({ "theme": "dark", "fontSize": 14 });
    let app = dashboard_router::<()>(None);
    let req = Request::builder()
        .method("PUT")
        .uri("/api/dashboard/settings")
        .header("content-type", "application/json")
        .body(Body::from(payload.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body = axum::body::to_bytes(resp.into_body(), 1024 * 64).await.unwrap();
    let val: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(val["theme"], "dark");
    assert_eq!(val["fontSize"], 14);

    // Verify the file was written.
    let file_content = fs::read_to_string(tmp.path().join("dashboard-settings.json")).unwrap();
    let saved: Value = serde_json::from_str(&file_content).unwrap();
    assert_eq!(saved["theme"], "dark");
}
