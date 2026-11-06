use super::*;

pub(crate) fn apply_repair_escalation_to_config(
    state: &ServerState,
    llm_config_template: &LlmConfig,
    config: &mut LlmConfig,
    telemetry: &mut std::collections::HashMap<String, serde_json::Value>,
) {
    if config.temperature.is_none() {
        config.temperature = Some(0.1);
    }
    config.telemetry_strength_tag = Some("sota_repair_cascade".to_string());
    let escalated_pick = {
        let models_handle = state.orchestrator.models_handle();
        crate::sync_poison::poison_rw_read(models_handle.read(), "models registry")
            .ok()
            .and_then(|reg| {
                let mut repair_intent = vox_orchestrator::models::SelectionIntent::repair_loop();
                repair_intent.prefer_local = true;
                repair_intent.allow_free_in_performance_mode = true;
                let req =
                    vox_orchestrator::models::ModelSelectionRequest::from_intent(repair_intent);
                vox_orchestrator::models::decide(&req, &reg).map(|d| d.outcome.model_spec)
            })
    };
    if let Some(spec) = escalated_pick {
        telemetry.insert(
            "escalated_repair_model_id".to_string(),
            serde_json::Value::String(spec.id.clone()),
        );
        if llm_config_template.base_url.is_none()
            && let Some(escalated_cfg) = model_spec_to_llm_config(&spec)
        {
            config.provider = escalated_cfg.provider;
            config.model = escalated_cfg.model;
            config.max_tokens = escalated_cfg.max_tokens;
        }
    }
}

/// Apply post-tool confidence escalation (`DEC-02`, `DEC-03`).
///
/// - When `pack` is `None`: returns `false` and leaves `config` and `telemetry` untouched.
/// - When `pack` is `Some(p)`:
///   - Rejects `PostToolDirective::DraftPr` if `!shortlist_has_pr_tool` and records
///     `"post_tool_rejected_directive": "draft_pr"` in `telemetry`.
///   - When `p.should_reenter_repair_escalation()` (`model_still_ok_prob < confidence_threshold`
///     or `PostToolDirective::ReenterRepairLoop`), re-enters [`apply_repair_escalation_to_config`]
///     (`SelectionIntent::repair_loop()` with `prefer_local = true` and
///     `allow_free_in_performance_mode = true`) and records `"post_tool_low_confidence_escalated": true`.
pub fn apply_post_tool_confidence_escalation(
    state: &ServerState,
    llm_config_template: &LlmConfig,
    config: &mut LlmConfig,
    telemetry: &mut std::collections::HashMap<String, serde_json::Value>,
    pack: Option<&super::lifecycle_hooks::PostToolDecisionPack>,
    shortlist_has_pr_tool: bool,
) -> bool {
    let Some(pack) = pack else {
        return false;
    };
    if pack.directive == Some(super::lifecycle_hooks::PostToolDirective::DraftPr)
        && !shortlist_has_pr_tool
    {
        telemetry.insert(
            "post_tool_rejected_directive".to_string(),
            serde_json::Value::String("draft_pr".to_string()),
        );
    } else if let Some(d) = pack.effective_directive(shortlist_has_pr_tool) {
        telemetry.insert(
            "post_tool_effective_directive".to_string(),
            serde_json::json!(d),
        );
    }
    if pack.should_reenter_repair_escalation() {
        apply_repair_escalation_to_config(state, llm_config_template, config, telemetry);
        telemetry.insert(
            "post_tool_low_confidence_escalated".to_string(),
            serde_json::Value::Bool(true),
        );
        telemetry.insert(
            "post_tool_model_still_ok_prob".to_string(),
            serde_json::json!(pack.model_still_ok_prob),
        );
        return true;
    }
    false
}

