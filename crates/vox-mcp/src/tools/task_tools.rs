//! Task management tool handlers for the Vox MCP server.
//!
//! Covers: submit, status, complete, fail, cancel, reorder, drain, and publish.
//!
//! ## Policy side effects
//! Submit/status paths participate in **interruption / attention policy** ([`super::attention_policy`]): they may call
//! [`ServerState::record_attention_event`](crate::server::ServerState::record_attention_event) and read trust snapshots when
//! questioning backlog or human-confirmation gates apply. That is **budget-plane telemetry**, not remote product analytics.

use vox_orchestrator::{
    AgentEventKind, AgentId, FileAffinity, TaskCategory, TaskEnqueueHints, TaskId, TaskPriority,
    session_context_envelope_key,
};
use vox_repository::{load_agent_scopes, normalize_task_path};
use vox_runtime::prompt_canonical;

use crate::params::{
    CompleteTaskParams, DrainAgentParams, FailTaskParams, PublishMessageParams, ReorderTaskParams,
    SubmitTaskParams, SubmitTaskResponse, TaskStatusParams, ToolResult,
};
use crate::server::ServerState;
use crate::tools::attention_policy::{
    evaluate_with_state, has_explicit_human_confirmation, pending_backlog_for_session,
    task_submit_signals, trust_for_session,
};
use crate::tools::session_identity::normalize_optional_session_id;

const REM_TASK_SCOPE: &str = "Limit `files` to paths under the agent scopes, or omit `agent_name` so routing picks a valid agent.";
const REM_QUESTIONING_PENDING: &str = "Call `vox_questioning_pending` for `question_id` / `question_options`, then `vox_questioning_submit_answer` with the same `session_id` as chat/plan (and optional `question_id` / `selected_option_id`), or continue until the open clarification is answered.";
const REM_PROMPT_SAFETY: &str =
    "Rewrite the task to remove injection patterns and disallowed content per Trust & Safety.";
const REM_TASK_SUBMIT: &str =
    "Check orchestrator health, queues, and that referenced files exist and are readable.";
const REM_TASK_ID: &str =
    "Confirm `task_id` with task/orchestrator status; it may be stale, completed, or cancelled.";
const REM_TASK_ORCH_OP: &str = "Verify task lifecycle state, file locks, and orchestrator health before complete/fail/cancel/reorder/drain.";
const REM_CONTEXT_ENVELOPE_JSON: &str =
    "Pass valid serialized ContextEnvelope JSON, or omit `context_envelope_json`.";
const REM_HARNESS_SPEC_JSON: &str =
    "Pass valid serialized AgentHarnessSpec JSON, or omit `harness_spec_json`.";

fn task_category_from_mcp_str(raw: &str) -> Option<TaskCategory> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "parsing" => Some(TaskCategory::Parsing),
        "type_checking" | "typechecking" => Some(TaskCategory::TypeChecking),
        "debugging" => Some(TaskCategory::Debugging),
        "research" => Some(TaskCategory::Research),
        "testing" => Some(TaskCategory::Testing),
        "general" => Some(TaskCategory::General),
        "ars" | "automated_reasoning" => Some(TaskCategory::Ars),
        "planning" | "plan" => Some(TaskCategory::Planning),
        "codegen" | "code_gen" | "implementation" => Some(TaskCategory::CodeGen),
        "review" => Some(TaskCategory::Review),
        _ => {
            tracing::debug!(%raw, "submit_task: unknown task_category; ignoring");
            None
        }
    }
}

fn parse_campaign_from_description(
    description: &str,
) -> (
    Option<String>,
    Option<vox_orchestrator::ReconstructionBenchmarkTier>,
) {
    let mut campaign_id = None;
    let mut tier = None;
    for token in description.split_whitespace() {
        let t = token.trim_matches(|c: char| c == '[' || c == ']' || c == ',' || c == ';');
        let t_lower = t.to_ascii_lowercase();
        if t_lower.starts_with("campaign:") {
            if campaign_id.is_some() {
                tracing::debug!("submit_task: multiple campaign tags found; using first");
                continue;
            }
            let v = &t["campaign:".len()..];
            let vv = v.trim();
            if !vv.is_empty() {
                campaign_id = Some(vv.to_string());
            }
        }
        if let Some(v) = t_lower.strip_prefix("tier:") {
            if tier.is_some() {
                tracing::debug!("submit_task: multiple tier tags found; using first");
                continue;
            }
            tier = match v.trim() {
                "issue_repair" => Some(vox_orchestrator::ReconstructionBenchmarkTier::IssueRepair),
                "subsystem_regen" => {
                    Some(vox_orchestrator::ReconstructionBenchmarkTier::SubsystemRegen)
                }
                "crate_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen),
                "repo_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::RepoRegen),
                other => {
                    tracing::debug!(tier = %other, "submit_task: unknown reconstruction tier; ignoring");
                    None
                }
            };
        }
    }
    (campaign_id, tier)
}

