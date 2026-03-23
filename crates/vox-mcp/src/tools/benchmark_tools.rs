//! MCP tools: query recent unified benchmark telemetry (`research_metrics` via [`vox_db`]).
//!
//! Writers use [`vox_db::benchmark_telemetry`] from CLI / tests when Codex is available.

use serde::Deserialize;

use crate::params::ToolResult;
use crate::server::ServerState;

/// Arguments for `vox_benchmark_list`.
#[derive(Debug, Deserialize)]
pub struct BenchmarkListParams {
    /// Max rows (default 50).
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_limit() -> i64 {
    50
}

/// List recent benchmark-class metrics for this repository (best-effort).
pub async fn benchmark_list(state: &ServerState, params: BenchmarkListParams) -> String {
    let Some(db) = state.db.as_ref() else {
        return ToolResult::<String>::err("VoxDb not attached; set VOX_DB_PATH / VOX_DB_URL.")
            .to_json();
    };
    let rid = state.repository.repository_id.clone();
    match db
        .store()
        .list_research_metrics_by_type("benchmark_event", &format!("bench:{rid}"), params.limit)
        .await
    {
        Ok(rows) => ToolResult::ok(rows).to_json(),
        Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
    }
}
