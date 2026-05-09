//! HTTP route stubs — PHASE_0a_STUB.
//!
//! The real implementation moved to `vox-orchestrator-mcp` during the 2026-05-08 reorg (Phase 4).
//! This module is a compatibility shim so existing `vox_orchestrator::services::routes` call sites
//! compile while migration is in progress.

use axum::Router;
use serde::Serialize;
use serde_json::{Value, json};

/// Build the orchestrator HTTP router.
///
/// # PHASE_0a_STUB
/// Returns an empty router. Full route table lives in `vox-orchestrator-mcp`.
pub fn router() -> Router {
    Router::new()
}

/// Wrap `data` in the standard API v2 envelope `{"v":1,"data":…,"cursor":…}`.
pub fn ok_page<T: Serialize>(data: T, cursor: Option<&str>) -> (Value, axum::http::StatusCode) {
    let body = json!({
        "v": 1,
        "data": serde_json::to_value(data).unwrap_or(Value::Null),
        "cursor": cursor,
    });
    (body, axum::http::StatusCode::OK)
}
