use super::*;

/// Task 1.3d (F24 wiring): attempt the tool-calling agent loop
/// ([`super::super::agent_loop::run_agent_turn`]) for the default (non-`cognitive_profile`)
/// `vox_chat_message` path.
///
/// Returns:
/// - `None` when this turn should fall back to the existing `call_llm` pipeline
///   unchanged — either because `has_attachment` is `true` (the mapper does not
///   handle vision/attachment content) or because the resolved model's
///   [`vox_orchestrator::models::ProviderType`] isn't one of the simple shapes
///   [`super::super::agent_loop::model_spec_to_llm_config`] covers (OpenRouter, Ollama).
/// - `Some(Ok(..))` / `Some(Err(..))` when the agent loop actually ran.
///
/// Model *selection*: this calls
/// `crate::llm_bridge::resolve_mcp_chat_model_with_rationale` (the rationale-carrying
/// sibling of `resolve_mcp_chat_model`, which `call_llm` also uses internally via
/// `mcp_infer_completion` — both delegate to the same
/// `resolve_mcp_chat_model_sync_inner`) so this hot path's `selection_reason` reaches
/// the `vox_chat_message` envelope for `ModelBadge`'s tooltip. Everything else about
/// *after* a model is chosen is unchanged. `temperature`/`top_p` are
/// `params.temperature`/`params.top_p` straight from the request, applied to the
/// mapped `LlmConfig` exactly as the `call_llm` fallback applies them via
/// `temperature_override`/`top_p_override`.
#[allow(clippy::too_many_arguments)]
pub(super) async fn try_run_agent_turn(
    state: &ServerState,
    system_prompt: &str,
    prior_conversation: Vec<vox_actor_runtime::llm::LlmChatMessage>,
    user_prompt: &str,
    // Task 8f: the user's own message for this turn (post-`@mention`-expansion,
    // pre-context-assembly) — used ONLY for capability-requirement inference
    // (`McpChatModelResolution::capability_prompt`). `user_prompt` above still
    // carries the turn-local assembled prompt (files/retrieved context) for
    // the actual LLM call below, while `prior_conversation` carries structured
    // prior turns for KV-cache prefix reuse (`SOTA-PROMPT-01`).
    capability_prompt: &str,
    session_id: &str,
    active_skill_id: Option<String>,
    has_attachment: bool,
    temperature: Option<f32>,
    top_p: Option<f32>,
    request_model_override: Option<&str>,
    tier: Option<&str>,
    clutch: Option<&str>,
    risk: Option<&str>,
    web_evidence_supplied: bool,
) -> Option<Result<AgentTurnResult, String>> {
    if has_attachment {
        return None;
    }

    // Budget gate, before any model resolution work. This function (not
    // `resolve_chat_llm_model`) is the real entry point for the DEFAULT
    // `vox_chat_message` path: it resolves via `resolve_mcp_chat_model` directly and,
    // when the model maps to a simple provider shape, dispatches through
    // `run_agent_turn` -> `vox_actor_runtime::llm::llm_chat` ->
    // `vox_llm_egress::chat_once` — a completely separate HTTP-issuing mechanism from
    // `mcp_infer_tool_completion`'s `infer_via_provider_adapter`, with no shared
    // ancestor and no budget check of its own. See `chat_model_resolve`'s module docs
    // for the full map of why the guard lives in three places, not one.
    if let Err(e) = crate::chat_model_resolve::enforce_budget_guard(state, Some(session_id)).await {
        return Some(Err(e));
    }

    let global_pref = match crate::sync_poison::poison_rw_read(
        state.mcp_chat_model_override.read(),
        "mcp_chat_model_override",
    ) {
        Ok(g) => g.clone(),
        Err(e) => return Some(Err(e.to_string())),
    };
    let pref = effective_model_pref(request_model_override, global_pref.as_deref());
    let context_fill_ratio =
        crate::llm_bridge::mcp_global_llm_context_fill_ratio(&state.orchestrator).or_else(|| {
            let prior_chars: usize = prior_conversation.iter().map(|m| m.content.len()).sum();
            let total_chars = system_prompt
                .len()
                .saturating_add(prior_chars)
                .saturating_add(user_prompt.len());
            let approx_tokens = (total_chars / 4).max(1);
            Some((approx_tokens as f32 / 128_000.0).clamp(0.0, 1.0))
        });
    let (classified_spoke, primary_spoke_score, _, _, _, turn_mode, _) =
        vox_orchestrator::models::ModelRegistry::score_conversational_turn_across_spokes(
            capability_prompt,
            None,
        );
    let (spoke_task_category, spoke_complexity) =
        if turn_mode == vox_orchestrator::models::ConversationalTurnMode::SelfRepairLoop {
            (vox_orchestrator::types::TaskCategory::Debugging, 7)
        } else if primary_spoke_score > 0 {
            vox_orchestrator::models::ModelRegistry::organic_spoke_routing_target(classified_spoke)
        } else {
            (vox_orchestrator::types::TaskCategory::CodeGen, 5)
        };
    let resolution_template = resolution_for_tier(
        tier,
        McpChatModelResolution {
            task_category: spoke_task_category,
            complexity: spoke_complexity,
            free_tier_fill_in_middle: !prior_conversation.is_empty(),
            context_fill_ratio,
            clutch: clutch.and_then(vox_orchestrator::mode::ClutchProfile::from_label),
            risk: risk.and_then(vox_orchestrator::mode::RiskPosture::from_label),
            web_evidence_supplied,
            capability_prompt: Some(capability_prompt.to_string()),
            turn_decision_pack: crate::llm_bridge::TurnDecisionPack::from_env(),
            ..Default::default()
        },
    );
    let choice = match crate::llm_bridge::resolve_mcp_chat_model_with_rationale(
        state,
        user_prompt,
        pref.as_deref(),
        resolution_template,
        Some(session_id),
    )
    .await
    {
        Ok(c) => c,
        // Model resolution failing here is not this function's problem to report —
        // let the existing `call_llm` path attempt (and correctly surface) it. But
        // when the caller pinned a model, don't swallow the reason it couldn't be
        // honored entirely: log it so it's debug-visible instead of vanishing
        // silently into an unrelated auto-selected fallback.
        Err(e) => {
            if request_model_override.is_some() {
                tracing::warn!(
                    target: "vox_mcp::model_selection",
                    pref = ?pref,
                    error = %e,
                    "user-supplied model pref failed to resolve; falling back to call_llm pipeline"
                );
            }
            return None;
        }
    };
    let model = choice.model;
    let is_free = choice.is_free;
    let selection_reason = choice.rationale;
    // One `routing_decision` per assistant turn, first in `events`, built only from the resolved
    // spec, the composer's mode (if it sent one) and a reason that never echoes a requested id
    // (docs/superpowers/plans/2026-09-28-chat-turn-trace.md).
    let routing_event = super::super::turn_events::routing_decision_event(
        &model,
        clutch.and_then(vox_orchestrator::mode::ClutchProfile::from_label),
        &super::super::turn_events::routing_reason(
            selection_reason.as_deref(),
            SelectionSource::classify(
                request_model_override
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
                Some(model.id.as_str()),
                global_pref.as_deref(),
            ),
        ),
    );
    let pinned = choice.pinned;

    let mut llm_config = super::super::agent_loop::model_spec_to_llm_config(&model)?;
    // Thread sampling overrides through on the mapped path exactly as the
    // `call_llm` fallback already does (via `temperature_override`/`top_p_override`
    // -> `mcp_infer_completion`) — without this, a caller's temperature/top_p would
    // be silently dropped for every provider this task newly routes through
    // `run_agent_turn` (OpenRouter/Ollama).
    llm_config.temperature = temperature;
    llm_config.top_p = top_p;

    // `Box::pin`: `run_agent_turn` dispatches tool calls through
    // `handle_tool_call_with_mode`, which (for the `vox_chat_message` tool
    // specifically) can call back into `chat_message` -> `try_run_agent_turn` ->
    // `run_agent_turn` — a real mutual-recursion cycle the compiler must be able
    // to size, hence the heap indirection here rather than a plain `.await`.
    match Box::pin(super::super::agent_loop::run_agent_turn(
        state,
        Some(session_id),
        prior_conversation,
        system_prompt.to_string(),
        user_prompt.to_string(),
        None, // permission_mode: `ChatMessageParams` carries no transport-authenticated
        // permission mode; `handle_tool_call_with_mode`'s fail-safe default (Ask)
        // applies, matching every other MCP call path that doesn't have one.
        active_skill_id,
        llm_config,
        super::super::agent_loop::DEFAULT_MAX_ITERATIONS,
        // Task G1: stream tokens over the existing agent-events bus as they
        // arrive, carrying this turn's session_id so the GUI's
        // `resolveSessionForEvent` can route the frame straight to the
        // quick-chat bubble instead of only updating once the full reply is
        // back.
        true,
    ))
    .await
    {
        Ok(outcome) => {
            tracing::info!(
                target: "vox_mcp::agent_loop",
                tool_calls_made = outcome.tool_calls_made,
                hit_iteration_limit = outcome.hit_iteration_limit,
                model = %outcome.model_used,
                "vox_chat_message agent-loop turn completed"
            );
            let final_text = if outcome.hit_iteration_limit {
                tracing::warn!(
                    target: "vox_mcp::agent_loop",
                    tool_calls_made = outcome.tool_calls_made,
                    max_iterations = super::super::agent_loop::DEFAULT_MAX_ITERATIONS,
                    "vox_chat_message agent-loop turn hit the iteration bound before the \
                     model returned a final answer"
                );
                format!(
                    "{}\n\n[Note: this response was cut off after {} tool-use round-trips \
                     without reaching a final answer.]",
                    outcome.final_text, outcome.tool_calls_made
                )
            } else {
                outcome.final_text
            };

            // Task M1/M2: fire-and-forget the three-axis join (harness_eval_run +
            // harness_eval_task_result + model_selection_event) for this turn, tagged
            // `triggered_by = "live_chat"` so `list_harness_eval_runs` keeps excluding it
            // from real `vox harness eval --live` reporting.
            //
            // `completeness_ok` here is `!hit_iteration_limit && !has_unbalanced_fence` — the
            // two `completeness_ok` components knowable synchronously at turn-completion time.
            // `hit_iteration_limit` stands in for the plan's `finish_reason == 'length'` check
            // (no wire finish_reason is captured anywhere between the egress response and
            // `AgentTurnOutcome` today — a real gap, left for a follow-up rather than a blind
            // cross-crate wire-parsing change); a turn that was cut off before a final answer
            // is the same failure shape either way. The third component, "user re-asked within
            // 2 turns", genuinely can't be known yet — it's queued via
            // `queue_live_chat_reask_check` and resolved retroactively below, by the NEXT
            // turn's own write (each new turn rescores its session's still-pending prior
            // turns before recording itself).
            let resolved_cost_usd = compute_turn_cost_usd(
                &model,
                is_free,
                outcome.cost_usd,
                outcome.prompt_tokens,
                outcome.completion_tokens,
                outcome.cache_read_tokens,
            );
            if let Some(db) = state.db.clone() {
                let task_id = format!(
                    "{session_id}-{}",
                    vox_telemetry::current_trace_ctx().trace_id
                );
                let model_id = model.id.clone();
                let blended = vox_orchestrator::models::blended_cost_per_1k(&model);
                let cost_tier = vox_orchestrator::models::cost_tier_for_blended(is_free, blended)
                    .as_str()
                    .to_string();
                let selection_reason_for_event = selection_reason.clone().unwrap_or_default();
                let completeness_ok = !outcome.hit_iteration_limit
                    && !vox_orchestrator::grounding::has_unbalanced_fence(&final_text);
                let session_run_prefix = session_id.to_string();
                let user_prompt_owned = user_prompt.to_string();
                // `outcome.latency_ms` used to have no path to this call at all -- this
                // record_live_chat_turn call passed a hardcoded `None` for both cost_usd and
                // latency_ms since Task M1. cost_usd genuinely isn't computed anywhere on
                // this path yet (a separate gap); latency_ms is now real (Task M3).
                let turn_latency_ms = outcome.latency_ms.map(|v| v as i64);
                let turn_cost_for_db = resolved_cost_usd;
                tokio::spawn(async move {
                    // Rescore this session's still-pending prior turns against this turn's
                    // prompt first — cheap (at most 2 rows), and keeps each pending row's
                    // window tight to "the next 2 actual user turns" rather than drifting if a
                    // later write races ahead of an earlier one.
                    if let Ok(pending) =
                        db.pending_live_chat_reask_checks(&session_run_prefix).await
                    {
                        for check in pending {
                            let matched = vox_orchestrator::grounding::looks_like_reask(
                                &check.user_prompt,
                                &user_prompt_owned,
                            );
                            let _ = db
                                .resolve_live_chat_reask_check(&check.run_id, matched)
                                .await;
                        }
                    }

                    let now_ms = chrono::Utc::now().timestamp_millis();
                    let Ok(run_id) = db
                        .record_live_chat_turn(
                            &task_id,
                            &model_id,
                            &cost_tier,
                            &selection_reason_for_event,
                            turn_cost_for_db,
                            turn_latency_ms,
                            completeness_ok,
                            now_ms,
                        )
                        .await
                    else {
                        return;
                    };
                    if completeness_ok {
                        let _ = db
                            .queue_live_chat_reask_check(
                                &run_id,
                                &task_id,
                                &session_run_prefix,
                                &user_prompt_owned,
                                now_ms,
                            )
                            .await;
                    }
                });
            }

            let turn_tool_ledger = outcome
                .telemetry
                .get("turn_tool_ledger")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let classified_spoke = outcome
                .telemetry
                .get("classified_spoke")
                .and_then(|v| v.as_str())
                .map(str::to_string);

            Some(Ok(AgentTurnResult {
                text: final_text,
                model_used: outcome.model_used,
                tokens: outcome.total_tokens,
                prompt_tokens: outcome.prompt_tokens,
                completion_tokens: outcome.completion_tokens,
                cache_read_tokens: outcome.cache_read_tokens,
                cost_usd: resolved_cost_usd,
                ttft_ms: outcome.ttft_ms,
                tpot_ms: outcome.tpot_ms,
                classified_spoke,
                turn_tool_ledger,
                selection_reason,
                pinned,
                events: std::iter::once(routing_event)
                    .chain(outcome.events)
                    .collect(),
            }))
        }
        Err(e) => Some(Err(e)),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_cognitive_profile_fallback(
    state: &ServerState,
    params: &ChatMessageParams,
    profile: &str,
    base_temperature: f32,
    system_prompt: &str,
    user_prompt: &str,
    expanded_prompt: &str,
    session_id: &str,
    web_evidence_supplied: bool,
) -> Result<(String, String, u64, Option<String>, Vec<Value>), (crate::chat_hop::TurnOutcome, String)>
{
    let resolution_template = McpChatModelResolution {
        allow_cheapest_fallback: profile == "fast",
        complexity: match profile {
            "reasoning" => 9,
            "creative" => 7,
            _ => 5,
        },
        web_evidence_supplied,
        capability_prompt: Some(expanded_prompt.to_string()),
        ..Default::default()
    };
    let profile_complexity = resolution_template.complexity;
    match resolve_chat_llm_model(
        state,
        user_prompt,
        resolution_template.clone(),
        Some(session_id),
    )
    .await
    {
        Ok((model, free_only)) => {
            let pref = effective_model_pref(
                params.model_override.as_deref(),
                match crate::sync_poison::poison_rw_read(
                    state.mcp_chat_model_override.read(),
                    "mcp_chat_model_override",
                ) {
                    Ok(g) => g.clone(),
                    Err(e) => {
                        tracing::warn!(error = %e, "mcp_chat_model_override poisoned");
                        None
                    }
                }
                .as_deref(),
            );
            let max_tokens = crate::llm_bridge::clamp_http_max_output_tokens(model.max_tokens);
            let routing = McpInferRouting {
                user_prompt,
                sticky_model_pref: pref.as_deref(),
                resolution_template,
                free_only,
                allow_cloud_ollama_fallback: true,
                selection_rationale: None,
                user_id: Some(session_id),
            };
            match crate::llm_bridge::mcp_infer_completion(
                state,
                model,
                "vox_chat_message",
                system_prompt,
                &routing,
                max_tokens,
                base_temperature,
                params.temperature,
                params.top_p,
                params.json_mode,
                params.attachment_manifest.clone(),
            )
            .await
            {
                Ok(r) => Ok((
                    r.0,
                    r.1,
                    r.2,
                    Some(format!(
                        "cognitive profile '{profile}' \u{2192} complexity {profile_complexity}; rationale not surfaced"
                    )),
                    vec![],
                )),
                Err(e) => Err((crate::chat_hop::turn_outcome_for_dispatch_err(&e), e)),
            }
        }
        Err(e) => {
            tracing::warn!(
                target: "vox_mcp::cognitive_routing",
                profile,
                error = %e,
                "cognitive profile model resolution failed — using standard routing"
            );
            match call_llm(
                state,
                system_prompt,
                user_prompt,
                Some(session_id),
                params.temperature,
                params.top_p,
                params.attachment_manifest.clone(),
                Some(expanded_prompt),
            )
            .await
            {
                Ok(r) => Ok((
                    r.0,
                    r.1,
                    r.2,
                    Some(format!(
                        "Fallback: cognitive-profile resolution failed ({e})"
                    )),
                    vec![],
                )),
                Err(e2) => Err((crate::chat_hop::TurnOutcome::LlmError, e2)),
            }
        }
    }
}
