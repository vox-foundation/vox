//! Hopper panel HTTP routes — Phase 4, P4-T13 (Hp-T6).
//!
//! Surfaces `vox_orchestrator::hopper::InMemoryHopper` (Hp-T1, Option A) via
//! REST. When Hp-T5 lands, replace `Arc<InMemoryHopper>` with
//! `Arc<dyn HopperIntake>` backed by the persistent store — HTTP handlers
//! need no changes.
//!
//! Routes:
//!   POST /api/v2/hopper/submit
//!   GET  /api/v2/hopper/inbox
//!   GET  /api/v2/hopper/assigned
//!   GET  /api/v2/hopper/history
//!   POST /api/v2/hopper/items/{id}/reprioritize

use std::sync::Arc;

use axum::Router;
use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::{get, post};
use serde::Deserialize;
use serde_json::{Value, json};
use vox_orchestrator::hopper::{
    DeveloperOverrideMint, HopperError, HopperIntake, HopperItemId, InMemoryHopper, IntakeSource,
    PriorityHint,
};
use vox_orchestrator::types::TaskPriority;

use crate::api::mesh_topology::MeshState;

// ── Serialization helpers ─────────────────────────────────────────────────────

fn priority_from_str(s: &str) -> TaskPriority {
    match s {
        "urgent" => TaskPriority::Urgent,
        "background" => TaskPriority::Background,
        _ => TaskPriority::Normal,
    }
}

fn priority_to_str(p: &TaskPriority) -> &'static str {
    match p {
        TaskPriority::Urgent => "urgent",
        TaskPriority::Normal => "normal",
        TaskPriority::Background => "background",
        _ => "normal",
    }
}

fn item_to_json(item: &vox_orchestrator::hopper::IntakeItem) -> Value {
    json!({
        "item_id":             item.item_id.0,
        "intent":              item.intent,
        "affinity_hints":      item.affinity_hints,
        "classified_priority": priority_to_str(&item.classified_priority),
        "confidence":          item.confidence,
        "privacy_class":       item.privacy_class,
        "state":               item.state.kind(),
        "submitted_at":        item.submitted_at,
        "session_id":          item.session_id,
        "source":              item.source.as_str(),
        "override_history": item.override_history.iter().map(|r| json!({
            "ts_micros":         r.ts_micros,
            "actor":             r.actor,
            "original_priority": priority_to_str(&r.original_priority),
            "new_priority":      priority_to_str(&r.new_priority),
            "reason":            r.reason,
            "audit_id":          r.audit_id,
        })).collect::<Vec<_>>(),
    })
}

// ── Handlers ──────────────────────────────────────────────────────────────────

pub async fn submit(
    Extension(hopper): Extension<Arc<InMemoryHopper>>,
    Json(req): Json<SubmitRequest>,
) -> Json<Value> {
    let hint = match req.priority_hint.as_deref() {
        Some("urgent") => PriorityHint::Urgent,
        Some("background") => PriorityHint::Background,
        Some("normal") => PriorityHint::Normal,
        _ => PriorityHint::Unspecified,
    };

    let item = hopper
        .submit(
            req.intent,
            req.affinity_hints.unwrap_or_default(),
            hint,
            IntakeSource::Developer,
            req.session_id,
        )
        .await;

    Json(json!({
        "v": 1,
        "data": {
            "item_id":             item.item_id.0,
            "classified_priority": priority_to_str(&item.classified_priority),
            "classified_affinity": item.affinity_hints,
            "confidence":          item.confidence,
        }
    }))
}

pub async fn list_inbox(Extension(hopper): Extension<Arc<InMemoryHopper>>) -> Json<Value> {
    let items: Vec<_> = hopper.inbox().await.iter().map(item_to_json).collect();
    Json(json!({ "v": 1, "data": items }))
}

pub async fn list_assigned(Extension(hopper): Extension<Arc<InMemoryHopper>>) -> Json<Value> {
    let items: Vec<_> = hopper.assigned().await.iter().map(item_to_json).collect();
    Json(json!({ "v": 1, "data": items }))
}

pub async fn list_history(Extension(hopper): Extension<Arc<InMemoryHopper>>) -> Json<Value> {
    let items: Vec<_> = hopper.history().await.iter().map(item_to_json).collect();
    Json(json!({ "v": 1, "data": items }))
}

pub async fn reprioritize(
    Extension(hopper): Extension<Arc<InMemoryHopper>>,
    State(state): State<MeshState>,
    Path(item_id): Path<String>,
    Json(req): Json<ReprioritizeRequest>,
) -> Result<Json<Value>, StatusCode> {
    if req.confirm_token.as_deref() != Some("yes-i-mean-it") {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Write a signed audit-log entry (SSOT §5.7 — every DeveloperOverride goes through audit_log).
    let entry = state
        .audit
        .record(
            &format!("hopper.reprioritize.{}", req.new_priority),
            &item_id,
            "dashboard-user",
            &req.reason,
        )
        .await;

    // Mint the DeveloperOverride capability token.
    let cap = DeveloperOverrideMint::new().mint("dashboard-user", &req.reason, &entry.audit_id);

    let new_priority = priority_from_str(&req.new_priority);
    let item = hopper
        .reprioritize(&HopperItemId(item_id), new_priority, cap)
        .await
        .map_err(|e| match e {
            HopperError::NotFound(_) => StatusCode::NOT_FOUND,
            HopperError::Terminal => StatusCode::CONFLICT,
        })?;

    Ok(Json(json!({
        "v": 1,
        "data": {
            "item_id":   item.item_id.0,
            "priority":  priority_to_str(&item.classified_priority),
            "audit_id":  entry.audit_id,
            "signature": entry.signature,
        }
    })))
}

// ── Request types ─────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SubmitRequest {
    pub intent: String,
    pub session_id: Option<String>,
    pub affinity_hints: Option<Vec<String>>,
    pub priority_hint: Option<String>,
}

#[derive(Deserialize)]
pub struct ReprioritizeRequest {
    pub new_priority: String,
    pub reason: String,
    #[serde(default)]
    pub confirm_token: Option<String>,
}

// ── Router factory ────────────────────────────────────────────────────────────

pub fn hopper_router<S>(mesh_state: MeshState, hopper: Arc<InMemoryHopper>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v2/hopper/submit", post(submit))
        .route("/api/v2/hopper/inbox", get(list_inbox))
        .route("/api/v2/hopper/assigned", get(list_assigned))
        .route("/api/v2/hopper/history", get(list_history))
        .route("/api/v2/hopper/items/{id}/reprioritize", post(reprioritize))
        .with_state(mesh_state)
        .layer(Extension(hopper))
}