pub(super) fn is_read_only_memoizable(name: &str) -> bool {
    !matches!(
        name,
        "vox_spawn_agent" | "vox_submit_task" | "vox_resolve_approval"
    ) && !crate::scope_guard::WRITE_TOOLS.contains(&name)
        && TOOL_REGISTRY
            .iter()
            .find(|e| e.name == name)
            .is_some_and(|e| e.http_read_role_eligible)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn dispatch_tool_calls_batch(
    state: &ServerState,
    session_id: Option<&str>,
    permission_mode: Option<&str>,
    calls: &[vox_actor_runtime::llm::EgressToolCall],
    read_tool_memo: &std::collections::HashMap<(String, String), (usize, String, String)>,
    parallel_tool_batches_executed: &mut usize,
    telemetry: &mut std::collections::HashMap<String, serde_json::Value>,
) -> Vec<(
    Result<String, anyhow::Error>,
    Option<crate::dispatch::ToolReceiptOutcome>,
)> {
    let all_read_only_batch =
        calls.len() >= 2 && calls.iter().all(|c| is_read_only_memoizable(&c.name));
    if all_read_only_batch {
        *parallel_tool_batches_executed += 1;
        telemetry.insert(
            "parallel_tool_batches_executed".to_string(),
            serde_json::json!(*parallel_tool_batches_executed),
        );
        let memo_snapshot = read_tool_memo.clone();
        let futs = calls.iter().map(|call| {
            let key = (call.name.clone(), canonicalize_tool_args_json(&call.arguments));
            let memo_hit = memo_snapshot.get(&key).cloned();
            let mut dispatch_args = call.arguments.clone();
            let hook_decision = evaluate_pre_tool_use_hook(&call.name, &dispatch_args);
            async move {
                let hook_note = match hook_decision {
                    HookDecision::Block { reason } => {
                        let blocked = serde_json::json!({"success": false, "error": reason, "hook_event": "PreToolUse"}).to_string();
                        return (Ok(blocked), None);
                    }
                    HookDecision::Allow { updated_input, additional_context } => {
                        if let Some(u) = updated_input { dispatch_args = u; }
                        additional_context
                    }
                };
                if let Some((first_iter, short_hash, preview)) = memo_hit {
                    let dedup_json = serde_json::json!({
                        "success": true,
                        "data": {"deduped_read_hit": true, "tool": call.name, "first_iteration": first_iter, "blake3": short_hash, "preview": preview}
                    }).to_string();
                    (Ok(dedup_json), None)
                } else {
                    let (mut res, receipt) = crate::floor_gate::with_agent_origin(
                        crate::dispatch::handle_tool_call_with_receipt(state, &call.name, dispatch_args, permission_mode),
                    ).await;
                    if let (Ok(body), Some(note)) = (&mut res, hook_note) {
                        body.push('\n');
                        body.push_str(&note);
                    }
                    (res, receipt)
                }
            }
        });
        return futures_util::future::join_all(futs).await;
    }

    let mut seq = Vec::with_capacity(calls.len());
    let mut hook_advisories: Vec<String> = Vec::new();
    for call in calls {
        let memo_key = (
            call.name.clone(),
            canonicalize_tool_args_json(&call.arguments),
        );
        let mut dispatch_args = call.arguments.clone();
        let mut hook_note: Option<String> = None;
        match evaluate_pre_tool_use_hook(&call.name, &dispatch_args) {
            HookDecision::Block { reason } => {
                let blocked = serde_json::json!({"success": false, "error": reason, "hook_event": "PreToolUse"}).to_string();
                seq.push((Ok(blocked), None));
                continue;
            }
            HookDecision::Allow {
                updated_input,
                additional_context,
            } => {
                if let Some(u) = updated_input {
                    dispatch_args = u;
                }
                if let Some(note) = additional_context {
                    hook_advisories.push(note.clone());
                    hook_note = Some(note);
                }
            }
        }
        if is_read_only_memoizable(&call.name)
            && let Some((first_iter, short_hash, preview)) = read_tool_memo.get(&memo_key).cloned()
        {
            let dedup_json = serde_json::json!({
                "success": true,
                "data": {"deduped_read_hit": true, "tool": call.name, "first_iteration": first_iter, "blake3": short_hash, "preview": preview}
            }).to_string();
            seq.push((Ok(dedup_json), None));
            continue;
        }
        // Milestone v4.9 (`URBS-QUARANTINE-02`): Hard pre-execution write guardrail for quarantined districts.
        if crate::scope_guard::WRITE_TOOLS.contains(&call.name.as_str())
            || matches!(
                call.name.as_str(),
                "vox_write_file"
                    | "vox_edit_file"
                    | "vox_apply_structured_edit"
                    | "write_file"
                    | "edit_file"
                    | "delete_file"
            )
        {
            let target_path = dispatch_args
                .get("path")
                .or_else(|| dispatch_args.get("file_path"))
                .or_else(|| dispatch_args.get("target_file"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(matched_prefix) = state.is_path_quarantined(target_path) {
                let guardrail_err = format!(
                    "[QUARANTINE GUARDRAIL: Path '{target_path}' is locked read-only by developer Clausura Ferrea (prefix '{matched_prefix}')]"
                );
                let blocked = serde_json::json!({
                    "success": false,
                    "error": guardrail_err,
                    "quarantine_blocked": true,
                    "quarantined_prefix": matched_prefix,
                })
                .to_string();
                telemetry.insert(
                    "quarantine_guardrail_blocks".to_string(),
                    serde_json::json!(1),
                );
                seq.push((Ok(blocked), None));
                continue;
            }
        }
        // T3 Wave 4/5 (`T3-LOOP-CHECKPOINT-01` & `T3-WORKTREE-CKPT-02`): Sparse pre-mutation Git plumbing shadow checkpoint
        // with monotonic per-session turn counter (`turn/1`, `turn/2`, `turn/3`, ...).
        if vox_orchestrator::agentos::checkpoint_engine::should_sparse_checkpoint(&call.name) {
            let ckpt_thread = session_id.filter(|s| !s.is_empty()).unwrap_or("default");
            let ckpt_turn = {
                let mut counters = state.checkpoint_turn_counters.lock();
                let entry = counters.entry(ckpt_thread.to_string()).or_insert(0);
                *entry += 1;
                *entry
            };
            let repo_root = state
                .workspace_root
                .as_deref()
                .unwrap_or(&state.repository.root);
            if let Ok(rec) = vox_orchestrator::agentos::checkpoint_engine::GitPlumbingCheckpointStore::capture_checkpoint(
                repo_root,
                ckpt_thread,
                ckpt_turn,
            ) {
                telemetry.insert(
                    "last_sparse_checkpoint_ref".to_string(),
                    serde_json::Value::String(rec.ref_name),
                );
            }
        }
        let bg_work_id = if matches!(call.name.as_str(), "vox_spawn_agent" | "vox_submit_task")
            && let Some(session) = session_id.filter(|s| !s.is_empty())
        {
            if dispatch_args.is_null() {
                dispatch_args = serde_json::Value::Object(serde_json::Map::new());
            }
            let awaited = dispatch_args
                .get("awaited")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            if let Some(obj) = dispatch_args.as_object_mut() {
                obj.insert(
                    "chat_session_id".to_string(),
                    serde_json::Value::String(session.to_string()),
                );
                obj.insert(
                    "origin_turn_id".to_string(),
                    serde_json::Value::String(call.id.clone()),
                );
            }
            let wid = format!("{}:{}", call.name, call.id);
            state.thread_run_queue.lock().register_background_work(
                session.to_string(),
                wid.clone(),
                awaited,
            );
            Some((session.to_string(), wid))
        } else {
            None
        };
        let (mut res, receipt) =
            crate::floor_gate::with_agent_origin(crate::dispatch::handle_tool_call_with_receipt(
                state,
                &call.name,
                dispatch_args,
                permission_mode,
            ))
            .await;
        if let Some((ref bg_session, ref wid)) = bg_work_id {
            state
                .thread_run_queue
                .lock()
                .complete_background_work(bg_session, wid);
        }
        if let (Ok(body), Some(note)) = (&mut res, hook_note) {
            body.push('\n');
            body.push_str(&note);
        }
        seq.push((res, receipt));
    }
    if !hook_advisories.is_empty() {
        telemetry.insert(
            "pre_tool_use_hook_advisories".to_string(),
            serde_json::json!(hook_advisories),
        );
    }
    if let Some(batch_note) = evaluate_post_tool_batch_hook(calls) {
        if let Some((Ok(last_body), _)) = seq.last_mut() {
            last_body.push('\n');
            last_body.push_str(&batch_note);
        }
        telemetry.insert(
            "post_tool_batch_hook_note".to_string(),
            serde_json::Value::String(batch_note),
        );
    }
    seq
}

pub(super) fn record_harness_issue_if_crossed(
    state: &ServerState,
    session_id: Option<&str>,
    harness_detection_enabled: bool,
    harness_scorer: &mut super::super::harness_issue_scorer::HarnessIssueScorer,
    call: &vox_actor_runtime::llm::EgressToolCall,
    content: &str,
) {
    if harness_detection_enabled {
        let is_error = content.starts_with("Error:")
            || crate::server_state::tool_json_envelope_is_error(content);
        // Redact before entering the scorer's activity buffer (sent to judge LLM and stored in evidence_json).
        let redacted_args = vox_redact::redact_args(&call.arguments).to_string();
        let redacted_content = if is_error {
            vox_redact::redact_owned(content)
        } else {
            String::new()
        };
        let crossed = harness_scorer.record(&call.name, &redacted_args, &redacted_content);
        if crossed {
            let recent_activity = harness_scorer.recent_activity();
            let db = state.db.clone();
            let session_key = session_id.map(str::to_string);
            // Resolve a concrete judge model ID rather than literal "auto".
            let judge_model = vox_orchestrator::models::select_with_default_registry(
                &vox_orchestrator::models::SelectionIntent::review(),
            )
            .map(|o| o.model_id)
            .unwrap_or_else(|| "google/gemini-3.1-pro".to_string());
            tokio::spawn(async move {
                let Some(issue) =
                    super::super::harness_issue_judge::judge(&recent_activity, &judge_model).await
                else {
                    return;
                };
                let Some(db) = db else {
                    return;
                };
                // Dedup against any still-pending issue for this session/category.
                if let Some(session_key) = session_key.as_deref() {
                    match db
                        .has_pending_harness_issue_for_session(session_key, &issue.category)
                        .await
                    {
                        Ok(true) => return,
                        Ok(false) => {}
                        Err(e) => {
                            tracing::warn!(
                                target: "harness_issue_judge",
                                error = %e,
                                "failed to check for a pending duplicate harness issue"
                            );
                        }
                    }
                }
                let insert_result = db
                    .insert_harness_issue(vox_db::NewHarnessIssue {
                        source: "chat_session",
                        session_key: session_key.as_deref(),
                        target_path: None,
                        detected_at_ms: chrono::Utc::now().timestamp_millis(),
                        category: &issue.category,
                        severity: &issue.severity,
                        summary: &issue.summary,
                        evidence_json: &serde_json::json!({
                            "excerpt": recent_activity
                        })
                        .to_string(),
                    })
                    .await;
                if let Err(e) = insert_result {
                    // Partial unique index on (session_key, category) enforces dedup across concurrent judge tasks.
                    if e.to_string().to_ascii_lowercase().contains("unique") {
                        tracing::debug!(
                            target: "harness_issue_judge",
                            "duplicate harness issue insert raced with another judge task; dropped"
                        );
                    } else {
                        tracing::warn!(
                            target: "harness_issue_judge",
                            error = %e,
                            "failed to insert detected harness issue"
                        );
                    }
                }
            });
            harness_scorer.reset();
        }
    }
}

/// Hermetic `vox harness eval` (`agent-loop-terminates`) check (`crates/vox-cli/src/commands/harness/eval.rs`).
#[cfg(feature = "eval-gate")]
pub async fn eval_gate_agent_loop_terminates_check() -> Result<(), String> {
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use vox_orchestrator::{
        AffinityGroupRegistry, Orchestrator, OrchestratorConfig, SessionConfig, SessionManager,
    };
    use vox_repository::{RepoCapabilities, RepositoryContext};
    use vox_skills::new_registry_arc;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    let tool_call_body = serde_json::json!({
        "id": "chatcmpl-eval-gate",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{"id": "call_1", "type": "function", "function": {"name": "vox_git_status", "arguments": "{}"}}],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    });
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_body))
        .mount(&server)
        .await;

    let cfg = OrchestratorConfig::for_testing();
    let orch_cfg = cfg.clone();
    let groups = AffinityGroupRegistry::new(vec![]);
    let session_cfg = SessionConfig {
        persist: false,
        sessions_dir: std::env::temp_dir().join("vox-harness-eval-agent-loop-sessions"),
        ..SessionConfig::default()
    };
    let session_manager =
        SessionManager::new(session_cfg).map_err(|e| format!("session manager: {e}"))?;
    let repository = RepositoryContext {
        root: PathBuf::from("."),
        git_root: None,
        repository_id: "harness-eval-agent-loop".into(),
        origin_url: None,
        capabilities: RepoCapabilities {
            vox_project: false,
            cargo_workspace: false,
            cargo_package: false,
            node_workspace: false,
            python_project: false,
            go_module: false,
            git: false,
        },
        has_vox_agents_dir: false,
        vox_toml: None,
    };
    let state = ServerState::hermetic_stub(
        cfg,
        repository,
        Arc::new(Orchestrator::with_groups(orch_cfg, groups)),
        Arc::new(Mutex::new(session_manager)),
        new_registry_arc(),
    );

    let mut llm_config = LlmConfig::openrouter("test-model");
    llm_config.base_url = Some(format!("{}/chat/completions", server.uri()));
    llm_config.api_key = Some("test-key".into());
    llm_config.telemetry_skip_interaction = true;

    let max_iterations = 3;
    let outcome = run_agent_turn(
        &state,
        Some("eval-gate"),
        vec![],
        "system prompt".to_string(),
        "eval-gate: loop forever please".to_string(),
        None,
        None,
        llm_config,
        max_iterations,
        false,
    )
    .await?;

    if !outcome.hit_iteration_limit {
        return Err("expected the loop to hit its iteration cap and stop".to_string());
    }
    if outcome.tool_calls_made != max_iterations {
        return Err(format!(
            "expected {max_iterations} tool calls (one per iteration), got {}",
            outcome.tool_calls_made
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn handle_repair_escalation_or_harvest_dpo(
    state: &ServerState,
    session_id: Option<&str>,
    classified_spoke: vox_corpus::training::contract::OrganicSpokeKind,
    user_message: &str,
    call: &vox_actor_runtime::llm::EgressToolCall,
    content: &mut String,
    is_error_or_verify_fail: bool,
    has_inline_verification_failure: bool,
    repair_repeated: bool,
    failed_tool_calls: &mut std::collections::HashMap<String, String>,
    synchronous_repair_interventions: &mut usize,
    repair_escalated: &mut bool,
    auto_dpo_pairs_harvested: &mut usize,
    telemetry: &mut std::collections::HashMap<String, serde_json::Value>,
) {
    let has_paper_code_warning = content.contains("[PAPER CODE WARNING:");
    if has_paper_code_warning {
        telemetry.insert(
            "unresolved_paper_code_warnings".to_string(),
            serde_json::json!(1),
        );
    }
    if is_error_or_verify_fail || has_paper_code_warning {
        if let Some(window) = build_edit_failure_recovery_window(
            state.workspace_root.as_deref(),
            &call.name,
            &call.arguments,
        ) {
            content.push_str(&window);
            telemetry.insert(
                "edit_window_recovery_injected".to_string(),
                serde_json::Value::Bool(true),
            );
        }
        let prior_failure_for_tool = failed_tool_calls
            .insert(call.name.clone(), call.arguments.to_string())
            .is_some();
        if has_inline_verification_failure || prior_failure_for_tool || repair_repeated {
            *synchronous_repair_interventions += 1;
            *repair_escalated = true;
            telemetry.insert(
                "repair_loop_escalated".to_string(),
                serde_json::Value::Bool(true),
            );
            telemetry.insert(
                "synchronous_repair_interventions".to_string(),
                serde_json::json!(*synchronous_repair_interventions),
            );
            content.push_str(
                "\n[HARNESS REPAIR INTERVENTION: Inline verification or repeated tool failure detected. Do NOT repeat the identical arguments. Fix the exact compiler/verification diagnostics above before finishing the turn.]",
            );
        }
    } else {
        telemetry.remove("unresolved_paper_code_warnings");
        if let Some(rejected_args) = failed_tool_calls.remove(&call.name) {
            let chosen_str = call.arguments.to_string();
            let (dpo_quality_metrics, dpo_quality_gate_passed) =
                evaluate_turn_quality_and_gate_dpo(user_message, &chosen_str, Some(&rejected_args));
            telemetry.insert(
                "dpo_chosen_quality_score".to_string(),
                serde_json::json!(dpo_quality_metrics.quality_score),
            );
            telemetry.insert(
                "dpo_quality_gate_passed".to_string(),
                serde_json::Value::Bool(dpo_quality_gate_passed),
            );
            let mut dpo_record =
                vox_corpus::training::contract::TrainingRecordV2::new_for_organic_spoke(
                    classified_spoke,
                    user_message,
                    format!("tool_repair:{}", call.name),
                    chosen_str,
                    vox_corpus::training::contract::OracleReceipt {
                        spoke: classified_spoke.to_core_spoke(),
                        passed: true,
                        stage: "live_harness_auto_repair".to_string(),
                        diagnostics: Vec::new(),
                        exit_code: Some(0),
                    },
                    vox_corpus::training::contract::RecordProvenance {
                        origin: "live_harness_auto_repair".to_string(),
                        source_id: session_id.unwrap_or("anonymous_turn").to_string(),
                        task_family: format!("{}:{}", classified_spoke.as_str(), call.name),
                        difficulty: 7,
                    },
                );
            dpo_record.rejected_response = Some(rejected_args);
            *auto_dpo_pairs_harvested += 1;
            telemetry.insert(
                "auto_dpo_pairs_harvested".to_string(),
                serde_json::json!(*auto_dpo_pairs_harvested),
            );
            telemetry.insert(
                "last_auto_dpo_spoke".to_string(),
                serde_json::Value::String(
                    dpo_record.effective_organic_spoke().as_str().to_string(),
                ),
            );
            if let Some(ref db) = state.db
                && let Ok(jsonl_line) = serde_json::to_string(&dpo_record)
                && vox_db::ops_agent_telemetry::insert_event(
                    db,
                    "0",
                    "live_harness_auto_repair_dpo",
                    Some(&jsonl_line),
                )
                .await
                .is_ok()
            {
                telemetry.insert(
                    "auto_dpo_persisted_to_db".to_string(),
                    serde_json::Value::Bool(true),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mutating_and_unknown_tools_are_not_memoizable() {
        for name in [
            "vox_spawn_agent",
            "vox_submit_task",
            "vox_resolve_approval",
            "vox_apply_structured_edit",
            "vox_generate_code",
            "definitely_not_a_registered_tool",
        ] {
            assert!(
                !is_read_only_memoizable(name),
                "{name} must not be memoized"
            );
        }
    }

    #[test]
    fn read_eligible_registry_tools_are_memoizable() {
        let entry = TOOL_REGISTRY
            .iter()
            .find(|e| {
                e.http_read_role_eligible
                    && !crate::scope_guard::WRITE_TOOLS.contains(&e.name)
                    && !matches!(
                        e.name,
                        "vox_spawn_agent" | "vox_submit_task" | "vox_resolve_approval"
                    )
            })
            .expect("registry has at least one read-eligible tool");
        assert!(is_read_only_memoizable(entry.name));
    }
}
