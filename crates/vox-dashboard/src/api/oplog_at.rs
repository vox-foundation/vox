//! `/api/v2/oplog/at/{ts}` — op-log snapshot at a given op-id cursor.
//!
//! Returns at most 500 op-log entries with op_id <= ts, ordered ascending.
//! The `ts` parameter is a monotonic op_id (u64) used as a "time" cursor
//! by the audit-log scrubber timeline slider (P4-T5).
//!
//! Phase 4: DB injection deferred — returns empty snapshot when no DB is
//! wired. The daemon (`vox-orchestrator-d`) injects `Arc<VoxDb>` at startup.

use axum::{
    Router,
    extract::{Path, State},
    response::Json,
    routing::get,
};
use serde_json::{Value, json};

use crate::api::mesh_topology::MeshState;

pub async fn get_oplog_at(State(_state): State<MeshState>, Path(ts): Path<u64>) -> Json<Value> {
    // Phase 4 stub — DB injection not yet wired to MeshState.
    // The real implementation queries convergence_op_log WHERE op_id <= ts
    // via vox_db::VoxDb, ordered by op_id ASC, LIMIT 500.
    Json(json!({
        "v": 1,
        "data": {
            "ts": ts,
            "op_count": 0,
            "ops": [],
            "note": "db-not-wired"
        }
    }))
}

/// Sub-router for `/api/v2/oplog/*` routes.
pub fn oplog_router<S>(state: MeshState) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/v2/oplog/at/{ts}", get(get_oplog_at))
        .with_state(state)
}
