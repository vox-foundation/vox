//! Dashboard mesh API — Task 2.1 route set.
//!
//! ## Routes
//!
//! ```text
//! GET  /api/v2/mesh/summary
//! GET  /api/v2/mesh/nodes
//! GET  /api/v2/mesh/edges
//! POST /api/v2/mesh/nodes/{id}/kill
//! POST /api/v2/mesh/nodes/{id}/pause
//! POST /api/v2/mesh/nodes/{id}/replay
//! ```
//!
//! ### GET /api/v2/mesh/summary
//!
//! Returns a snapshot of the mesh topology used by the StatusBar and MeshSummaryBar.
//!
//! ```json
//! { "v": 1, "data": { "nodes": 0, "queue": 0, "errors": 0,
//!                     "default_model": "—", "build_state": "idle" } }
//! ```
//!
//! ### GET /api/v2/mesh/nodes
//!
//! Returns the full node list. Each node follows the `AgentNode` shape:
//! ```json
//! { "id": "lex-2", "kind": "agent", "status": "idle",
//!   "orchestrator": "orchestrator-7c2a", "model": "sonnet-4.6",
//!   "uptime_ms": 0, "tokens": 0, "cost_usd": 0.0,
//!   "current_task": null, "last_events": [] }
//! ```
//!
//! ### GET /api/v2/mesh/edges
//!
//! Returns the edge list. Each edge follows the `MeshEdge` shape:
//! ```json
//! { "from": "orchestrator-7c2a", "to": "lex-2",
//!   "kind": "channel", "status": "idle" }
//! ```
//!
//! ### POST /api/v2/mesh/nodes/{id}/kill|pause|replay
//!
//! All three return `{ "v":1, "data": { "id": <id>, "action": <action> } }`.
//! Real wiring to the orchestrator EventBus lands in Phase 2 backend work.
//!
//! All fields are static stubs in Phase 1; Phase 2 replaces them with live reads
//! from the orchestrator EventBus (`MeshTopologyChanged`, `BuildStageKind`).

use axum::{
    Router,
    extract::Path,
    response::Json,
    routing::{get, post},
};
use serde_json::{Value, json};

// ── GET /api/v2/mesh/summary ──────────────────────────────────────────────────

async fn get_summary() -> Json<Value> {
    // Phase 2.6 stub — string values so the KPI chips render without
    // a JS toString step. Real values from orchestrator EventBus in Phase 2.
    Json(json!({
        "v": 1,
        "data": {
            "nodes":         "7",
            "active":        "0",
            "blocked":       "0",
            "errors":        "0",
            "tok_s":         "0",
            "cost_h":        "$0.00",
            "default_model": "—",
            "build_state":   "idle"
        }
    }))
}

// ── GET /api/v2/mesh/nodes ────────────────────────────────────────────────────
// Returns the fixture workspace topology so the Mesh surface has something to
// render before a live orchestrator is attached. Phase 2 replaces this stub
// with a live read from the orchestrator mesh registry.

async fn get_nodes() -> Json<Value> {
    Json(json!({
        "v": 1,
        "data": [
            {
                "id": "orchestrator-7c2a", "kind": "orchestrator",
                "status": "idle", "orchestrator": null,
                "model": "sonnet-4.6", "uptime_ms": 0,
                "tokens": 0, "cost_usd": 0.0,
                "current_task": null, "last_events": []
            },
            {
                "id": "orchestrator-3f1b", "kind": "orchestrator",
                "status": "idle", "orchestrator": null,
                "model": "opus-4.7", "uptime_ms": 0,
                "tokens": 0, "cost_usd": 0.0,
                "current_task": null, "last_events": []
            },
            {
                "id": "lex-2", "kind": "agent",
                "status": "idle", "orchestrator": "orchestrator-7c2a",
                "model": "sonnet-4.6", "uptime_ms": 0,
                "tokens": 0, "cost_usd": 0.0,
                "current_task": null, "last_events": []
            },
            {
                "id": "parse-1", "kind": "agent",
                "status": "idle", "orchestrator": "orchestrator-7c2a",
                "model": "sonnet-4.6", "uptime_ms": 0,
                "tokens": 0, "cost_usd": 0.0,
                "current_task": null, "last_events": []
            },
            {
                "id": "hir-3", "kind": "agent",
                "status": "idle", "orchestrator": "orchestrator-7c2a",
                "model": "sonnet-4.6", "uptime_ms": 0,
                "tokens": 0, "cost_usd": 0.0,
                "current_task": null, "last_events": []
            },
            {
                "id": "typecheck-1", "kind": "agent",
                "status": "idle", "orchestrator": "orchestrator-3f1b",
                "model": "haiku-4.5", "uptime_ms": 0,
                "tokens": 0, "cost_usd": 0.0,
                "current_task": null, "last_events": []
            },
            {
                "id": "codegen-2", "kind": "agent",
                "status": "idle", "orchestrator": "orchestrator-3f1b",
                "model": "haiku-4.5", "uptime_ms": 0,
                "tokens": 0, "cost_usd": 0.0,
                "current_task": null, "last_events": []
            }
        ]
    }))
}

// ── GET /api/v2/mesh/edges ────────────────────────────────────────────────────

async fn get_edges() -> Json<Value> {
    Json(json!({
        "v": 1,
        "data": [
            { "from": "orchestrator-7c2a", "to": "lex-2",
              "kind": "channel", "status": "idle" },
            { "from": "orchestrator-7c2a", "to": "parse-1",
              "kind": "channel", "status": "idle" },
            { "from": "orchestrator-7c2a", "to": "hir-3",
              "kind": "channel", "status": "idle" },
            { "from": "orchestrator-3f1b", "to": "typecheck-1",
              "kind": "channel", "status": "idle" },
            { "from": "orchestrator-3f1b", "to": "codegen-2",
              "kind": "channel", "status": "idle" },
            { "from": "orchestrator-7c2a", "to": "orchestrator-3f1b",
              "kind": "delegation", "status": "idle" }
        ]
    }))
}

// ── POST /api/v2/mesh/nodes/{id}/kill|pause|replay ────────────────────────────
// Stub acknowledgements. Phase 2 wires these to the orchestrator kill/pause
// signal bus. Returns immediately with a confirmation envelope.

async fn node_kill(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ "v": 1, "data": { "id": id, "action": "kill" } }))
}

async fn node_pause(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ "v": 1, "data": { "id": id, "action": "pause" } }))
}

async fn node_replay(Path(id): Path<String>) -> Json<Value> {
    Json(json!({ "v": 1, "data": { "id": id, "action": "replay" } }))
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn mesh_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v2/mesh/summary", get(get_summary))
        .route("/api/v2/mesh/nodes", get(get_nodes))
        .route("/api/v2/mesh/edges", get(get_edges))
        .route("/api/v2/mesh/nodes/{id}/kill", post(node_kill))
        .route("/api/v2/mesh/nodes/{id}/pause", post(node_pause))
        .route("/api/v2/mesh/nodes/{id}/replay", post(node_replay))
}
