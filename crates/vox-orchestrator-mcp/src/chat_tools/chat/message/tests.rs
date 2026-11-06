use super::*;
use serial_test::serial;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use vox_db::store::types::ModelOutcome;
use vox_db::{DbConfig, VoxDb};
use vox_orchestrator::models::spec::PricingSource;
use vox_orchestrator::models::{ModelCapabilities, ModelSpec, ProviderType};
use vox_orchestrator::{
    AffinityGroupRegistry, Orchestrator, OrchestratorConfig, SessionConfig, SessionManager,
};
use vox_repository::{RepoCapabilities, RepositoryContext};
use vox_skills::new_registry_arc;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::chat_tools::params::{ANTI_LAZINESS_RIDER, ChatMessageParams};

// Shared with `agent_loop::tests` — see that lock's doc comment: a private
// per-module lock does not serialize against a sibling module's private
// lock, so both modules that mutate `OPENROUTER_BASE_URL`/`OPENROUTER_API_KEY`
// must hold the same crate-wide lock.
use crate::chat_tools::chat::agent_loop::CHAT_MESSAGE_ENV_LOCK;

fn test_state() -> ServerState {
    let cfg = OrchestratorConfig::for_testing();
    let orch_cfg = cfg.clone();
    let groups = AffinityGroupRegistry::new(vec![]);
    let session_cfg = SessionConfig {
        persist: false,
        sessions_dir: std::env::temp_dir().join("vox-mcp-message-test-sessions"),
        ..SessionConfig::default()
    };
    let session_manager = SessionManager::new(session_cfg).expect("session manager");
    let repository = RepositoryContext {
        root: PathBuf::from("."),
        git_root: None,
        repository_id: "message-test".into(),
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
    ServerState::hermetic_stub(
        cfg,
        repository,
        Arc::new(Orchestrator::with_groups(orch_cfg, groups)),
        Arc::new(Mutex::new(session_manager)),
        new_registry_arc(),
    )
}

fn model_spec(provider_type: ProviderType, id: &str) -> ModelSpec {
    ModelSpec {
        id: id.to_string(),
        canonical_slug: id.to_string(),
        provider: "test".to_string(),
        provider_type,
        max_tokens: 8192,
        cost_per_1k: 0.0,
        cost_per_1k_input: 0.0,
        cost_per_1k_output: 0.0,
        observed_cost_per_1k: None,
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source: PricingSource::Bootstrap,
        is_free: false,
        strengths: Vec::new(),
        capabilities: ModelCapabilities::default(),
        supported_parameters: Vec::new(),
    }
}

fn plain_response_body(content: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content},
            "finish_reason": "stop",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    })
}

/// Task 4 spec review finding: `try_run_agent_turn` is the *actual* entry point
/// for the DEFAULT `vox_chat_message` path (no `cognitive_profile` set) — it
/// resolves a model and dispatches via `run_agent_turn` ->
/// `vox_actor_runtime::llm::llm_chat` -> `vox_llm_egress::chat_once`, entirely
/// bypassing `resolve_chat_llm_model` (whose own budget gate therefore never runs
/// for this path) and the pre-existing `vox_orchestrator::BudgetGate` inside
/// `mcp_infer_tool_completion` (which this path also never reaches). This test
/// exercises that exact function — the real default-path entry point, not
/// `resolve_chat_llm_model` — end to end, and confirms it is now budget-gated via
/// the `enforce_budget_guard` call added at its top.
#[tokio::test]
#[serial]
async fn default_chat_path_refuses_when_daily_budget_exceeded() {
    let prior = std::env::var("VOX_BUDGET_USD").ok();
    // SAFETY: `#[serial]` — no concurrent env mutation in this crate's tests.
    unsafe { std::env::set_var("VOX_BUDGET_USD", "0.01") };
    vox_config::snapshot::bump(&["VOX_BUDGET_USD"]);

    let db = VoxDb::connect(DbConfig::Memory)
        .await
        .expect("open in-memory db");
    db.record_llm_outcome(ModelOutcome {
        session_id: "default-path-test",
        user_id: None,
        tenant_id: None,
        prompt: "p",
        response: "r",
        model_id: "m",
        provider: "openrouter",
        task_category: "general",
        strength_tag: "generalist",
        latency_ms: Some(10),
        input_tokens: Some(5),
        output_tokens: Some(5),
        cache_read_tokens: Some(0),
        trace_id: None,
        context_utilization_pct: None,
        success: true,
        cost_usd: Some(0.02),
        quality_score: Some(1.0),
        ttft_ms: None,
        tpot_ms: None,
    })
    .await
    .expect("record spend");

    let state = ServerState::new_test()
        .await
        .with_db_initialized(Arc::new(db))
        .await;

    // No `cognitive_profile` involved at all — this calls the exact function
    // `chat_message`'s `None =>` arm (the default path) calls, with the same
    // shape of arguments (no attachment, real session id).
    let result = try_run_agent_turn(
        &state,
        "system prompt",
        vec![],
        "hello",
        "hello",
        "default-path-test",
        None,
        false,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
    )
    .await;

    // SAFETY: `#[serial]` — restore prior env state before asserting/panicking.
    unsafe {
        match &prior {
            Some(v) => std::env::set_var("VOX_BUDGET_USD", v),
            None => std::env::remove_var("VOX_BUDGET_USD"),
        }
    }
    vox_config::snapshot::bump(&["VOX_BUDGET_USD"]);

    let err = result
            .expect("budget-exceeded must refuse via Some(Err(..)), not silently fall through to call_llm via None")
            .expect_err("expected budget guard to refuse dispatch");
    assert!(
        err.to_lowercase().contains("budget"),
        "expected error to mention budget, got: {err}"
    );
    assert_eq!(
        crate::chat_hop::turn_outcome_for_dispatch_err(&err),
        crate::chat_hop::TurnOutcome::BudgetDenied,
        "budget guard refusal must classify as BudgetDenied, not LlmError"
    );
}

