//! Mesh REST surface — Phase 4 wiring.
//!
//! Live handlers live in `mesh_topology.rs`.
//! Action handlers (kill/pause/drain/replay) are stubs pending P4-T7.
//! Invite bearer (Add-a-Node) lives in `mesh_invite.rs` (P4-T2).
//!
//! ## Routes
//!
//! ```text
//! GET  /api/v2/mesh/summary
//! GET  /api/v2/mesh/nodes
//! GET  /api/v2/mesh/edges
//! POST /api/v2/mesh/invite
//! POST /api/v2/mesh/invite/preview
//! POST /api/v2/mesh/join
//! POST /api/v2/mesh/nodes/{id}/kill
//! POST /api/v2/mesh/nodes/{id}/pause
//! POST /api/v2/mesh/nodes/{id}/drain
//! POST /api/v2/mesh/nodes/{id}/replay
//! GET  /api/v2/mesh/models
//! ```

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde_json::{Value, json};

use crate::api::mesh_invite::mint;
use crate::api::mesh_join;
use crate::api::mesh_topology::{MeshState, get_edges, get_nodes, get_summary};

// ── Action stubs (P4-T7 replaces these with signed audit-log versions) ───────

async fn node_kill(
    State(_state): State<MeshState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({ "v": 1, "error": "not implemented — wired in P4-T7", "id": id, "action": "kill" })),
    )
}

async fn node_pause(
    State(_state): State<MeshState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({ "v": 1, "error": "not implemented — wired in P4-T7", "id": id, "action": "pause" })),
    )
}

async fn node_drain(
    State(_state): State<MeshState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({ "v": 1, "error": "not implemented — wired in P4-T7", "id": id, "action": "drain" })),
    )
}

async fn node_replay(
    State(_state): State<MeshState>,
    Path(id): Path<String>,
) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({ "v": 1, "error": "not implemented — wired in P4-T7", "id": id, "action": "replay" })),
    )
}

// ── Model registry stub (P4-T12 handler wired here in its task) ──────────────

async fn get_models(State(_state): State<MeshState>) -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(json!({ "v": 1, "error": "not implemented — wired in P4-T12", "data": [] })),
    )
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn mesh_router<S>(state: MeshState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v2/mesh/summary", get(get_summary))
        .route("/api/v2/mesh/nodes", get(get_nodes))
        .route("/api/v2/mesh/edges", get(get_edges))
        .route("/api/v2/mesh/models", get(get_models))
        .route("/api/v2/mesh/invite", post(mint))
        .route("/api/v2/mesh/invite/preview", post(mesh_join::preview))
        .route("/api/v2/mesh/join", post(mesh_join::join))
        .route("/api/v2/mesh/nodes/{id}/kill", post(node_kill))
        .route("/api/v2/mesh/nodes/{id}/pause", post(node_pause))
        .route("/api/v2/mesh/nodes/{id}/drain", post(node_drain))
        .route("/api/v2/mesh/nodes/{id}/replay", post(node_replay))
        .with_state(state)
}