fn parse_benchmark_tier(raw: &str) -> Option<vox_orchestrator::ReconstructionBenchmarkTier> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "issue_repair" => Some(vox_orchestrator::ReconstructionBenchmarkTier::IssueRepair),
        "subsystem_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::SubsystemRegen),
        "crate_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen),
        "repo_regen" => Some(vox_orchestrator::ReconstructionBenchmarkTier::RepoRegen),
        _ => None,
    }
}

fn enqueue_hints_from_submit_params(params: &SubmitTaskParams) -> Option<TaskEnqueueHints> {
    let category = params
        .task_category
        .as_deref()
        .and_then(task_category_from_mcp_str);
    let execution_role = match category {
        Some(TaskCategory::Parsing) => Some(vox_orchestrator::AgentExecutionRole::Builder),
        Some(TaskCategory::TypeChecking) => Some(vox_orchestrator::AgentExecutionRole::Builder),
        Some(TaskCategory::Research) => Some(vox_orchestrator::AgentExecutionRole::Researcher),
        Some(TaskCategory::Testing) => Some(vox_orchestrator::AgentExecutionRole::Verifier),
        Some(TaskCategory::Review) => Some(vox_orchestrator::AgentExecutionRole::Verifier),
        Some(TaskCategory::Debugging) => Some(vox_orchestrator::AgentExecutionRole::Reproducer),
        Some(TaskCategory::CodeGen) => Some(vox_orchestrator::AgentExecutionRole::Builder),
        Some(TaskCategory::General)
        | Some(TaskCategory::Ars)
        | Some(TaskCategory::Planning) => Some(vox_orchestrator::AgentExecutionRole::Planner),
        _ => None,
    };
    let (campaign_from_desc, tier_from_desc) = parse_campaign_from_description(&params.description);
    let campaign_id = params
        .campaign_id
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or(campaign_from_desc);
    let benchmark_tier = params
        .benchmark_tier
        .as_deref()
        .and_then(parse_benchmark_tier)
        .or(tier_from_desc);
    let thread_id = normalize_optional_session_id(params.thread_id.as_deref());
    let harness_spec_json = params
        .harness_spec_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);
    if category.is_none()
        && params.complexity.is_none()
        && params.model_preference.is_none()
        && params.model_override.is_none()
        && campaign_id.is_none()
        && benchmark_tier.is_none()
        && execution_role.is_none()
        && thread_id.is_none()
        && harness_spec_json.is_none()
        && params.tool_hints.is_empty()
        && params.research_hints.is_empty()
    {
        return None;
    }
    Some(TaskEnqueueHints {
        task_category: category,
        complexity: params.complexity.map(|c| c.clamp(1, 10)),
        model_preference: params.model_preference.clone(),
        model_override: params.model_override.clone(),
        campaign_id,
        benchmark_tier,
        execution_role,
        thread_id,
        harness_spec_json,
        tool_hints: params.tool_hints.clone(),
        research_hints: params.research_hints.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_params(description: &str) -> SubmitTaskParams {
        SubmitTaskParams {
            description: description.to_string(),
            files: vec![],
            priority: None,
            agent_name: None,
            capabilities: None,
            task_category: None,
            complexity: None,
            model_preference: None,
            model_override: None,
            session_id: None,
            thread_id: None,
            planning_mode: None,
            goal_type: None,
            retrieval: None,
            context_envelope_json: None,
            harness_spec_json: None,
            goal_scope: None,
            max_plan_depth: None,
            campaign_id: None,
            benchmark_tier: None,
            trace_id: None,
            correlation_id: None,
        }
    }

    #[test]
    fn parse_campaign_from_description_extracts_campaign_and_tier_tokens() {
        let (cid, tier) =
            parse_campaign_from_description("do work [campaign:alpha1] [tier:crate_regen]");
        assert_eq!(cid.as_deref(), Some("alpha1"));
        assert_eq!(
            tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen)
        );
    }

    #[test]
    fn parse_campaign_from_description_is_case_insensitive_for_prefixes() {
        let (cid, tier) =
            parse_campaign_from_description("do work [Campaign:Alpha] [TIER:repo_regen]");
        assert_eq!(cid.as_deref(), Some("Alpha"));
        assert_eq!(
            tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::RepoRegen)
        );
    }

    #[test]
    fn enqueue_hints_from_submit_params_returns_none_when_no_signals_present() {
        let params = base_params("plain task");
        assert!(enqueue_hints_from_submit_params(&params).is_none());
    }

    #[test]
    fn enqueue_hints_from_submit_params_maps_testing_category_to_verifier_role() {
        let mut params = base_params("run tests");
        params.task_category = Some("testing".to_string());
        let hints = enqueue_hints_from_submit_params(&params).expect("hints");
        assert_eq!(
            hints.execution_role,
            Some(vox_orchestrator::AgentExecutionRole::Verifier)
        );
    }

    #[test]
    fn enqueue_hints_from_submit_params_merges_campaign_tokens_from_description() {
        let mut params = base_params("fix bug campaign:campA tier:issue_repair");
        params.complexity = Some(8);
        let hints = enqueue_hints_from_submit_params(&params).expect("hints");
        assert_eq!(hints.campaign_id.as_deref(), Some("campA"));
        assert_eq!(
            hints.benchmark_tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::IssueRepair)
        );
        assert_eq!(hints.complexity, Some(8));
    }

    #[test]
    fn enqueue_hints_prefers_structured_campaign_and_tier_over_description_tags() {
        let mut params = base_params("campaign:desc tier:issue_repair");
        params.campaign_id = Some("structured".to_string());
        params.benchmark_tier = Some("crate_regen".to_string());
        let hints = enqueue_hints_from_submit_params(&params).expect("hints");
        assert_eq!(hints.campaign_id.as_deref(), Some("structured"));
        assert_eq!(
            hints.benchmark_tier,
            Some(vox_orchestrator::ReconstructionBenchmarkTier::CrateRegen)
        );
    }

    #[test]
    fn socrates_context_from_retrieval_preserves_verification_and_quality_signals() {
        let retrieval = crate::memory::RetrievalEvidenceEnvelope {
            trigger: crate::memory::RetrievalTriggerMode::ExplicitToolQuery,
            retrieval_tier: "hybrid".to_string(),
            memory_hit_count: 2,
            knowledge_hit_count: 1,
            chunk_hit_count: 1,
            repo_hit_count: 1,
            used_vector: true,
            used_bm25: true,
            used_lexical_fallback: false,
            contradiction_count: 1,
            top_score: Some(0.73),
            search_intent: "factual_lookup".to_string(),
            selected_mode: "hybrid".to_string(),
            backend_mix: vec!["memory_vector".to_string(), "chunk_fts".to_string()],
            source_diversity: 3,
            evidence_quality: 0.68,
            citation_coverage: 0.75,
            verification_performed: true,
            verification_reason: Some("weak_evidence_quality".to_string()),
            verification_query: Some("alpha beta".to_string()),
            recommended_next_action: Some("focus_codex".to_string()),
            search_plan: serde_json::json!({ "intent": "factual_lookup" }),
            search_diagnostics: serde_json::json!({ "verification_performed": true }),
            sqlite_journal_mode: None,
            sqlite_fts5_reported: None,
            sqlite_foreign_keys_on: None,
            rrf_fused_hit_count: 0,
        };
        let ctx = socrates_context_from_retrieval(&retrieval);
        assert_eq!(ctx.source_diversity, 3);
        assert!((ctx.evidence_quality - 0.68).abs() < f64::EPSILON);
        assert!((ctx.citation_coverage - 0.75).abs() < f64::EPSILON);
        assert!(ctx.verification_performed);
        assert_eq!(
            ctx.verification_reason.as_deref(),
            Some("weak_evidence_quality")
        );
        assert_eq!(ctx.recommended_next_action.as_deref(), Some("focus_codex"));
    }
}