/// Task 8f, fix round 2: when `resolve_chat_llm_model` errors, `chat_message`'s
/// `cognitive_profile` branch falls through to plain `call_llm(state,
/// &system_prompt, &user_prompt, ...)` — `user_prompt` there is the fully
/// assembled prompt (history/open-files/`[SELECTED TEXT]`/retrieved/web
/// context). This must still infer capabilities from the user's own message
/// (`expanded_prompt`), not from that assembled prompt.
///
/// Forces `resolve_chat_llm_model` to error via an exceeded daily budget —
/// `resolve_chat_llm_model` calls `enforce_budget_guard` *before* any model
/// resolution work, so it fails deterministically and hermetically (no
/// network reachable: no HTTP mock is even needed, since the fallback
/// `call_llm` call below also hits `mcp_infer_completion`'s own budget gate
/// before any dispatch). `params.selected_text` injects "latest news" into
/// `context_parts` (`[SELECTED TEXT]`) without needing multi-turn history —
/// the user's actual message (`prompt`) is "refactor this", with no
/// web-search cue. An error mentioning "budget" (not "capability"/"not
/// allowed") proves resolution accepted the pinned web-incapable model
/// despite "latest news" sitting in the injected `[SELECTED TEXT]` block.
#[tokio::test]
#[serial]
async fn cognitive_profile_fallback_infers_capabilities_from_users_message_not_injected_context() {
    let prior = std::env::var("VOX_BUDGET_USD").ok();
    // SAFETY: `#[serial]` — no concurrent env mutation in this crate's tests.
    unsafe { std::env::set_var("VOX_BUDGET_USD", "0.01") };
    vox_config::snapshot::bump(&["VOX_BUDGET_USD"]);

    let db = VoxDb::connect(DbConfig::Memory)
        .await
        .expect("open in-memory db");
    db.record_llm_outcome(ModelOutcome {
        session_id: "cognitive-fallback-capability-test",
        user_id: None,
        tenant_id: None,
        prompt: "p",
        response: "r",
        model_id: "m",
        provider: "openrouter",
        task_category: "general",
        strength_tag: "generalist",
        latency_ms: Some(10),
        input_tokens: Some(5),
        output_tokens: Some(5),
        cache_read_tokens: Some(0),
        trace_id: None,
        context_utilization_pct: None,
        success: true,
        cost_usd: Some(0.02),
        quality_score: Some(1.0),
        ttft_ms: None,
        tpot_ms: None,
    })
    .await
    .expect("record spend");

    let state = ServerState::new_test()
        .await
        .with_db_initialized(Arc::new(db))
        .await;

    // Web-incapable model, pinned via the sticky override so the fallback
    // `call_llm`'s resolution takes the id-lookup branch (no scored
    // selection, no API key dependency).
    let model_id = "cognitive-fallback-web-incapable";
    {
        let handle = state.orchestrator.models_handle();
        handle
            .write()
            .expect("models lock")
            .register(model_spec(ProviderType::OpenRouter, model_id));
    }
    *state.mcp_chat_model_override.write() = Some(model_id.to_string());

    let params: ChatMessageParams = serde_json::from_value(serde_json::json!({
        "prompt": "refactor this",
        "selected_text": "what's the latest news on rust async?",
        "cognitive_profile": "fast",
        "session_id": "cognitive-fallback-capability-test",
    }))
    .expect("chat message params");

    let response_json = chat_message(&state, params).await;

    // SAFETY: `#[serial]` — restore prior env state before asserting/panicking.
    unsafe {
        match &prior {
            Some(v) => std::env::set_var("VOX_BUDGET_USD", v),
            None => std::env::remove_var("VOX_BUDGET_USD"),
        }
    }
    vox_config::snapshot::bump(&["VOX_BUDGET_USD"]);

    let parsed: serde_json::Value =
        serde_json::from_str(&response_json).expect("chat_message must return valid JSON");
    assert_eq!(
        parsed["success"], false,
        "budget-exceeded turn must fail: {response_json}"
    );
    let err = parsed["error"].as_str().unwrap_or_default();
    assert!(
        err.to_lowercase().contains("budget"),
        "expected a budget error (proving capability resolution accepted the pinned \
             web-incapable model despite \"latest news\" in injected [SELECTED TEXT]) — got a \
             capability-gate or other error instead: {response_json}"
    );
}

