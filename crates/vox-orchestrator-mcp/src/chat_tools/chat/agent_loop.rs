//! Task 1.3c (harness implementation spec §3.3): the real multi-turn, tool-calling
//! agent loop — the piece that finally wires together Tasks 1.1 (conversation
//! history), 1.2 (tool selection), 1.3a (tool_calls parsed from the wire response),
//! and 1.3b (tool_calls/tool_call_id carried on request messages).
//!
//! Finding F24 ("no code path in Vox ever passes tools to a model"): before this
//! module, every chat completion built its request with `tools: None`. This module
//! is the first that (a) selects a bounded tool subset for the turn
//! ([`crate::llm_bridge::tool_selection::select_tools_for_turn`]), (b) sends it to
//! the model via [`vox_actor_runtime::llm::llm_chat`] (the only call path that
//! carries `tools`/`tool_calls` end to end — see Task 1.3a/1.3b), (c) dispatches any
//! requested calls through [`crate::dispatch::handle_tool_call_with_mode`], and (d)
//! feeds the results back to the model as `role: "tool"` messages, looping until the
//! model stops requesting tools or a hard iteration bound is hit.
//!
//! Deliberately out of scope here (see Task 1.3c notes in the harness implementation
//! spec): per-turn skill-pin plumbing for `active_skill_id` (the caller can pass one
//! in if it has it cheaply at hand; this module does not go hunting for it), and any
//! new permission-mode taxonomy or dispatch mechanism — `permission_mode` is passed
//! straight through to `handle_tool_call_with_mode` exactly as that function already
//! requires (an authenticated transport-layer value, never LLM-composed).

use vox_actor_runtime::ActivityOptions;
use vox_actor_runtime::llm::{
    LlmChatMessage, LlmConfig, LlmResponse, LlmToolDef, llm_chat, llm_stream_events_activity,
};
use vox_mcp_registry::TOOL_REGISTRY;
use vox_orchestrator::events::AgentEventKind;
use vox_orchestrator::models::{ModelSpec, ProviderType};
use vox_orchestrator::types::AgentId;

use crate::input_schemas::tool_input_schema;
use crate::llm_bridge::tool_selection::{DEFAULT_RANKED_MAX_TOOLS, TurnContext};
use crate::server_state::ServerState;

/// Serializes test-only access to the process-global `OPENROUTER_BASE_URL` /
/// `OPENROUTER_API_KEY` env vars. Any `#[cfg(test)]` module in this crate that
/// mutates those vars (to point them at a `wiremock` server) must lock this —
/// a per-file lock is not enough, since `cargo test` runs test binaries'
/// tests concurrently across modules and a private per-module lock does not
/// serialize against a sibling module's private lock, letting two tests
/// stomp each other's env var value mid-request.
#[cfg(test)]
pub(crate) static CHAT_MESSAGE_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub mod ast_slice_and_preflight;
mod batch_and_eval_gate;
mod context_and_memo;
pub mod lifecycle_hooks;
mod salvage_and_stream;
pub mod tool_refusal_detector;
mod turn_events;
pub mod zero_paper_and_ratchet;

pub use ast_slice_and_preflight::{
    FROZEN_PREFIX_CORE_TOOLS, PrefixLockedToolSelection, PreflightWriteReport, SymbolSlice,
    extract_symbol_slice, fold_code_to_ast_skeleton, preflight_check_write_guards,
    select_prefix_locked_tools_for_turn,
};
pub(crate) use context_and_memo::*;
pub use lifecycle_hooks::{
    HookDecision, HookEvent, OracleConsultationAdvisory, POST_TOOL_DECISION_PACK_SCHEMA_V1,
    PostToolDecisionPack, PostToolDirective, canonicalize_workspace_command,
    consult_oracle_model_advisory, consult_oracle_model_advisory_with_pack,
    evaluate_post_tool_batch_hook, evaluate_post_tool_batch_with_decision_pack,
    evaluate_pre_tool_use_hook, evaluate_stop_hook, evaluate_stop_hook_with_decision_pack,
    resolve_mens_fast_apply_target, resolve_mens_fast_apply_target_with_pack,
};
pub use salvage_and_stream::try_parse_streaming_read_prefetch;
pub(crate) use salvage_and_stream::*;
pub use turn_events::AgentTurnOutcome;
pub(crate) use turn_events::{
    DEFAULT_MAX_ITERATIONS, model_spec_to_llm_config, receipt_turn_event, turn_event_for_result,
};
pub use zero_paper_and_ratchet::{
    TestRatchetReport, ZeroPaperAuditReport, audit_zero_paper_and_laziness,
    detect_hollow_and_stub_bodies, detect_magic_values_and_special_casing,
    detect_paper_code_symbols, extract_top_level_symbol_catalog, split_production_and_test_source,
    verify_test_ratchet_integrity,
};

