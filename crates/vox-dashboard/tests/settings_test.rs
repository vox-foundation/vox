use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{Value, json};
use std::env;
use std::fs;
use tower::ServiceExt;
use vox_dashboard::{api::settings::SettingsState, dashboard_router};

struct EnvGuard(&'static str);
impl Drop for EnvGuard {
    fn drop(&mut self) {
        unsafe {
            env::remove_var(self.0);
        }
    }
}

fn setup_dummy_assets(dir: &tempfile::TempDir) {
    let index_path = dir.path().join("index.html");
    fs::write(index_path, "<html><head></head><body></body></html>").unwrap();
    unsafe {
        env::set_var("VOX_DASHBOARD_ASSET_DIR", dir.path().to_str().unwrap());
    }
}

// ---------------------------------------------------------------------------
// Unit-level tests — exercise SettingsState directly (no HTTP stack)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn put_token_stores_only_masked_last4() {
    let temp_dir = tempfile::tempdir().unwrap();
    let settings_json = temp_dir.path().join("settings.json");
    let state = SettingsState::for_test(settings_json.clone());

    state.put_token_mask("anthropic", "9f2c").await.unwrap();

    let snap = state.snapshot().await;
    assert_eq!(snap["tokens.anthropic.last4"], "9f2c");
    assert_eq!(snap["tokens.anthropic.status"], "ok");
    assert!(
        snap["tokens.anthropic.added_ms"].is_number(),
        "added_ms should be a number"
    );

    // Critical: the full token must NOT appear in the persisted file.
    let json_on_disk = fs::read_to_string(&settings_json).unwrap();
    assert!(!json_on_disk.contains("sk-"), "full token leaked to disk");
    assert!(json_on_disk.contains("9f2c"), "last4 should be on disk");
}

#[tokio::test]
async fn remove_token_clears_keys() {
    let temp_dir = tempfile::tempdir().unwrap();
    let state = SettingsState::for_test(temp_dir.path().join("settings.json"));

    state.put_token_mask("openai", "ab12").await.unwrap();
    {
        let snap = state.snapshot().await;
        assert_eq!(snap["tokens.openai.last4"], "ab12");
    }

    state.remove_token("openai").await.unwrap();
    let snap = state.snapshot().await;
    assert!(
        snap.get("tokens.openai.last4").is_none(),
        "last4 should be gone"
    );
    assert!(
        snap.get("tokens.openai.added_ms").is_none(),
        "added_ms should be gone"
    );
    assert!(
        snap.get("tokens.openai.status").is_none(),
        "status should be gone"
    );
}

#[tokio::test]
async fn put_token_mask_persists_across_reload() {
    let temp_dir = tempfile::tempdir().unwrap();
    let settings_json = temp_dir.path().join("settings.json");

    {
        let state = SettingsState::for_test(settings_json.clone());
        state.put_token_mask("anthropic", "z9q1").await.unwrap();
    }

    // Reload from disk in a new SettingsState instance.
    let state2 = SettingsState::for_test(settings_json);
    let snap = state2.snapshot().await;
    assert_eq!(snap["tokens.anthropic.last4"], "z9q1");
    assert_eq!(snap["tokens.anthropic.status"], "ok");
}

// ---------------------------------------------------------------------------
// HTTP-level tests — exercise the routes through the full Axum router
// ---------------------------------------------------------------------------

