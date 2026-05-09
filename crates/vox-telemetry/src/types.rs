//! Canonical telemetry constants, event types, and validation helpers.
//!
//! All `METRIC_TYPE_*` constants and [`validate_research_metric_row`] previously
//! lived in `vox-db::research_metrics_contract`. That module now re-exports
//! everything from here so downstream code continues to compile unchanged.
//!
//! - Human contract: `docs/src/reference/telemetry-metric-contract.md`
//! - Sensitivity classes (S0–S3): `docs/src/architecture/telemetry-retention-sensitivity-ssot.md`
//! - Taxonomy / owners: `docs/src/architecture/telemetry-taxonomy-contracts-ssot.md`
//! - Design: `docs/src/architecture/telemetry-unification-design-2026.md`

// ── TelemetryError ────────────────────────────────────────────────────────

/// Error returned by [`validate_research_metric_row`] and sink operations.
#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("telemetry validation: {0}")]
    Validation(String),
}

// ── Payload size and field length limits ────────────────────────────────────

/// Upper bound on `metadata_json` payload size.
pub const RESEARCH_METRICS_METADATA_JSON_MAX_BYTES: usize = 256 * 1024;
/// `session_id` length cap.
pub const RESEARCH_METRICS_SESSION_ID_MAX_CHARS: usize = 512;
/// `metric_type` length cap.
pub const RESEARCH_METRICS_METRIC_TYPE_MAX_CHARS: usize = 128;

// ── Canonical `metric_type` values — existing (S0–S2) ────────────────────────

/// S0 — Coarse benchmark names, timings, ratios.
pub const METRIC_TYPE_BENCHMARK_EVENT: &str = "benchmark_event";
/// S0–S1 — Compiler / fixture ids.
pub const METRIC_TYPE_SYNTAX_K_EVENT: &str = "syntax_k_event";
/// S1 (payload may reach S2) — MCP surface calibration.
pub const METRIC_TYPE_SOCRATES_SURFACE: &str = "socrates_surface";
/// S2 — BM25/vector fusion stats.
pub const METRIC_TYPE_MEMORY_HYBRID_FUSION: &str = "memory_hybrid_fusion";
/// S1–S2 — Durable workflow journal JSON.
pub const METRIC_TYPE_WORKFLOW_JOURNAL_ENTRY: &str = "workflow_journal_entry";
/// S1 — Mesh/registry audit; must not store bearer tokens.
pub const METRIC_TYPE_POPULI_CONTROL_EVENT: &str = "populi_control_event";
/// S1–S2 — Questioning KPIs.
pub const METRIC_TYPE_QUESTIONING_EVENT: &str = "questioning_event";
/// S1 — Tool execution tracking (duration, timeouts, compute/cost).
pub const METRIC_TYPE_AGENT_EXEC_TIME: &str = "agent_exec_time";
/// S1 — Routing policy / capability-gate events from runtime/orchestrator.
pub const METRIC_TYPE_MODEL_ROUTE_EVENT: &str = "model_route_event";

// ── Orchestrator policy decision metrics (D1–D10) — S1 ────────────────────

/// D6: Circuit breaker trip — doom-loop detected.
pub const METRIC_TYPE_CIRCUIT_BREAKER_TRIP: &str = "orch.circuit_breaker.trip";
/// D3: Socrates confidence fusion decision.
pub const METRIC_TYPE_SOCRATES_FUSION: &str = "orch.socrates.fusion";
/// D1: Model tier routing decision (Economy / Standard / Strong).
pub const METRIC_TYPE_MODEL_TIER_ROUTE: &str = "orch.routing.tier";
/// D2: Plan-mode vs. ReAct mode decision.
pub const METRIC_TYPE_PLAN_MODE_DECISION: &str = "orch.plan.mode_decision";
/// D9: HITL interrupt — human-in-the-loop escalation triggered.
pub const METRIC_TYPE_HITL_INTERRUPT: &str = "orch.hitl.interrupt";
/// D5: Risk matrix score event.
pub const METRIC_TYPE_RISK_SCORE: &str = "orch.risk.score";
/// D8: Privacy routing decision.
pub const METRIC_TYPE_PRIVACY_ROUTE_DECISION: &str = "orch.privacy.route_decision";
/// D7: Cache hit prediction for provider routing.
pub const METRIC_TYPE_CACHE_HIT_PREDICTION: &str = "orch.cache.hit_prediction";
/// D7: Budget gate decision (Proceed / Downgrade / Halt).
pub const METRIC_TYPE_BUDGET_DECISION: &str = "orch.budget.decision";
/// D10: Calibration loop run observation.
pub const METRIC_TYPE_CALIBRATION_RUN: &str = "orch.calibration.run";
/// D10: Calibration drift alert — predicted vs. observed confidence diverging.
pub const METRIC_TYPE_DRIFT_ALERT: &str = "orch.calibration.drift_alert";
/// D10: Contextual bandit arm update.
pub const METRIC_TYPE_BANDIT_UPDATE: &str = "orch.calibration.bandit_update";
/// D4: Sub-agent dispatched.
pub const METRIC_TYPE_SUBAGENT_DISPATCH: &str = "orch.subagent.dispatch";
/// D4: Sub-agent chain depth exceeded configured cap.
pub const METRIC_TYPE_CHAIN_DEPTH_ALERT: &str = "orch.subagent.chain_depth_alert";

// ── New metric types (Phase B–D) — defined here, used when emit sites are added ─

