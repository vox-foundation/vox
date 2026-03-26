//! MCP tools: query recent unified benchmark telemetry (`research_metrics` via [`vox_db`]).
//!
//! Writers use [`vox_db::benchmark_telemetry`] from CLI / tests when Codex is available.

use serde::Deserialize;

use crate::params::ToolResult;
use crate::server::ServerState;

const REM_VOXDB_ATTACH: &str =
    "Attach VoxDb via `VOX_DB_PATH` / `VOX_DB_URL` on the MCP server before querying benchmark telemetry.";
const REM_BENCHMARK_DB: &str =
    "Verify Turso connectivity and that `research_metrics` (benchmark rows) migrations are applied.";

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
        return ToolResult::<String>::err_with_remediation(
            "VoxDb not attached; set VOX_DB_PATH / VOX_DB_URL.",
            REM_VOXDB_ATTACH,
        )
        .to_json();
    };
    let rid = state.repository.repository_id.clone();
    match db
        .list_research_metrics_by_type("benchmark_event", &format!("bench:{rid}"), params.limit)
        .await
    {
        Ok(rows) => ToolResult::ok(rows).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB).to_json(),
    }
}

/// Arguments for `vox_benchmark_record`.
#[derive(Debug, serde::Deserialize)]
pub struct BenchmarkRecordParams {
    /// Benchmark name (e.g., "build_time", "eval_p95").
    pub name: String,
    /// Metric value (f64), e.g., duration in seconds.
    pub value: Option<f64>,
    /// Optional structured details (JSON).
    pub details: Option<serde_json::Value>,
}

/// Record a benchmark-class metric for this repository.
pub async fn benchmark_record(state: &ServerState, params: BenchmarkRecordParams) -> String {
    let Some(db) = state.db.as_ref() else {
        return ToolResult::<String>::err_with_remediation(
            "VoxDb not attached; set VOX_DB_PATH / VOX_DB_URL.",
            REM_VOXDB_ATTACH,
        )
        .to_json();
    };
    let rid = state.repository.repository_id.clone();
    match db
        .record_benchmark_event(&rid, &params.name, params.value, params.details)
        .await
    {
        Ok(_) => ToolResult::ok("Recorded.").to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB).to_json(),
    }
}
