//! Execution time telemetry types for `agent_exec_history`.
//! Sensitivity: **S1 (OperationalTracing)** — tool names + durations only.

use std::time::Instant;

/// Write-path record for one execution observation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecTimeRecord<'a> {
    /// Stable tool fingerprint, e.g. `"mcp:vox_build_crate"`, `"process:unity_editor"`.
    /// Convention: prefix with `"mcp:"` for MCP tools, `"process:"` for OS-launched processes.
    pub tool_key: &'a str,
    pub repository_id: &'a str,
    pub duration_ms: u64,
    /// Budget the agent planned for (None if first-ever run or no forecast was available).
    pub timeout_budget_ms: Option<u64>,
    pub compute_tokens_used: Option<u64>,
    pub vendor_cost_usd_micros: Option<i64>,
    pub attention_cost_ms: Option<u64>,
    pub outcome: ExecOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExecOutcome {
    Success,
    Timeout,
    Error,
}

impl ExecOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            ExecOutcome::Success => "success",
            ExecOutcome::Timeout => "timeout",
            ExecOutcome::Error => "error",
        }
    }
}

/// Historical latency profile returned by [`crate::VoxDb::query_tool_latency`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolLatencyProfile {
    pub tool_key: String,
    /// Total successful observations in the query window.
    pub sample_count: i64,
    pub avg_ms: f64,
    /// Approximate 90th-percentile (via ORDER BY + OFFSET trick; no extension required).
    pub p90_ms: f64,
    pub max_ms: i64,
    /// Fraction of all observations (including non-success) that ended in timeout.
    pub timeout_rate: f64,
    /// Suggested agent wait: `(p90_ms * safety_multiplier).ceil() as u64`.
    pub recommended_budget_ms: u64,
}

/// Times an async closure and records the observation to `agent_exec_history`.
/// Recording errors are logged and swallowed — never block the primary operation.
pub struct TimedExecution {
    pub tool_key: String,
    pub repository_id: String,
    pub timeout_budget_ms: Option<u64>,
    pub db: Option<crate::VoxDb>, // clone of Arc-backed connection
    pub compute_tokens_used: Option<u64>,
    pub vendor_cost_usd_micros: Option<i64>,
    pub attention_cost_ms: Option<u64>,
}

impl TimedExecution {
    pub fn new(
        tool_key: impl Into<String>,
        repository_id: impl Into<String>,
        timeout_budget_ms: Option<u64>,
        db: Option<crate::VoxDb>,
    ) -> Self {
        Self {
            tool_key: tool_key.into(),
            repository_id: repository_id.into(),
            timeout_budget_ms,
            db,
            compute_tokens_used: None,
            vendor_cost_usd_micros: None,
            attention_cost_ms: None,
        }
    }

    pub fn with_costs(
        mut self,
        tokens: Option<u64>,
        vendor_micros: Option<i64>,
        attention_ms: Option<u64>,
    ) -> Self {
        self.compute_tokens_used = tokens;
        self.vendor_cost_usd_micros = vendor_micros;
        self.attention_cost_ms = attention_ms;
        self
    }

    pub async fn run<F, Fut, T, E>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let start = Instant::now();
        let result = f().await;
        let duration_ms = start.elapsed().as_millis() as u64;
        let outcome = if result.is_ok() {
            ExecOutcome::Success
        } else {
            ExecOutcome::Error
        };
        if let Some(db) = self.db {
            let record = ExecTimeRecord {
                tool_key: &self.tool_key,
                repository_id: &self.repository_id,
                duration_ms,
                timeout_budget_ms: self.timeout_budget_ms,
                compute_tokens_used: self.compute_tokens_used,
                vendor_cost_usd_micros: self.vendor_cost_usd_micros,
                attention_cost_ms: self.attention_cost_ms,
                outcome,
            };
            if let Err(e) = db.record_exec_time(&record).await {
                tracing::warn!(
                    ?e,
                    tool_key = self.tool_key,
                    "agent_exec_history: write failed (swallowed)"
                );
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exec_outcome_serialization() {
        assert_eq!(ExecOutcome::Success.as_str(), "success");
        assert_eq!(ExecOutcome::Timeout.as_str(), "timeout");
        assert_eq!(ExecOutcome::Error.as_str(), "error");
    }

    #[tokio::test]
    async fn timed_execution_fixture_no_db() {
        // Run with no DB, ensure it doesn't panic and returns the result safely
        let te = TimedExecution::new("test:tool", "repo_1", None, None);

        let result = te.run(|| async { Ok::<i32, String>(42) }).await;

        assert_eq!(result, Ok(42));
    }
}