/// Fix Task 6: the turn's elapsed time is already computed (`llm_started.elapsed()`)
/// but was never threaded into the envelope `data` object the GUI's `ModelBadge`
/// reads. Mirrors `agent_loop::tests::chat_message_default_path_sends_tools_bearing_request`'s
/// wiremock harness to drive a real, successful `chat_message` call and assert the
/// returned envelope's `data.latency_ms` field is present.
#[tokio::test]
#[allow(unsafe_code)] // env var mutation under a process-wide lock, like agent_loop's test
#[allow(clippy::await_holding_lock)]
async fn chat_message_envelope_includes_latency_ms() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body("no tools needed")),
        )
        .mount(&server)
        .await;

    let prev_base = std::env::var("OPENROUTER_BASE_URL").ok();
    let prev_key = std::env::var("OPENROUTER_API_KEY").ok();
    unsafe {
        std::env::set_var("OPENROUTER_BASE_URL", server.uri());
        std::env::set_var("OPENROUTER_API_KEY", "test-key");
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

    let state = test_state();
    let model_id = "test-openrouter-model-latency";
    {
        let handle = state.orchestrator.models_handle();
        let mut registry = handle.write().expect("models registry lock");
        registry.register(model_spec(ProviderType::OpenRouter, model_id));
    }
    *state.mcp_chat_model_override.write() = Some(model_id.to_string());

    let params: ChatMessageParams =
        serde_json::from_value(serde_json::json!({ "prompt": "hello there" }))
            .expect("chat message params");

    let response_json = chat_message(&state, params).await;

    unsafe {
        match prev_base {
            Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
            None => std::env::remove_var("OPENROUTER_BASE_URL"),
        }
        match prev_key {
            Some(v) => std::env::set_var("OPENROUTER_API_KEY", v),
            None => std::env::remove_var("OPENROUTER_API_KEY"),
        }
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

    let parsed: serde_json::Value =
        serde_json::from_str(&response_json).expect("chat_message must return valid JSON");
    assert_eq!(
        parsed["success"], true,
        "chat_message should succeed via the mapped agent-loop path: {response_json}"
    );
    let data = &parsed["data"];
    assert!(
        data["model_used"].as_str().is_some(),
        "sanity check: envelope should carry a model_used string: {response_json}"
    );
    assert!(
        data.get("latency_ms").and_then(|v| v.as_u64()).is_some(),
        "chat_message envelope `data` must carry a `latency_ms` field for ModelBadge: {response_json}"
    );
}

#[tokio::test]
#[allow(unsafe_code)]
#[allow(clippy::await_holding_lock)]
async fn every_turn_carries_a_research_trace_event() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(plain_response_body("hello back")))
        .mount(&server)
        .await;
    let prev_base = std::env::var("OPENROUTER_BASE_URL").ok();
    let prev_key = std::env::var("OPENROUTER_API_KEY").ok();
    unsafe {
        std::env::set_var("OPENROUTER_BASE_URL", server.uri());
        std::env::set_var("OPENROUTER_API_KEY", "test-key");
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);
    let state = test_state();
    let model_id = "test-openrouter-model-trace";
    {
        let handle = state.orchestrator.models_handle();
        handle
            .write()
            .expect("models lock")
            .register(model_spec(ProviderType::OpenRouter, model_id));
    }
    *state.mcp_chat_model_override.write() = Some(model_id.to_string());

    let params: ChatMessageParams =
        serde_json::from_value(serde_json::json!({ "prompt": "hi" })).expect("params");
    let response_json = chat_message(&state, params).await;

    unsafe {
        match prev_base {
            Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
            None => std::env::remove_var("OPENROUTER_BASE_URL"),
        }
        match prev_key {
            Some(v) => std::env::set_var("OPENROUTER_API_KEY", v),
            None => std::env::remove_var("OPENROUTER_API_KEY"),
        }
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

    let parsed: serde_json::Value = serde_json::from_str(&response_json).expect("json");
    assert_eq!(parsed["success"], true, "{response_json}");
    let ev = &parsed["data"]["events"][0];
    assert_eq!(ev["kind"], "research_trace", "{response_json}");
    assert_eq!(ev["mode"], "none");
    assert_eq!(ev["stages"][0]["stage"], "detection");
    // D1 regression: a greeting must not reach web retrieval.
    assert!(
        ev["stages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["stage"] != "retrieval"),
        "{ev}"
    );
    // Task 8d: a None-intent turn has no numbered research sources to
    // conflict with, so the autonomous-retrieval preamble keeps its web
    // leg and must NOT record a "preamble" skipped stage.
    assert!(
        ev["stages"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["stage"] != "preamble"),
        "{ev}"
    );
}

/// Task 8d: on a Quick-intent turn, the numbered research sources
/// (`run_quick`) are this turn's web evidence, so the autonomous-retrieval
/// preamble must skip its own web leg and record why. Mutation guard: if
/// the `skip_preamble_web` gate in `chat_message` (or the
/// `web_research_enabled` seam it sets on `run_retrieval_bundle`'s policy)
/// is removed, this test's `preamble` stage assertion fails.
#[tokio::test]
#[allow(unsafe_code)]
#[allow(clippy::await_holding_lock)]
async fn quick_research_turn_skips_the_preamble_web_leg() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body("no sources found")),
        )
        .mount(&server)
        .await;
    let prev_base = std::env::var("OPENROUTER_BASE_URL").ok();
    let prev_key = std::env::var("OPENROUTER_API_KEY").ok();
    let model_id = "test-openrouter-model-preamble-skip";
    unsafe {
        std::env::set_var("OPENROUTER_BASE_URL", server.uri());
        std::env::set_var("OPENROUTER_API_KEY", "test-key");
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);
    let state = test_state();
    {
        let handle = state.orchestrator.models_handle();
        let mut spec = model_spec(ProviderType::OpenRouter, model_id);
        // Task 8e fix round 1: this test used to also force the process
        // env var `VOX_MODEL_FORCE` to this model id, purely to keep
        // resolution deterministic against a host `~/.vox/config.toml`
        // pin (see the removed comment in git blame). That workaround is
        // now redundant — `vox-orchestrator-mcp`'s hermetic test ctor
        // (`src/lib.rs`) points `VOX_HOME` at an empty temp dir before any
        // test runs, so `~/.vox/config.toml` can never supply
        // `VOX_MODEL_FORCE` here — and it raced: mutating a process-wide
        // env var under `CHAT_MESSAGE_ENV_LOCK` does not serialize
        // against `llm_bridge::model_route_policy::tests`' *different*
        // `INFERENCE_PROFILE_TEST_LOCK`, so those tests could
        // transiently observe this model id as a strict pin and fail.
        // `mcp_chat_model_override` below (private per-`ServerState`
        // state, no cross-test env race) is exactly the sticky-pref path
        // every other test in this file already uses, and reaches the
        // same capability gate `caps_ok` enforces on the strict-pin path
        // — so keep the full-capability grant to prove this test's
        // subject (the research trace) does not accidentally depend on
        // it.
        spec.capabilities = vox_orchestrator::models::ModelCapabilities {
            supports_json: true,
            supports_vision: true,
            supports_native_tools: true,
            supports_tool_use: true,
            supports_reasoning: true,
            supports_web_search: true,
            supports_image_generation: true,
            supports_audio_input: true,
            supports_audio_output: true,
            supports_file_input: true,
            supports_jsonl: true,
            writes_vox: true,
            ..vox_orchestrator::models::ModelCapabilities::default()
        };
        handle.write().expect("models lock").register(spec);
    }
    *state.mcp_chat_model_override.write() = Some(model_id.to_string());

    // Explicit `/research` is classified Quick regardless of heuristic
    // cues (see `research_intent::classify_research_intent`), so this
    // turn's mode is deterministic without depending on wording heuristics.
    let params: ChatMessageParams = serde_json::from_value(serde_json::json!({
        "prompt": "/research latest tokio release"
    }))
    .expect("params");
    let response_json = chat_message(&state, params).await;

    unsafe {
        match prev_base {
            Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
            None => std::env::remove_var("OPENROUTER_BASE_URL"),
        }
        match prev_key {
            Some(v) => std::env::set_var("OPENROUTER_API_KEY", v),
            None => std::env::remove_var("OPENROUTER_API_KEY"),
        }
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

    let parsed: serde_json::Value = serde_json::from_str(&response_json).expect("json");
    assert_eq!(parsed["success"], true, "{response_json}");
    let ev = &parsed["data"]["events"][0];
    assert_eq!(ev["kind"], "research_trace", "{response_json}");
    assert_eq!(ev["mode"], "quick", "{ev}");
    let stages = ev["stages"].as_array().unwrap();
    let preamble = stages
        .iter()
        .find(|s| s["stage"] == "preamble")
        .unwrap_or_else(|| panic!("expected a preamble stage in the trace: {ev}"));
    assert_eq!(preamble["status"], "skipped", "{preamble}");
    assert_eq!(
        preamble["summary"],
        "autonomous web retrieval skipped: numbered research sources are the web \
             evidence this turn",
        "{preamble}"
    );
    // Task 13: the chat role's model is recorded as the id the provider
    // reported ("test-model" in the mock body) — not a strict pin here
    // (sticky override), so no requested pin.
    assert_eq!(
        ev["models"],
        serde_json::json!([{"role": "chat", "requested": null, "resolved": "test-model"}]),
        "{ev}"
    );
    assert_eq!(ev["model"], "test-model", "{ev}");
}