pub use batch_and_eval_gate::apply_post_tool_confidence_escalation;
pub(crate) use batch_and_eval_gate::apply_repair_escalation_to_config;
#[cfg(feature = "eval-gate")]
pub use batch_and_eval_gate::eval_gate_agent_loop_terminates_check;
use batch_and_eval_gate::{
    dispatch_tool_calls_batch, handle_repair_escalation_or_harvest_dpo, is_read_only_memoizable,
    record_harness_issue_if_crossed,
};
/// Run one user turn of the tool-calling agent loop to completion.
///
/// `messages` passed to the first `llm_chat` call is `[system] + prior_conversation
/// + [user]`; callers build `prior_conversation` via
/// [`super::conversation::load_conversation`] and the system prompt via
/// [`crate::chat_tools::build_system_prompt_with_skill`] — this function does not
///   duplicate either concern, it only takes the already-assembled strings/history.
///
/// Loops: select tools for the turn, call `llm_chat` with them, and if the response
/// carries `tool_calls`, dispatch each one and append the assistant tool-call
/// message plus one `role: "tool"` result message per call, then loop. Stops as
/// soon as a response has no tool_calls (returns its `content` as the final
/// answer), or after `max_iterations` model round-trips (whichever comes first —
/// see [`DEFAULT_MAX_ITERATIONS`] for why this bound must be real and finite).
///
/// `stream_tokens`: Task G1 (chat-harness-unification plan, Phase G). When
/// `true`, each iteration first tries [`llm_stream_activity`] instead of
/// [`llm_chat`], emitting `AgentEventKind::TokenStreamed { session_id, .. }`
/// on `state.orchestrator.event_bus()` — the SAME bus `orch_daemon` already
/// forwards to the GUI's `vox://agent-events` listener for background-task
/// streaming — as each content chunk arrives, so the GUI's quick-chat bubble
/// fills in progressively instead of only updating once the whole reply is
/// back. `vox_llm_egress::wire::stream_once` only parses `delta.content`, not
/// `delta.tool_calls`, so a turn where the model requests a tool call instead
/// of answering directly streams zero chunks; that (or any other stream
/// error) falls back to one ordinary non-streaming `llm_chat` call so tool
/// dispatch is never affected — see [`stream_final_answer`]'s doc comment.
/// `false` (every pre-existing caller) reproduces the exact prior behavior:
/// one plain `llm_chat` call per iteration, no bus emissions.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_agent_turn(
    state: &ServerState,
    session_id: Option<&str>,
    prior_conversation: Vec<LlmChatMessage>,
    system_prompt: String,
    user_message: String,
    permission_mode: Option<&str>,
    active_skill_id: Option<String>,
    llm_config_template: LlmConfig,
    max_iterations: usize,
    stream_tokens: bool,
) -> Result<AgentTurnOutcome, String> {
    let mut harness_scorer = super::harness_issue_scorer::HarnessIssueScorer::new();
    // Read once per turn, not per tool call — the flag cannot change mid-turn
    // (no code path mutates it during a single `run_agent_turn` invocation).
    let harness_detection_enabled = {
        let cfg_handle = state.orchestrator.config_handle();
        crate::sync_poison::poison_rw_read(cfg_handle.read(), "orchestrator config")
            .map(|cfg| cfg.harness_issue_detection_enabled)
            .unwrap_or(false)
    };
    let mut messages: Vec<LlmChatMessage> = Vec::with_capacity(prior_conversation.len() + 2);
    messages.push(LlmChatMessage {
        role: "system".into(),
        content: system_prompt,
        ..Default::default()
    });
    messages.extend(prior_conversation);
    messages.push(LlmChatMessage {
        role: "user".into(),
        content: vox_orchestrator::context_sanitizer::escape_user_prompt_special_tokens(
            &user_message,
        ),
        ..Default::default()
    });

    let mut turn_ctx = TurnContext {
        permission_mode: permission_mode.map(str::to_string),
        lanes: vec!["ai", "app"],
        active_skill_id,
        max_tools: DEFAULT_RANKED_MAX_TOOLS,
        // Never offer chat-turn-entry tools (`vox_chat_*`, e.g. `vox_chat_message`)
        // as callable tools inside an agent turn. `select_tools_for_turn`'s
        // general-purpose skill-permission filter
        // (`skill_permissions::is_skill_infrastructure_tool`) deliberately
        // whitelists `vox_chat_*` through *skill* restrictions — that's correct for
        // other, non-recursive callers, so this exclusion is supplied here at the
        // `run_agent_turn` call site rather than being hardcoded in
        // `tool_selection.rs`. Without it, a model could dispatch
        // `vox_chat_message` via `handle_tool_call_with_mode`, which re-enters
        // `chat_message -> try_run_agent_turn -> run_agent_turn` — each re-entrant
        // call gets its own fresh `max_iterations` budget, so
        // `DEFAULT_MAX_ITERATIONS` alone would not bound the recursion depth.
        //
        // This is applied INSIDE `select_tools_for_turn`, before the `max_tools`
        // cap, so a `vox_chat_*` entry sitting within the first `DEFAULT_MAX_TOOLS`
        // registry-order entries no longer consumes a cap slot only to be
        // discarded afterward (which used to leave turns with fewer than
        // `DEFAULT_MAX_TOOLS` usable tools and could push a genuinely useful tool
        // just past the cap boundary out of reach).
        exclude_name_prefixes: vec!["vox_chat_"],
        // Registry order is alphabetical, so the plain `.take(max_tools)`
        // truncation made the delegation tools unreachable on every chat turn:
        // after the ai/app lane filter `vox_spawn_agent` is candidate ~166 of
        // ~188 and the cut lands at 40. Pin them so a chat turn can actually
        // delegate.
        //
        // This is not free: the cap is fixed, so each pin evicts the tool that
        // previously held the last slot. These three pins displaced
        // `vox_check_file_owner`, `vox_check_mood`, and `vox_check_workspace`
        // (previously slots 38-40, the cut ending at `vox_check_workspace`).
        // Every further pin costs another tool the same way — the real fix is
        // relevance ranking before truncation, not a longer pin list.
        pin_names: vec![
            "vox_spawn_agent",
            "vox_submit_task",
            "vox_task_status",
            "vox_answer_question",
        ],
    };
    let raw_ranking_utterance = extract_raw_utterance_for_tool_ranking(&user_message);
    // CAT-01/CAT-02 (flag `VOX_TURN_DECISION_PACKS`, default off ⇒ unchanged): compose the
    // existing skill/MCP/CLI catalogs into a top-K shortlist and let a calibrated choice hoist
    // an MCP tool into `pin_names`. Skill activation is deliberately NOT applied here: it would
    // install an allowlist that narrows the whole turn's tool surface. Fails open below threshold.
    let decision_packs_on = crate::chat_tools::skill_catalog::decision_packs_enabled();
    let mut chatarc_telemetry: Option<(bool, String)> = None;
    if decision_packs_on {
        use crate::chat_tools::skill_catalog as sc;
        let skill_entries: Vec<sc::CatalogEntry> = state
            .skill_registry
            .list(None)
            .iter()
            .map(|m| sc::CatalogEntry {
                name: m.name.clone(),
                description: m.description.clone(),
                reliability: None,
            })
            .collect();
        let shortlist = sc::build_chatarc_shortlist(
            raw_ranking_utterance,
            &skill_entries,
            TOOL_REGISTRY,
            &vox_capability_registry::bundled_index().rows,
            sc::MAX_CHATARC_SHORTLIST_K,
        );
        let choice = sc::lexical_dominance_choice(&shortlist, sc::CHATARC_DEFAULT_THRESHOLD);
        let mut probe = turn_ctx.clone();
        let outcome = sc::apply_chatarc_choice_to_turn(&shortlist, Some(&choice), &mut probe);
        turn_ctx.pin_names = probe.pin_names;
        chatarc_telemetry = Some((shortlist.has_pr_tool(), format!("{outcome:?}")));
    }
    let prior_spoke_hint = messages[1..messages.len().saturating_sub(1)]
        .iter()
        .rev()
        .find(|m| m.role == "user" && !m.content.trim().is_empty())
        .and_then(|m| {
            let (spoke, score, _, _, _, _, _) =
                vox_orchestrator::models::ModelRegistry::score_conversational_turn_across_spokes(
                    &m.content, None,
                );
            (score >= 3).then_some(spoke)
        });
    let prefix_locked = select_prefix_locked_tools_for_turn(
        TOOL_REGISTRY,
        &state.skill_registry,
        &turn_ctx,
        raw_ranking_utterance,
        prior_spoke_hint,
    );
    let classified_spoke = prefix_locked.classified_spoke;
    let tool_defs: Vec<LlmToolDef> = prefix_locked
        .selected
        .iter()
        .enumerate()
        .map(|(rank, entry)| compact_tool_def_for_rank(rank, entry, tool_input_schema(entry.name)))
        .collect();

    let activity_options = ActivityOptions::new();
    let mut model_used = String::new();
    let mut tool_calls_made = 0usize;
    let mut total_tokens = 0u64;
    let mut total_prompt_tokens = 0u64;
    let mut total_completion_tokens = 0u64;
    let mut total_cache_read_tokens = 0u64;
    let mut total_cost_usd: Option<f64> = None;
    let mut events: Vec<serde_json::Value> = Vec::new();
    let mut latency_ms: Option<u64> = None;
    let mut ttft_ms: Option<u64> = None;
    let mut tpot_ms: Option<f64> = None;
    // Task B2 salvage policy + Milestone v2.7..v3.4 SOTA harness telemetry.
    let mut telemetry: std::collections::HashMap<String, serde_json::Value> =
        std::collections::HashMap::new();
    telemetry.insert(
        "classified_spoke".to_string(),
        serde_json::Value::String(classified_spoke.as_str().to_string()),
    );
    telemetry.insert(
        "frozen_prefix_tools_count".to_string(),
        serde_json::json!(prefix_locked.frozen_prefix_count),
    );
    if let Some((has_pr, outcome)) = chatarc_telemetry {
        telemetry.insert(
            "chatarc_shortlist_has_pr_tool".to_string(),
            serde_json::Value::Bool(has_pr),
        );
        telemetry.insert(
            "chatarc_outcome".to_string(),
            serde_json::Value::String(outcome),
        );
    }
    if let Some(hint) = prefix_locked.deferred_catalog_hint {
        if let Some(last_msg) = messages.last_mut()
            && last_msg.role == "user"
            && !last_msg
                .content
                .contains("[DEFERRED MCP TOOLS AVAILABLE ON DEMAND:")
        {
            last_msg.content.push_str("\n\n");
            last_msg.content.push_str(&hint);
        }
        telemetry.insert(
            "deferred_tool_catalog_hint".to_string(),
            serde_json::Value::String(hint),
        );
    }
    let mut repair_loop_scorer = super::harness_issue_scorer::HarnessIssueScorer::new();
    let mut repair_escalated = false;
    let mut repair_deescalations = 0usize;
    let mut synchronous_repair_interventions = 0usize;
    let mut parallel_tool_batches_executed = 0usize;
    let mut auto_dpo_pairs_harvested = 0usize;
    let mut intraturn_observations_masked = 0usize;
    let mut intraturn_tokens_saved = 0usize;
    let mut kv_prefix_preserved_iterations = 0usize;
    let mut intraturn_read_tools_deduped = 0usize;
    let mut intraturn_read_memo_invalidations = 0usize;
    let mut read_tool_memo: std::collections::HashMap<(String, String), (usize, String, String)> =
        std::collections::HashMap::new();
    let mut tool_msg_iterations: Vec<(usize, usize, String)> = Vec::new();
    let mut executed_tool_ledger: Vec<String> = Vec::new();
    let mut failed_tool_calls: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut stop_hook_intervened = false;
    let mut false_refusal_retries = 0usize;
    let mut search_tool_calls_in_turn = 0usize;

    for iteration in 0..max_iterations {
        // T3 Wave 4 (`T3-LOOP-CHECKPOINT-04`): Drain any mid-turn steer interventions promoted via
        // `ThreadRunQueue::promote_to_steer` and inject them into the active turn's message list.
        if let Some(sid) = session_id.filter(|s| !s.is_empty()) {
            let steered: Vec<String> = state.steered_prompts.lock().remove(sid).unwrap_or_default();
            if !steered.is_empty() {
                telemetry.insert(
                    "mid_turn_steers_injected".to_string(),
                    serde_json::json!(steered.len()),
                );
                for intervention in steered {
                    messages.push(LlmChatMessage {
                        role: "user".into(),
                        content: intervention,
                        ..Default::default()
                    });
                }
            }
        }
        if iteration >= 2 {
            if !provider_uses_prefix_kv_cache(&llm_config_template)
                || total_prompt_tokens > KV_CACHE_OVERFLOW_MASK_THRESHOLD_TOKENS
            {
                let (masked_now, saved_now) = mask_stale_intraturn_tool_messages(
                    &mut messages,
                    &tool_msg_iterations,
                    iteration,
                );
                if masked_now > 0 {
                    intraturn_observations_masked += masked_now;
                    intraturn_tokens_saved += saved_now;
                    telemetry.insert(
                        "intraturn_observations_masked".to_string(),
                        serde_json::json!(intraturn_observations_masked),
                    );
                    telemetry.insert(
                        "intraturn_tokens_saved".to_string(),
                        serde_json::json!(intraturn_tokens_saved),
                    );
                }
            } else {
                kv_prefix_preserved_iterations += 1;
                telemetry.insert(
                    "kv_prefix_preserved_iterations".to_string(),
                    serde_json::json!(kv_prefix_preserved_iterations),
                );
            }
        }
        let mut config = llm_config_template.clone();
        if repair_escalated {
            apply_repair_escalation_to_config(
                state,
                &llm_config_template,
                &mut config,
                &mut telemetry,
            );
        }
        config.tools = if tool_defs.is_empty() {
            None
        } else if search_tool_calls_in_turn >= 3 {
            telemetry.insert(
                "search_circuit_breaker_tripped".to_string(),
                serde_json::Value::Bool(true),
            );
            let filtered: Vec<LlmToolDef> = tool_defs
                .iter()
                .filter(|t| !tool_refusal_detector::is_search_circuit_breaker_tool(&t.name))
                .cloned()
                .collect();
            if filtered.is_empty() {
                None
            } else {
                Some(filtered)
            }
        } else {
            Some(tool_defs.clone())
        };

        let resp = if stream_tokens {
            match stream_final_answer(
                state,
                &activity_options,
                &messages,
                config.clone(),
                session_id,
            )
            .await
            {
                Some(r) => r,
                None => match llm_chat(&activity_options, messages.clone(), config).await {
                    vox_actor_runtime::ActivityResult::Ok(Ok(r)) => r,
                    vox_actor_runtime::ActivityResult::Ok(Err(e)) => return Err(e),
                    vox_actor_runtime::ActivityResult::Failed(e) => return Err(format!("{e:?}")),
                    vox_actor_runtime::ActivityResult::Cancelled => {
                        return Err("llm_chat activity cancelled".to_string());
                    }
                },
            }
        } else {
            match llm_chat(&activity_options, messages.clone(), config).await {
                vox_actor_runtime::ActivityResult::Ok(Ok(r)) => r,
                vox_actor_runtime::ActivityResult::Ok(Err(e)) => return Err(e),
                vox_actor_runtime::ActivityResult::Failed(e) => return Err(format!("{e:?}")),
                vox_actor_runtime::ActivityResult::Cancelled => {
                    return Err("llm_chat activity cancelled".to_string());
                }
            }
        };
        model_used = resp.model.clone();
        let iter_prompt_tokens = if resp.prompt_tokens > 0 {
            u64::from(resp.prompt_tokens)
        } else {
            messages
                .iter()
                .map(|m| {
                    vox_orchestrator::compaction::CompactionEngine::estimate_tokens(&m.content)
                        as u64
                })
                .sum()
        };
        let iter_completion_tokens = u64::from(resp.completion_tokens);
        total_prompt_tokens += iter_prompt_tokens;
        total_completion_tokens += iter_completion_tokens;
        total_tokens += u64::from(resp.prompt_tokens) + u64::from(resp.completion_tokens);
        total_cache_read_tokens += u64::from(resp.cache_read_tokens);
        if total_cache_read_tokens > 0 {
            telemetry.insert(
                "cache_read_tokens".to_string(),
                serde_json::json!(total_cache_read_tokens),
            );
        }
        if let Some(c) = resp.cost_usd {
            *total_cost_usd.get_or_insert(0.0) += c;
        }
        *latency_ms.get_or_insert(0) += resp.latency_ms;
        if ttft_ms.is_none() {
            ttft_ms = resp.ttft_ms;
        }
        tpot_ms = resp.tpot_ms.or(tpot_ms);

        // Task B2 + Phase 165 (`SOTA-UNIVERSAL-SALVAGE-01..03`) multi-format & multi-call salvage:
        // (1) structured `tool_calls` wins when present and non-empty;
        // (2) otherwise scan text content for 1..N tool-shaped JSON/XML blocks across all 5 dialects;
        // (3) otherwise this is a plain-text answer.
        let wire_calls = resp.tool_calls.unwrap_or_default();
        let (calls, salvaged) = if !wire_calls.is_empty() {
            (wire_calls, false)
        } else {
            let salvaged_candidates = salvage_tool_calls_from_text(&resp.content, &tool_defs);
            if !salvaged_candidates.is_empty() {
                let synthesized =
                    serde_json::from_value(serde_json::Value::Array(salvaged_candidates))
                        .unwrap_or_default();
                (synthesized, true)
            } else {
                (Vec::new(), false)
            }
        };

        if !calls.is_empty() {
            if salvaged {
                telemetry.insert(
                    "tool_call_salvaged".to_string(),
                    serde_json::Value::Bool(true),
                );
                telemetry.insert(
                    "salvaged_tool_calls_count".to_string(),
                    serde_json::json!(calls.len()),
                );
            }
            messages.push(LlmChatMessage {
                role: "assistant".into(),
                content: resp.content,
                tool_calls: Some(calls.clone()),
                ..Default::default()
            });

            let dispatched_outcomes = dispatch_tool_calls_batch(
                state,
                session_id,
                permission_mode,
                &calls,
                &read_tool_memo,
                &mut parallel_tool_batches_executed,
                &mut telemetry,
            )
            .await;

            let was_repair_escalated_at_iter_start = repair_escalated;
            let mut iter_all_calls_clean = true;
            let mut iter_failed_calls = 0usize;
            for (call, (result, receipt)) in calls.iter().zip(dispatched_outcomes.into_iter()) {
                tool_calls_made += 1;
                if tool_refusal_detector::is_search_circuit_breaker_tool(&call.name) {
                    search_tool_calls_in_turn += 1;
                }
                let dispatch_ok = result.is_ok();
                let mut content = match result {
                    Ok(s) => s,
                    Err(e) => format!("Error: {e}"),
                };
                let is_deduped_read_hit = content.contains("\"deduped_read_hit\":true");
                if is_deduped_read_hit {
                    intraturn_read_tools_deduped += 1;
                    telemetry.insert(
                        "intraturn_read_tools_deduped".to_string(),
                        serde_json::json!(intraturn_read_tools_deduped),
                    );
                }
                let call_succeeded = dispatch_ok
                    && !content.starts_with("Error:")
                    && !crate::server_state::tool_json_envelope_is_error(&content);
                if crate::scope_guard::WRITE_TOOLS.contains(&call.name.as_str()) {
                    let evicted =
                        invalidate_read_memo_for_write(&mut read_tool_memo, &call.arguments);
                    if evicted > 0 {
                        intraturn_read_memo_invalidations += 1;
                        telemetry.insert(
                            "intraturn_read_memo_invalidations".to_string(),
                            serde_json::json!(intraturn_read_memo_invalidations),
                        );
                    }
                } else if call_succeeded
                    && !is_deduped_read_hit
                    && is_read_only_memoizable(&call.name)
                {
                    let digest =
                        vox_crypto::hex_encode(&vox_crypto::secure_hash(content.as_bytes()));
                    let short_hash = digest.chars().take(16).collect::<String>();
                    let preview = content.chars().take(160).collect::<String>();
                    read_tool_memo.insert(
                        (
                            call.name.clone(),
                            canonicalize_tool_args_json(&call.arguments),
                        ),
                        (iteration, short_hash, preview),
                    );
                }
                let has_inline_verification_failure = content
                    .contains("\"auto_verification_failed\":true")
                    || content.contains("AUTO_VERIFICATION_FAILED");

                // SOTA-CASCADE-01..04 & SOTA-FLYWHEEL-01: Synchronous repair intervention
                // and live intra-turn Auto-DPO contrast pair harvesting.
                let is_error_or_verify_fail = !call_succeeded || has_inline_verification_failure;
                if is_error_or_verify_fail {
                    iter_all_calls_clean = false;
                    iter_failed_calls += 1;
                }
                let repair_repeated = repair_loop_scorer.record(
                    &call.name,
                    &call.arguments.to_string(),
                    if is_error_or_verify_fail {
                        &content
                    } else {
                        ""
                    },
                );
                handle_repair_escalation_or_harvest_dpo(
                    state,
                    session_id,
                    classified_spoke,
                    &user_message,
                    call,
                    &mut content,
                    is_error_or_verify_fail,
                    has_inline_verification_failure,
                    repair_repeated,
                    &mut failed_tool_calls,
                    &mut synchronous_repair_interventions,
                    &mut repair_escalated,
                    &mut auto_dpo_pairs_harvested,
                    &mut telemetry,
                )
                .await;

                // Derived from the RESULT (`call_succeeded`, computed above from
                // what dispatch actually did), never from `call.arguments` alone
                // — see `turn_event_for_result`'s doc comment for why that
                // distinction is security-load-bearing.
                if let Some(ev) =
                    turn_event_for_result(&call.name, &call.arguments, &content, call_succeeded)
                {
                    events.push(ev);
                }
                if let Some(r) = receipt {
                    events.push(receipt_turn_event(
                        &r,
                        state.orchestrator.verify_tool_receipt(&r.receipt_id),
                    ));
                }

                record_harness_issue_if_crossed(
                    state,
                    session_id,
                    harness_detection_enabled,
                    &mut harness_scorer,
                    call,
                    &content,
                );

                let (capped_content, capped_tokens_saved) =
                    cap_and_summarize_tool_output(&call.name, &content);
                if capped_tokens_saved > 0 {
                    intraturn_tokens_saved += capped_tokens_saved;
                    telemetry.insert(
                        "intraturn_tokens_saved".to_string(),
                        serde_json::json!(intraturn_tokens_saved),
                    );
                }
                executed_tool_ledger.push(format!(
                    "{}:{}",
                    call.name,
                    if call_succeeded { "ok" } else { "err" }
                ));
                messages.push(crate::tool_images::llm_tool_message(
                    call.id.clone(),
                    call.name.clone(),
                    capped_content,
                    &vox_config::paths::browser_frames_cache_dir(),
                ));
                let pushed_idx = messages.len() - 1;
                tool_msg_iterations.push((pushed_idx, iteration, call.name.clone()));
                crate::tool_images::retain_latest_tool_image(&mut messages);
            }
            if decision_packs_on && !repair_escalated && !calls.is_empty() {
                // DEC-02: post-tool confidence = clean-call share of this batch; below the
                // threshold it re-enters the existing repair escalation (no new loop).
                let pack = PostToolDecisionPack {
                    schema_version: POST_TOOL_DECISION_PACK_SCHEMA_V1.to_string(),
                    model_still_ok_prob: 1.0 - (iter_failed_calls as f64 / calls.len() as f64),
                    confidence_threshold: 0.5,
                    directive: None,
                    block_reason: None,
                };
                if pack.should_reenter_repair_escalation() {
                    repair_escalated = true;
                    telemetry.insert(
                        "post_tool_low_confidence_escalated".to_string(),
                        serde_json::Value::Bool(true),
                    );
                    telemetry.insert(
                        "post_tool_model_still_ok_prob".to_string(),
                        serde_json::json!(pack.model_still_ok_prob),
                    );
                }
            }
            if should_deescalate_repair_cascade(
                was_repair_escalated_at_iter_start,
                iter_all_calls_clean,
            ) {
                repair_escalated = false;
                repair_deescalations += 1;
                telemetry.insert(
                    "repair_deescalations".to_string(),
                    serde_json::json!(repair_deescalations),
                );
            }
            if !executed_tool_ledger.is_empty() {
                telemetry.insert(
                    "turn_tool_ledger".to_string(),
                    serde_json::json!(executed_tool_ledger),
                );
            }

            if iteration + 1 == max_iterations {
                return Ok(AgentTurnOutcome {
                    final_text: String::new(),
                    model_used,
                    tool_calls_made,
                    tool_calls_dispatched: tool_calls_made,
                    reply_text: None,
                    telemetry,
                    hit_iteration_limit: true,
                    total_tokens,
                    events,
                    latency_ms,
                    ttft_ms,
                    tpot_ms,
                    cache_read_tokens: total_cache_read_tokens,
                    cost_usd: total_cost_usd,
                    prompt_tokens: total_prompt_tokens,
                    completion_tokens: total_completion_tokens,
                });
            }
            // Otherwise loop: ask the model again with the tool results appended.
        } else {
            if tool_refusal_detector::should_retry_false_tool_refusal(
                &resp.content,
                tool_defs.len(),
                tool_calls_made,
                false_refusal_retries >= tool_refusal_detector::MAX_REFUSAL_CORRECTIONS,
            ) && iteration + 1 < max_iterations
            {
                false_refusal_retries += 1;
                telemetry.insert(
                    "false_tool_refusal_retried".to_string(),
                    serde_json::Value::Bool(true),
                );
                telemetry.insert(
                    "false_tool_refusal_retry_count".to_string(),
                    serde_json::json!(false_refusal_retries),
                );
                let active_names: Vec<&str> = tool_defs.iter().map(|t| t.name.as_str()).collect();
                let reminder =
                    tool_refusal_detector::build_tool_refusal_recovery_reminder(&active_names);
                messages.push(LlmChatMessage {
                    role: "assistant".into(),
                    content: resp.content,
                    ..Default::default()
                });
                messages.push(LlmChatMessage {
                    role: "user".into(),
                    content: reminder,
                    ..Default::default()
                });
                continue;
            }
            let unresolved_paper_warnings = telemetry
                .get("unresolved_paper_code_warnings")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            if !stop_hook_intervened
                && unresolved_paper_warnings > 0
                && iteration + 1 < max_iterations
                && let HookDecision::Block { reason } =
                    evaluate_stop_hook(&resp.content, unresolved_paper_warnings)
            {
                stop_hook_intervened = true;
                telemetry.insert(
                    "stop_hook_intervened".to_string(),
                    serde_json::Value::Bool(true),
                );
                messages.push(LlmChatMessage {
                    role: "assistant".into(),
                    content: resp.content,
                    ..Default::default()
                });
                messages.push(LlmChatMessage {
                    role: "user".into(),
                    content: format!("[STOP HOOK QUALITY GATE INTERVENTION: {reason}]"),
                    ..Default::default()
                });
                continue;
            }
            let final_text = Box::pin(super::mermaid_repair::observe_and_repair_turn_mermaid(
                state,
                &resp.content,
                permission_mode,
                &mut tool_calls_made,
                &mut events,
                &mut telemetry,
            ))
            .await;
            let (quality_metrics, quality_dpo_eligible) =
                evaluate_turn_quality_and_gate_dpo(&user_message, &final_text, None);
            telemetry.insert(
                "response_quality_score".to_string(),
                serde_json::json!(quality_metrics.quality_score),
            );
            telemetry.insert(
                "response_quality_bucket".to_string(),
                serde_json::json!(quality_metrics.bucket),
            );
            telemetry.insert(
                "response_quality_dpo_eligible".to_string(),
                serde_json::Value::Bool(quality_dpo_eligible),
            );
            {
                let models_handle = state.orchestrator.models_handle();
                if let Ok(mut reg) =
                    crate::sync_poison::poison_rw_write(models_handle.write(), "models registry")
                {
                    reg.record_response_quality_observation(
                        &model_used,
                        quality_metrics.quality_score,
                    );
                }
            }
            return Ok(AgentTurnOutcome {
                final_text: final_text.clone(),
                model_used,
                tool_calls_made,
                tool_calls_dispatched: tool_calls_made,
                reply_text: Some(final_text),
                telemetry,
                hit_iteration_limit: false,
                total_tokens,
                events,
                latency_ms,
                ttft_ms,
                tpot_ms,
                cache_read_tokens: total_cache_read_tokens,
                cost_usd: total_cost_usd,
                prompt_tokens: total_prompt_tokens,
                completion_tokens: total_completion_tokens,
            });
        }
    }

    // Unreachable in practice: the `iteration + 1 == max_iterations` check above
    // always returns before the loop would fall through here. Kept as a defensive
    // fallback so the function has no silent infinite-loop path even if the loop
    // bound logic above is ever refactored incorrectly.
    Ok(AgentTurnOutcome {
        final_text: String::new(),
        model_used,
        tool_calls_made,
        tool_calls_dispatched: tool_calls_made,
        reply_text: None,
        telemetry,
        hit_iteration_limit: true,
        total_tokens,
        events,
        latency_ms,
        ttft_ms,
        tpot_ms,
        cache_read_tokens: total_cache_read_tokens,
        cost_usd: total_cost_usd,
        prompt_tokens: total_prompt_tokens,
        completion_tokens: total_completion_tokens,
    })
}

/// Compute deterministic Flesch-Kincaid response quality metrics (`0.0..=100.0`) and gate
/// whether a `(chosen_response, rejected_response)` pair qualifies for Auto-DPO preference
/// harvesting (`T3-ROUTER-DPO-FLY-03`).
#[must_use]
pub fn evaluate_turn_quality_and_gate_dpo(
    prompt: &str,
    chosen_response: &str,
    rejected_response: Option<&str>,
) -> (
    vox_orchestrator::models::quality_analyzer::ResponseQualityMetrics,
    bool,
) {
    let chosen_metrics = vox_orchestrator::models::quality_analyzer::analyze_response_quality(
        prompt,
        chosen_response,
    );
    let should_harvest_dpo = if let Some(rejected) = rejected_response {
        let rejected_metrics =
            vox_orchestrator::models::quality_analyzer::analyze_response_quality(prompt, rejected);
        chosen_metrics.quality_score >= 60.0
            && (chosen_metrics.quality_score - rejected_metrics.quality_score) >= 5.0
    } else {
        chosen_metrics.quality_score >= 70.0
    };
    (chosen_metrics, should_harvest_dpo)
}

#[cfg(test)]
mod tests;
