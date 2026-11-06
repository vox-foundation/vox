use serde_json::Value;

use super::super::params::{ChatMessageParams, ChatTranscriptEntry};
use super::super::{build_system_prompt_with_skill, now_ts, ts_to_date_str};
use super::conversation::{load_conversation_with_audit, trim_persisted_history};
use super::hydrate::context_history_or_hydrate;
use super::mentions::chat_grounding_score;
use crate::chat_model_resolve::resolve_chat_llm_model;
use crate::chat_socrates_meta::{
    LlmSurfaceTelemetry, clarification_turn_for_session, mcp_questioning_session_key,
    socrates_surface_tags, socrates_tool_meta, spawn_questioning_trace_from_socrates,
    spawn_socrates_telemetry_with_llm,
};
use crate::journey_envelope;
use crate::llm_bridge::{McpChatModelResolution, McpInferRouting, call_llm, call_llm_with_pref};
use crate::memory::{RetrievalTriggerMode, run_retrieval_bundle};
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::session_identity::normalize_chat_session_id;
use vox_actor_runtime::prompt_canonical;
use vox_orchestrator::session_context_envelope_key;

const REM_CHAT_CANONICAL: &str = "Rewrite the prompt to remove disallowed content / injection patterns; simplify objectives and retry.";
const REM_LLM_COMPLETION: &str = "Check inference logs, rate limits, and backend health; verify API keys via `vox secrets doctor`.";