fn apply_mcp_trace_to_context_envelope(
    env: &mut vox_orchestrator::ContextEnvelope,
    trace_id: Option<&str>,
    correlation_id: Option<&str>,
) {
    if let Some(t) = trace_id.map(str::trim).filter(|s| !s.is_empty()) {
        env.provenance.trace_id = Some(t.to_string());
    }
    if let Some(c) = correlation_id.map(str::trim).filter(|s| !s.is_empty()) {
        env.provenance.correlation_id = Some(c.to_string());
    }
}

fn socrates_context_from_retrieval(
    retrieval: &crate::memory::RetrievalEvidenceEnvelope,
) -> vox_orchestrator::SocratesTaskContext {
    vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: retrieval.retrieval_tier.clone(),
        memory_hit_count: retrieval.memory_hit_count,
        knowledge_hit_count: retrieval.knowledge_hit_count,
        chunk_hit_count: retrieval.chunk_hit_count,
        repo_hit_count: retrieval.repo_hit_count,
        rrf_fused_hit_count: retrieval.rrf_fused_hit_count,
        used_vector: retrieval.used_vector,
        used_bm25: retrieval.used_bm25,
        used_lexical_fallback: retrieval.used_lexical_fallback,
        contradiction_count: retrieval.contradiction_count,
        source_diversity: retrieval.source_diversity,
        evidence_quality: retrieval.evidence_quality,
        citation_coverage: retrieval.citation_coverage,
        verification_performed: retrieval.verification_performed,
        verification_reason: retrieval.verification_reason.clone(),
        recommended_next_action: retrieval.recommended_next_action.clone(),
    }
    .to_task_context()
}

