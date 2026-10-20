//! [`ResearchMetricsSink`] — writes telemetry events to the `research_metrics` table
//! via [`crate::VoxDb::append_research_metric`].
//!
//! Exhaustively maps all 26 [`TelemetryEvent`] variants to `research_metrics` rows,
//! provides [`init_process_telemetry`] for process-startup recorder wiring, and
//! provides [`crate::VoxDb::audit_research_metric_shapes`] (`TYPING-02`, `DBAUDIT-01`).

use std::sync::Arc;

use vox_telemetry::{
    AiFixtureEvent, AuditRunEvent, BuildSummaryEvent, ClassificationEvent, CommandUsageEvent,
    ConfidencePromotionEvent, DefaultDecisionEvent, DiscoveryEvent, DoctorProjectCheckEvent,
    DomainMisguidanceEvent, EditPatternEvent, ErrorEvent, ErrorSurfaceEvent, HarnessUsageEvent,
    LintAutofixEvent, LintFindingEvent, METRIC_TYPE_AUDIT_RUN, METRIC_TYPE_BUILD_SUMMARY_EVENT,
    METRIC_TYPE_COMMAND_USAGE, METRIC_TYPE_CONFIDENCE_PROMOTION, METRIC_TYPE_DEFAULT_DECISION,
    METRIC_TYPE_DOCTOR_PROJECT_CHECK, METRIC_TYPE_DOMAIN_MISGUIDANCE, METRIC_TYPE_EDIT_PATTERN,
    METRIC_TYPE_ERROR_EVENT, METRIC_TYPE_ERROR_SURFACE, METRIC_TYPE_FIXTURE_HOLE_OBSERVED,
    METRIC_TYPE_FIXTURE_MODEL_INTENT, METRIC_TYPE_FIXTURE_PROMPT_DISPATCH,
    METRIC_TYPE_FIXTURE_SEARCH_DISPATCH, METRIC_TYPE_HARNESS_USAGE,
    METRIC_TYPE_LINT_AUTOFIX_APPLIED, METRIC_TYPE_LINT_AUTOFIX_REJECTED, METRIC_TYPE_LINT_FINDING,
    METRIC_TYPE_MODEL_BENCH_CASE, METRIC_TYPE_MODEL_CALL_EVENT, METRIC_TYPE_MODEL_CLASSIFICATION,
    METRIC_TYPE_MODEL_DISCOVERY, METRIC_TYPE_MODEL_PROMPT, METRIC_TYPE_REPAIR_ATTEMPT,
    METRIC_TYPE_REPAIR_OUTCOME, METRIC_TYPE_SELECTION_DECISION, METRIC_TYPE_SKILL_ACTIVATION,
    METRIC_TYPE_SUBAGENT_DISPATCH, METRIC_TYPE_TASK_ROOT_SUMMARY, METRIC_TYPE_TRAINING_THROUGHPUT,
    ModelBenchCaseEvent, ModelCallEvent, ModelPromptEvent, RepairAttemptEvent, RepairOutcomeEvent,
    SelectionDecisionEvent, SkillActivationEvent, TaskRootSummaryEvent, TelemetryEvent,
    TelemetryRecorder, TrainingThroughputEvent,
};

