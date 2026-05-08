use axum::{Router, routing::get};
use serde_json::json;

use super::ok;

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/health", get(handler))
}

async fn handler() -> axum::Json<serde_json::Value> {
    ok(json!({ "status": "ok" }))
}
