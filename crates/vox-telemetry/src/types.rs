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
/// S1 — AgentOS guardrail denied a tool preflight (mutation / destructive heuristic).
pub const METRIC_TYPE_AGENTOS_GUARDRAIL_DENY: &str = "orch.agentos.guardrail_deny";

// ── New metric types (Phase B–D) — defined here, used when emit sites are added ─

/// S1 — Per-LLM-call record: tokens, cache, latency, cost, error class. (Phase B)
pub const METRIC_TYPE_MODEL_CALL_EVENT: &str = "model_call_event";
/// S1 — Top-level task completion rollup: total tokens, cost, wall time, depth. (Phase C)
pub const METRIC_TYPE_TASK_ROOT_SUMMARY: &str = "task.root_summary";
/// S0 — Build summary mirrored from build_run after `vox ci build-timings`. (Phase D)
pub const METRIC_TYPE_BUILD_SUMMARY_EVENT: &str = "build.summary";
/// S1 — Generic subsystem error / retry event. (Phase D)
pub const METRIC_TYPE_ERROR_EVENT: &str = "telemetry.error";

/// S1 — AI-first `@ai` fixture: intent routing metadata (task category, strengths).
pub const METRIC_TYPE_FIXTURE_MODEL_INTENT: &str = "fixture.model.intent_resolved";
/// S1 — AI-first `@prompt` fixture: cascade dispatch observation.
pub const METRIC_TYPE_FIXTURE_PROMPT_DISPATCH: &str = "fixture.prompt.dispatch";
/// S1 — AI-first `@search` fixture: retrieval dispatch observation.
pub const METRIC_TYPE_FIXTURE_SEARCH_DISPATCH: &str = "fixture.search.dispatch";
/// S1 — AI-first `@hole` fixture: compile-time hole observed (staleness / inventory).
pub const METRIC_TYPE_FIXTURE_HOLE_OBSERVED: &str = "fixture.hole.observed";

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
/// Called by `vox_db::VoxDb::append_research_metric` on every write.
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
    /// Per-LLM-call record (Phase B).
    ModelCall(ModelCallEvent),
    /// Top-level task completion rollup (Phase C).
    TaskRootSummary(TaskRootSummaryEvent),
    /// Build summary mirrored from `vox ci build-timings` (Phase D).
    BuildSummary(BuildSummaryEvent),
    /// Subsystem error / retry event (Phase D).
    Error(ErrorEvent),
    /// AI-first language fixtures (`@ai`, `@prompt`, `@subagent`, `@search`, `@hole`).
    AiFixture(AiFixtureEvent),
}

/// Payload aligned with `contracts/telemetry/fixture-model-intent-resolved.v1.schema.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct FixtureModelIntentResolvedEvent {
    pub task_category: String,
    #[serde(default)]
    pub strengths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_max: Option<String>,
    pub resolved_model_hint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// Payload aligned with `contracts/telemetry/orch-subagent-dispatch.v1.schema.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SubagentDispatchTelemetryPayload {
    pub metric_type: String,
    pub decision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub complexity: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_task_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_depth: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_latency_ms: Option<u64>,
}

/// Payload aligned with `contracts/telemetry/fixture-prompt-dispatch.v1.schema.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct PromptDispatchTelemetryEvent {
    pub stage: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub redact_count: u32,
}

/// Payload aligned with `contracts/telemetry/fixture-search-dispatch.v1.schema.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SearchDispatchTelemetryEvent {
    pub corpus: String,
    pub outcome: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

/// Payload aligned with `contracts/telemetry/fixture-hole-observed.v1.schema.json`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct HoleObservedTelemetryEvent {
    pub cache_key: String,
    pub observation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewer: Option<String>,
}

/// Tagged union for AI fixture telemetry (serialized as JSON `metadata_json` in research_metrics).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(tag = "fixture_kind", rename_all = "snake_case")]
pub enum AiFixtureEvent {
    ModelIntent(FixtureModelIntentResolvedEvent),
    SubagentDispatch(SubagentDispatchTelemetryPayload),
    PromptDispatch(PromptDispatchTelemetryEvent),
    SearchDispatch(SearchDispatchTelemetryEvent),
    HoleObserved(HoleObservedTelemetryEvent),
}

/// Back-compat alias for older docs / actor-runtime re-exports.
pub type OrchSubagentDispatchEvent = SubagentDispatchTelemetryPayload;

/// Payload for a `TelemetryEvent::ResearchMetric`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResearchMetricEvent {
    pub session_id: String,
    pub metric_type: String,
    pub metric_value: Option<f64>,
    pub metadata_json: Option<String>,
}

/// Per-LLM-call performance record. Persisted as `research_metrics` row with
/// `metric_type = METRIC_TYPE_MODEL_CALL_EVENT`.
///
/// Unlocks (in aggregate): cache hit rate, cost-per-task, p95 latency by model and route,
/// token efficiency. Sensitivity: **S1 (OperationalTracing)**.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelCallEvent {
    pub model: String,
    pub provider: String,
    pub route_profile: Option<String>,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    /// Anthropic prompt-cache: tokens read from cache (~10× cheaper than fresh input).
    pub cache_read_input_tokens: Option<u32>,
    /// Anthropic prompt-cache: tokens written to create the cache (~25% premium).
    pub cache_creation_input_tokens: Option<u32>,
    pub latency_ms: u64,
    pub cost_usd: f64,
    pub cost_source: String,
    pub error_class: Option<String>,
    pub retry_attempt: u32,
    pub task_id: Option<u64>,
    pub parent_task_id: Option<u64>,
    pub trace_id: Option<String>,
    pub caller_agent_id: Option<String>,
}