/// Submit a new task to the orchestrator (async).
///
/// Routes the task to the best agent based on file affinity, acquires locks,
/// and enqueues it for processing.
pub async fn submit_task(state: &ServerState, params: SubmitTaskParams) -> String {
    let mut params = params;
    let normalized_session_id = normalize_optional_session_id(params.session_id.as_deref());
    if params.session_id.is_none() {
        tracing::debug!(
            target: "vox_mcp::session",
            tool = "vox_submit_task",
            "session_id omitted; submitting with unscoped session context"
        );
    }
    if params.session_id.is_some() && normalized_session_id.is_none() {
        tracing::debug!(
            target: "vox_mcp::session",
            tool = "vox_submit_task",
            "session_id trimmed to empty; treating as absent"
        );
    }

    // Session-scoped envelopes are attached inside `submit_task_with_agent`. MCP only overrides
    // when the client passes an explicit `retrieval` payload (may differ from the store).
    let explicit_retrieval = params.retrieval.as_ref();
    let explicit_context_envelope = match params
        .context_envelope_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => match serde_json::from_str::<vox_orchestrator::ContextEnvelope>(raw) {
            Ok(env) => Some((env, raw.to_string())),
            Err(err) => {
                return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                    format!("invalid context_envelope_json: {err}"),
                    REM_CONTEXT_ENVELOPE_JSON,
                )
                .to_json();
            }
        },
        None => None,
    };
    if params.thread_id.is_none()
        && let Some((env, _)) = &explicit_context_envelope
    {
        params.thread_id = env
            .subject
            .thread_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(std::string::ToString::to_string);
    }
    let explicit_harness_spec = match params
        .harness_spec_json
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(raw) => match serde_json::from_str::<vox_orchestrator::AgentHarnessSpec>(raw) {
            Ok(mut harness) => {
                let expected_thread_id = params
                    .thread_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .or_else(|| {
                        explicit_context_envelope.as_ref().and_then(|(env, _)| {
                            env.subject
                                .thread_id
                                .as_deref()
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                        })
                    });
                let expectations = vox_orchestrator::HarnessIngestExpectations {
                    repository_id: state.repository.repository_id.as_str(),
                    session_id: normalized_session_id.as_deref(),
                    thread_id: expected_thread_id,
                };
                vox_orchestrator::apply_harness_subject_defaults(&mut harness, expectations);
                if let Err(errs) =
                    vox_orchestrator::validate_agent_harness_ingest(&harness, expectations)
                {
                    return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                        format!("invalid harness_spec_json: {}", errs.join("; ")),
                        REM_HARNESS_SPEC_JSON,
                    )
                    .to_json();
                }
                match serde_json::to_string(&harness) {
                    Ok(normalized) => Some((harness, normalized)),
                    Err(err) => {
                        return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                            format!("failed to normalize harness_spec_json: {err}"),
                            REM_HARNESS_SPEC_JSON,
                        )
                        .to_json();
                    }
                }
            }
            Err(err) => {
                return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                    format!("invalid harness_spec_json: {err}"),
                    REM_HARNESS_SPEC_JSON,
                )
                .to_json();
            }
        },
        None => None,
    };
    if let Some((_, normalized_harness_spec_json)) = &explicit_harness_spec {
        params.harness_spec_json = Some(normalized_harness_spec_json.clone());
    }
    if explicit_retrieval.is_some() && explicit_context_envelope.is_some() {
        return ToolResult::<SubmitTaskResponse>::err_with_remediation(
            "Provide only one of `retrieval` or `context_envelope_json`".to_string(),
            "Remove one field and resubmit; `context_envelope_json` is canonical.",
        )
        .to_json();
    }

    // Phase 7.3: Scope enforcement
    if let Some(agent_name) = &params.agent_name {
        if let Some(scopes) = load_agent_scopes(&state.repository.root, agent_name) {
            if !scopes.is_empty() && !scopes.iter().any(|s| s == "**" || s == "**/*") {
                for f in &params.files {
                    let mut ok = false;
                    let path_str = normalize_task_path(&state.repository.root, &f.path);
                    for s in &scopes {
                        if s.ends_with("/**") {
                            let prefix = s.trim_end_matches("/**");
                            if path_str.starts_with(prefix) {
                                ok = true;
                                break;
                            }
                        } else if path_str == *s {
                            ok = true;
                            break;
                        }
                    }
                    if !ok {
                        return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                            format!(
                                "Agent '{}' tried to edit outside its scope. File '{}' does not match scope {:?}",
                                agent_name, f.path, scopes
                            ),
                            REM_TASK_SCOPE,
                        )
                        .to_json();
                    }
                }
            }
        }
    }

    let bypass_questioning_gate = std::env::var("VOX_SUBMIT_TASK_BYPASS_QUESTIONING_GATE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !bypass_questioning_gate {
        if let (Some(db), Some(sid)) = (&state.db, normalized_session_id.as_deref()) {
            match db
                .has_pending_clarification_for_mcp_session(sid, &state.repository.repository_id)
                .await
            {
                Ok(true) => {
                    return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                        "Socrates clarification pending for this MCP session; resolve before submitting tasks."
                            .to_string(),
                        REM_QUESTIONING_PENDING,
                    )
                    .to_json();
                }
                Err(e) => tracing::debug!(error = %e, "questioning gate: pending check failed"),
                Ok(false) => {}
            }
        }
    }

    if state.orchestrator_config.attention_enabled {
        let bm = state.orchestrator.budget_manager_handle();
        let att_snap = {
            let g = vox_orchestrator::sync_lock::rw_read(&*bm);
            g.attention_snapshot()
        };
        if let vox_orchestrator::GateResult::AttentionExhausted { message, .. } =
            vox_orchestrator::BudgetGate::check_attention_snapshot(
                &att_snap,
                &state.orchestrator_config,
            )
        {
            return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                message,
                "Resolve open clarifications or raise VOX_ORCHESTRATOR_ATTENTION_BUDGET_MS / disable VOX_ORCHESTRATOR_ATTENTION_ENABLED for shadow mode.",
            )
            .to_json();
        }

        let write_file_count = params
            .files
            .iter()
            .filter(|f| matches!(f.access, crate::params::FileAccess::Write))
            .count();
        let submit_priority = match params.priority.as_deref() {
            Some("urgent") => TaskPriority::Urgent,
            Some("background") => TaskPriority::Background,
            _ => TaskPriority::Normal,
        };
        let backlog = pending_backlog_for_session(state, normalized_session_id.as_deref());
        let trust = trust_for_session(state, normalized_session_id.as_deref());
        let signals = task_submit_signals(
            &params.description,
            write_file_count,
            submit_priority,
            backlog,
            trust,
            state.orchestrator_config.attention_interrupt_cost_ms,
        );
        let decision = evaluate_with_state(state, &signals, &att_snap);
        match decision {
            vox_orchestrator::InterruptionDecision::RequireHumanBeforeContinue {
                reason, ..
            } => {
                if !has_explicit_human_confirmation(&params.description) {
                    return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                        format!(
                            "Task submit requires explicit human confirmation: {reason}. Add one of [approval:confirm], [approval:reviewed], [human-approved] to the description once reviewed."
                        ),
                        "Review high-risk scope, then resubmit with explicit approval marker.",
                    )
                    .to_json();
                }
            }
            vox_orchestrator::InterruptionDecision::DeferUntilCheckpoint { reason }
            | vox_orchestrator::InterruptionDecision::BatchWithExistingPrompt { reason } => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                state.record_attention_event(vox_orchestrator::AttentionEvent {
                    agent_id: state
                        .orchestrator
                        .agent_for_session_id(normalized_session_id.as_deref().unwrap_or_default())
                        .unwrap_or(AgentId(0)),
                    task_id: None,
                    event_type: vox_orchestrator::AttentionEventType::PolicyDeferred,
                    tier: vox_orchestrator::ApprovalTier::Confirm,
                    cost_ms: 0,
                    outcome: vox_orchestrator::ApprovalOutcome::AutoApproved,
                    trust_score_at_time: trust,
                    effective_complexity: (write_file_count as f64).clamp(0.0, 10.0),
                    decision_entropy_bits: signals.expected_information_gain_bits,
                    timestamp_ms: ts,
                    channel: Some("vox_submit_task".to_string()),
                    policy_reason: Some(reason),
                });
            }
            vox_orchestrator::InterruptionDecision::ProceedAutonomously { reason } => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                state.record_attention_event(vox_orchestrator::AttentionEvent {
                    agent_id: state
                        .orchestrator
                        .agent_for_session_id(normalized_session_id.as_deref().unwrap_or_default())
                        .unwrap_or(AgentId(0)),
                    task_id: None,
                    event_type: vox_orchestrator::AttentionEventType::PolicyProceedAuto,
                    tier: vox_orchestrator::ApprovalTier::AutoApprove,
                    cost_ms: 0,
                    outcome: vox_orchestrator::ApprovalOutcome::AutoApproved,
                    trust_score_at_time: trust,
                    effective_complexity: (write_file_count as f64).clamp(0.0, 10.0),
                    decision_entropy_bits: signals.expected_information_gain_bits,
                    timestamp_ms: ts,
                    channel: Some("vox_submit_task".to_string()),
                    policy_reason: Some(reason),
                });
            }
            vox_orchestrator::InterruptionDecision::InterruptNow { .. } => {}
        }
    }

    let orch = &state.orchestrator;

    let manifest: Vec<FileAffinity> = params
        .files
        .iter()
        .map(|f| match f.access {
            crate::params::FileAccess::Write => FileAffinity::write(&f.path),
            crate::params::FileAccess::Read => FileAffinity::read(&f.path),
        })
        .collect();

    let priority = params.priority.as_deref().map(|p| match p {
        "background" => TaskPriority::Background,
        "urgent" => TaskPriority::Urgent,
        _ => TaskPriority::Normal,
    });
    let planning_mode = params.planning_mode.as_deref().and_then(|m| match m {
        "auto" => Some(vox_orchestrator::PlanningMode::Auto),
        "direct" => Some(vox_orchestrator::PlanningMode::Direct),
        "force_plan" => Some(vox_orchestrator::PlanningMode::ForcePlan),
        "workflow_only" => Some(vox_orchestrator::PlanningMode::WorkflowOnly),
        _ => None,
    });

    // Prompt canonicalization: normalize and order-invariant pack to reduce order bias
    let (description, canonical_info) = match prompt_canonical::canonicalize_prompt(
        &params.description,
        true, // order_invariant
        true, // run_safety_pass: reject injection attempts and surface in Trust & Safety
    ) {
        Ok(c) => {
            tracing::debug!(
                "prompt_canonical: task description hash {} -> {} conflict warnings",
                c.original_hash,
                c.conflict_warnings.len()
            );
            let warnings = if c.conflict_warnings.is_empty() {
                None
            } else {
                Some(c.conflict_warnings)
            };
            (c.text, Some((true, warnings, Some(c.original_hash))))
        }
        Err(e) => {
            orch.event_bus().emit(AgentEventKind::InjectionDetected {
                detail: e.to_string(),
            });
            return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                format!("Prompt safety: {e}"),
                REM_PROMPT_SAFETY,
            )
            .to_json();
        }
    };

    let repo_id = state.repository.repository_id.as_str();
    let session_context_to_store: Option<vox_orchestrator::ContextEnvelope> =
        if let Some(sid) = normalized_session_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let base: Option<vox_orchestrator::ContextEnvelope> =
                if let Some((ref env, _)) = explicit_context_envelope {
                    Some(env.clone())
                } else if let Some(retrieval) = explicit_retrieval {
                    Some(retrieval.to_context_envelope(repo_id, Some(sid)))
                } else {
                    None
                };
            if let Some(mut base) = base {
                apply_mcp_trace_to_context_envelope(
                    &mut base,
                    params.trace_id.as_deref(),
                    params.correlation_id.as_deref(),
                );
                let ingest_expectations =
                    vox_orchestrator::context_lifecycle::ContextIngestExpectations {
                        repository_id: repo_id,
                        session_id: Some(sid),
                    };
                if let Err(e) = vox_orchestrator::context_lifecycle::apply_context_lifecycle_policy(
                    &state.orchestrator_config,
                    &base,
                    ingest_expectations,
                    vox_orchestrator::context_lifecycle::ContextIngestSource::McpSubmitTask,
                ) {
                    return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                        format!("context lifecycle policy rejected envelope: {e}"),
                        REM_CONTEXT_ENVELOPE_JSON,
                    )
                    .to_json();
                }
                let context_key = session_context_envelope_key(sid);
                let ctx_handle = orch.context_handle();
                let existing_json = match crate::sync_poison::poison_rw_read(
                    ctx_handle.read(),
                    "orchestrator context",
                ) {
                    Ok(g) => g.get(&context_key),
                    Err(e) => {
                        tracing::warn!(
                            error = %e,
                            "submit_task: could not read context store for merge; treating as empty"
                        );
                        None
                    }
                };
                let mut merged = match vox_orchestrator::context_lifecycle::merge_context_envelope_for_session_store(
                    existing_json.as_deref(),
                    &base,
                    state.orchestrator_config.context_lifecycle_shadow,
                ) {
                    Ok(m) => m,
                    Err(e) => {
                        return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                            format!("context envelope merge blocked: {e}"),
                            "Change conflict_policy.merge_strategy when updating session context, or clear the prior session envelope key.",
                        )
                        .to_json();
                    }
                };
                vox_orchestrator::context_lifecycle::clamp_context_envelope_injection_budget(&mut merged);
                if let Err(e) = vox_orchestrator::context_lifecycle::apply_context_lifecycle_policy(
                    &state.orchestrator_config,
                    &merged,
                    ingest_expectations,
                    vox_orchestrator::context_lifecycle::ContextIngestSource::SessionStoreWrite,
                ) {
                    return ToolResult::<SubmitTaskResponse>::err_with_remediation(
                        format!("context lifecycle policy rejected merged envelope: {e}"),
                        REM_CONTEXT_ENVELOPE_JSON,
                    )
                    .to_json();
                }
                Some(merged)
            } else {
                None
            }
        } else {
            None
        };

    let enqueue_hints = enqueue_hints_from_submit_params(&params);
    let submit_result: Result<TaskId, String> = if params.planning_mode.is_some() {
        orch.submit_goal(
            description.clone(),
            manifest,
            priority,
            planning_mode,
            normalized_session_id.clone(),
            enqueue_hints,
        )
        .await
        .map_err(|e| e.to_string())
    } else {
        state.submit_task_with_agent_backend(
            description.clone(),
            manifest,
            priority,
            params.agent_name.clone(),
            params.capabilities.clone(),
            enqueue_hints,
            normalized_session_id.clone(),
        )
        .await
    };
    match submit_result {
        Ok(task_id) => {
            if let Some(ref merged_env) = session_context_to_store {
                if let Some(env) =
                    vox_orchestrator::SessionRetrievalEnvelope::from_context_envelope(merged_env)
                {
                    let soc = env.to_task_context();
                    if let Err(e) = orch.attach_socrates_context(task_id, soc) {
                        tracing::warn!(
                            task_id = task_id.0,
                            error = %e,
                            "failed to attach Socrates context from merged session envelope"
                        );
                    }
                }
            } else if let Some(retrieval) = explicit_retrieval {
                let soc = socrates_context_from_retrieval(retrieval);
                if let Err(e) = orch.attach_socrates_context(task_id, soc) {
                    tracing::warn!(
                        task_id = task_id.0,
                        error = %e,
                        "failed to attach Socrates retrieval context to submitted task"
                    );
                }
            } else if let Some((context_envelope, _)) = &explicit_context_envelope
                && let Some(env) =
                    vox_orchestrator::SessionRetrievalEnvelope::from_context_envelope(
                        context_envelope,
                    )
            {
                let soc = env.to_task_context();
                if let Err(e) = orch.attach_socrates_context(task_id, soc) {
                    tracing::warn!(
                        task_id = task_id.0,
                        error = %e,
                        "failed to attach Socrates context from context_envelope_json"
                    );
                }
            }
            if let Some(ref merged) = session_context_to_store {
                if let Some(sid) = merged
                    .subject
                    .session_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                {
                    let context_key = session_context_envelope_key(sid);
                    if let Ok(raw) = serde_json::to_string(merged) {
                        let ctx_handle = state.orchestrator.context_handle();
                        match crate::sync_poison::poison_rw_write(
                            ctx_handle.write(),
                            "orchestrator context",
                        ) {
                            Ok(mut ctx) => {
                                ctx.set(vox_orchestrator::AgentId(0), &context_key, raw, 3600);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    task_id = task_id.0,
                                    "submit_task: context store poisoned while persisting context envelope"
                                );
                            }
                        }
                    }
                }
            }
            if let Some((_, Some(ref w), _)) = canonical_info {
                if !w.is_empty() {
                    orch.event_bus()
                        .emit(AgentEventKind::PromptConflictDetected {
                            task_id,
                            warnings: w.clone(),
                        });
                }
            }
            let shadow_plan_adequacy =
                if params.planning_mode.is_none() && state.orchestrator_config.plan_adequacy_shadow {
                    let pseudo = vec![crate::tools::chat_tools::params::PlanTask {
                        id: 1,
                        description: description.clone(),
                        files: params.files.iter().map(|f| f.path.clone()).collect(),
                        estimated_complexity: params.complexity.unwrap_or(5).clamp(1, 10),
                        depends_on: vec![],
                    }];
                    let router_hint = params.goal_type.as_deref().and_then(|g| {
                        match g.trim().to_ascii_lowercase().as_str() {
                            "research" | "investigation" | "explore" | "discovery" => Some(8u8),
                            "refactor" | "migration" | "modernize" => Some(6u8),
                            "testing" | "test" | "qa" => Some(5u8),
                            "docs" | "documentation" => Some(4u8),
                            _ => None,
                        }
                    });
                    let rep = crate::tools::chat_tools::analyze_plan_gaps(
                        &description,
                        params.files.len(),
                        router_hint,
                        None,
                        &pseudo,
                        None,
                    );
                    tracing::info!(
                        target: "vox_mcp::submit_plan_adequacy",
                        task_id = task_id.0,
                        adequacy_score = rep.adequacy.score,
                        is_too_thin = rep.adequacy.is_too_thin,
                        reason_codes = ?rep.adequacy.reason_codes,
                        critical_count = rep.critical_count,
                        aggregate_unresolved_risk = rep.aggregate_unresolved_risk,
                        "direct vox_submit_task: pseudo-plan adequacy shadow (use vox_plan when decomposition helps)",
                    );
                    Some(crate::params::SubmitShadowAdequacy {
                        score: rep.adequacy.score,
                        is_too_thin: rep.adequacy.is_too_thin,
                        reason_codes: rep.adequacy.reason_codes.clone(),
                        critical_count: rep.critical_count,
                        aggregate_unresolved_risk: rep.aggregate_unresolved_risk,
                    })
                } else {
                    None
                };
            let agent_id = orch
                .task_assignments_copy()
                .get(&task_id)
                .map(|a| a.0)
                .unwrap_or(0);
            let (prompt_canonicalized, conflict_warnings, original_prompt_hash) =
                canonical_info.unwrap_or((false, None, None));
            let v2 = state
                .orchestrator_config
                .orchestration_migration
                .orchestration_v2_enabled;
            ToolResult::ok(SubmitTaskResponse {
                task_id: task_id.0,
                agent_id,
                prompt_canonicalized: Some(prompt_canonicalized),
                conflict_warnings,
                original_prompt_hash,
                orchestration_contract: if v2 { Some("v2".to_string()) } else { None },
                shadow_plan_adequacy,
            })
            .to_json()
        }
        Err(e) => {
            let msg = format!("{e}");
            let remediation =
                if msg.contains("scope") || msg.contains("Scope") || msg.contains("outside") {
                    REM_TASK_SCOPE
                } else {
                    REM_TASK_SUBMIT
                };
            ToolResult::<SubmitTaskResponse>::err_with_remediation(msg, remediation).to_json()
        }
    }
}