/// Map any [`TelemetryEvent`] into its `(session_id, metric_type, metric_value, metadata_json)` tuple.
#[must_use]
pub fn map_telemetry_event_to_row(
    event: &TelemetryEvent,
) -> Option<(String, String, Option<f64>, Option<String>)> {
    match event {
        TelemetryEvent::ResearchMetric(e) => Some((
            e.session_id.clone(),
            e.metric_type.clone(),
            e.metric_value,
            e.metadata_json.clone(),
        )),
        TelemetryEvent::ModelCall(e) => Some((
            format!(
                "model:{}",
                e.task_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "0".into())
            ),
            METRIC_TYPE_MODEL_CALL_EVENT.to_string(),
            Some(e.cost_usd),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::TaskRootSummary(e) => Some((
            format!("task:{}", e.task_id),
            METRIC_TYPE_TASK_ROOT_SUMMARY.to_string(),
            Some(e.total_cost_usd),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::BuildSummary(e) => Some((
            format!("build:{}", e.build_id),
            METRIC_TYPE_BUILD_SUMMARY_EVENT.to_string(),
            Some(e.wall_time_ms as f64),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::Error(e) => Some((
            format!("error:{}", e.subsystem),
            METRIC_TYPE_ERROR_EVENT.to_string(),
            Some(if e.retried { 0.0 } else { 1.0 }),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::AiFixture(ev) => {
            let (sid, mt) = match ev {
                AiFixtureEvent::ModelIntent(m) => (
                    format!(
                        "fixture:intent:{}",
                        m.trace_id.as_deref().unwrap_or("unknown")
                    ),
                    METRIC_TYPE_FIXTURE_MODEL_INTENT,
                ),
                AiFixtureEvent::SubagentDispatch(p) => (
                    format!(
                        "fixture:subagent:{}",
                        p.session_id.as_deref().unwrap_or("unknown")
                    ),
                    METRIC_TYPE_SUBAGENT_DISPATCH,
                ),
                AiFixtureEvent::PromptDispatch(p) => (
                    format!("fixture:prompt:{}", p.stage),
                    METRIC_TYPE_FIXTURE_PROMPT_DISPATCH,
                ),
                AiFixtureEvent::SearchDispatch(s) => (
                    format!("fixture:search:{}", s.corpus),
                    METRIC_TYPE_FIXTURE_SEARCH_DISPATCH,
                ),
                AiFixtureEvent::HoleObserved(h) => (
                    format!("fixture:hole:{}", h.cache_key),
                    METRIC_TYPE_FIXTURE_HOLE_OBSERVED,
                ),
            };
            Some((sid, mt.to_string(), None, serde_json::to_string(ev).ok()))
        }
        TelemetryEvent::LintFinding(e) => Some((
            format!("lint:{}", e.repository_id.as_deref().unwrap_or("vox")),
            METRIC_TYPE_LINT_FINDING.to_string(),
            Some(e.line as f64),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::LintAutofix(e) => {
            let mt = if e.outcome == "rejected" {
                METRIC_TYPE_LINT_AUTOFIX_REJECTED
            } else {
                METRIC_TYPE_LINT_AUTOFIX_APPLIED
            };
            Some((
                format!("lint:{}", e.repository_id.as_deref().unwrap_or("vox")),
                mt.to_string(),
                Some(if e.outcome == "applied" { 1.0 } else { 0.0 }),
                serde_json::to_string(e).ok(),
            ))
        }
        TelemetryEvent::RepairAttempt(e) => Some((
            format!("repair:{}", e.repository_id.as_deref().unwrap_or("vox")),
            METRIC_TYPE_REPAIR_ATTEMPT.to_string(),
            Some(e.cost_usd),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::RepairOutcome(e) => Some((
            format!("repair:{}", e.repository_id.as_deref().unwrap_or("vox")),
            METRIC_TYPE_REPAIR_OUTCOME.to_string(),
            Some(e.total_cost_usd),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::AuditRun(e) => Some((
            format!("audit:{}", e.repository_id.as_deref().unwrap_or("vox")),
            METRIC_TYPE_AUDIT_RUN.to_string(),
            Some(e.duration_seconds),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::SelectionDecision(e) => Some((
            format!(
                "model_autonomic:{}",
                e.repository_id.as_deref().unwrap_or("vox")
            ),
            METRIC_TYPE_SELECTION_DECISION.to_string(),
            None,
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::ModelDiscovery(e) => Some((
            format!("model_autonomic:{}", e.source),
            METRIC_TYPE_MODEL_DISCOVERY.to_string(),
            e.max_context_tokens.map(|v| v as f64),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::ModelClassification(e) => Some((
            "model_autonomic:classify".to_string(),
            METRIC_TYPE_MODEL_CLASSIFICATION.to_string(),
            Some(e.confidence as f64),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::ConfidencePromotion(e) => Some((
            "model_autonomic:promote".to_string(),
            METRIC_TYPE_CONFIDENCE_PROMOTION.to_string(),
            None,
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::DoctorProjectCheck(e) => Some((
            format!("doctor:{}", e.repository_id.as_deref().unwrap_or("vox")),
            METRIC_TYPE_DOCTOR_PROJECT_CHECK.to_string(),
            Some(e.duration_seconds),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::CommandUsage(e) => Some((
            "cli:command_usage".to_string(),
            METRIC_TYPE_COMMAND_USAGE.to_string(),
            None,
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::SkillActivation(e) => Some((
            "mcp:skill_activation".to_string(),
            METRIC_TYPE_SKILL_ACTIVATION.to_string(),
            Some(if e.accepted { 1.0 } else { 0.0 }),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::EditPattern(e) => Some((
            "mcp:edit_pattern".to_string(),
            METRIC_TYPE_EDIT_PATTERN.to_string(),
            None,
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::HarnessUsage(e) => Some((
            "mcp:harness_usage".to_string(),
            METRIC_TYPE_HARNESS_USAGE.to_string(),
            None,
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::ErrorSurface(e) => Some((
            "mcp:error_surface".to_string(),
            METRIC_TYPE_ERROR_SURFACE.to_string(),
            Some(if e.recoverable { 0.0 } else { 1.0 }),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::DefaultDecision(e) => Some((
            "config:default_decision".to_string(),
            METRIC_TYPE_DEFAULT_DECISION.to_string(),
            e.magnitude_bucket.map(|v| v as f64),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::ModelPrompt(e) => Some((
            "mcp:model_prompt".to_string(),
            METRIC_TYPE_MODEL_PROMPT.to_string(),
            None,
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::ModelBenchCase(e) => Some((
            "bench:model_bench_case".to_string(),
            METRIC_TYPE_MODEL_BENCH_CASE.to_string(),
            Some(e.latency_ms as f64),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::TrainingThroughput(e) => Some((
            "mens:training_throughput".to_string(),
            METRIC_TYPE_TRAINING_THROUGHPUT.to_string(),
            Some(if e.success { 1.0 } else { 0.0 }),
            serde_json::to_string(e).ok(),
        )),
        TelemetryEvent::DomainMisguidance(e) => Some((
            "socrates:domain_misguidance".to_string(),
            METRIC_TYPE_DOMAIN_MISGUIDANCE.to_string(),
            Some(1.0),
            serde_json::to_string(e).ok(),
        )),
        _ => None,
    }
}

/// `TelemetryRecorder` sink backed by a live `VoxDb` connection.
pub struct ResearchMetricsSink {
    db: Arc<crate::VoxDb>,
}

impl ResearchMetricsSink {
    pub fn new(db: crate::VoxDb) -> Self {
        Self { db: Arc::new(db) }
    }

    /// Persist a [`TelemetryEvent`] synchronously in an async context (used by tests and batch writers).
    pub async fn record_async(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), crate::store::StoreError> {
        if let Some((session_id, metric_type, metric_value, metadata_json)) =
            map_telemetry_event_to_row(event)
        {
            self.db
                .append_research_metric(
                    &session_id,
                    &metric_type,
                    metric_value,
                    metadata_json.as_deref(),
                )
                .await?;
        }
        Ok(())
    }
}

impl TelemetryRecorder for ResearchMetricsSink {
    fn record(&self, event: &TelemetryEvent) {
        if let Some((session_id, metric_type, metric_value, metadata_json)) =
            map_telemetry_event_to_row(event)
        {
            let db = Arc::clone(&self.db);
            tokio::spawn(async move {
                if let Err(err) = db
                    .append_research_metric(
                        &session_id,
                        &metric_type,
                        metric_value,
                        metadata_json.as_deref(),
                    )
                    .await
                {
                    tracing::warn!(
                        ?err,
                        %metric_type,
                        "ResearchMetricsSink: append_research_metric failed"
                    );
                }
            });
        }
    }
}

/// Debug stderr telemetry sink enabled when `VOX_TELEMETRY=debug`.
pub struct StderrDebugSink;

impl TelemetryRecorder for StderrDebugSink {
    fn record(&self, event: &TelemetryEvent) {
        if let Ok(json) = serde_json::to_string(event) {
            eprintln!("[vox-telemetry-debug] {json}");
        }
    }
}

/// Initialize the process-global telemetry recorder (`TELEM-02`).
///
/// - `vox-orchestrator-d` passes `Some((*db).clone())` so daemon-routed events persist to `research_metrics`.
/// - `vox-gui` passes `None` so it never opens a competing `VoxDb` file lock (`CORE-04`).
pub fn init_process_telemetry(db: Option<crate::VoxDb>) {
    let cfg = vox_telemetry::TelemetryConfig::from_env();
    if !cfg.enabled || vox_telemetry::org_policy_disabled() {
        vox_telemetry::set_global_recorder(Arc::new(vox_telemetry::NoOpRecorder));
        return;
    }
    let mut sinks: Vec<Arc<dyn TelemetryRecorder>> = Vec::new();
    if let Some(db) = db {
        sinks.push(Arc::new(ResearchMetricsSink::new(db)));
    }
    if cfg.debug_to_stderr {
        sinks.push(Arc::new(StderrDebugSink));
    }
    if sinks.is_empty() {
        vox_telemetry::set_global_recorder(Arc::new(vox_telemetry::NoOpRecorder));
    } else {
        vox_telemetry::set_global_recorder(Arc::new(vox_telemetry::CompositeRecorder::new(sinks)));
    }
}

// ── Strongly-typed VoxDb research_metrics payload & shape audit (`TYPING-02`, `DBAUDIT-01`) ──

/// Strongly-typed deserialized payload for a `research_metrics` row.
#[derive(Debug, Clone)]
pub enum TypedResearchMetricPayload {
    ResearchMetric(Option<serde_json::Value>),
    ModelCall(ModelCallEvent),
    TaskRootSummary(TaskRootSummaryEvent),
    BuildSummary(BuildSummaryEvent),
    Error(ErrorEvent),
    AiFixture(AiFixtureEvent),
    LintFinding(LintFindingEvent),
    LintAutofix(LintAutofixEvent),
    RepairAttempt(RepairAttemptEvent),
    RepairOutcome(RepairOutcomeEvent),
    AuditRun(AuditRunEvent),
    SelectionDecision(SelectionDecisionEvent),
    ModelDiscovery(DiscoveryEvent),
    ModelClassification(ClassificationEvent),
    ConfidencePromotion(ConfidencePromotionEvent),
    DoctorProjectCheck(DoctorProjectCheckEvent),
    CommandUsage(CommandUsageEvent),
    SkillActivation(SkillActivationEvent),
    EditPattern(EditPatternEvent),
    HarnessUsage(HarnessUsageEvent),
    ErrorSurface(ErrorSurfaceEvent),
    DefaultDecision(DefaultDecisionEvent),
    ModelPrompt(ModelPromptEvent),
    ModelBenchCase(ModelBenchCaseEvent),
    TrainingThroughput(TrainingThroughputEvent),
    DomainMisguidance(DomainMisguidanceEvent),
}

impl TypedResearchMetricPayload {
    /// Parse `metadata_json` strictly according to `metric_type`.
    pub fn parse_by_metric_type(
        metric_type: &str,
        metadata_json: Option<&str>,
    ) -> Result<Self, String> {
        let raw = metadata_json.unwrap_or("");
        match metric_type {
            METRIC_TYPE_MODEL_CALL_EVENT => serde_json::from_str::<ModelCallEvent>(raw)
                .map(Self::ModelCall)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_TASK_ROOT_SUMMARY => serde_json::from_str::<TaskRootSummaryEvent>(raw)
                .map(Self::TaskRootSummary)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_BUILD_SUMMARY_EVENT => serde_json::from_str::<BuildSummaryEvent>(raw)
                .map(Self::BuildSummary)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_ERROR_EVENT => serde_json::from_str::<ErrorEvent>(raw)
                .map(Self::Error)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_FIXTURE_MODEL_INTENT
            | METRIC_TYPE_SUBAGENT_DISPATCH
            | METRIC_TYPE_FIXTURE_PROMPT_DISPATCH
            | METRIC_TYPE_FIXTURE_SEARCH_DISPATCH
            | METRIC_TYPE_FIXTURE_HOLE_OBSERVED => serde_json::from_str::<AiFixtureEvent>(raw)
                .map(Self::AiFixture)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_LINT_FINDING => serde_json::from_str::<LintFindingEvent>(raw)
                .map(Self::LintFinding)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_LINT_AUTOFIX_APPLIED | METRIC_TYPE_LINT_AUTOFIX_REJECTED => {
                serde_json::from_str::<LintAutofixEvent>(raw)
                    .map(Self::LintAutofix)
                    .map_err(|e| e.to_string())
            }
            METRIC_TYPE_REPAIR_ATTEMPT => serde_json::from_str::<RepairAttemptEvent>(raw)
                .map(Self::RepairAttempt)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_REPAIR_OUTCOME => serde_json::from_str::<RepairOutcomeEvent>(raw)
                .map(Self::RepairOutcome)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_AUDIT_RUN => serde_json::from_str::<AuditRunEvent>(raw)
                .map(Self::AuditRun)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_SELECTION_DECISION => serde_json::from_str::<SelectionDecisionEvent>(raw)
                .map(Self::SelectionDecision)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_MODEL_DISCOVERY => serde_json::from_str::<DiscoveryEvent>(raw)
                .map(Self::ModelDiscovery)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_MODEL_CLASSIFICATION => serde_json::from_str::<ClassificationEvent>(raw)
                .map(Self::ModelClassification)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_CONFIDENCE_PROMOTION => {
                serde_json::from_str::<ConfidencePromotionEvent>(raw)
                    .map(Self::ConfidencePromotion)
                    .map_err(|e| e.to_string())
            }
            METRIC_TYPE_DOCTOR_PROJECT_CHECK => {
                serde_json::from_str::<DoctorProjectCheckEvent>(raw)
                    .map(Self::DoctorProjectCheck)
                    .map_err(|e| e.to_string())
            }
            METRIC_TYPE_COMMAND_USAGE => serde_json::from_str::<CommandUsageEvent>(raw)
                .map(Self::CommandUsage)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_SKILL_ACTIVATION => serde_json::from_str::<SkillActivationEvent>(raw)
                .map(Self::SkillActivation)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_EDIT_PATTERN => serde_json::from_str::<EditPatternEvent>(raw)
                .map(Self::EditPattern)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_HARNESS_USAGE => serde_json::from_str::<HarnessUsageEvent>(raw)
                .map(Self::HarnessUsage)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_ERROR_SURFACE => serde_json::from_str::<ErrorSurfaceEvent>(raw)
                .map(Self::ErrorSurface)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_DEFAULT_DECISION => serde_json::from_str::<DefaultDecisionEvent>(raw)
                .map(Self::DefaultDecision)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_MODEL_PROMPT => serde_json::from_str::<ModelPromptEvent>(raw)
                .map(Self::ModelPrompt)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_MODEL_BENCH_CASE => serde_json::from_str::<ModelBenchCaseEvent>(raw)
                .map(Self::ModelBenchCase)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_TRAINING_THROUGHPUT => serde_json::from_str::<TrainingThroughputEvent>(raw)
                .map(Self::TrainingThroughput)
                .map_err(|e| e.to_string()),
            METRIC_TYPE_DOMAIN_MISGUIDANCE => serde_json::from_str::<DomainMisguidanceEvent>(raw)
                .map(Self::DomainMisguidance)
                .map_err(|e| e.to_string()),
            vox_telemetry::METRIC_TYPE_BENCHMARK_EVENT
            | vox_telemetry::METRIC_TYPE_SYNTAX_K_EVENT
            | vox_telemetry::METRIC_TYPE_SOCRATES_SURFACE
            | vox_telemetry::METRIC_TYPE_MEMORY_HYBRID_FUSION
            | vox_telemetry::METRIC_TYPE_WORKFLOW_JOURNAL_ENTRY
            | vox_telemetry::METRIC_TYPE_POPULI_CONTROL_EVENT
            | vox_telemetry::METRIC_TYPE_QUESTIONING_EVENT
            | vox_telemetry::METRIC_TYPE_AGENT_EXEC_TIME
            | vox_telemetry::METRIC_TYPE_MODEL_ROUTE_EVENT => {
                if raw.is_empty() {
                    Ok(Self::ResearchMetric(None))
                } else {
                    serde_json::from_str::<serde_json::Value>(raw)
                        .map(|v| Self::ResearchMetric(Some(v)))
                        .map_err(|e| e.to_string())
                }
            }
            other => Err(format!("unknown metric_type: {other}")),
        }
    }
}

/// Strongly-typed row rehydrated from `research_metrics`.
#[derive(Debug, Clone)]
pub struct TypedResearchMetricRow {
    pub session_id: String,
    pub metric_type: String,
    pub metric_value: Option<f64>,
    pub payload: TypedResearchMetricPayload,
}

/// Shape verification report returned by [`crate::VoxDb::audit_research_metric_shapes`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DbShapeAuditReport {
    pub total_rows_checked: usize,
    pub valid_typed_rows: usize,
    pub unknown_metric_types: Vec<String>,
    pub malformed_metadata_rows: Vec<(String, String)>,
    pub unexpected_null_metric_values: Vec<(String, String)>,
}

impl DbShapeAuditReport {
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.unknown_metric_types.is_empty()
            && self.malformed_metadata_rows.is_empty()
            && self.unexpected_null_metric_values.is_empty()
            && self.total_rows_checked == self.valid_typed_rows
    }
}

impl crate::VoxDb {
    /// Audit `research_metrics` rows matching `session_prefix` (pass `""` to scan all sessions)
    /// and verify that every row deserializes into [`TypedResearchMetricPayload`] with expected numeric values (`DBAUDIT-01`).
    pub async fn audit_research_metric_shapes(
        &self,
        session_prefix: &str,
    ) -> Result<DbShapeAuditReport, crate::store::StoreError> {
        let rows = self
            .list_research_metrics_by_session(session_prefix, None, 10_000)
            .await?;
        let mut report = DbShapeAuditReport {
            total_rows_checked: rows.len(),
            ..Default::default()
        };

        for (session_id, metric_type, metric_value, metadata_json) in rows {
            let requires_numeric = matches!(
                metric_type.as_str(),
                METRIC_TYPE_MODEL_CALL_EVENT
                    | METRIC_TYPE_TASK_ROOT_SUMMARY
                    | METRIC_TYPE_BUILD_SUMMARY_EVENT
                    | METRIC_TYPE_REPAIR_ATTEMPT
                    | METRIC_TYPE_REPAIR_OUTCOME
                    | METRIC_TYPE_AUDIT_RUN
                    | METRIC_TYPE_DOCTOR_PROJECT_CHECK
                    | METRIC_TYPE_SKILL_ACTIVATION
                    | METRIC_TYPE_ERROR_SURFACE
                    | METRIC_TYPE_MODEL_BENCH_CASE
                    | METRIC_TYPE_TRAINING_THROUGHPUT
                    | METRIC_TYPE_DOMAIN_MISGUIDANCE
            );
            if requires_numeric && !metric_value.is_some_and(f64::is_finite) {
                report
                    .unexpected_null_metric_values
                    .push((session_id.clone(), metric_type.clone()));
                continue;
            }

            match TypedResearchMetricPayload::parse_by_metric_type(
                &metric_type,
                metadata_json.as_deref(),
            ) {
                Ok(_payload) => {
                    report.valid_typed_rows += 1;
                }
                Err(err) if err.starts_with("unknown metric_type:") => {
                    report.unknown_metric_types.push(metric_type);
                }
                Err(err) => {
                    report
                        .malformed_metadata_rows
                        .push((session_id, format!("{metric_type}: {err}")));
                }
            }
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_telemetry::{
        FixtureModelIntentResolvedEvent, METRIC_TYPE_BENCHMARK_EVENT, ResearchMetricEvent,
    };

    #[test]
    fn research_metrics_sink_is_recorder() {
        fn _assert_recorder<T: vox_telemetry::TelemetryRecorder>() {}
        _assert_recorder::<ResearchMetricsSink>();
    }

    #[tokio::test]
    async fn all_26_telemetry_event_variants_persist_and_pass_db_shape_audit() {
        let db = crate::VoxDb::in_memory().await.expect("in-memory VoxDb");
        let sink = ResearchMetricsSink::new(db.clone());

        let events = vec![
            TelemetryEvent::ResearchMetric(ResearchMetricEvent {
                session_id: "bench:r1".into(),
                metric_type: METRIC_TYPE_BENCHMARK_EVENT.into(),
                metric_value: Some(1.0),
                metadata_json: Some(r#"{"ok":true}"#.into()),
            }),
            TelemetryEvent::ModelCall(ModelCallEvent {
                model: "qwen3.5-4b".into(),
                provider: "ollama".into(),
                route_profile: Some("local".into()),
                selection_rationale: None,
                prompt_tokens: 120,
                completion_tokens: 64,
                cache_read_input_tokens: None,
                cache_creation_input_tokens: None,
                latency_ms: 140,
                cost_usd: 0.0,
                cost_source: "local".into(),
                error_class: None,
                retry_attempt: 0,
                task_id: Some(1),
                parent_task_id: None,
                trace_id: None,
                caller_agent_id: None,
            }),
            TelemetryEvent::TaskRootSummary(TaskRootSummaryEvent {
                task_id: 1,
                trace_id: "t1".into(),
                repository_id: None,
                outcome: "completed".into(),
                total_input_tokens: 120,
                total_output_tokens: 64,
                total_cost_usd: 0.0,
                wall_time_ms: 200,
                child_call_count: 1,
                max_span_depth: 1,
                subagent_fanout: 0,
            }),
            TelemetryEvent::BuildSummary(BuildSummaryEvent {
                build_id: "b1".into(),
                outcome: "success".into(),
                wall_time_ms: 950,
                crates_compiled: 5,
                error_count: 0,
                invocation_context: Some("cli".into()),
            }),
            TelemetryEvent::Error(ErrorEvent {
                subsystem: "llm.http".into(),
                error_class: "timeout".into(),
                retried: true,
                retry_attempt: 1,
                http_status: None,
                task_id: None,
                trace_id: None,
                model: None,
                provider: None,
            }),
            TelemetryEvent::AiFixture(AiFixtureEvent::ModelIntent(
                FixtureModelIntentResolvedEvent {
                    task_category: "CodeGen".into(),
                    strengths: vec!["codegen".into()],
                    tier_max: None,
                    resolved_model_hint: "mens".into(),
                    trace_id: Some("t1".into()),
                },
            )),
            TelemetryEvent::LintFinding(LintFindingEvent {
                rule_id: "vox/rule".into(),
                diagnostic_id: None,
                severity: "warning".into(),
                relative_path: "src/main.vox".into(),
                line: 10,
                autofix_available: true,
                confidence: None,
                repository_id: Some("vox".into()),
            }),
            TelemetryEvent::LintAutofix(LintAutofixEvent {
                rule_id: "vox/rule".into(),
                diagnostic_id: None,
                outcome: "applied".into(),
                reason: None,
                relative_path: "src/main.vox".into(),
                line: 10,
                repository_id: Some("vox".into()),
            }),
            TelemetryEvent::RepairAttempt(RepairAttemptEvent {
                attempt_number: 1,
                diagnostics_in: 3,
                diagnostics_out: 0,
                files_touched: 1,
                cost_usd: 0.01,
                duration_ms: 500,
                panel_member_id: None,
                repository_id: Some("vox".into()),
            }),
            TelemetryEvent::RepairOutcome(RepairOutcomeEvent {
                final_state: "success".into(),
                attempts_used: 1,
                attempts_budget: 3,
                total_cost_usd: 0.01,
                total_duration_ms: 500,
                residual_diagnostics: 0,
                note: None,
                repository_id: Some("vox".into()),
            }),
            TelemetryEvent::AuditRun(AuditRunEvent {
                thing: "code".into(),
                outcome: "ok".into(),
                corpus_hash: "blake3:123".into(),
                corpus_size: 10,
                duration_seconds: 0.5,
                cumulative_cost_usd: 0.0,
                unreachable_panel_member_count: 0,
                panel_version: None,
                umbrella_run: false,
                repository_id: Some("vox".into()),
            }),
            TelemetryEvent::SelectionDecision(SelectionDecisionEvent {
                intent_caller: None,
                task: "codegen".into(),
                axes: (33, 33, 34),
                chosen_model: "mens".into(),
                reason: "scored".into(),
                premium_alias_key: None,
                repository_id: Some("vox".into()),
            }),
            TelemetryEvent::ModelDiscovery(DiscoveryEvent {
                source: "populi_mesh".into(),
                model_id: "mens-qwen35".into(),
                description: None,
                max_context_tokens: Some(32768),
            }),
            TelemetryEvent::ModelClassification(ClassificationEvent {
                model_id: "mens-qwen35".into(),
                classifier_model: "local".into(),
                tier: "local".into(),
                strengths: vec!["codegen".into()],
                confidence: 0.95,
            }),
            TelemetryEvent::ConfidencePromotion(ConfidencePromotionEvent {
                model_id: "mens-qwen35".into(),
                from: "shadowed".into(),
                to: "confirmed".into(),
                evidence: "scoreboard_threshold".into(),
            }),
            TelemetryEvent::DoctorProjectCheck(DoctorProjectCheckEvent {
                project_root: ".".into(),
                files_total: 4,
                files_passing: 4,
                files_failing: 0,
                outcome: "green".into(),
                duration_seconds: 0.12,
                repository_id: Some("vox".into()),
            }),
            TelemetryEvent::CommandUsage(CommandUsageEvent {
                verb: "check".into(),
                exit_class: "success".into(),
                duration_bucket: "lt1s".into(),
            }),
            TelemetryEvent::SkillActivation(SkillActivationEvent {
                skill_id_hash: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                    .into(),
                trigger_source: "pinned".into(),
                accepted: true,
                surface: "mcp".into(),
            }),
            TelemetryEvent::EditPattern(EditPatternEvent {
                op_type: "replace".into(),
                file_kind: "rust".into(),
                size_bucket: "10_to_50_lines".into(),
            }),
            TelemetryEvent::HarnessUsage(HarnessUsageEvent {
                tool_call_kind: "edit".into(),
                mode: "auto".into(),
            }),
            TelemetryEvent::ErrorSurface(ErrorSurfaceEvent {
                error_class: "timeout".into(),
                subsystem: "llm_http".into(),
                recoverable: true,
            }),
            TelemetryEvent::DefaultDecision(DefaultDecisionEvent {
                decision_id: "llm_max_concurrent".into(),
                chosen: "8".into(),
                outcome: "comfortable".into(),
                magnitude_bucket: Some(1),
            }),
            TelemetryEvent::ModelPrompt(ModelPromptEvent {
                canonical_model_id: "local_ollama".into(),
                profile_variant_id: "confirmed_v1".into(),
                task_category: "coding".into(),
                quality_bucket: "excellent".into(),
            }),
            TelemetryEvent::ModelBenchCase(ModelBenchCaseEvent {
                model_id: "mens-qwen35".into(),
                fixture_id: "spoke_vox_01".into(),
                task_category: "codegen".into(),
                passed: true,
                latency_ms: 140,
            }),
            TelemetryEvent::TrainingThroughput(TrainingThroughputEvent {
                gpu_vendor: "apple_metal".into(),
                vram_bucket: "24_to_48gb".into(),
                seq_len_bucket: "512_to_2k".into(),
                tok_per_sec_bucket: "50_to_200".into(),
                success: true,
            }),
            TelemetryEvent::DomainMisguidance(DomainMisguidanceEvent {
                culprit_domain: "docs.example.org".into(),
                domain_category: "official_docs".into(),
                defect_class: "hallucinated_api".into(),
            }),
        ];

        assert_eq!(events.len(), 26);
        for ev in &events {
            sink.record_async(ev).await.expect("record_async");
        }

        let report = db
            .audit_research_metric_shapes("")
            .await
            .expect("audit_research_metric_shapes");
        assert_eq!(report.total_rows_checked, 26);
        assert_eq!(report.valid_typed_rows, 26);
        assert!(report.is_clean(), "expected clean report, got {report:?}");
    }
}
