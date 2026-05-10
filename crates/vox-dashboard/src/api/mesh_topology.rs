//! Live read of orchestrator mesh state. Replaces the static fixture in `api/mesh.rs`.
//!
//! Two surfaces:
//!   - REST: `GET /api/v2/mesh/{summary,nodes,edges}` snapshot the current state.
//!   - WS:   `MeshTopologyChanged` / `MeshNodeBudget` / `MeshActionCommitted`
//!           events stream over `/v1/ws`.
//!
//! Snapshot freshness contract: every snapshot is consistent against the
//! orchestrator state at the instant the request handler ran. Updates after
//! that arrive over WS — the client reconciles by id.

use axum::extract::State;
use axum::response::Json;
use serde_json::{json, Value};
use std::sync::Arc;
use vox_orchestrator::events::EventBus;
use vox_orchestrator::mesh::MeshRegistry;

/// Shared state injected into every mesh handler via `State<MeshState>`.
#[derive(Clone)]
pub struct MeshState {
    pub registry: Arc<MeshRegistry>,
    pub bus: Arc<EventBus>,
}

pub async fn get_summary(State(state): State<MeshState>) -> Json<Value> {
    let snapshot = state.registry.snapshot().await;
    Json(json!({
        "v": 1,
        "data": {
            "nodes":         snapshot.nodes.len().to_string(),
            "active":        snapshot.active_count().to_string(),
            "blocked":       snapshot.blocked_count().to_string(),
            "errors":        snapshot.error_count().to_string(),
            "tok_s":         format!("{:.0}", snapshot.tokens_per_sec),
            "cost_h":        format!("${:.2}", snapshot.cost_usd_per_hour),
            "default_model": snapshot.default_model,
            "build_state":   snapshot.build_state,
        }
    }))
}

pub async fn get_nodes(State(state): State<MeshState>) -> Json<Value> {
    let snapshot = state.registry.snapshot().await;
    let data: Vec<Value> = snapshot
        .nodes
        .iter()
        .map(|n| {
            json!({
                "id":               n.id,
                "kind":             n.kind.as_str(),
                "status":           n.status.as_str(),
                "orchestrator":     n.orchestrator,
                "model":            n.model,
                "uptime_ms":        n.uptime_ms,
                "tokens":           n.tokens_24h,
                "cost_usd":         n.cost_usd_24h,
                "current_task":     n.current_task,
                "last_events":      n.last_events,
                "privacy_class":    n.privacy_class.as_str(),
                "heartbeat_age_ms": n.heartbeat_age_ms,
            })
        })
        .collect();
    Json(json!({ "v": 1, "data": data }))
}

pub async fn get_edges(State(state): State<MeshState>) -> Json<Value> {
    let snapshot = state.registry.snapshot().await;
    let data: Vec<Value> = snapshot
        .edges
        .iter()
        .map(|e| {
            json!({
                "from":   e.from,
                "to":     e.to,
                "kind":   e.kind.as_str(),
                "status": e.status.as_str(),
            })
        })
        .collect();
    Json(json!({ "v": 1, "data": data }))
}