/// Get the current status of a specific task.
pub async fn task_status(state: &ServerState, params: TaskStatusParams) -> String {
    if let Some(client) = state.orch_daemon_client_for_task_status_rpc() {
        match client.task_status(params.task_id).await {
            Ok(v) => {
                if let Some(label) = v.get("status").and_then(|s| s.as_str()) {
                    return ToolResult::ok(label.to_string()).to_json();
                }
                tracing::debug!(
                    ?v,
                    "task_status: orch.task_status RPC missing status field; falling back to embed"
                );
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "task_status: orch.task_status RPC failed; falling back to embed"
                );
            }
        }
    }

    let task_id = TaskId(params.task_id);
    match state.orchestrator.task_lifecycle_status_label(task_id) {
        Some(label) => ToolResult::ok(label).to_json(),
        None => ToolResult::<String>::err_with_remediation(
            format!("task {} not found", params.task_id),
            REM_TASK_ID,
        )
        .to_json(),
    }
}

/// Mark a task as completed, releasing its file locks (async).
pub async fn complete_task(state: &ServerState, params: CompleteTaskParams) -> String {
    let task_id = TaskId(params.task_id);
    let assigned = state.orchestrator.agent_assigned_to_task(task_id);
    let attestation = vox_orchestrator::CompletionAttestation {
        completion_summary: params.completion_summary,
        checks_passed: params.checks_passed,
        evidence_citations: params.evidence_citations,
        artifact_paths: params.artifact_paths.into_iter().map(Into::into).collect(),
        declared_non_placeholder: params.declared_non_placeholder,
        force_risky: params.force_risky,
        force_risky_reason: params.force_risky_reason,
    };
    let res = state
        .complete_task_with_attestation_backend(task_id, Some(attestation))
        .await;

    match res {
        Ok(()) => {
            // Gamification: update the agent-scoped companion (matches event_router / HUD).
            if let (Some(db), Some(aid)) = (&state.db, assigned) {
                let uid = vox_ludus::db::canonical_user_id();
                let id = format!("agent-{}", aid.0);
                let mut companion = match vox_ludus::db::list_companions(db, &uid).await {
                    Ok(comps) => comps
                        .into_iter()
                        .find(|c: &vox_ludus::companion::Companion| c.id == id),
                    Err(_) => None,
                }
                .unwrap_or_else(|| {
                    vox_ludus::companion::Companion::new(
                        &id,
                        &uid,
                        format!("Agent {}", aid.0),
                        "vox",
                    )
                });

                companion.interact(vox_ludus::companion::Interaction::TaskCompleted);
                let _ = vox_ludus::db::upsert_companion(db, &companion).await;
            }
            ToolResult::ok("task completed".to_string()).to_json()
        }
        Err(e) => ToolResult::<String>::err_with_remediation(e, REM_TASK_ORCH_OP).to_json(),
    }
}