mod persist_and_envelope;
mod posture_and_routing;
mod preamble_and_retrieval;
mod try_agent_turn;
use persist_and_envelope::persist_and_build_chat_envelope;
pub(crate) use posture_and_routing::*;
pub use posture_and_routing::{
    SelectionDto, SelectionSource, effective_model_pref, resolution_for_tier,
};
use preamble_and_retrieval::prepare_turn_context_and_retrieval;
use try_agent_turn::{execute_cognitive_profile_fallback, try_run_agent_turn};
/// Handle a user chat message. Resolves @mentions, injects context from the editor,
/// calls the best available LLM, persists to session history, and returns the updated history.
///
/// **Session Isolation**: History is keyed by `params.session_id` (defaulting to `"default"`).
/// Each unique session_id maintains a completely independent chat transcript in the
/// orchestrator `ContextStore`. Pass a stable UUID/slug per-window to prevent context bleeding.
///
/// **Autonomous Research**: Before invoking the LLM, this function silently queries the
/// `MemoryManager` and knowledge graph for facts related to the prompt. High-relevance hits
/// are injected as `[AUTONOMOUS RESEARCH]` preamble blocks so the model has evidence without
/// the user needing to explicitly invoke search tools.
///
/// **Cognitive Profile Routing**: Pass `"fast"`, `"reasoning"`, or `"creative"` to influence
/// model selection and temperature without changing the MCP tool contract.
pub async fn chat_message(state: &ServerState, mut params: ChatMessageParams) -> String {
    let hop_started = std::time::Instant::now();
    let hop_trace = params
        .trace_id
        .clone()
        .or_else(|| params.correlation_id.clone());
    let hop_turn = params.turn_id.clone();
    let emit_turn_hop =
        |session: Option<&str>, success: bool, outcome: crate::chat_hop::TurnOutcome| {
            crate::chat_hop::record_turn_boundary(
                state,
                hop_trace.clone(),
                hop_turn.clone(),
                session.map(str::to_string),
                success,
                hop_started.elapsed().as_millis() as u64,
                outcome,
            );
        };

    // Hydrate any omitted chat posture knobs from the persisted session posture so
    // real-time GUI/CLI knob adjustments govern all future turns in the session.
    let mut active_posture =
        load_persisted_session_chat_posture(state, params.session_id.as_deref()).await;
    let mut posture_updated = false;
    macro_rules! merge_posture_opt {
        ($field:ident) => {
            if params.$field.is_some() {
                if active_posture.$field != params.$field {
                    active_posture.$field = params.$field.clone();
                    posture_updated = true;
                }
            } else if active_posture.$field.is_some() {
                params.$field = active_posture.$field.clone();
            }
        };
    }
    merge_posture_opt!(clutch);
    merge_posture_opt!(risk);
    merge_posture_opt!(tier);
    merge_posture_opt!(mode);
    merge_posture_opt!(oversight_level);
    merge_posture_opt!(kb_scope);
    merge_posture_opt!(grounding_check_enabled);
    if posture_updated {
        persist_session_chat_posture(state, params.session_id.as_deref(), &active_posture).await;
    }

    // 1. Resolve @mentions and @rule:<name> tags in the prompt (`T3-SALVAGE-RULES-02`)
    let workspace_root = state
        .workspace_root
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let mut rule_catalog = super::mentions::load_workspace_user_rules(&workspace_root);
    if let Some(ref db) = state.db
        && let Ok(Some(raw_rules)) = db.get_user_preference("local_user", "gui.user_rules").await
        && let Ok(db_rules) =
            serde_json::from_str::<Vec<super::mentions::UserRuleEntry>>(&raw_rules)
    {
        for entry in db_rules {
            if let Some(existing) = rule_catalog.iter_mut().find(|r| r.name == entry.name) {
                *existing = entry;
            } else {
                rule_catalog.push(entry);
            }
        }
    }
    let (expanded_prompt, mention_files) = super::mentions::resolve_mentions_with_rules(
        &params.prompt,
        &workspace_root,
        &state.mention_path_cache,
        &rule_catalog,
    );
    // Research (spec §4) classifies and searches from THIS prompt — the user's own
    // words, NOT the @mention-expanded text (a mention inlines up to 8000 chars of
    // file content, whose words must not trip research cues; and dispatch's
    // `timeout_for_chat_message_args` classifies this same raw prompt, so the two
    // must agree or a Deep turn gets the ordinary timeout) — and before
    // `canonicalize_prompt` below wraps it in "Objectives
    // (treat as a single set; order does not imply priority):\n\n1. ..."
    // boilerplate for the LLM's own framing. That boilerplate is not the user's
    // question: fed into a search engine verbatim it returns garbage (observed
    // live: an astrophysics arXiv dump for a Gemini-model question), and it
    // swallows a leading `/deepresearch`/`/research` slash command so
    // `classify_research_intent`'s `strip_command` never matches, forcing every
    // command onto the heuristic-cue fallback instead of the explicit path.
    let raw_prompt_for_research = params.prompt.clone();
    // Research (spec §4): classify from the RAW (pre-canonicalization) prompt —
    // see the doc comment above — and do it *before* the autonomous-retrieval
    // preamble below, so the preamble knows whether this turn's numbered
    // research sources (quick/deep) will already supply web evidence (Task 8d).
    let intent = super::research_intent::classify_research_intent(
        &raw_prompt_for_research,
        params.force_research,
        params.research_scope.as_deref(),
    );
    let mut research_trace = super::research_turn::ResearchTrace::new(intent.clone());
    let (expanded_prompt, canonical_meta) = match prompt_canonical::canonicalize_prompt(
        &expanded_prompt,
        true, // order_invariant
        true, // run_safety_pass
    ) {
        Ok(c) => {
            let hash = c.original_hash;
            let conflict_count = c.conflict_warnings.len();
            let objective_count = c.objectives.len();
            (c.text, Some((hash, conflict_count, objective_count)))
        }
        Err(e) => {
            emit_turn_hop(None, false, crate::chat_hop::TurnOutcome::Unknown);
            return ToolResult::<String>::err_with_remediation(
                format!("Prompt rejected by safety canonicalizer: {e}"),
                REM_CHAT_CANONICAL,
            )
            .to_json();
        }
    };
    let mention_count = mention_files.len();
    if let Some((original_hash, conflict_count, objective_count)) = canonical_meta {
        tracing::debug!(
            target: "vox_mcp::prompt_canonical",
            original_hash = %original_hash,
            conflict_warning_count = conflict_count,
            objective_count = objective_count,
            "chat prompt canonicalized"
        );
    }

    // Resolve session id early (before the rest of the function needs it) so we can
    // load this session's prior conversation turns and thread them into the request —
    // see Task 1.1 / Finding F25: history was persisted and returned for display but
    // never actually sent to the model.
    let (session_id, implicit_session_default) =
        normalize_chat_session_id(params.session_id.as_deref());
    let (prior_conversation, compaction_audit) =
        load_conversation_with_audit(state, session_id.as_str()).await;

    // 2a. Build context preamble from editor state.
    // Phase 143 (SOTA-PROMPT-01): Keep turn-local `context_parts` separate from
    // `prior_conversation` so the structured multi-turn `run_agent_turn` path
    // sends `[system, ...prior_conversation, turn_user_prompt]` with a stable
    // KV-cache prefix, while the legacy single-string `call_llm` fallback still
    // receives `[CONVERSATION HISTORY]` in `user_prompt`.
    let prepared = prepare_turn_context_and_retrieval(
        state,
        &params,
        session_id.as_str(),
        expanded_prompt,
        mention_files,
        mention_count,
        &intent,
        &mut research_trace,
    )
    .await;
    let expanded_prompt = prepared.expanded_prompt;
    let turn_user_prompt = prepared.turn_user_prompt;
    let explicit_search_result = prepared.explicit_search_result;
    let deep_answer = prepared.deep_answer;
    let retrieval_evidence = prepared.retrieval_evidence;
    let preamble_skipped_reason = prepared.preamble_skipped_reason;
    let all_context_files = prepared.all_context_files;
    let user_prompt = if prior_conversation.is_empty() {
        turn_user_prompt.clone()
    } else {
        let transcript = prior_conversation
            .iter()
            .map(|m| format!("{}: {}", m.role, m.content))
            .collect::<Vec<_>>()
            .join("\n");
        format!("[CONVERSATION HISTORY]:\n{transcript}\n\n{turn_user_prompt}")
    };

    // 3. Call LLM with cognitive-profile aware routing.
    // When cognitive_profile is set we use mcp_infer_completion() with an explicit
    // resolution template — the same pattern already used by inline_edit() and ghost_text().
    let thread_id_for_envelope = params.thread_id.clone();
    let journey_id = params
        .journey_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            use vox_orchestrator::id_allocator::VoxIdAllocatorV2;
            format!("journey_{}", state.id_allocator.allocate())
        });
    if implicit_session_default {
        tracing::debug!(
            target: "vox_mcp::session",
            tool = "vox_chat_message",
            "session_id omitted; using default chat session bucket"
        );
    }
    // F3: resolve sticky model override to pass as model_key for profile injection.
    let sticky_model_key: Option<String> = effective_model_pref(
        params.model_override.as_deref(),
        match crate::sync_poison::poison_rw_read(
            state.mcp_chat_model_override.read(),
            "mcp_chat_model_override",
        ) {
            Ok(g) => g.clone(),
            Err(_) => None,
        }
        .as_deref(),
    );
    // Pre-flight Battery Unit balance gate (`T3-HIST-BATTERY-04` & `T3-STREAM-QUEUE-03`):
    {
        use vox_orchestrator::budget::tracker::PreflightBudgetDecision;
        let check_model = if params.tier.as_deref() == Some("local")
            || params.clutch.as_deref() == Some("free")
        {
            "local/mens"
        } else {
            sticky_model_key.as_deref().unwrap_or("auto")
        };
        let decision = state
            .battery_ledger
            .lock()
            .check_preflight_balance(check_model, 0.001);
        if let PreflightBudgetDecision::InsufficientBalance {
            remaining_bu,
            required_bu,
        } = decision
        {
            if sticky_model_key.is_none() {
                // Auto-routing with exhausted BU: gracefully steer to zero-cost local tier instead of bricking the session.
                params.tier = Some("local".to_string());
                params.clutch = Some("free".to_string());
            } else {
                emit_turn_hop(
                    Some(session_id.as_str()),
                    false,
                    crate::chat_hop::TurnOutcome::Unknown,
                );
                return ToolResult::<String>::err_with_remediation(
                    format!(
                        "Insufficient Battery Units for model '{check_model}': {remaining_bu} BU remaining, {required_bu} BU required."
                    ),
                    "Switch to a zero-cost local/MENS/Ollama model (`tier: \"local\"` or `clutch: \"free\"`) or replenish workspace budget.",
                )
                .to_json();
            }
        }
    }
    let raw_system_prompt = build_system_prompt_with_skill(
        state,
        None,
        params.skill.as_deref(),
        sticky_model_key.as_deref(),
        &params.skill_exclusions,
    )
    .await;
    let system_prompt = {
        let mut sys_vars = super::system_prompt_vars::build_live_runtime_vars(
            &state.repository.root,
            &ts_to_date_str(now_ts()),
            params.active_file.as_deref(),
        );
        sys_vars.session_id = session_id.clone();
        if let Some(ref mk) = sticky_model_key {
            sys_vars.model_id = mk.clone();
        }
        if let Some(prof) = params.cognitive_profile.as_deref() {
            sys_vars.cognitive_profile = prof.to_string();
        }
        super::system_prompt_vars::interpolate_system_prompt_variables(
            &raw_system_prompt,
            &sys_vars,
        )
    };
    let llm_started = std::time::Instant::now();

    // The strict pin the chat model resolved through (Task 13) — set only by
    // the agent-loop path, the one that carries the resolver's `McpModelChoice`.
    let mut chat_pin: Option<String> = None;
    let mut turn_prompt_tokens: u64 = 0;
    let mut turn_completion_tokens: u64 = 0;
    let mut turn_cache_read_tokens: u64 = 0;
    let mut turn_cost_usd: Option<f64> = None;
    let mut turn_ttft_ms: Option<u64> = None;
    let mut turn_tpot_ms: Option<f64> = None;
    let mut turn_classified_spoke: Option<String> = None;
    let mut turn_tool_ledger: Vec<String> = Vec::new();
    let (response_text, model_used, tokens, selection_reason, events) = if let Some(deep) =
        deep_answer
    {
        match deep {
            Ok(answer) => (
                answer,
                research_trace.model.clone().unwrap_or_default(),
                0u64,
                Some("deep research pipeline synthesis".to_string()),
                vec![],
            ),
            Err(e) => (
                // Status message, not an answer: the trace shows the failing stage and the sources.
                format!(
                    "Deep research failed: {e}\n\nNo answer was generated. The research trace below shows which stage failed and what was retrieved."
                ),
                "none".to_string(),
                0u64,
                Some("deep research failed".to_string()),
                vec![],
            ),
        }
    } else if let Some(local_res) = explicit_search_result {
        (
            local_res,
            "local/knowledgebase-fts5".to_string(),
            0u64,
            Some("Zero-cost local knowledgebase search".to_string()),
            vec![],
        )
    } else {
        match params.cognitive_profile.as_deref() {
            Some(profile) => {
                let base_temperature = if profile == "creative" {
                    0.8_f32
                } else {
                    0.3_f32
                };
                let effective_temp = params.temperature.or(Some(base_temperature));
                // Phase 142 (SOTA-STREAM-04): Route cognitive_profile through the
                // tool-calling agent loop first when no multimodal attachment or
                // strict json_mode is required, falling back to `mcp_infer_completion`
                // only when unmapped or when json_mode/attachments are requested.
                let profile_clutch = params.clutch.as_deref().or(match profile {
                    "reasoning" => Some("genius"),
                    "fast" => Some("fast"),
                    _ => None,
                });
                let agent_loop_attempt = if !params.json_mode {
                    try_run_agent_turn(
                        state,
                        &system_prompt,
                        prior_conversation.clone(),
                        &turn_user_prompt,
                        &expanded_prompt,
                        session_id.as_str(),
                        params.skill.clone(),
                        params.attachment_manifest.is_some(),
                        effective_temp,
                        params.top_p,
                        params.model_override.as_deref(),
                        params.tier.as_deref(),
                        profile_clutch,
                        params.risk.as_deref(),
                        !research_trace.sources.is_empty(),
                    )
                    .await
                } else {
                    None
                };
                if let Some(Ok(r)) = agent_loop_attempt {
                    chat_pin = r.pinned;
                    turn_prompt_tokens = r.prompt_tokens;
                    turn_completion_tokens = r.completion_tokens;
                    turn_cache_read_tokens = r.cache_read_tokens;
                    turn_cost_usd = r.cost_usd;
                    turn_ttft_ms = r.ttft_ms;
                    turn_tpot_ms = r.tpot_ms;
                    turn_classified_spoke = r.classified_spoke;
                    turn_tool_ledger = r.turn_tool_ledger;
                    (
                        r.text,
                        r.model_used,
                        r.tokens,
                        r.selection_reason.or_else(|| {
                            Some(format!(
                                "cognitive profile '{profile}' routed via SOTA tool loop"
                            ))
                        }),
                        r.events,
                    )
                } else {
                    match execute_cognitive_profile_fallback(
                        state,
                        &params,
                        profile,
                        base_temperature,
                        &system_prompt,
                        &user_prompt,
                        &expanded_prompt,
                        session_id.as_str(),
                        !research_trace.sources.is_empty(),
                    )
                    .await
                    {
                        Ok(tuple) => tuple,
                        Err((outcome, err_msg)) => {
                            emit_turn_hop(Some(session_id.as_str()), false, outcome);
                            return ToolResult::<String>::err_with_remediation(
                                format!("LLM error: {err_msg}"),
                                REM_LLM_COMPLETION,
                            )
                            .to_json();
                        }
                    }
                }
            }
            // Default (no cognitive_profile) chat path: Task 1.3d (F24 wiring). Attempt
            // the tool-calling agent loop (Task 1.3c) whenever the resolved model maps
            // to a simple provider shape and no multimodal attachment is present (the
            // mapper does not handle vision/attachment content — see
            // `super::agent_loop::model_spec_to_llm_config`). Otherwise fall back to
            // the existing `call_llm` -> `mcp_infer_completion` pipeline unchanged,
            // which still handles every other provider plus vision/budget/fallback.
            None => match try_run_agent_turn(
                state,
                &system_prompt,
                prior_conversation.clone(),
                &turn_user_prompt,
                // Task 8f: capability requirements come from the user's own
                // message (`expanded_prompt`), not the assembled `user_prompt`
                // above (which carries `[CONVERSATION HISTORY]`/`[OPEN FILES]`/
                // retrieved and web-research context).
                &expanded_prompt,
                session_id.as_str(),
                params.skill.clone(),
                params.attachment_manifest.is_some(),
                params.temperature,
                params.top_p,
                params.model_override.as_deref(),
                params.tier.as_deref(),
                params.clutch.as_deref(),
                params.risk.as_deref(),
                // Quick research already ran web retrieval and injected numbered
                // sources into `user_prompt` — the model doesn't need its own
                // built-in web search capability for this turn (D-fix: see
                // `McpChatModelResolution::web_evidence_supplied`).
                !research_trace.sources.is_empty(),
            )
            .await
            {
                Some(Ok(r)) => {
                    chat_pin = r.pinned;
                    turn_prompt_tokens = r.prompt_tokens;
                    turn_completion_tokens = r.completion_tokens;
                    turn_cache_read_tokens = r.cache_read_tokens;
                    turn_cost_usd = r.cost_usd;
                    turn_ttft_ms = r.ttft_ms;
                    turn_tpot_ms = r.tpot_ms;
                    turn_classified_spoke = r.classified_spoke;
                    turn_tool_ledger = r.turn_tool_ledger;
                    (r.text, r.model_used, r.tokens, r.selection_reason, r.events)
                }
                Some(Err(e)) => {
                    if e.contains("429") || e.to_ascii_lowercase().contains("rate limit") {
                        state.thread_run_queue.lock().set_limit_recovery(
                            session_id.clone(),
                            vox_orchestrator::thread_run_queue::LimitRecoveryState {
                                provider_id: sticky_model_key
                                    .clone()
                                    .unwrap_or_else(|| "auto".to_string()),
                                message: e.clone(),
                                retry_at_ms: (now_ts() + 60) * 1000,
                            },
                        );
                    }
                    emit_turn_hop(
                        Some(session_id.as_str()),
                        false,
                        crate::chat_hop::turn_outcome_for_dispatch_err(&e),
                    );
                    return ToolResult::<String>::err_with_remediation(
                        format!("LLM error: {e}"),
                        REM_LLM_COMPLETION,
                    )
                    .to_json();
                }
                // Bug 2 fix: `try_run_agent_turn` already resolved (or tried to
                // resolve) this turn's model using `params.model_override` /
                // `params.tier`. Falling through to plain `call_llm` here used to
                // silently discard both and re-resolve from only the
                // process-global override with `allow_cheapest_fallback: true`
                // unconditionally and no free-tier enforcement -- a different
                // model could be picked with no error surfaced, and a `tier:
                // "local"` request could silently escape free-tier enforcement.
                // `call_llm_with_pref` threads the same per-request pref/tier
                // through so this fallback resolves consistently with the path
                // that just failed, instead of picking an unrelated model.
                None => match call_llm_with_pref(
                    state,
                    &system_prompt,
                    &user_prompt,
                    Some(session_id.as_str()),
                    params.temperature,
                    params.top_p,
                    params.attachment_manifest.clone(),
                    params.model_override.as_deref(),
                    params.tier.as_deref(),
                    // Task 8f: this is the attachment/non-agent-loop fallback
                    // (`try_run_agent_turn` returned `None`) — it must infer
                    // capability requirements from the user's own message too,
                    // not from `user_prompt` (the assembled prompt with
                    // history/open-files/retrieved/web context).
                    Some(&expanded_prompt),
                )
                .await
                {
                    // `call_llm_with_pref` resolves via the rationale-carrying resolver
                    // internally but doesn't return the rationale through its
                    // `(String, String, u64)` return type — out of scope for this task
                    // (see `try_run_agent_turn`'s doc comment). Record a truthful reason
                    // for the missing detail rather than a silent `None`.
                    Ok(r) => (
                        r.0,
                        r.1,
                        r.2,
                        Some(
                            "Fallback: attachment present, or provider shape unmapped".to_string(),
                        ),
                        vec![],
                    ),
                    Err(e) => {
                        if e.contains("429") || e.to_ascii_lowercase().contains("rate limit") {
                            state.thread_run_queue.lock().set_limit_recovery(
                                session_id.clone(),
                                vox_orchestrator::thread_run_queue::LimitRecoveryState {
                                    provider_id: sticky_model_key
                                        .clone()
                                        .unwrap_or_else(|| "auto".to_string()),
                                    message: e.clone(),
                                    retry_at_ms: (now_ts() + 60) * 1000,
                                },
                            );
                        }
                        emit_turn_hop(
                            Some(session_id.as_str()),
                            false,
                            crate::chat_hop::turn_outcome_for_dispatch_err(&e),
                        );
                        return ToolResult::<String>::err_with_remediation(
                            format!("LLM error: {e}"),
                            REM_LLM_COMPLETION,
                        )
                        .to_json();
                    }
                },
            },
        }
    };

    // TRUST-03: model output is never written back into the knowledge base from a chat turn.

    let chat_q_key =
        mcp_questioning_session_key(state, "vox_chat_message", Some(session_id.as_str()));
    state.record_questioning_attention_spend(&chat_q_key, llm_started.elapsed().as_millis() as u64);

    // Debit the pilot AttentionBudget for this completed chat turn. This mirrors
    // what the now-deleted `ChatTaskProcessor::process` (commit bd2ade59ae) used to
    // do via `Orchestrator::record_chat_attention` after every successful chat
    // turn — real chat turns moved to this synchronous `chat_message` path, but the
    // debit call was never carried over, so the GUI's `AttentionBudgetMeter`
    // (driven purely by `AttentionBudget.spent_ms`, see `record_chat_attention`'s
    // doc comment on `Orchestrator`) silently stopped moving for normal chat
    // activity. `state.record_questioning_attention_spend` above debits a
    // different, unrelated structure (`ServerState`'s in-memory
    // `questioning_attention_bounds` map) and does not substitute for this.
    // Token counts are estimated from the actual prompt/response text via
    // `CompactionEngine::estimate_tokens`, matching `ChatTaskProcessor`'s original
    // input/output token estimation (it also estimated from `task.description`/
    // `reply_text` rather than using provider-reported usage numbers).
    let est_input_tokens =
        vox_orchestrator::compaction::CompactionEngine::estimate_tokens(&user_prompt) as u32;
    let est_output_tokens =
        vox_orchestrator::compaction::CompactionEngine::estimate_tokens(&response_text) as u32;
    state
        .orchestrator
        .record_chat_attention(est_input_tokens, est_output_tokens);

    tracing::info!(
        target: "vox_mcp::populi_kpi",
        tool = "vox_chat_message",
        model_id = %model_used,
        tokens,
        elapsed_ms = llm_started.elapsed().as_millis() as u64,
        cognitive_profile = params.cognitive_profile.as_deref().unwrap_or("standard"),
        "mcp chat LLM round-trip"
    );

    let result = persist_and_build_chat_envelope(
        state,
        &params,
        session_id.clone(),
        journey_id,
        thread_id_for_envelope,
        user_prompt,
        response_text.clone(),
        model_used,
        tokens,
        selection_reason,
        events,
        chat_pin,
        turn_prompt_tokens,
        turn_completion_tokens,
        turn_cache_read_tokens,
        turn_cost_usd,
        turn_ttft_ms,
        turn_tpot_ms,
        turn_classified_spoke,
        turn_tool_ledger,
        all_context_files,
        retrieval_evidence,
        preamble_skipped_reason,
        compaction_audit,
        mention_count,
        est_input_tokens,
        llm_started,
        &intent,
        research_trace,
    )
    .await;
    let turn_outcome = if response_text.contains("cut off after")
        && response_text.contains("tool-use round-trips")
    {
        crate::chat_hop::TurnOutcome::IterationLimit
    } else {
        crate::chat_hop::TurnOutcome::Ok
    };
    emit_turn_hop(
        Some(session_id.as_str()),
        matches!(turn_outcome, crate::chat_hop::TurnOutcome::Ok),
        turn_outcome,
    );

    ToolResult::ok(result).to_json()
}

#[cfg(test)]
#[allow(
    unsafe_code,
    reason = "test-only std::env::set_var (unsafe on edition 2024); serialized via #[serial]"
)]
mod tests;

#[cfg(test)]
#[path = "message_turn_trace_tests.rs"]
mod turn_trace_tests;

#[cfg(test)]
#[path = "message_kb_leak_tests.rs"]
mod kb_leak_tests;
