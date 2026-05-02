//! Dashboard settings persistence: GET/PUT /api/dashboard/settings
//!
//! Stores a flat JSON object under `$VOX_CONFIG_DIR/dashboard-settings.json`
//! (falls back to `$HOME/.vox/dashboard-settings.json`).
//! Replaces `vscode.workspace.getConfiguration("vox")` for ported features.

use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, put},
};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct SettingsState {
    inner: Arc<RwLock<Map<String, Value>>>,
    path: Arc<PathBuf>,
}

fn settings_path() -> PathBuf {
    if let Ok(dir) = std::env::var("VOX_CONFIG_DIR") {
        return PathBuf::from(dir).join("dashboard-settings.json");
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".vox").join("dashboard-settings.json")
}

impl SettingsState {
    pub fn new() -> Self {
        let path = settings_path();
        let map = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str::<Map<String, Value>>(&s).ok())
                .unwrap_or_default()
        } else {
            Map::new()
        };
        Self {
            inner: Arc::new(RwLock::new(map)),
            path: Arc::new(path),
        }
    }
}

async fn get_settings(
    State(s): State<SettingsState>,
) -> Json<Value> {
    let map = s.inner.read().await;
    Json(Value::Object(map.clone()))
}

async fn put_settings(
    State(s): State<SettingsState>,
    Json(body): Json<Map<String, Value>>,
) -> Result<Json<Value>, StatusCode> {
    let updated = {
        let mut map = s.inner.write().await;
        for (k, v) in &body {
            map.insert(k.clone(), v.clone());
        }
        map.clone()
    }; // write lock released before async I/O
    let serialized = serde_json::to_string_pretty(&updated)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if let Some(parent) = s.path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(&*s.path, serialized)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Value::Object(updated)))
}

pub fn settings_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let state = SettingsState::new();
    Router::new()
        .route("/api/dashboard/settings", get(get_settings))
        .route("/api/dashboard/settings", put(put_settings))
        .with_state(state)
}
