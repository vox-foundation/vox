//! MCP tools: query recent unified benchmark telemetry (`research_metrics` via [`vox_db`]).
//!
//! Writers use [`vox_db::benchmark_telemetry`] from CLI / tests when Codex is available.
//! Sensitivity / trust taxonomy for stored rows: `docs/src/architecture/telemetry-trust-ssot.md` (tool descriptions in
//! `contracts/mcp/tool-registry.canonical.yaml` point callers at the same SSOT).

use serde::Deserialize;
use vox_db::research_metrics_contract::{
    METRIC_TYPE_BENCHMARK_EVENT, METRIC_TYPE_SYNTAX_K_EVENT, TelemetryWriteOptions,
};

use crate::mcp_tools::params::ToolResult;
use crate::mcp_tools::server_state::ServerState;

const REM_VOXDB_ATTACH: &str = "Attach VoxDb via `VOX_DB_PATH` / `VOX_DB_URL` on the MCP server before querying benchmark telemetry.";
const REM_BENCHMARK_DB: &str = "Verify Turso connectivity and that `research_metrics` (benchmark rows) migrations are applied.";

/// Arguments for `vox_benchmark_list`.
#[derive(Debug, Deserialize)]
pub struct BenchmarkListParams {
    /// Max rows (default 50).
    #[serde(default = "default_limit")]
    pub limit: i64,
    /// Metric type selector: `benchmark_event` (default) or `syntax_k_event`.
    #[serde(default)]
    pub metric_type: Option<String>,
    /// Data source/view selector.
    ///
    /// - `research_metrics` (default): benchmark rows from `research_metrics`.
    /// - `build_health`: latest build summary from `build_*` tables.
    /// - `build_regressions`: latest-run regressions from `build_*` tables.
    /// - `build_warnings`: warning hotspots from `build_*` tables.
    /// - `dependency_shape`: latest dependency-shape snapshot from `build_run_dependency_shape`.
    #[serde(default)]
    pub source: Option<String>,
    /// Optional run id for `build_regressions`; defaults to latest run.
    #[serde(default)]
    pub run_id: Option<i64>,
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
    let source = params.source.as_deref().unwrap_or("research_metrics");
    if source != "research_metrics" {
        let payload = match source {
            "build_health" => match db.query_build_health(&rid).await {
                Ok(v) => ToolResult::ok(serde_json::json!(v)).to_json(),
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB)
                        .to_json()
                }
            },
            "build_regressions" => {
                let run_id = if let Some(id) = params.run_id {
                    id
                } else {
                    match db.query_latest_build_run_id(&rid).await {
                        Ok(Some(id)) => id,
                        Ok(None) => return ToolResult::ok(Vec::<serde_json::Value>::new()).to_json(),
                        Err(e) => {
                            return ToolResult::<String>::err_with_remediation(
                                format!("{e}"),
                                REM_BENCHMARK_DB,
                            )
                            .to_json();
                        }
                    }
                };
                match db.query_build_regressions(&rid, run_id).await {
                    Ok(v) => ToolResult::ok(serde_json::json!(v)).to_json(),
                    Err(e) => {
                        ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB)
                            .to_json()
                    }
                }
            }
            "build_warnings" => match db.query_build_warnings(&rid, params.limit).await {
                Ok(v) => ToolResult::ok(serde_json::json!(v)).to_json(),
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB)
                        .to_json()
                }
            },
            "dependency_shape" => match db.query_latest_build_dependency_shape(&rid).await {
                Ok(v) => ToolResult::ok(serde_json::json!(v)).to_json(),
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB)
                        .to_json()
                }
            },
            _ => ToolResult::<String>::err_with_remediation(
                "Invalid source. Use research_metrics, build_health, build_regressions, build_warnings, or dependency_shape.",
                REM_BENCHMARK_DB,
            )
            .to_json(),
        };
        return payload;
    }

    let metric_type = params
        .metric_type
        .as_deref()
        .unwrap_or("benchmark_event")
        .to_string();
    let session_prefix = match metric_type.as_str() {
        "syntax_k_event" => format!("syntaxk:{rid}"),
        _ => format!("bench:{rid}"),
    };
    match db
        .list_research_metrics_by_type(&metric_type, &session_prefix, params.limit)
        .await
    {
        Ok(rows) => ToolResult::ok(serde_json::json!(rows)).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB).to_json()
        }
    }
}

/// Arguments for `vox_benchmark_record`.
#[derive(Debug, serde::Deserialize)]
pub struct BenchmarkRecordParams {
    /// Benchmark name (e.g., "build_time", "eval_p95").
    pub name: String,
    /// Fixture id for syntax-k events (`benchmark_event` ignores this).
    #[serde(default)]
    pub fixture_id: Option<String>,
    /// Metric type selector: `benchmark_event` (default) or `syntax_k_event`.
    #[serde(default)]
    pub metric_type: Option<String>,
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
    let metric_type = params
        .metric_type
        .as_deref()
        .unwrap_or(METRIC_TYPE_BENCHMARK_EVENT)
        .to_string();
    let res = match metric_type.as_str() {
        METRIC_TYPE_SYNTAX_K_EVENT => {
            db.record_syntax_k_event(
                &rid,
                &params.name,
                params.fixture_id.as_deref().unwrap_or("manual"),
                params.value,
                params.details,
            )
            .await
        }
        _ => {
            db.record_benchmark_event(&rid, &params.name, params.value, None, params.details)
                .await
        }
    };
    match res {
        Ok(_) => ToolResult::ok("Recorded.").to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_BENCHMARK_DB).to_json()
        }
    }
}