/// Regression test for the "attention budget meter never increments during
/// chat activity" bug: `ChatTaskProcessor` (the only prior caller of
/// `Orchestrator::record_chat_attention`) was deleted in commit `bd2ade59ae`
/// once real chat turns moved to this synchronous `chat_message` path, but
/// nothing replaced the debit call here. Drives a real, successful
/// `chat_message` call through the same wiremock harness as
/// `chat_message_envelope_includes_latency_ms` and asserts the
/// orchestrator's `BudgetManager` attention `spent_ms` actually increased —
/// proving the GUI's `AttentionBudgetMeter` (driven purely by `spent_ms`)
/// moves for normal chat activity again.
#[tokio::test]
#[allow(unsafe_code)] // env var mutation under a process-wide lock, like agent_loop's test
#[allow(clippy::await_holding_lock)]
async fn chat_message_debits_attention_budget_on_success() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body("no tools needed")),
        )
        .mount(&server)
        .await;

    let prev_base = std::env::var("OPENROUTER_BASE_URL").ok();
    let prev_key = std::env::var("OPENROUTER_API_KEY").ok();
    unsafe {
        std::env::set_var("OPENROUTER_BASE_URL", server.uri());
        std::env::set_var("OPENROUTER_API_KEY", "test-key");
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

    let state = test_state();
    let model_id = "test-openrouter-model-attention";
    {
        let handle = state.orchestrator.models_handle();
        let mut registry = handle.write().expect("models registry lock");
        registry.register(model_spec(ProviderType::OpenRouter, model_id));
    }
    *state.mcp_chat_model_override.write() = Some(model_id.to_string());

    let spent_before =
        vox_orchestrator::sync_lock::rw_read(&*state.orchestrator.budget_manager_handle())
            .attention_snapshot()
            .spent_ms;

    let params: ChatMessageParams =
        serde_json::from_value(serde_json::json!({ "prompt": "hello there" }))
            .expect("chat message params");

    let response_json = chat_message(&state, params).await;

    unsafe {
        match prev_base {
            Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
            None => std::env::remove_var("OPENROUTER_BASE_URL"),
        }
        match prev_key {
            Some(v) => std::env::set_var("OPENROUTER_API_KEY", v),
            None => std::env::remove_var("OPENROUTER_API_KEY"),
        }
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

    let parsed: serde_json::Value =
        serde_json::from_str(&response_json).expect("chat_message must return valid JSON");
    assert_eq!(
        parsed["success"], true,
        "chat_message should succeed via the mapped agent-loop path: {response_json}"
    );

    let spent_after =
        vox_orchestrator::sync_lock::rw_read(&*state.orchestrator.budget_manager_handle())
            .attention_snapshot()
            .spent_ms;
    assert!(
        spent_after > spent_before,
        "a successful chat turn must debit AttentionBudget.spent_ms via \
             Orchestrator::record_chat_attention so the GUI meter moves; \
             before={spent_before} after={spent_after}"
    );
}

// ── Task A2: per-request model override + composer tier ──────────────

#[test]
fn request_override_beats_process_global() {
    assert_eq!(
        effective_model_pref(Some("req"), Some("glob")),
        Some("req".into())
    );
}

#[test]
fn falls_back_to_process_global() {
    assert_eq!(
        effective_model_pref(None, Some("glob")),
        Some("glob".into())
    );
}

#[test]
fn blank_request_override_is_not_a_pick() {
    // A blank must not shadow the global, and must never reach the
    // resolver -- which treats "" as an explicit, unresolvable model id.
    assert_eq!(
        effective_model_pref(Some("  "), Some("glob")),
        Some("glob".into())
    );
    assert_eq!(effective_model_pref(Some(""), None), None);
}

#[test]
fn tier_maps_to_resolution_not_cognitive_profile() {
    let r = resolution_for_tier(Some("local"), McpChatModelResolution::default());
    assert!(r.enforce_free_tier_only || r.allow_cheapest_fallback);
    let auto = resolution_for_tier(None, McpChatModelResolution::default());
    assert!(auto.allow_cheapest_fallback);
}

// ── Phase B / Task B1: honest selection classification ───────────────
// Nested so `cargo test -p vox-orchestrator-mcp selection` (the plan's
// documented command) matches these by module path.
mod selection {
    use super::*;

    #[test]
    fn source_is_classified_from_the_resolved_model_not_the_request() {
        // The honesty bug: classifying from the request means a pinned model
        // that the resolver silently ignored still reads "Your pick".
        assert_eq!(
            SelectionSource::classify(Some("m"), Some("m"), None),
            SelectionSource::UserOverride
        );
        assert_eq!(
            SelectionSource::classify(Some("gone"), Some("other"), None),
            SelectionSource::Fallback
        );
        assert_eq!(
            SelectionSource::classify(None, Some("auto"), None),
            SelectionSource::AutoRouted
        );
        assert_eq!(
            SelectionSource::classify(None, Some("g"), Some("g")),
            SelectionSource::Global
        );
    }

    #[test]
    fn fallback_always_carries_a_rationale() {
        let d = SelectionDto::fallback("free-x", "requested `gone` is not in the registry");
        assert_eq!(d.source, SelectionSource::Fallback);
        assert!(
            d.rationale
                .expect("rationale")
                .contains("not in the registry")
        );
    }
}

/// Regression test for the bug the assertion above was too weak to catch:
/// `r.enforce_free_tier_only || r.allow_cheapest_fallback` can never fail for
/// `"local"` even when `allow_cheapest_fallback` silently regressed to
/// `false`, because `enforce_free_tier_only` alone satisfies the OR. Pin
/// both fields explicitly, and require the whole struct to differ from
/// `McpChatModelResolution::default()` in at least one field -- the
/// independent-review finding was that `"cloud"` produced a byte-identical
/// no-op vs default.
#[test]
fn tier_local_sets_free_tier_and_keeps_cheapest_fallback() {
    let base = McpChatModelResolution::default();
    let r = resolution_for_tier(Some("local"), McpChatModelResolution::default());
    assert!(r.enforce_free_tier_only, "local must force free-tier-only");
    assert!(
        r.allow_cheapest_fallback,
        "local must not silently drop the pre-existing cheapest-fallback \
             degradation path just because a tier was named -- \
             enforce_free_tier_if_needed always re-swaps to a free model anyway, \
             so this can never leak a paid pick"
    );
    assert_ne!(
        r.enforce_free_tier_only, base.enforce_free_tier_only,
        "local's resolution must be observably different from default()"
    );
}

#[test]
fn tier_mesh_sets_latency_critical_and_keeps_cheapest_fallback() {
    let base = McpChatModelResolution::default();
    let r = resolution_for_tier(Some("mesh"), McpChatModelResolution::default());
    assert!(r.free_tier_latency_critical);
    assert!(
        r.allow_cheapest_fallback,
        "mesh must not silently drop the cheapest-fallback degradation path"
    );
    assert_ne!(
        r.free_tier_latency_critical,
        base.free_tier_latency_critical
    );
}

/// Regression test for the specific "cloud is a no-op" finding:
/// `McpChatModelResolution::default()` already has `allow_cheapest_fallback:
/// false`, so the old `cloud` arm (which only ever set that field to
/// `false`) produced a struct indistinguishable from doing nothing. The
/// fixed arm must differ from both `default()` and from "auto" (`None`,
/// which always allows cheapest-fallback).
#[test]
fn tier_cloud_is_not_a_no_op() {
    let base = McpChatModelResolution::default();
    let auto = resolution_for_tier(None, McpChatModelResolution::default());
    let cloud = resolution_for_tier(Some("cloud"), McpChatModelResolution::default());

    assert!(
        !cloud.allow_cheapest_fallback,
        "cloud must not degrade to the cheapest free/paid fallback"
    );
    assert_ne!(
        cloud.allow_cheapest_fallback, auto.allow_cheapest_fallback,
        "cloud must be observably different from auto/unset tier"
    );
    assert_ne!(
        cloud.complexity, base.complexity,
        "cloud must be observably different from McpChatModelResolution::default() \
             in at least one field -- the reviewed bug was a byte-identical no-op"
    );
}

/// An unrecognized/typo'd tier string (`tier` is free-text at the Rust type
/// level) must be an explicit, documented alias for auto/default -- not an
/// accidental match of a *different* recognized tier's behavior. This pins
/// the documented decision in `resolution_for_tier`'s `Some(other) =>` arm.
#[test]
fn unknown_tier_is_documented_alias_for_auto_not_a_recognized_tier() {
    let auto = resolution_for_tier(None, McpChatModelResolution::default());
    let typo = resolution_for_tier(Some("Cloud"), McpChatModelResolution::default());

    assert_eq!(typo.allow_cheapest_fallback, auto.allow_cheapest_fallback);
    assert_eq!(typo.enforce_free_tier_only, auto.enforce_free_tier_only);
    assert_eq!(
        typo.free_tier_latency_critical,
        auto.free_tier_latency_critical
    );

    let local = resolution_for_tier(Some("local"), McpChatModelResolution::default());
    assert_ne!(
        typo.enforce_free_tier_only, local.enforce_free_tier_only,
        "a typo'd tier must never silently behave like a *different* \
             recognized tier"
    );
}

/// The trap: `cognitive_profile` takes `fast|reasoning|creative` and a
/// non-`None` value switches the whole turn off the agent loop onto
/// `mcp_infer_completion` (losing tool calls and `selection_reason`). A
/// composer tier must therefore never land in it. Pinned at the only place
/// a tier can enter the daemon: `ChatMessageParams` deserialization.
#[test]
fn tier_never_populates_cognitive_profile() {
    for tier in ["local", "mesh", "cloud", "auto"] {
        let params: ChatMessageParams = serde_json::from_value(serde_json::json!({
            "prompt": "hi",
            "tier": tier,
            "clutch": "genius",
            "risk": "low",
            "model_override": "some/model",
        }))
        .expect("composer payload must deserialize");
        assert_eq!(params.tier.as_deref(), Some(tier));
        assert_eq!(
            params.cognitive_profile, None,
            "tier {tier:?} must not set cognitive_profile -- doing so would                  route the turn to mcp_infer_completion and kill tool calls"
        );
    }
}

#[test]
fn kb_scope_mask_controls_retrieval_legs_deterministically() {
    let off = KbScopeMask::from_kb_scope(Some("off"));
    assert!(!off.enabled && !off.rrf && !off.memory && !off.knowledge && !off.chunks && !off.repo);

    let docs = KbScopeMask::from_kb_scope(Some("docs"));
    assert!(
        docs.enabled && !docs.rrf && !docs.memory && docs.knowledge && docs.chunks && !docs.repo
    );

    let code = KbScopeMask::from_kb_scope(Some("code"));
    assert!(
        code.enabled && code.rrf && !code.memory && !code.knowledge && !code.chunks && code.repo
    );

    let mem = KbScopeMask::from_kb_scope(Some("memory"));
    assert!(mem.enabled && !mem.rrf && mem.memory && !mem.knowledge && !mem.chunks && !mem.repo);

    let verified = KbScopeMask::from_kb_scope(Some("verified"));
    assert!(
        verified.enabled
            && verified.rrf
            && verified.memory
            && verified.knowledge
            && verified.chunks
            && !verified.repo
    );

    let auto = KbScopeMask::from_kb_scope(Some("auto"));
    assert!(auto.enabled && auto.rrf && auto.memory && auto.knowledge && auto.chunks && auto.repo);
    assert_eq!(auto, KbScopeMask::from_kb_scope(None));
}

#[tokio::test]
async fn persisted_session_chat_posture_round_trips_and_falls_back_to_default() {
    let db = VoxDb::connect(DbConfig::Memory)
        .await
        .expect("open in-memory db");
    let state = ServerState::new_test()
        .await
        .with_db_initialized(Arc::new(db))
        .await;
    let posture = PersistedSessionChatPosture {
        clutch: Some("free".into()),
        risk: Some("low".into()),
        tier: Some("local".into()),
        mode: Some("verify".into()),
        execution_mode: Some("chat".into()),
        oversight_level: Some("strict".into()),
        kb_scope: Some("code".into()),
        grounding_check_enabled: Some(true),
        compaction_strategy: Some("conservative".into()),
    };
    persist_session_chat_posture(&state, Some("sess-live"), &posture).await;
    let loaded = load_persisted_session_chat_posture(&state, Some("sess-live")).await;
    assert_eq!(loaded, posture);
    // Any new session without an explicit session override inherits the global default posture.
    let inherited = load_persisted_session_chat_posture(&state, Some("sess-future")).await;
    assert_eq!(inherited, posture);
}

#[test]
fn governance_context_hint_formats_mode_oversight_and_dry_run() {
    assert_eq!(build_governance_context_hint(None, None, None), None);
    assert_eq!(
        build_governance_context_hint(Some("plan"), Some("4"), Some(true)).as_deref(),
        Some(
            "[EXECUTION GOVERNANCE]: mode=plan, oversight_level=4, dry_run=true (read-only simulation; do not mutate files)"
        )
    );
}

#[test]
fn should_run_auto_chat_preamble_skips_phatic_and_self_contained_code_turns() {
    assert_eq!(
        should_run_auto_chat_preamble("hi", None, false),
        (false, Some("phatic_or_conversational_greeting"))
    );
    assert_eq!(
        should_run_auto_chat_preamble("thanks!", Some("auto"), false),
        (false, Some("phatic_or_conversational_greeting"))
    );
    assert_eq!(
        should_run_auto_chat_preamble("refactor this function", None, true),
        (false, Some("self_contained_code_context"))
    );
    assert_eq!(
        should_run_auto_chat_preamble(
            "search across the repo for other callers of this function",
            None,
            true
        ),
        (true, None)
    );
    assert_eq!(
        should_run_auto_chat_preamble("hi", Some("memory"), false),
        (true, None),
        "explicit non-auto kb_scope must always run when enabled"
    );
    assert_eq!(
        should_run_auto_chat_preamble("how does the orchestrator route free models?", None, false),
        (true, None)
    );
    assert_eq!(
        should_run_auto_chat_preamble(
            "um, define fn add_one open paren x colon int close paren arrow int open brace return x plus 1 close brace",
            None,
            false
        ),
        (false, Some("self_contained_speech_to_code"))
    );
    assert_eq!(
        should_run_auto_chat_preamble(
            "Fix the return type in:\n```vox\nfn calc(x: int) to int { return x + 1 }\n```",
            None,
            false
        ),
        (false, Some("self_contained_inline_code_snippet"))
    );
}

#[tokio::test]
async fn system_prompt_contains_single_rider_and_is_day_stable() {
    let state = test_state();
    let p1 = build_system_prompt_with_skill(&state, None, None, None, &[]).await;
    let p2 = build_system_prompt_with_skill(&state, None, None, None, &[]).await;
    assert_eq!(
        p1, p2,
        "build_system_prompt_with_skill must produce a byte-identical prefix across turns within the same calendar day"
    );
    let rider_occurrences = p1.matches(ANTI_LAZINESS_RIDER).count();
    assert_eq!(
        rider_occurrences, 1,
        "system prompt must include ANTI_LAZINESS_RIDER exactly once, got {rider_occurrences}"
    );
}

#[test]
fn compute_turn_cost_usd_applies_cache_read_discount_and_respects_wire_cost() {
    let mut spec = model_spec(ProviderType::Anthropic, "anthropic/claude-sonnet-4.6");
    spec.cost_per_1k_input = 0.003;
    spec.cost_per_1k_output = 0.015;
    spec.cache_read_cost_per_1k = 0.0003;
    spec.supports_prompt_caching = true;

    // Wire cost present -> returned directly.
    assert_eq!(
        compute_turn_cost_usd(&spec, false, Some(0.042), 2000, 500, 1500),
        Some(0.042)
    );
    // No wire cost, 2000 prompt tokens (1500 cached, 500 uncached) + 500 output tokens:
    // uncached: 0.5 * 0.003 = 0.0015
    // cached:   1.5 * 0.0003 = 0.00045
    // output:   0.5 * 0.015 = 0.0075
    // total = 0.00945
    let fallback = compute_turn_cost_usd(&spec, false, None, 2000, 500, 1500).unwrap();
    assert!((fallback - 0.00945).abs() < 1e-8, "got {fallback}");

    assert_eq!(
        task_category_for_spoke_slug(Some("rust_workspace")),
        "CodeGen"
    );
    assert_eq!(
        task_category_for_spoke_slug(Some("mcp_tool_chain")),
        "ToolUse"
    );
}
