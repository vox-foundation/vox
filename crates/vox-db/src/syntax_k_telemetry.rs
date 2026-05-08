//! Syntax K telemetry via `research_metrics` (no extra DDL).
//!
//! Session key pattern: `syntaxk:<repository_id>`; metric type: `syntax_k_event`.
//! Shape `details` JSON to stay compatible with `contracts/eval/syntax-k-event.schema.json` (and within
//! [`crate::research_metrics_contract::RESEARCH_METRICS_METADATA_JSON_MAX_BYTES`] once wrapped in metadata).

use serde::Serialize;

use crate::research_metrics_contract::{METRIC_TYPE_SYNTAX_K_EVENT, TelemetryWriteOptions};
use crate::{StoreError, VoxDb};

/// Payload for one syntax-K observation.
#[derive(Debug, Clone, Serialize)]
pub struct SyntaxKEventMeta {
    /// Logical observation name (`webir_json`, `emit_tsx_preview`, `regression_gate`, ...).
    pub name: String,
    /// Repository id from `vox-repository` when known.
    pub repository_id: String,
    /// Fixture ID used by the compiler benchmark harness.
    pub fixture_id: String,
    /// Structured event payload (usually `vox_compiler_emit::syntax_k::SyntaxKEvent` JSON).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl VoxDb {
    /// Append one syntax-k row to `research_metrics` under `syntaxk:<repository_id>`.
    pub async fn record_syntax_k_event(
        &self,
        repository_id: &str,
        name: &str,
        fixture_id: &str,
        metric_value: Option<f64>,
        details: Option<serde_json::Value>,
    ) -> Result<i64, StoreError> {
        let meta = SyntaxKEventMeta {
            name: name.to_string(),
            repository_id: repository_id.to_string(),
            fixture_id: fixture_id.to_string(),
            details,
        };
        let json =
            serde_json::to_string(&meta).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let tw = TelemetryWriteOptions::new(repository_id);
        self.append_research_metric(
            &tw.session_syntaxk(),
            METRIC_TYPE_SYNTAX_K_EVENT,
            metric_value,
            Some(&json),
        )
        .await
    }

    /// List newest syntax-k rows for `repository_id` as `(session_id, metric_value, metadata_json)`.
    pub async fn list_syntax_k_events(
        &self,
        repository_id: &str,
        limit: i64,
    ) -> Result<Vec<(String, Option<f64>, Option<String>)>, StoreError> {
        let tw = TelemetryWriteOptions::new(repository_id);
        self.list_research_metrics_by_type(METRIC_TYPE_SYNTAX_K_EVENT, &tw.session_syntaxk(), limit)
            .await
    }
}
