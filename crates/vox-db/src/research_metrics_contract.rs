//! Validation helpers for `research_metrics` writes (size and naming guards).
//!
//! Contract SSOT: `docs/src/reference/telemetry-metric-contract.md`.

use crate::store::types::StoreError;

/// Upper bound on `metadata_json` payload size to limit accidental pathological inserts.
pub const RESEARCH_METRICS_METADATA_JSON_MAX_BYTES: usize = 256 * 1024;

/// `session_id` and `metric_type` length caps (SQLite TEXT, but keep client payloads bounded).
pub const RESEARCH_METRICS_SESSION_ID_MAX_CHARS: usize = 512;
pub const RESEARCH_METRICS_METRIC_TYPE_MAX_CHARS: usize = 128;

// ── Canonical `metric_type` values ─────────────────────────────────────────────

pub const METRIC_TYPE_BENCHMARK_EVENT: &str = "benchmark_event";
pub const METRIC_TYPE_SYNTAX_K_EVENT: &str = "syntax_k_event";
pub const METRIC_TYPE_SOCRATES_SURFACE: &str = "socrates_surface";
pub const METRIC_TYPE_WORKFLOW_JOURNAL_ENTRY: &str = "workflow_journal_entry";
pub const METRIC_TYPE_POPULI_CONTROL_EVENT: &str = "populi_control_event";
pub const METRIC_TYPE_QUESTIONING_EVENT: &str = "questioning_event";
pub const METRIC_TYPE_MEMORY_HYBRID_FUSION: &str = "memory_hybrid_fusion";

// ── Session id prefixes (`<prefix><repository_id>`) ─────────────────────────

pub const SESSION_PREFIX_BENCH: &str = "bench:";
pub const SESSION_PREFIX_SYNTAXK: &str = "syntaxk:";
pub const SESSION_PREFIX_MCP: &str = "mcp:";
pub const SESSION_PREFIX_WORKFLOW: &str = "workflow:";
pub const SESSION_PREFIX_MENS: &str = "mens:";

/// Hybrid retrieval telemetry uses a fixed session id (not `mcp:<repository_id>`).
pub const SESSION_ID_MEMORY_HYBRID_FUSION: &str = "socrates:retrieval";

/// Repository-scoped `research_metrics.session_id` values and canonical `metric_type` strings.
///
/// Use this (and the [`METRIC_TYPE_*`] constants) from producers so namespaces stay consistent.
#[derive(Debug, Clone, Copy)]
pub struct TelemetryWriteOptions<'a> {
    pub repository_id: &'a str,
}

impl<'a> TelemetryWriteOptions<'a> {
    #[inline]
    pub fn new(repository_id: &'a str) -> Self {
        Self { repository_id }
    }

    #[inline]
    pub fn session_bench(&self) -> String {
        format!("{SESSION_PREFIX_BENCH}{}", self.repository_id)
    }

    #[inline]
    pub fn session_syntaxk(&self) -> String {
        format!("{SESSION_PREFIX_SYNTAXK}{}", self.repository_id)
    }

    #[inline]
    pub fn session_mcp(&self) -> String {
        format!("{SESSION_PREFIX_MCP}{}", self.repository_id)
    }

    #[inline]
    pub fn session_workflow(&self) -> String {
        format!("{SESSION_PREFIX_WORKFLOW}{}", self.repository_id)
    }

    #[inline]
    pub fn session_mens(&self) -> String {
        format!("{SESSION_PREFIX_MENS}{}", self.repository_id)
    }
}

fn valid_metric_type_chars(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | ':'))
}

/// Validate inputs before `INSERT INTO research_metrics`.
pub fn validate_research_metric_row(
    session_id: &str,
    metric_type: &str,
    metadata_json: Option<&str>,
) -> Result<(), StoreError> {
    if session_id.is_empty() {
        return Err(StoreError::Db(
            "research_metrics: session_id must be non-empty".into(),
        ));
    }
    if session_id.len() > RESEARCH_METRICS_SESSION_ID_MAX_CHARS {
        return Err(StoreError::Db(format!(
            "research_metrics: session_id exceeds {} characters",
            RESEARCH_METRICS_SESSION_ID_MAX_CHARS
        )));
    }
    if metric_type.is_empty() {
        return Err(StoreError::Db(
            "research_metrics: metric_type must be non-empty".into(),
        ));
    }
    if metric_type.len() > RESEARCH_METRICS_METRIC_TYPE_MAX_CHARS {
        return Err(StoreError::Db(format!(
            "research_metrics: metric_type exceeds {} characters",
            RESEARCH_METRICS_METRIC_TYPE_MAX_CHARS
        )));
    }
    if !valid_metric_type_chars(metric_type) {
        return Err(StoreError::Db(format!(
            "research_metrics: metric_type {metric_type:?} contains disallowed characters"
        )));
    }
    if let Some(m) = metadata_json {
        if m.len() > RESEARCH_METRICS_METADATA_JSON_MAX_BYTES {
            return Err(StoreError::Db(format!(
                "research_metrics: metadata_json exceeds {} bytes",
                RESEARCH_METRICS_METADATA_JSON_MAX_BYTES
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_session_and_metric_type() {
        assert!(validate_research_metric_row("", "t", None).is_err());
        assert!(validate_research_metric_row("s", "", None).is_err());
    }

    #[test]
    fn accepts_colon_in_metric_type() {
        assert!(validate_research_metric_row("sess", "mcp:foo_bar", None).is_ok());
    }

    #[test]
    fn rejects_disallowed_metric_type_chars() {
        assert!(validate_research_metric_row("s", "bad type", None).is_err());
        assert!(validate_research_metric_row("s", "bad/type", None).is_err());
    }

    #[test]
    fn enforces_metadata_size_cap() {
        let big = "x".repeat(RESEARCH_METRICS_METADATA_JSON_MAX_BYTES + 1);
        assert!(validate_research_metric_row("s", "t", Some(&big)).is_err());
        let ok = "x".repeat(RESEARCH_METRICS_METADATA_JSON_MAX_BYTES);
        assert!(validate_research_metric_row("s", "t", Some(&ok)).is_ok());
    }

    #[test]
    fn telemetry_write_options_builds_expected_sessions() {
        let o = TelemetryWriteOptions::new("rid42");
        assert_eq!(o.session_bench(), "bench:rid42");
        assert_eq!(o.session_syntaxk(), "syntaxk:rid42");
        assert_eq!(o.session_mcp(), "mcp:rid42");
        assert_eq!(o.session_workflow(), "workflow:rid42");
        assert_eq!(o.session_mens(), "mens:rid42");
    }
}