/// Mark a task as failed with a reason (async).
pub async fn fail_task(state: &ServerState, params: FailTaskParams) -> String {
    let task_id = TaskId(params.task_id);
    let assigned = state.orchestrator.agent_assigned_to_task(task_id);
    let res = state.fail_task_backend(task_id, params.reason).await;

    match res {
        Ok(()) => {
            if let (Some(db), Some(aid)) = (&state.db, assigned) {
                let uid = vox_ludus::db::canonical_user_id();
                let id = format!("agent-{}", aid.0);
                let mut companion = match vox_ludus::db::list_companions(db, &uid).await {
                    Ok(comps) => comps
                        .into_iter()
                        .find(|c: &vox_ludus::companion::Companion| c.id == id),
                    Err(_) => None,
                }
                .unwrap_or_else(|| {
                    vox_ludus::companion::Companion::new(
                        &id,
                        &uid,
                        format!("Agent {}", aid.0),
                        "vox",
                    )
                });

                companion.interact(vox_ludus::companion::Interaction::TaskFailed);
                let _ = vox_ludus::db::upsert_companion(db, &companion).await;
            }
            ToolResult::ok("task marked as failed".to_string()).to_json()
        }
        Err(e) => ToolResult::<String>::err_with_remediation(e, REM_TASK_ORCH_OP).to_json(),
    }
}

