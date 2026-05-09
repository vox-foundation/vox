//! Dashboard models API — stub endpoints for Phase 4 live wiring.
//!
//! ## Routes
//!
//! ```text
//! GET  /api/v2/models/usage_24h
//! ```
//!
//! ### GET /api/v2/models/usage_24h
//!
//! Returns a 24-hour cost/token summary for the Models surface and StatusBar cost widget.
//!
//! Response envelope (`{"v":1,"data":{...}}`):
//! ```json
//! {
//!   "v": 1,
//!   "data": {
//!     "total_usd":    0.0,
//!     "buckets_5min": []
//!   }
//! }
//! ```
//!
//! `buckets_5min` is an array of `{ "ts_ms": <epoch_ms>, "usd": <f64> }` objects
//! covering the past 24 h in 5-minute increments (288 buckets max).
//!
//! All fields are static stubs in Phase 1/2; Phase 4 replaces them with live reads
//! from the orchestrator EventBus (`ThroughputTick`, `CostTick`).

use axum::{Router, response::Json, routing::get};
use serde_json::{Value, json};

async fn get_usage_24h() -> Json<Value> {
    Json(json!({
        "v": 1,
        "data": {
            "total_usd":    0.0,
            "buckets_5min": []
        }
    }))
}

pub fn models_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/api/v2/models/usage_24h", get(get_usage_24h))
}
