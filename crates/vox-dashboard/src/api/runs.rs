//! Dashboard runs API — Phase 3 route set.
//!
//! ## Routes
//!
//! ```text
//! GET  /api/v2/runs
//! GET  /api/v2/runs/{id}
//! ```
//!
//! ### GET /api/v2/runs
//!
//! Returns the full run list. Each run follows the `RunSummary` shape:
//! ```json
//! { "id": "run-a1b2", "started": "2026-05-04T10:23:00Z",
//!   "duration_ms": 4821, "model": "sonnet-4.6",
//!   "status": "ok", "cost_usd": 0.0142, "tokens": 8432,
//!   "events": [ { "id": "e1", "kind": "task.started", "ts_ms": 0,
//!                 "label": "Orchestrator started" }, ... ] }
//! ```
//!
//! ### GET /api/v2/runs/{id}
//!
//! Returns the same shape for a single run.
//! Returns 404 `{ "v":1, "error": { "code": "NOT_FOUND" } }` for unknown ids.
//!
//! All values are static fixture stubs in Phase 3.
//! Phase 3 live wiring: subscribe to `run.started` WS events and prepend rows.

use axum::{
    Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
};
use serde_json::{Value, json};

// ── fixture data ──────────────────────────────────────────────────────────────

fn fixture_runs() -> Value {
    json!([
        {
            "id": "run-a1b2",
            "started": "2026-05-04T10:23:00Z",
            "duration_ms": 4821,
            "model": "sonnet-4.6",
            "status": "ok",
            "cost_usd": 0.0142,
            "tokens": 8432,
            "events": [
                { "id": "e1", "kind": "task.started",   "ts_ms": 0,    "label": "Orchestrator started" },
                { "id": "e2", "kind": "agent.spawned",  "ts_ms": 120,  "label": "lex-2 spawned" },
                { "id": "e3", "kind": "agent.spawned",  "ts_ms": 135,  "label": "parse-1 spawned" },
                { "id": "e4", "kind": "agent.spawned",  "ts_ms": 148,  "label": "hir-3 spawned" },
                { "id": "e5", "kind": "task.completed", "ts_ms": 4821, "label": "Run completed" }
            ]
        },
        {
            "id": "run-c3d4",
            "started": "2026-05-04T10:18:42Z",
            "duration_ms": 12043,
            "model": "opus-4.7",
            "status": "ok",
            "cost_usd": 0.0871,
            "tokens": 31240,
            "events": [
                { "id": "e1", "kind": "task.started",   "ts_ms": 0,     "label": "Orchestrator started" },
                { "id": "e2", "kind": "agent.spawned",  "ts_ms": 200,   "label": "lex-2 spawned" },
                { "id": "e3", "kind": "agent.spawned",  "ts_ms": 220,   "label": "parse-1 spawned" },
                { "id": "e4", "kind": "agent.spawned",  "ts_ms": 240,   "label": "hir-3 spawned" },
                { "id": "e5", "kind": "agent.spawned",  "ts_ms": 260,   "label": "typecheck-1 spawned" },
                { "id": "e6", "kind": "agent.spawned",  "ts_ms": 280,   "label": "codegen-2 spawned" },
                { "id": "e7", "kind": "task.completed", "ts_ms": 12043, "label": "Run completed" }
            ]
        },
        {
            "id": "run-e5f6",
            "started": "2026-05-04T10:11:05Z",
            "duration_ms": 831,
            "model": "haiku-4.5",
            "status": "error",
            "cost_usd": 0.0008,
            "tokens": 512,
            "events": [
                { "id": "e1", "kind": "task.started", "ts_ms": 0,   "label": "Orchestrator started" },
                { "id": "e2", "kind": "agent.spawned","ts_ms": 100,  "label": "lex-2 spawned" },
                { "id": "e3", "kind": "agent.error",  "ts_ms": 831,  "label": "lex-2 panicked: unexpected EOF" }
            ]
        },
        {
            "id": "run-g7h8",
            "started": "2026-05-04T09:55:20Z",
            "duration_ms": 7329,
            "model": "sonnet-4.6",
            "status": "ok",
            "cost_usd": 0.0294,
            "tokens": 14820,
            "events": [
                { "id": "e1", "kind": "task.started",   "ts_ms": 0,    "label": "Orchestrator started" },
                { "id": "e2", "kind": "agent.spawned",  "ts_ms": 180,  "label": "lex-2 spawned" },
                { "id": "e3", "kind": "agent.spawned",  "ts_ms": 195,  "label": "parse-1 spawned" },
                { "id": "e4", "kind": "task.completed", "ts_ms": 7329, "label": "Run completed" }
            ]
        },
        {
            "id": "run-i9j0",
            "started": "2026-05-04T09:41:00Z",
            "duration_ms": 2104,
            "model": "sonnet-4.6",
            "status": "ok",
            "cost_usd": 0.0063,
            "tokens": 3841,
            "events": [
                { "id": "e1", "kind": "task.started",   "ts_ms": 0,    "label": "Orchestrator started" },
                { "id": "e2", "kind": "agent.spawned",  "ts_ms": 110,  "label": "lex-2 spawned" },
                { "id": "e3", "kind": "task.completed", "ts_ms": 2104, "label": "Run completed" }
            ]
        }
    ])
}

// ── GET /api/v2/runs ──────────────────────────────────────────────────────────

async fn list_runs() -> Json<Value> {
    Json(json!({ "v": 1, "data": fixture_runs() }))
}

// ── GET /api/v2/runs/{id} ─────────────────────────────────────────────────────

async fn get_run(Path(id): Path<String>) -> impl IntoResponse {
    let runs = fixture_runs();
    let found = runs
        .as_array()
        .and_then(|arr| arr.iter().find(|r| r["id"] == id))
        .cloned();

    match found {
        Some(run) => (StatusCode::OK, Json(json!({ "v": 1, "data": run }))).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "v": 1, "error": { "code": "NOT_FOUND", "id": id } })),
        )
            .into_response(),
    }
}

// ── Router ────────────────────────────────────────────────────────────────────

pub fn runs_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v2/runs",      get(list_runs))
        .route("/api/v2/runs/{id}", get(get_run))
}
