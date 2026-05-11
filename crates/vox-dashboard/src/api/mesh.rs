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
//! GET  /api/v2/mesh/budget
//! POST /api/v2/mesh/invite
//! POST /api/v2/mesh/invite/preview
//! POST /api/v2/mesh/join
//! POST /api/v2/mesh/nodes/{id}/kill
//! POST /api/v2/mesh/nodes/{id}/pause
//! POST /api/v2/mesh/nodes/{id}/drain
//! POST /api/v2/mesh/nodes/{id}/replay
//! GET  /api/v2/mesh/models
//! ```

use axum::extract::State;
use axum::{
    Router,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde_json::{Value, json};

use crate::api::mesh_actions::{node_drain, node_kill, node_pause, node_replay};
use crate::api::mesh_invite::mint;
use crate::api::mesh_join;
use crate::api::mesh_topology::{MeshState, get_budget, get_edges, get_nodes, get_summary};

#[allow(dead_code)]
fn mesh_policy_stack_anchor() {
    let _: fn(&vox_mesh_types::donation_policy::WorkerDonationPolicy) -> String =
        vox_mesh_policy::pretty_print;
}

// ── Model registry stub (P4-T12 handler wired here in its task) ──────────────

async fn get_models(State(_state): State<MeshState>) -> (StatusCode, Json<Value>) {
    let reg = vox_mesh_models::ModelRegistry::empty();
    let models = reg.all_models();
    (StatusCode::OK, Json(json!({ "v": 1, "data": models })))
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
        .route("/api/v2/mesh/budget", get(get_budget))
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
