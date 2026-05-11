//! Destructive mesh action endpoints — kill / pause / drain / replay (P4-T7).
//!
//! Every route requires a confirmation body:
//!   `{"reason": "...", "confirm_token": "yes-i-mean-it"}`
//! Rejected with 400 if the confirm_token is absent or wrong.
//!
//! Every accepted action emits a signed audit-log entry via `audit_log::AuditWriter`
//! and broadcasts `MeshActionCommitted` on the event bus.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use vox_orchestrator::events::AgentEventKind;
use vox_orchestrator::MeshAction;

use crate::api::mesh_topology::MeshState;

const CONFIRM_TOKEN: &str = "yes-i-mean-it";

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    pub reason: String,
    #[serde(default)]
    pub confirm_token: Option<String>,
}

pub async fn node_kill(
    state: State<MeshState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> Result<Json<Value>, StatusCode> {
    handle_destructive(state, id, "kill", req).await
}

pub async fn node_pause(
    state: State<MeshState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> Result<Json<Value>, StatusCode> {
    handle_destructive(state, id, "pause", req).await
}

pub async fn node_drain(
    state: State<MeshState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> Result<Json<Value>, StatusCode> {
    handle_destructive(state, id, "drain", req).await
}

pub async fn node_replay(
    state: State<MeshState>,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> Result<Json<Value>, StatusCode> {
    handle_destructive(state, id, "replay", req).await
}

async fn handle_destructive(
    State(state): State<MeshState>,
    id: String,
    action: &str,
    req: ActionRequest,
) -> Result<Json<Value>, StatusCode> {
    if req.confirm_token.as_deref() != Some(CONFIRM_TOKEN) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let entry = state
        .audit
        .record(action, &id, "dashboard-user", &req.reason)
        .await;

    let mesh_action = match action {
        "kill"   => MeshAction::Kill,
        "pause"  => MeshAction::Pause,
        "drain"  => MeshAction::Drain,
        "replay" => MeshAction::Replay,
        _ => unreachable!(),
    };

    state.bus.emit(AgentEventKind::MeshActionCommitted {
        node_id:         id.clone(),
        action:          mesh_action,
        actor:           "dashboard-user".into(),
        signed_audit_id: entry.audit_id.clone(),
    });

    tracing::info!(
        audit_id = %entry.audit_id,
        action   = %action,
        target   = %id,
        "vox.mesh.action.committed"
    );

    Ok(Json(json!({
        "v": 1,
        "data": {
            "audit_id":  entry.audit_id,
            "signature": entry.signature,
            "action":    action,
            "target":    id,
        }
    })))
}
