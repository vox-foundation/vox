//! Dashboard settings store: flat key-value JSON, persisted to disk on PUT.
//!
//! Phase 1+ surfaces use the following dotted-key namespace convention. All keys
//! are stored as flat strings; nesting is conceptual.
//!
//! ## Identity
//! - `identity.user_name`           — string
//! - `identity.user_email`          — string
//!
//! ## API tokens (write-only via `put_token_mask`; reads return only the masked last-4)
//! - `tokens.<provider>.last4`      — string (last 4 chars of the token, after `put_token_mask`)
//! - `tokens.<provider>.added_ms`   — number (epoch ms when added)
//! - `tokens.<provider>.status`     — "ok" | "missing" — written by orchestrator on use
//! - The full token is NEVER persisted by SettingsState. `put_token_mask` writes the masked
//!   last-4 only; the full token is intended to flow to the orchestrator via a separate
//!   secrets channel (deferred to Phase 8).
//!
//! ## Budget
//! - `budget.monthly_cap_usd`       — number
//! - `budget.soft_cap_usd`          — number
//! - `budget.per_model.<id>.cap_usd` — number
//!
//! ## Telemetry
//! - `telemetry.timings`            — bool
//! - `telemetry.crashes`            — bool
//! - `telemetry.topology_snapshots` — bool
//!
//! ## Routing
//! - `routing.auto_enabled`         — bool
//! - `routing.rules`                — JSON array (opaque to SettingsState)
//!
//! ## Command palette
//! - `cmdk.recents`                 — JSON array (opaque)
//!
//! Settings drift: on read, missing keys are returned as JSON null. Surfaces are
//! responsible for their own defaults.
//!
//! ---
//! Stores a flat JSON object under `$VOX_CONFIG_DIR/dashboard-settings.json`
//! (falls back to `$HOME/.vox/dashboard-settings.json`).
//! Replaces `vscode.workspace.getConfiguration("vox")` for ported features.

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, put},
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
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
    PathBuf::from(home)
        .join(".vox")
        .join("dashboard-settings.json")
}

fn epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Default for SettingsState {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsState {
    pub fn new() -> Self {
        let path = settings_path();
        Self::with_path(path)
    }

    /// Test seam: construct a `SettingsState` pointing at an explicit path.
    /// The existing public `new()` API is unchanged.
    pub fn with_path(path: PathBuf) -> Self {
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

    /// Convenience alias used in tests (mirrors the task spec).
    /// Equivalent to `with_path`; kept as a named test-seam so callers don't
    /// need to know the internal constructor name.
    pub fn for_test(path: PathBuf) -> Self {
        Self::with_path(path)
    }

    /// Return a snapshot of the current in-memory map.
    pub async fn snapshot(&self) -> Map<String, Value> {
        self.inner.read().await.clone()
    }

    async fn flush(&self, map: Map<String, Value>) -> Result<(), std::io::Error> {
        let serialized = serde_json::to_string_pretty(&map)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if let Some(parent) = self.path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&*self.path, serialized).await
    }

    /// Set the last-4 masked record for a provider's API token. The full token is NOT
    /// persisted; only the masked last-4, the timestamp, and a status flag are stored.
    ///
    /// This is the ONLY API that should accept a full token from the client. The full
    /// token is consumed in-process and discarded — it is not written to disk by
    /// SettingsState. (A future secrets vault will route the full token elsewhere;
    /// for now it is up to the caller to do something with it before calling this.)
    pub async fn put_token_mask(&self, provider: &str, last4: &str) -> Result<(), std::io::Error> {
        let now_ms = epoch_ms();
        let snapshot = {
            let mut map = self.inner.write().await;
            map.insert(
                format!("tokens.{provider}.last4"),
                Value::String(last4.to_string()),
            );
            map.insert(
                format!("tokens.{provider}.added_ms"),
                Value::Number(now_ms.into()),
            );
            map.insert(
                format!("tokens.{provider}.status"),
                Value::String("ok".to_string()),
            );
            map.clone()
        }; // write lock released before async I/O
        self.flush(snapshot).await
    }

    /// Remove a provider's token mask record entirely.
    pub async fn remove_token(&self, provider: &str) -> Result<(), std::io::Error> {
        let snapshot = {
            let mut map = self.inner.write().await;
            map.remove(&format!("tokens.{provider}.last4"));
            map.remove(&format!("tokens.{provider}.added_ms"));
            map.remove(&format!("tokens.{provider}.status"));
            map.clone()
        }; // write lock released before async I/O
        self.flush(snapshot).await
    }
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

async fn get_settings(State(s): State<SettingsState>) -> Json<Value> {
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
    s.flush(updated.clone())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(Value::Object(updated)))
}

#[derive(Deserialize)]
struct PutTokenBody {
    token: Option<String>,
}

/// PUT /api/dashboard/settings/tokens/{provider}
///
/// Body: `{ "token": "sk-..." }`
///
/// Stores only the masked last-4 of the token. The full token is NOT persisted,
/// NOT logged, and NOT echoed back. Returns `{"v":1,"data":{...}}` on success.
async fn put_token_route(
    State(s): State<SettingsState>,
    Path(provider): Path<String>,
    Json(body): Json<PutTokenBody>,
) -> Result<Json<Value>, StatusCode> {
    let token = match body.token {
        Some(t) if !t.is_empty() => t,
        _ => return Err(StatusCode::BAD_REQUEST),
    };
    let last4 = token[token.len().saturating_sub(4)..].to_string();
    // Discard the full token — do not log, store, or echo it.
    drop(token);

    s.put_token_mask(&provider, &last4)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "v": 1,
        "data": {
            "provider": provider,
            "last4": last4,
            "status": "ok"
        }
    })))
}

/// DELETE /api/dashboard/settings/tokens/{provider}
///
/// Removes the token mask record for the given provider.
async fn delete_token_route(
    State(s): State<SettingsState>,
    Path(provider): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    s.remove_token(&provider)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({
        "v": 1,
        "data": { "provider": provider, "removed": true }
    })))
}

pub fn settings_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let state = SettingsState::new();
    Router::new()
        .route("/api/dashboard/settings", get(get_settings))
        .route("/api/dashboard/settings", put(put_settings))
        .route(
            "/api/dashboard/settings/tokens/{provider}",
            put(put_token_route),
        )
        .route(
            "/api/dashboard/settings/tokens/{provider}",
            delete(delete_token_route),
        )
        .with_state(state)
}