#[serial_test::serial]
#[tokio::test]
async fn put_token_route_via_http() {
    let tmp = tempfile::tempdir().unwrap();
    setup_dummy_assets(&tmp);
    let dir_str = tmp.path().to_str().unwrap().to_string();
    unsafe {
        env::set_var("VOX_CONFIG_DIR", &dir_str);
    }
    let _guard = EnvGuard("VOX_CONFIG_DIR");

    let app = dashboard_router::<()>(None);
    let body = json!({ "token": "sk-test-9f2c" }).to_string();
    let req = Request::builder()
        .method("PUT")
        .uri("/api/dashboard/settings/tokens/anthropic")
        .header("content-type", "application/json")
        .body(Body::from(body))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 64)
        .await
        .unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();

    // Envelope shape
    assert_eq!(val["v"], 1);
    assert_eq!(val["data"]["provider"], "anthropic");
    assert_eq!(val["data"]["last4"], "9f2c");
    assert_eq!(val["data"]["status"], "ok");

    // The full token must NOT appear in the response body.
    let raw = std::str::from_utf8(&bytes).unwrap();
    assert!(
        !raw.contains("sk-test-9f2c"),
        "full token must not appear in response"
    );

    // The persisted file must contain only last4, not the full token.
    let json_on_disk = fs::read_to_string(tmp.path().join("dashboard-settings.json")).unwrap();
    assert!(
        !json_on_disk.contains("sk-test-9f2c"),
        "full token leaked to disk"
    );
    assert!(json_on_disk.contains("9f2c"), "last4 must be on disk");
}

#[serial_test::serial]
#[tokio::test]
async fn put_token_route_returns_400_on_missing_token() {
    let tmp = tempfile::tempdir().unwrap();
    setup_dummy_assets(&tmp);
    let dir_str = tmp.path().to_str().unwrap().to_string();
    unsafe {
        env::set_var("VOX_CONFIG_DIR", &dir_str);
    }
    let _guard = EnvGuard("VOX_CONFIG_DIR");

    let app = dashboard_router::<()>(None);

    // Missing "token" field
    let req = Request::builder()
        .method("PUT")
        .uri("/api/dashboard/settings/tokens/anthropic")
        .header("content-type", "application/json")
        .body(Body::from(json!({}).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[serial_test::serial]
#[tokio::test]
async fn put_token_route_returns_400_on_empty_token() {
    let tmp = tempfile::tempdir().unwrap();
    setup_dummy_assets(&tmp);
    let dir_str = tmp.path().to_str().unwrap().to_string();
    unsafe {
        env::set_var("VOX_CONFIG_DIR", &dir_str);
    }
    let _guard = EnvGuard("VOX_CONFIG_DIR");

    let app = dashboard_router::<()>(None);

    let req = Request::builder()
        .method("PUT")
        .uri("/api/dashboard/settings/tokens/anthropic")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "token": "" }).to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[serial_test::serial]
#[tokio::test]
async fn delete_token_route_removes_keys() {
    let tmp = tempfile::tempdir().unwrap();
    setup_dummy_assets(&tmp);
    let dir_str = tmp.path().to_str().unwrap().to_string();
    unsafe {
        env::set_var("VOX_CONFIG_DIR", &dir_str);
    }
    let _guard = EnvGuard("VOX_CONFIG_DIR");

    // First PUT a token.
    let app = dashboard_router::<()>(None);
    let put_req = Request::builder()
        .method("PUT")
        .uri("/api/dashboard/settings/tokens/openai")
        .header("content-type", "application/json")
        .body(Body::from(json!({ "token": "sk-openai-abcd" }).to_string()))
        .unwrap();
    let put_resp = app.clone().oneshot(put_req).await.unwrap();
    assert_eq!(put_resp.status(), StatusCode::OK);

    // Then DELETE it.
    let del_req = Request::builder()
        .method("DELETE")
        .uri("/api/dashboard/settings/tokens/openai")
        .body(Body::empty())
        .unwrap();
    let del_resp = app.oneshot(del_req).await.unwrap();
    assert_eq!(del_resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(del_resp.into_body(), 1024 * 64)
        .await
        .unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(val["v"], 1);
    assert_eq!(val["data"]["provider"], "openai");
    assert_eq!(val["data"]["removed"], true);

    // Key must be absent from disk.
    let json_on_disk = fs::read_to_string(tmp.path().join("dashboard-settings.json")).unwrap();
    assert!(
        !json_on_disk.contains("openai"),
        "token keys should be gone from disk"
    );
}