/// Top-level task completion rollup. Persisted as `research_metrics` row with
/// `metric_type = METRIC_TYPE_TASK_ROOT_SUMMARY`.
///
/// One row per top-level task. Aggregates totals across all child agent calls
/// and LLM calls within the task. Token/cost aggregates are populated in Phase D
/// (wall_time_ms is 0 as a placeholder until task start time is threaded through).
/// Sensitivity: **S1 (OperationalTracing)**.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskRootSummaryEvent {
    pub task_id: u64,
    pub trace_id: String,
    pub repository_id: Option<String>,
    /// "completed" | "failed" | "doubted" | "cancelled"
    pub outcome: String,
    pub wall_time_ms: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub child_call_count: u32,
    pub max_span_depth: u16,
    pub subagent_fanout: u32,
}

/// Build pipeline summary. Persisted as `research_metrics` row with
/// `metric_type = METRIC_TYPE_BUILD_SUMMARY_EVENT`.
///
/// Emitted after `vox ci build-timings` completes or fails.
/// Sensitivity: **S0 (OperationalMetrics — no user content)**.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildSummaryEvent {
    /// Identifier for the build run (CI job ID, local session ID, etc.).
    pub build_id: String,
    /// "success" | "failure" | "cancelled"
    pub outcome: String,
    /// Total wall-clock time of the build in milliseconds.
    pub wall_time_ms: u64,
    /// Number of crates compiled.
    pub crates_compiled: u32,
    /// Number of compile errors, if any.
    pub error_count: u32,
    /// Top-level invocation context: "ci" | "local" | "watch".
    pub invocation_context: Option<String>,
}

/// Generic subsystem error / retry event. Persisted as `research_metrics` row with
/// `metric_type = METRIC_TYPE_ERROR_EVENT`.
///
/// Emitted at the call site of notable errors: HTTP failures, 429 rate-limits,
/// circuit-breaker trips. Carries enough context to filter by subsystem and
/// error kind without embedding user content.
/// Sensitivity: **S1 (OperationalTracing)**.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ErrorEvent {
    /// Dot-separated subsystem path, e.g. "llm.http", "orch.circuit_breaker".
    pub subsystem: String,
    /// Short kebab-case error class, e.g. "rate-limited", "connection-timeout".
    pub error_class: String,
    /// HTTP status code, if applicable.
    pub http_status: Option<u16>,
    /// Which retry attempt this is (0 = first failure, 1 = after first retry, …).
    pub retry_attempt: u32,
    /// Whether the call was ultimately retried after this error.
    pub retried: bool,
    /// Optional model identifier if the error occurred during an LLM call.
    pub model: Option<String>,
    /// Optional provider identifier.
    pub provider: Option<String>,
    /// Optional task id from the ambient TraceContext.
    pub task_id: Option<u64>,
    /// Optional trace id from the ambient TraceContext.
    pub trace_id: Option<String>,
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
            METRIC_TYPE_FIXTURE_MODEL_INTENT,
            METRIC_TYPE_FIXTURE_PROMPT_DISPATCH,
            METRIC_TYPE_FIXTURE_SEARCH_DISPATCH,
            METRIC_TYPE_FIXTURE_HOLE_OBSERVED,
        ] {
            assert!(
                validate_research_metric_row("sess", mt, None).is_ok(),
                "{mt} failed validation"
            );
        }
    }

    #[test]
    fn ai_fixture_event_round_trip() {
        let ev = AiFixtureEvent::ModelIntent(FixtureModelIntentResolvedEvent {
            task_category: "CodeGen".into(),
            strengths: vec!["codegen".into()],
            tier_max: Some("Pro".into()),
            resolved_model_hint: "openrouter/auto".into(),
            trace_id: Some("t1".into()),
        });
        let outer = TelemetryEvent::AiFixture(ev.clone());
        let json = serde_json::to_string(&outer).expect("serialize");
        let back: TelemetryEvent = serde_json::from_str(&json).expect("deserialize");
        let TelemetryEvent::AiFixture(AiFixtureEvent::ModelIntent(m)) = back else {
            panic!("variant lost");
        };
        assert_eq!(m.task_category, "CodeGen");
        assert_eq!(m.resolved_model_hint, "openrouter/auto");
    }

    #[test]
    fn model_call_event_serialize_round_trip() {
        let e = ModelCallEvent {
            model: "claude-opus-4-7".into(),
            provider: "anthropic".into(),
            route_profile: Some("strong".into()),
            prompt_tokens: 1234,
            completion_tokens: 567,
            cache_read_input_tokens: Some(800),
            cache_creation_input_tokens: Some(50),
            latency_ms: 2400,
            cost_usd: 0.0152,
            cost_source: "provider_reported".into(),
            error_class: None,
            retry_attempt: 0,
            task_id: Some(42),
            parent_task_id: None,
            trace_id: Some("abc-123".into()),
            caller_agent_id: None,
        };
        let event = TelemetryEvent::ModelCall(e.clone());
        let json = serde_json::to_string(&event).expect("serialize");
        let back: TelemetryEvent = serde_json::from_str(&json).expect("deserialize");
        let TelemetryEvent::ModelCall(back) = back else {
            panic!("variant lost in round trip")
        };
        assert_eq!(back.model, e.model);
        assert_eq!(back.cache_read_input_tokens, e.cache_read_input_tokens);
        assert_eq!(back.trace_id, e.trace_id);
    }
}