/// S1 — Per-LLM-call record: tokens, cache, latency, cost, error class. (Phase B)
pub const METRIC_TYPE_MODEL_CALL_EVENT: &str = "model_call_event";
/// S1 — Top-level task completion rollup: total tokens, cost, wall time, depth. (Phase C)
pub const METRIC_TYPE_TASK_ROOT_SUMMARY: &str = "task.root_summary";
/// S0 — Build summary mirrored from build_run after `vox ci build-timings`. (Phase D)
pub const METRIC_TYPE_BUILD_SUMMARY_EVENT: &str = "build.summary";
/// S1 — Generic subsystem error / retry event. (Phase D)
pub const METRIC_TYPE_ERROR_EVENT: &str = "telemetry.error";

// ── Session id prefixes ──────────────────────────────────────────────────────

pub const SESSION_PREFIX_BENCH: &str = "bench:";
pub const SESSION_PREFIX_SYNTAXK: &str = "syntaxk:";
pub const SESSION_PREFIX_MCP: &str = "mcp:";
pub const SESSION_PREFIX_WORKFLOW: &str = "workflow:";
pub const SESSION_PREFIX_MENS: &str = "mens:";
pub const SESSION_PREFIX_ROUTE: &str = "route:";

/// Hybrid retrieval telemetry uses a fixed session id (not `mcp:<repository_id>`).
pub const SESSION_ID_MEMORY_HYBRID_FUSION: &str = "socrates:retrieval";

// ── TelemetryWriteOptions ────────────────────────────────────────────────────

/// Repository-scoped session_id builder. Use with `METRIC_TYPE_*` constants.
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

    #[inline]
    pub fn session_route(&self) -> String {
        format!("{SESSION_PREFIX_ROUTE}{}", self.repository_id)
    }
}

// ── Validation ───────────────────────────────────────────────────────────────

fn valid_metric_type_chars(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | ':'))
}

/// Validate inputs before `INSERT INTO research_metrics`.
///
/// Called by [`vox_db::VoxDb::append_research_metric`] on every write.
pub fn validate_research_metric_row(
    session_id: &str,
    metric_type: &str,
    metadata_json: Option<&str>,
) -> Result<(), TelemetryError> {
    if session_id.is_empty() {
        return Err(TelemetryError::Validation(
            "research_metrics: session_id must be non-empty".into(),
        ));
    }
    if session_id.len() > RESEARCH_METRICS_SESSION_ID_MAX_CHARS {
        return Err(TelemetryError::Validation(format!(
            "research_metrics: session_id exceeds {} characters",
            RESEARCH_METRICS_SESSION_ID_MAX_CHARS
        )));
    }
    if metric_type.is_empty() {
        return Err(TelemetryError::Validation(
            "research_metrics: metric_type must be non-empty".into(),
        ));
    }
    if metric_type.len() > RESEARCH_METRICS_METRIC_TYPE_MAX_CHARS {
        return Err(TelemetryError::Validation(format!(
            "research_metrics: metric_type exceeds {} characters",
            RESEARCH_METRICS_METRIC_TYPE_MAX_CHARS
        )));
    }
    if !valid_metric_type_chars(metric_type) {
        return Err(TelemetryError::Validation(format!(
            "research_metrics: metric_type {metric_type:?} contains disallowed characters"
        )));
    }
    if let Some(m) = metadata_json {
        if m.len() > RESEARCH_METRICS_METADATA_JSON_MAX_BYTES {
            return Err(TelemetryError::Validation(format!(
                "research_metrics: metadata_json exceeds {} bytes",
                RESEARCH_METRICS_METADATA_JSON_MAX_BYTES
            )));
        }
        if metric_type == METRIC_TYPE_MODEL_ROUTE_EVENT
            && (!m.contains("\"trace_id\"") || !m.contains("\"route_policy_profile\""))
        {
            return Err(TelemetryError::Validation(
                "research_metrics: model_route_event metadata_json must include trace_id and route_policy_profile".into(),
            ));
        }
    }
    Ok(())
}

// ── TelemetryEvent ─────────────────────────────────────────────────────────

/// A single telemetry observation that may be routed to one or more sinks.
///
/// New variants are added in later phases; each variant maps to one or more
/// `METRIC_TYPE_*` constants. All variants are `non_exhaustive` so older sinks
/// can ignore unknown events safely via `_ => {}` arms.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TelemetryEvent {
    /// Wraps the existing research_metrics row. Passed through ResearchMetricsSink unchanged.
    ResearchMetric(ResearchMetricEvent),
    // Phase B adds: ModelCall(ModelCallEvent)
    // Phase C adds: TaskRootSummary(TaskRootSummaryEvent)
    // Phase D adds: BuildSummary(BuildSummaryEvent), Error(ErrorEvent)
}

/// Payload for a `TelemetryEvent::ResearchMetric`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResearchMetricEvent {
    pub session_id: String,
    pub metric_type: String,
    pub metric_value: Option<f64>,
    pub metadata_json: Option<String>,
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
        assert_eq!(o.session_route(), "route:rid42");
    }

    #[test]
    fn new_phase_b_d_constants_pass_validation() {
        for mt in [
            METRIC_TYPE_MODEL_CALL_EVENT,
            METRIC_TYPE_TASK_ROOT_SUMMARY,
            METRIC_TYPE_BUILD_SUMMARY_EVENT,
            METRIC_TYPE_ERROR_EVENT,
        ] {
            assert!(
                validate_research_metric_row("sess", mt, None).is_ok(),
                "{mt} failed validation"
            );
        }
    }
}