/// Cancel a task by ID.
pub async fn cancel_task(state: &ServerState, params: crate::params::CancelTaskParams) -> String {
    match state.cancel_task_backend(TaskId(params.task_id)).await {
        Ok(()) => ToolResult::ok("Task cancelled successfully".to_string()).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e, REM_TASK_ORCH_OP).to_json(),
    }
}

/// Change the priority of a queued task.
pub async fn reorder_task(state: &ServerState, params: crate::params::ReorderTaskParams) -> String {
    let priority = match params.priority.as_str() {
        "urgent" => TaskPriority::Urgent,
        "background" => TaskPriority::Background,
        _ => TaskPriority::Normal,
    };

    match state.reorder_task_backend(TaskId(params.task_id), priority).await {
        Ok(()) => ToolResult::ok("Task reordered successfully".to_string()).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e, REM_TASK_ORCH_OP).to_json(),
    }
}

/// Remove all queued tasks from an agent without retiring it.
pub async fn drain_agent(state: &ServerState, params: DrainAgentParams) -> String {
    match state.drain_agent_backend(AgentId(params.agent_id)).await {
        Ok(n) => ToolResult::ok(format!("Agent drained {n} tasks")).to_json(),
        Err(e) => ToolResult::<String>::err_with_remediation(e, REM_TASK_ORCH_OP).to_json(),
    }
}

/// Publish a message to the bulletin board.
pub async fn publish_message(state: &ServerState, _params: PublishMessageParams) -> String {
    let orch = &state.orchestrator;
    let board = orch.bulletin();
    board.publish(vox_orchestrator::AgentMessage::DependencyReady { task_id: TaskId(0) });
    ToolResult::ok("message published".to_string()).to_json()
}
