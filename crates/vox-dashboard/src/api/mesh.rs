//! Dashboard mesh API — stub endpoints for Phase 2 live wiring.
//!
//! ## Routes
//!
//! ```text
//! GET  /api/v2/mesh/summary
//! ```
//!
//! ### GET /api/v2/mesh/summary
//!
//! Returns a snapshot of the mesh topology for the StatusBar and Mesh surface.
//!
//! Response envelope (`{"v":1,"data":{...}}`):
//! ```json
//! {
//!   "v": 1,
//!   "data": {
//!     "nodes":         0,
//!     "queue":         0,
//!     "errors":        0,
//!     "default_model": "—",
//!     "build_state":   "idle"
//!   }
//! }
//! ```
//!
//! All fields are static stubs in Phase 1; Phase 2 replaces them with live reads
//! from the orchestrator EventBus (`MeshTopologyChanged`, `BuildStageKind`).

use axum::{Router, response::Json, routing::get};
use serde_json::{Value, json};

async fn get_summary() -> Json<Value> {
    Json(json!({
        "v": 1,
        "data": {
            "nodes":         0,
            "queue":         0,
            "errors":        0,
            "default_model": "—",
            "build_state":   "idle"
        }
    }))
}

pub fn mesh_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v2/mesh/summary", get(get_summary))
}
