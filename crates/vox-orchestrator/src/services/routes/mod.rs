//! Versioned HTTP API routes (`/api/v2/...`) for the dashboard surfaces.
//!
//! Every route in this module returns a JSON envelope:
//!   success: `{ "v": 1, "data": <payload>, "cursor": <opt-string> }`
//!   error:   `{ "v": 1, "error": { "code": "...", "message": "..." } }`

use axum::{Json, Router};
use serde::Serialize;
use serde_json::{json, Value};

pub mod health;

/// Wrap a serializable payload in the success envelope.
pub fn ok<T: Serialize>(data: T) -> Json<Value> {
    Json(json!({ "v": 1, "data": data }))
}

/// Wrap an error code and message in the error envelope.
pub fn err(code: &str, message: &str) -> Json<Value> {
    Json(json!({ "v": 1, "error": { "code": code, "message": message } }))
}

/// Build the router nested at `/api/v2`.
///
/// Generic over `S` so it can be merged into any `Router<S>` — the routes
/// in this namespace are stateless and do not access `S`.
///
/// Existing routes outside this namespace (`/v1/ws`, `/v1/tools/call`,
/// `/api/dashboard/settings`) are unaffected.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().nest("/api/v2", Router::new().merge(health::router::<S>()))
}
