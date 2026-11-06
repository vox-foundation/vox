use super::*;
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

fn test_state() -> ServerState {
    let cfg = OrchestratorConfig::for_testing();
    let orch_cfg = cfg.clone();
    let groups = AffinityGroupRegistry::new(vec![]);
    let session_cfg = SessionConfig {
        persist: false,
        sessions_dir: std::env::temp_dir().join("vox-mcp-agent-loop-test-sessions"),
        ..SessionConfig::default()
    };
    let session_manager = SessionManager::new(session_cfg).expect("session manager");
    let repository = RepositoryContext {
        root: PathBuf::from("."),
        git_root: None,
        repository_id: "agent-loop-test".into(),
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

fn test_config(base_url: String) -> LlmConfig {
    LlmConfig {
        provider: "openrouter".into(),
        model: "test-model".into(),
        cost_per_1k: None,
        base_url: Some(base_url),
        api_key: Some("test-key".into()),
        temperature: None,
        top_p: None,
        max_tokens: None,
        response_format: None,
        tools: None,
        tool_choice: None,
        timeout_ms: None,
        telemetry_session_id: None,
        telemetry_user_id: None,
        telemetry_task_category: None,
        telemetry_strength_tag: None,
        telemetry_trace_id: None,
        telemetry_attempt_number: None,
        telemetry_skip_interaction: true,
        seed: None,
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

fn tool_call_response_body() -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "vox_git_status",
                        "arguments": "{}",
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    })
}

/// A single-turn response with no tool_calls must return immediately with that
/// text, without ever touching tool dispatch.
#[tokio::test]
async fn no_tool_calls_returns_immediately_with_final_text() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(plain_response_body("hello, no tools needed here")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        None,
        vec![],
        "system prompt".to_string(),
        "hi".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    assert_eq!(outcome.final_text, "hello, no tools needed here");
    assert_eq!(outcome.tool_calls_made, 0);
    assert!(!outcome.hit_iteration_limit);
}

/// A response with tool_calls must dispatch through `handle_tool_call_with_mode`
/// (here pointed at the real, read-only, side-effect-free `vox_git_status` tool)
/// and the *next* model call must receive a `role: "tool"` message correlated by
/// `tool_call_id` — proved by asserting on the second request body the mock
/// server actually received.
#[tokio::test]
async fn tool_call_dispatches_and_feeds_result_back_with_matching_call_id() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response_body()))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(plain_response_body("done, saw the tool result")),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        None,
        vec![],
        "system prompt".to_string(),
        "what's the git status?".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    assert_eq!(outcome.final_text, "done, saw the tool result");
    assert_eq!(outcome.tool_calls_made, 1);
    assert!(!outcome.hit_iteration_limit);

    // Inspect the second request the mock server received: it must carry a
    // `role: "tool"` message with `tool_call_id: "call_1"` correlating back to
    // the call the first response requested.
    let requests = server.received_requests().await.expect("received requests");
    assert_eq!(requests.len(), 2, "expected exactly two model round-trips");
    let second_body: serde_json::Value =
        serde_json::from_slice(&requests[1].body).expect("second request body is JSON");
    let msgs = second_body["messages"].as_array().expect("messages array");
    let tool_msg = msgs
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("a role:tool message must be present in the follow-up request");
    assert_eq!(tool_msg["tool_call_id"], "call_1");
    assert_eq!(tool_msg["name"], "vox_git_status");
}

#[tokio::test]
async fn tool_call_emits_a_verifiable_tool_receipt_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response_body()))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(plain_response_body("done, saw the tool result")),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        None,
        vec![],
        "system prompt".to_string(),
        "what's the git status?".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    assert_eq!(outcome.final_text, "done, saw the tool result");
    assert_eq!(outcome.tool_calls_made, 1);
    assert!(!outcome.hit_iteration_limit);

    let receipt_events: Vec<&serde_json::Value> = outcome
        .events
        .iter()
        .filter(|e| e["kind"] == "tool_receipt")
        .collect();
    assert_eq!(
        receipt_events.len(),
        1,
        "outcome.events must hold exactly one tool_receipt event"
    );
    let ev = receipt_events[0];
    assert_eq!(ev["tool"], "vox_git_status");
    let receipt_id = ev["receipt_id"]
        .as_str()
        .expect("receipt_id must be a string");
    assert!(!receipt_id.is_empty());
    assert_eq!(ev["fulfilled"], true);
    assert_eq!(ev["verified"], true);
    assert!(
        state.orchestrator.verify_tool_receipt(receipt_id),
        "state.orchestrator must verify the receipt id"
    );
}

/// Recursion-safety regression: `vox_chat_message` (and any other `vox_chat_*`
/// tool) must never appear in the `tools` array sent to the model from within
/// `run_agent_turn`. `select_tools_for_turn`'s general-purpose skill-permission
/// filter (`skill_permissions::is_skill_infrastructure_tool`) whitelists
/// `vox_chat_*` through *skill* restrictions — correct for other, non-recursive
/// callers — so without an explicit exclusion here, a model could be offered
/// `vox_chat_message` as a callable tool, dispatch it via
/// `handle_tool_call_with_mode`, and re-enter `chat_message ->
/// try_run_agent_turn -> run_agent_turn`, where each re-entrant call gets its
/// own fresh `max_iterations` budget (no real recursion-depth bound).
#[tokio::test]
async fn vox_chat_message_is_never_offered_as_a_tool() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body("no tools needed")),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    run_agent_turn(
        &state,
        None,
        vec![],
        "system prompt".to_string(),
        "hi".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    let requests = server.received_requests().await.expect("received requests");
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value =
        serde_json::from_slice(&requests[0].body).expect("request body is JSON");
    let tools = body
        .get("tools")
        .and_then(|t| t.as_array())
        .expect("tools array must be present");
    assert!(
        !tools.is_empty(),
        "sanity check: the turn should still offer some non-chat tools"
    );
    assert!(
        tools.iter().all(|t| {
            let name = t["function"]["name"].as_str().unwrap_or_default();
            !name.starts_with("vox_chat_")
        }),
        "no vox_chat_* tool (e.g. vox_chat_message) may ever be offered inside \
         run_agent_turn — doing so would allow unbounded re-entrant recursion; \
         tools sent: {tools:?}"
    );
}

/// If the model always returns tool_calls, the loop must still terminate at
/// `max_iterations` rather than looping forever — the hard safety bound
/// described on [`DEFAULT_MAX_ITERATIONS`] must be real.
#[tokio::test]
async fn max_iterations_bound_actually_stops_the_loop() {
    let server = MockServer::start().await;
    // Every response requests a tool call — never a final answer.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response_body()))
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let max_iterations = 3;
    let outcome = run_agent_turn(
        &state,
        None,
        vec![],
        "system prompt".to_string(),
        "loop forever please".to_string(),
        None,
        None,
        config,
        max_iterations,
        false,
    )
    .await
    .expect("run_agent_turn should return Ok even when the bound is hit");

    assert!(
        outcome.hit_iteration_limit,
        "loop must report that it stopped due to the iteration bound"
    );
    // One tool call dispatched per iteration.
    assert_eq!(outcome.tool_calls_made, max_iterations);

    let requests = server.received_requests().await.expect("received requests");
    assert_eq!(
        requests.len(),
        max_iterations,
        "loop must make exactly max_iterations model round-trips, not hang"
    );
}

/// A repeated-error tool call body: the model keeps calling an unknown tool
/// name (so `handle_tool_call_with_mode` deterministically fails with
/// `Error: Unknown tool: ...`, without needing any real side-effecting tool
/// to error) with identical arguments every time — exactly the pattern
/// [`super::harness_issue_scorer::HarnessIssueScorer`] is designed to flag.
fn repeated_error_tool_call_response_body() -> serde_json::Value {
    serde_json::json!({
        "id": "chatcmpl-test",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_1",
                    "type": "function",
                    "function": {
                        "name": "vox_definitely_not_a_real_tool",
                        "arguments": "{}",
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    })
}

/// Proves the `harness_issue_detection_enabled` config gate actually gates
/// something: with it turned OFF, feeding `run_agent_turn` enough
/// error-shaped tool results to normally cross the scorer's `THRESHOLD`
/// must not produce any `scientia_harness_issues` row. Uses a real
/// in-memory `VoxDb` (unlike `test_state()`'s hermetic, DB-less default) so
/// the assertion is a genuine "no row was written" check, not just "no
/// panic occurred".
#[tokio::test]
async fn detection_disabled_gate_prevents_any_harness_issue_row() {
    let server = MockServer::start().await;
    // Every response requests the same failing tool call, enough times to
    // cross THRESHOLD (3) if the scorer were active.
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(repeated_error_tool_call_response_body()),
        )
        .mount(&server)
        .await;

    let mut state = test_state();
    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
        .await
        .expect("open in-memory db");
    state.db = Some(std::sync::Arc::new(db));

    {
        let cfg_handle = state.orchestrator.config_handle();
        let mut cfg = cfg_handle.write().expect("config lock");
        cfg.harness_issue_detection_enabled = false;
    }

    let config = test_config(format!("{}/chat/completions", server.uri()));
    let max_iterations = 5;
    let outcome = run_agent_turn(
        &state,
        Some("gate-test-session"),
        vec![],
        "system prompt".to_string(),
        "keep hitting the same broken tool".to_string(),
        None,
        None,
        config,
        max_iterations,
        false,
    )
    .await
    .expect("run_agent_turn should succeed even though every tool call errors");

    assert_eq!(outcome.tool_calls_made, max_iterations);

    // Give the (should-never-have-been-spawned) fire-and-forget judge task a
    // moment to run, in case the gate were broken and it fired anyway.
    // drift-allow(timeout-literal,duplicate-numeric-literal): test fixture duration, not an HTTP request timeout; a shared constant needs a vox-config edge
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let db = state.db.as_ref().expect("db attached");
    let rows = db
        .list_harness_issues(None, None, 10)
        .await
        .expect("list_harness_issues");
    assert!(
        rows.is_empty(),
        "harness_issue_detection_enabled = false must prevent any \
         scientia_harness_issues row from being written, got {rows:?}"
    );
}

/// The `vox harness eval` `agent-loop-terminates` golden task body itself must
/// succeed — i.e. the check it performs (loop terminates at the iteration
/// bound against a model that always requests tools) must actually hold.
/// Only compiled with `--features eval-gate` (matching the function under
/// test) — run `cargo test -p vox-orchestrator-mcp --lib --features
/// eval-gate` to exercise it.
#[cfg(feature = "eval-gate")]
#[tokio::test]
async fn eval_gate_agent_loop_terminates_check_reports_ok_when_loop_is_bounded() {
    let result = eval_gate_agent_loop_terminates_check().await;
    assert!(result.is_ok(), "{result:?}");
}

fn model_spec(provider_type: vox_orchestrator::models::ProviderType, id: &str) -> ModelSpec {
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
        pricing_source: vox_orchestrator::models::spec::PricingSource::Bootstrap,
        is_free: false,
        strengths: Vec::new(),
        capabilities: vox_orchestrator::models::ModelCapabilities::default(),
        supported_parameters: Vec::new(),
    }
}

/// Task 1.3d mapper test: an OpenRouter-routed [`ModelSpec`] must convert to a
/// working [`LlmConfig`] (this is the case `message.rs` now routes through
/// `run_agent_turn` instead of `mcp_infer_completion`).
#[test]
fn model_spec_to_llm_config_maps_openrouter() {
    let spec = model_spec(ProviderType::OpenRouter, "openrouter/some-model");
    let cfg = model_spec_to_llm_config(&spec).expect("openrouter must map");
    assert_eq!(cfg.provider, "openrouter");
    assert_eq!(cfg.model, "openrouter/some-model");
}

/// Task 5: GoogleDirect and Anthropic map to OpenRouter LlmConfig, preserving
/// agent loop tool execution.
#[test]
fn model_spec_to_llm_config_maps_google_direct_and_anthropic() {
    let spec_g = model_spec(ProviderType::GoogleDirect, "google/gemini-2.0-flash");
    let cfg_g = model_spec_to_llm_config(&spec_g).expect("google direct must map");
    assert_eq!(cfg_g.provider, "openrouter");
    assert_eq!(cfg_g.model, "google/gemini-2.0-flash");

    let spec_a = model_spec(ProviderType::Anthropic, "anthropic/claude-3-5-sonnet");
    let cfg_a = model_spec_to_llm_config(&spec_a).expect("anthropic must map");
    assert_eq!(cfg_a.provider, "openrouter");
    assert_eq!(cfg_a.model, "anthropic/claude-3-5-sonnet");

    let spec_g_unslugged = model_spec(ProviderType::GoogleDirect, "gemini-2.0-flash");
    let cfg_g_unslugged =
        model_spec_to_llm_config(&spec_g_unslugged).expect("google direct unslugged must map");
    assert_eq!(cfg_g_unslugged.model, "google/gemini-2.0-flash");

    let spec_a_unslugged = model_spec(ProviderType::Anthropic, "claude-3-5-sonnet");
    let cfg_a_unslugged =
        model_spec_to_llm_config(&spec_a_unslugged).expect("anthropic unslugged must map");
    assert_eq!(cfg_a_unslugged.model, "anthropic/claude-3-5-sonnet");

    let spec_mistral = model_spec(ProviderType::Mistral, "mistral/mistral-large");
    let cfg_mistral =
        model_spec_to_llm_config(&spec_mistral).expect("mistral must map for tool loop");
    assert_eq!(cfg_mistral.provider, "mistral");

    let spec_groq = model_spec(ProviderType::Groq, "llama-3.3-70b-versatile");
    let cfg_groq = model_spec_to_llm_config(&spec_groq).expect("groq must map for tool loop");
    assert_eq!(cfg_groq.provider, "groq");

    let spec_deepseek = model_spec(ProviderType::DeepSeek, "deepseek-chat");
    let cfg_deepseek =
        model_spec_to_llm_config(&spec_deepseek).expect("deepseek must map for tool loop");
    assert_eq!(cfg_deepseek.provider, "deepseek");

    let spec_unhandled = model_spec(
        ProviderType::Custom("custom-endpoint".into()),
        "custom/model",
    );
    assert!(model_spec_to_llm_config(&spec_unhandled).is_none());
}

/// Catches: restoring the `?` on the OllamaUrl secret lookup. With it,
/// an unset OLLAMA_URL makes this return None and the caller falls back
/// to the NON-TOOL mcp_infer_completion path -- an agentic turn silently
/// loses its tools with no error anywhere.
#[test]
#[allow(unsafe_code)] // test-only env cleanup under the crate-wide lock; no cross-test hazard
fn ollama_keeps_tool_calling_when_the_url_secret_is_unset() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    // SAFETY: restored below; no other test here reads these keys.
    unsafe {
        std::env::remove_var("OLLAMA_URL");
        std::env::remove_var("POPULI_URL");
        std::env::remove_var("VOX_POPULI_LOCAL_OLLAMA_URL");
    }
    vox_config::snapshot::bump(&["OLLAMA_URL", "POPULI_URL", "VOX_POPULI_LOCAL_OLLAMA_URL"]);
    let spec = model_spec(ProviderType::Ollama, "qwen3:8b");
    let cfg = model_spec_to_llm_config(&spec)
        .expect("Ollama must map to a config, not None -- None disables tools");
    assert_eq!(cfg.provider, "ollama");
    assert_eq!(
        cfg.base_url.as_deref(),
        Some("http://localhost:11434/v1/chat/completions"),
        "must fall back to the config SSOT default"
    );
}

/// Task B2 (MENS end-to-end completion, Route A): a VoxLocal-routed turn
/// must send tool schemas on the wire, dispatch a requested call, and feed
/// the RESULT back as a second round-trip — the exact gap `.is_some()`
/// cannot see (a `VoxLocal => Some(LlmConfig::default())` stub would pass a
/// weaker assertion and then fail against a nonexistent base URL).
#[tokio::test]
#[allow(unsafe_code)] // env var mutation under a process-wide lock, like existing env tests
#[allow(clippy::await_holding_lock)] // intentional: the std Mutex must stay held for the
// entire test body to serialize access to the process-global VOX_LOCAL_ENDPOINT env var
// against any other test that might touch it; this test never runs concurrently with
// itself, so there's no deadlock risk from the held guard.
async fn a_mens_turn_dispatches_a_tool_and_feeds_the_result_back() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let server = MockServer::start().await;
    // Turn 1: the model requests a tool. Turn 2: it answers using the result.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response_body()))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(plain_response_body("done")))
        .mount(&server)
        .await;

    let prev = std::env::var("VOX_LOCAL_ENDPOINT").ok();
    // SAFETY: restored below under the same env lock; no other test in this
    // module reads VOX_LOCAL_ENDPOINT.
    unsafe {
        std::env::set_var("VOX_LOCAL_ENDPOINT", server.uri());
    }

    let spec = model_spec(ProviderType::VoxLocal, "mens/demo-run");
    let cfg = model_spec_to_llm_config(&spec).expect("VoxLocal must map");
    assert!(
        cfg.base_url
            .as_deref()
            .is_some_and(|u| u.ends_with("/v1/chat/completions")),
        "the arm must point at chat-completions, not /generate: {:?}",
        cfg.base_url
    );
    assert_eq!(
        cfg.max_tokens,
        Some(spec.max_tokens),
        "the catalog's claim must reach the wire"
    );

    let state = test_state();
    let out = run_agent_turn(
        &state,
        Some("sess-b2"),
        vec![],
        "system prompt".to_string(),
        "what's the git status?".to_string(),
        None,
        None,
        cfg,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    match prev {
        Some(v) => unsafe { std::env::set_var("VOX_LOCAL_ENDPOINT", v) },
        None => unsafe { std::env::remove_var("VOX_LOCAL_ENDPOINT") },
    }

    let requests = server.received_requests().await.expect("received requests");
    let bodies: Vec<serde_json::Value> = requests
        .iter()
        .map(|r| serde_json::from_slice(&r.body).expect("json body"))
        .collect();
    assert_eq!(
        bodies.len(),
        2,
        "a tool call must produce a second round-trip"
    );
    assert!(
        bodies[0]["tools"].as_array().is_some_and(|t| !t.is_empty()),
        "tool schemas must be sent, or the model can never request one"
    );
    assert_eq!(
        bodies[1]["messages"].as_array().unwrap().last().unwrap()["role"],
        "tool",
        "the tool RESULT must be fed back — this is what is_some() cannot see"
    );
    assert_eq!(out.tool_calls_dispatched, 1);
}

/// Task B2 salvage policy step 2: a tool call embedded as prose (a fenced
/// ```json block) instead of structured `tool_calls` — the expected
/// failure mode of a small model — must still dispatch, tagged
/// `tool_call_salvaged: true` so a salvage is never indistinguishable from
/// a clean structured call.
#[tokio::test]
async fn a_near_miss_tool_call_is_salvaged_and_tagged_not_hard_failed() {
    let server = MockServer::start().await;
    // `vox_answer_question` (unlike `vox_git_status`) is actually within the
    // turn's offered-tool set (alphabetical registry order, capped at
    // `DEFAULT_MAX_TOOLS` — see `select_tools_for_turn`), which the salvage
    // policy requires before treating any text-embedded JSON as a real call.
    let near_miss = plain_response_body(
        "I'll use vox_answer_question. ```json\n{\"name\":\"vox_answer_question\",\"arguments\":{}}\n```",
    );
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(near_miss))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(plain_response_body("done, saw the tool result")),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let out = run_agent_turn(
        &state,
        Some("sess-salvage"),
        vec![],
        "system prompt".to_string(),
        "what's the git status?".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed even with a near-miss tool call");

    assert_eq!(
        out.tool_calls_dispatched, 1,
        "a near-miss must still dispatch"
    );
    assert_eq!(
        out.telemetry.get("tool_call_salvaged"),
        Some(&serde_json::Value::Bool(true)),
        "a salvage must be visible, not indistinguishable from a clean structured call"
    );
}

/// Task B2 salvage policy step 3: ordinary prose with no tool-shaped JSON
/// anywhere completes as a plain answer — "the model declined to call a
/// tool" is not a turn failure.
#[tokio::test]
async fn no_tool_shaped_text_completes_as_a_plain_answer_not_a_turn_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body(
                "The git status looks clean, nothing to report.",
            )),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let out = run_agent_turn(
        &state,
        Some("sess-plain"),
        vec![],
        "system prompt".to_string(),
        "what's the git status?".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed with a plain-text answer");

    assert_eq!(out.tool_calls_dispatched, 0);
    assert!(
        out.reply_text.is_some(),
        "declining to call a tool must not fail the turn"
    );
}

/// Task 1.3d end-to-end proof that F24 is fixed for the mapped-provider case:
/// calling the real, live `vox_chat_message` handler (`chat_message`, exactly
/// what the MCP server dispatches to) for an OpenRouter-routed model results in
/// an actual HTTP request that carries a non-empty `tools` array — i.e. a real
/// chat message now causes a real tool-bearing request, closing the gap
/// Finding F24 named ("no code path in Vox ever passes tools to a model").
#[tokio::test]
#[allow(unsafe_code)] // env var mutation under a process-wide lock, like existing env tests
#[allow(clippy::await_holding_lock)] // intentional: the std Mutex must stay held for the
// entire test body to serialize access to the process-global OPENROUTER_BASE_URL /
// OPENROUTER_API_KEY env vars against any other test that might touch them; this test
// never runs concurrently with itself, so there's no deadlock risk from the held guard.
async fn chat_message_default_path_sends_tools_bearing_request() {
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
    let model_id = "test-openrouter-model";
    {
        let handle = state.orchestrator.models_handle();
        let mut registry = handle.write().expect("models registry lock");
        registry.register(model_spec(ProviderType::OpenRouter, model_id));
    }
    // Sticky override: deterministically select the model we just registered,
    // bypassing scorer heuristics that are out of scope for this test.
    *state.mcp_chat_model_override.write() = Some(model_id.to_string());

    let params: crate::chat_tools::params::ChatMessageParams =
        serde_json::from_value(serde_json::json!({ "prompt": "hello there" }))
            .expect("chat message params");

    let response_json = super::super::message::chat_message(&state, params).await;

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

    assert!(
        !response_json.contains("\"error\""),
        "chat_message should succeed via the mapped agent-loop path: {response_json}"
    );

    let requests = server.received_requests().await.expect("received requests");
    assert!(
        !requests.is_empty(),
        "vox_chat_message must have made at least one HTTP request to the model"
    );
    // Find the actual chat-completion POST body sent for `model_id` (parses to
    // JSON with a `messages` array and `model` == the resolved chat model)
    // rather than assuming it's `requests[0]` or the first request with a
    // `messages` array — the same mock server also receives unrelated internal
    // LLM calls (e.g. the autonomous-research "gap analyst" follow-up-query
    // generator, which runs against `openrouter/auto` and never carries tools)
    // triggered earlier in `chat_message`, before the real tool-bearing
    // completion request this test is asserting on.
    let body = requests
        .iter()
        .find_map(|r| {
            let v: serde_json::Value = serde_json::from_slice(&r.body).ok()?;
            v.get("messages")?.as_array()?;
            if v.get("model").and_then(|m| m.as_str()) != Some(model_id) {
                return None;
            }
            Some(v)
        })
        .expect("no request with a JSON `messages` body for the resolved chat model was received");
    let tools = body
        .get("tools")
        .and_then(|t| t.as_array())
        .expect("real vox_chat_message call must send a non-null `tools` array — F24");
    assert!(
        !tools.is_empty(),
        "tools array must be non-empty — a real chat message must offer real tools"
    );
}

/// Regression test: `params.temperature`/`params.top_p` must reach the wire
/// request on the mapped (`run_agent_turn`) path exactly as they already do on
/// the `call_llm` fallback path — `try_run_agent_turn` must not silently drop
/// a caller's sampling overrides for the providers this task newly wires up.
#[tokio::test]
#[allow(unsafe_code)] // env var mutation under a process-wide lock, like existing env tests
#[allow(clippy::await_holding_lock)] // see chat_message_default_path_sends_tools_bearing_request
async fn chat_message_default_path_honors_temperature_and_top_p() {
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
    let model_id = "test-openrouter-model-temp";
    {
        let handle = state.orchestrator.models_handle();
        let mut registry = handle.write().expect("models registry lock");
        registry.register(model_spec(ProviderType::OpenRouter, model_id));
    }
    *state.mcp_chat_model_override.write() = Some(model_id.to_string());

    let params: crate::chat_tools::params::ChatMessageParams = serde_json::from_value(
        serde_json::json!({ "prompt": "hello there", "temperature": 0.11, "top_p": 0.42 }),
    )
    .expect("chat message params");

    let response_json = super::super::message::chat_message(&state, params).await;

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

    assert!(
        !response_json.contains("\"error\""),
        "chat_message should succeed via the mapped agent-loop path: {response_json}"
    );

    let requests = server.received_requests().await.expect("received requests");
    // See `chat_message_default_path_sends_tools_bearing_request` — the mock
    // server also receives the autonomous-research "gap analyst" follow-up-query
    // request (`model: "openrouter/auto"`, no sampling overrides) before the
    // real completion request for `model_id`, so the first `messages`-bearing
    // body is not necessarily the one under test.
    let body = requests
        .iter()
        .find_map(|r| {
            let v: serde_json::Value = serde_json::from_slice(&r.body).ok()?;
            v.get("messages")?.as_array()?;
            if v.get("model").and_then(|m| m.as_str()) != Some(model_id) {
                return None;
            }
            Some(v)
        })
        .expect("no request with a JSON `messages` body for the resolved chat model was received");
    assert_eq!(
        body.get("temperature").and_then(serde_json::Value::as_f64),
        Some(0.11_f64),
        "params.temperature must reach the wire request on the mapped path: {body}"
    );
    assert_eq!(
        body.get("top_p").and_then(serde_json::Value::as_f64),
        Some(0.42_f64),
        "params.top_p must reach the wire request on the mapped path: {body}"
    );
}

// -----------------------------------------------------------------------
// Task G1: sync-path token streaming
// -----------------------------------------------------------------------

/// `stream_tokens: true` against a real SSE response: `TokenStreamed`
/// events carrying the turn's `session_id` must appear on the existing
/// agent-events bus (`state.orchestrator.event_bus()` — the same bus
/// `orch_daemon` forwards to the GUI) as chunks arrive, concatenating to
/// the same text `run_agent_turn`'s own return value reports, and exactly
/// one HTTP round-trip must occur (no non-streaming fallback needed).
#[tokio::test]
async fn stream_tokens_emits_token_streamed_events_with_session_id() {
    let server = MockServer::start().await;
    let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi \"}}]}\n\
               data: {\"choices\":[{\"delta\":{\"content\":\"there\"}}]}\n\
               data: [DONE]\n\n";
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse),
        )
        .expect(1)
        .mount(&server)
        .await;

    let state = test_state();
    let mut rx = state.orchestrator.event_bus().subscribe();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        Some("sess-g1"),
        vec![],
        "system prompt".to_string(),
        "hi".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        true,
    )
    .await
    .expect("run_agent_turn should succeed");

    assert_eq!(outcome.final_text, "Hi there");
    assert_eq!(outcome.tool_calls_made, 0);
    assert!(
        outcome.latency_ms.is_some(),
        "Task M3: streaming latency_ms must no longer be the hardcoded 0"
    );
    assert!(
        outcome.ttft_ms.is_some(),
        "Task M3: a genuinely streamed reply must report a real time-to-first-token"
    );
    assert!(
        outcome.tpot_ms.is_some(),
        "Task M3: a genuinely streamed reply with completion tokens must report tpot_ms"
    );

    let mut streamed_text = String::new();
    let mut saw_session_id = false;
    while let Ok(evt) = rx.try_recv() {
        if let AgentEventKind::TokenStreamed {
            text, session_id, ..
        } = evt.kind
        {
            streamed_text.push_str(&text);
            assert_eq!(
                session_id.as_deref(),
                Some("sess-g1"),
                "every TokenStreamed frame from the sync path must carry this turn's session_id"
            );
            saw_session_id = true;
        }
    }
    assert!(
        saw_session_id,
        "expected at least one TokenStreamed event on the bus"
    );
    assert_eq!(streamed_text, "Hi there");
}

/// `stream_tokens: true` against a plain (non-SSE) JSON response — the
/// observable shape of a turn where the model requested a tool call
/// instead of streaming text (`vox_llm_egress::wire::stream_once` doesn't
/// parse `delta.tool_calls`, so that turn streams zero content chunks).
/// Must fall back to one ordinary non-streaming `llm_chat` call and reach
/// the exact same outcome `stream_tokens: false` would, proving tool
/// dispatch is unaffected by this task.
#[tokio::test]
async fn stream_tokens_falls_back_to_non_streaming_when_stream_yields_no_content() {
    let server = MockServer::start().await;
    // `stream_tokens_falls_back_to_non_streaming_when_stream_yields_no_content`:
    // iteration 1 returns a non-SSE JSON body with tool_calls (parsed in 1 request),
    // iteration 2 returns a non-SSE JSON body with plain text (parsed in 1 request).
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_response_body()))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(plain_response_body("done, saw the tool result")),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        Some("sess-g1-fallback"),
        vec![],
        "system prompt".to_string(),
        "what's the git status?".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        true,
    )
    .await
    .expect("run_agent_turn should succeed via the non-streaming fallback");

    assert_eq!(outcome.final_text, "done, saw the tool result");
    assert_eq!(
        outcome.tool_calls_made, 1,
        "the tool call from the first (streaming-attempted, empty) iteration must still \
         dispatch via the non-streaming fallback"
    );

    let requests = server.received_requests().await.expect("received requests");
    assert_eq!(
        requests.len(),
        4,
        "non-SSE mock responses fall back to non-streaming llm_chat per iteration"
    );
}

#[tokio::test]
async fn stream_tokens_parses_sse_delta_tool_calls_in_single_pass_without_duplicate_post() {
    let server = MockServer::start().await;
    let sse_tool_call = "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_sse_1\",\"type\":\"function\",\"function\":{\"name\":\"vox_git_status\",\"arguments\":\"{\"}}]}}]}\n\
                         data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"}\"}}]}}]}\n\
                         data: [DONE]\n\n";
    let sse_final = "data: {\"choices\":[{\"delta\":{\"content\":\"Single-pass SSE tool turn complete.\"}}]}\n\
                     data: [DONE]\n\n";
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_tool_call),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_final),
        )
        .up_to_n_times(1)
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        Some("sess-sse-tool-single-pass"),
        vec![],
        "system prompt".to_string(),
        "check git status".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        true,
    )
    .await
    .expect("single-pass SSE tool call turn must succeed");

    assert_eq!(outcome.final_text, "Single-pass SSE tool turn complete.");
    assert_eq!(outcome.tool_calls_made, 1);
    let requests = server.received_requests().await.expect("received requests");
    assert_eq!(
        requests.len(),
        2,
        "2 iterations (1 SSE tool call + 1 SSE final answer) must make 2 HTTP requests, never 4"
    );
}

#[tokio::test]
async fn agent_loop_cannot_approve_floored_park() {
    let server = MockServer::start().await;
    let state = test_state();

    let s = state.clone();
    let mut call = tokio::spawn(async move {
        crate::dispatch::handle_tool_call_with_mode(
            &s,
            "vox_skill_install",
            serde_json::json!({}),
            Some("accept_all"),
        )
        .await
    });

    // drift-allow(timeout-literal,duplicate-numeric-literal): local test approval poll deadline
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut approval_id = None;
    while tokio::time::Instant::now() < deadline {
        let list = state.pending_approvals.list();
        if let Some(entry) = list.iter().find(|e| e.tool == "vox_skill_install") {
            approval_id = Some(entry.approval_id.clone());
            break;
        }
        // drift-allow(timeout-literal,duplicate-numeric-literal): local test approval poll interval
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let aid = approval_id.expect("vox_skill_install should park");

    let resolve_tool_body = serde_json::json!({
        "id": "chatcmpl-test",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_resolve_1",
                    "type": "function",
                    "function": {
                        "name": "vox_resolve_approval",
                        "arguments": serde_json::to_string(&serde_json::json!({
                            "approval_id": aid,
                            "outcome": "approve",
                        })).unwrap(),
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    });

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(resolve_tool_body))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(plain_response_body("done")))
        .mount(&server)
        .await;

    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        None,
        vec![],
        "system prompt".to_string(),
        "please approve".to_string(),
        Some("accept_all"),
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    assert_eq!(outcome.tool_calls_made, 1);

    let requests = server.received_requests().await.expect("received requests");
    assert_eq!(requests.len(), 2, "expected exactly two model round-trips");
    let second_body: serde_json::Value =
        serde_json::from_slice(&requests[1].body).expect("second request body is JSON");
    let msgs = second_body["messages"].as_array().expect("messages array");
    let tool_msg = msgs
        .iter()
        .find(|m| m["role"] == "tool")
        .expect("tool message exists");
    let tool_content = tool_msg["content"]
        .as_str()
        .expect("tool content is string");
    assert!(
        tool_content.contains("FLOOR_SELF_APPROVAL_REFUSED"),
        "expected FLOOR_SELF_APPROVAL_REFUSED in tool response, got: {tool_content}"
    );

    assert!(
        state
            .pending_approvals
            .list()
            .iter()
            .any(|e| e.approval_id == aid),
        "parked approval should still be listed"
    );
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(
        !call.is_finished(),
        "spawned install should not finish while approval was refused"
    );

    // 5. Clean up as human would
    assert!(
        state
            .pending_approvals
            .resolve(&aid, vox_orchestrator::ApprovalOutcome::Rejected)
    );
    let raw = (&mut call)
        .await
        .expect("join")
        .expect("dispatch returns Ok");
    assert!(
        crate::server_state::tool_json_envelope_is_error(&raw),
        "spawned install should return error envelope on rejection"
    );
}

#[tokio::test]
async fn run_agent_turn_automatically_observes_and_repairs_broken_mermaid() {
    let server = MockServer::start().await;
    let broken_mermaid_reply = "Here is the architecture:\n```mermaid\nflowchart TD\n    subgraph Loop\n    A[Step 1 (Init) <br/> Ready] -- Yes (Iter < Max) --> B[Done]\n```\n";
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body(broken_mermaid_reply)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        None,
        vec![],
        "system prompt".to_string(),
        "show me a diagram".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    // Must have dispatched `vox_validate_mermaid` automatically and repaired the diagram.
    assert_eq!(outcome.tool_calls_made, 1);
    assert!(outcome.final_text.contains("A[\"Step 1 (Init) - Ready\"]"));
    assert!(outcome.final_text.contains("-- \"Yes (Iter < Max)\" -->"));
    assert!(outcome.final_text.contains("    end"));
    assert_eq!(
        outcome.telemetry.get("mermaid_repaired"),
        Some(&serde_json::json!(true))
    );
    assert!(
        outcome
            .events
            .iter()
            .any(|e| e["kind"] == "mermaid_repaired" && e["repaired"] == true),
        "expected mermaid_repaired event in outcome.events: {:?}",
        outcome.events
    );
    assert!(
        outcome
            .events
            .iter()
            .any(|e| e["kind"] == "tool_receipt" && e["tool_name"] == "vox_validate_mermaid"),
        "expected tool_receipt for vox_validate_mermaid in outcome.events: {:?}",
        outcome.events
    );
}

#[tokio::test]
async fn run_agent_turn_executes_read_only_tool_batch_in_parallel_and_records_telemetry() {
    let server = MockServer::start().await;
    let batch_tool_body = serde_json::json!({
        "id": "chatcmpl-batch",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_git",
                        "type": "function",
                        "function": {
                            "name": "vox_git_status",
                            "arguments": "{}",
                        },
                    },
                    {
                        "id": "call_repo",
                        "type": "function",
                        "function": {
                            "name": "vox_repo_status",
                            "arguments": "{}",
                        },
                    }
                ],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 12, "completion_tokens": 8},
    });
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(batch_tool_body))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body(
                "both read-only tools completed in parallel",
            )),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        Some("sess-parallel-batch"),
        vec![],
        "system prompt".to_string(),
        "Run a structured PowerShell check of workspace git status and repo status".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    assert_eq!(outcome.tool_calls_made, 2);
    assert_eq!(
        outcome.telemetry.get("parallel_tool_batches_executed"),
        Some(&serde_json::json!(1)),
        "read-only multi-tool batch must execute via futures::future::join_all"
    );
    assert_eq!(
        outcome.telemetry.get("classified_spoke"),
        Some(&serde_json::json!("structured_shell")),
        "turn must record its classified spoke in telemetry"
    );
}

#[tokio::test]
async fn run_agent_turn_synchronously_intervenes_on_verification_failure_escalates_and_harvests_auto_dpo()
 {
    let server = MockServer::start().await;
    // Iteration 1 & 2: model calls `vox_validate_mermaid` with empty `{}` (which returns
    // `"success":false` because neither `code` nor `markdown` is supplied). Iteration 3:
    // after synchronous `[HARNESS REPAIR INTERVENTION]`, model supplies valid `code` and succeeds.
    let bad_call_1 = serde_json::json!({
        "id": "chatcmpl-fail-1",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_fail_1",
                    "type": "function",
                    "function": {
                        "name": "vox_validate_mermaid",
                        "arguments": "{}",
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    });
    let bad_call_2 = serde_json::json!({
        "id": "chatcmpl-fail-2",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_fail_2",
                    "type": "function",
                    "function": {
                        "name": "vox_validate_mermaid",
                        "arguments": "{}",
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    });
    let repaired_call_3 = serde_json::json!({
        "id": "chatcmpl-repaired-3",
        "model": "test-model",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_ok_3",
                    "type": "function",
                    "function": {
                        "name": "vox_validate_mermaid",
                        "arguments": "{\"code\":\"flowchart TD\\n    A --> B\"}",
                    },
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    });

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(bad_call_1))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(bad_call_2))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repaired_call_3))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(plain_response_body("repaired tool call succeeded")),
        )
        .mount(&server)
        .await;

    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
        .await
        .expect("open in-memory db");
    let state = test_state().with_db_initialized(Arc::new(db)).await;
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        Some("sess-repair-dpo"),
        vec![],
        "system prompt".to_string(),
        "Dispatch MCP tool vox_git_status and verify schema".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed after repair");

    assert_eq!(outcome.tool_calls_made, 3);
    assert_eq!(
        outcome.telemetry.get("repair_loop_escalated"),
        Some(&serde_json::json!(true)),
        "repeated tool error must trigger synchronous repair escalation"
    );
    assert_eq!(
        outcome.telemetry.get("repair_deescalations"),
        Some(&serde_json::json!(1)),
        "clean repair iteration must de-escalate repair_escalated back to primary model (SOTA-CASCADE-DEESC-01)"
    );
    assert_eq!(
        outcome.telemetry.get("synchronous_repair_interventions"),
        Some(&serde_json::json!(1)),
        "synchronous repair intervention must fire on 2nd failure"
    );
    assert_eq!(
        outcome.telemetry.get("auto_dpo_pairs_harvested"),
        Some(&serde_json::json!(1)),
        "successful repair after failure must harvest a live Auto-DPO contrast pair"
    );
    assert_eq!(
        outcome.telemetry.get("auto_dpo_persisted_to_db"),
        Some(&serde_json::json!(true)),
        "harvested live Auto-DPO contrast pair must persist to VoxDb (SOTA-LIVE-DPO-02)"
    );
    assert!(
        outcome
            .telemetry
            .get("escalated_repair_model_id")
            .and_then(|v| v.as_str())
            .is_some(),
        "escalated repair iteration must resolve SelectionIntent::repair_loop model id (SOTA-LIVE-DPO-01)"
    );

    let requests = server.received_requests().await.expect("received requests");
    let third_body: serde_json::Value =
        serde_json::from_slice(&requests[2].body).expect("third request body is JSON");
    assert_eq!(
        third_body
            .get("temperature")
            .and_then(serde_json::Value::as_f64),
        Some(0.1_f64),
        "escalated repair iteration must lower temperature to 0.1"
    );
    let msgs = third_body["messages"].as_array().expect("messages array");
    let has_intervention = msgs.iter().any(|m| {
        m["role"] == "tool"
            && m["content"]
                .as_str()
                .is_some_and(|s| s.contains("[HARNESS REPAIR INTERVENTION"))
    });
    assert!(
        has_intervention,
        "tool message on escalated iteration must contain [HARNESS REPAIR INTERVENTION]"
    );
}

#[test]
fn extract_raw_utterance_strips_context_preambles_for_tool_ranking() {
    let assembled = "[ACTIVE FILE]: src/main.vox (line 10)\n[OPEN FILES]: src/main.vox, Cargo.toml\n\ncheck git status and diff";
    assert_eq!(
        extract_raw_utterance_for_tool_ranking(assembled),
        "check git status and diff"
    );
}

#[test]
fn cap_and_summarize_tool_output_bounds_oversized_payload_and_preserves_head_and_tail() {
    let head_marker = "HEAD_MARKER_START ";
    let tail_marker = " [INLINE VERIFICATION: syntax=ok] TAIL_MARKER_END";
    let middle = "x".repeat(8_000);
    let oversized = format!("{head_marker}{middle}{tail_marker}");
    let (capped, saved_tokens) = cap_and_summarize_tool_output("vox_read_file", &oversized);
    assert!(
        capped.len() < oversized.len(),
        "oversized tool output must be capped"
    );
    assert!(
        saved_tokens > 400,
        "expected >400 tokens saved, got {saved_tokens}"
    );
    assert!(capped.starts_with(head_marker), "head must be preserved");
    assert!(
        capped.ends_with(tail_marker),
        "tail (including inline verification block) must be preserved"
    );
    assert!(
        capped.contains("[INTRA-TURN TOOL OUTPUT CAPPED: tool=vox_read_file"),
        "capped output must carry blake3 digest marker"
    );
}

#[test]
fn mask_stale_intraturn_tool_messages_masks_old_iterations_and_preserves_recent() {
    let mut messages = vec![
        LlmChatMessage {
            role: "system".to_string(),
            content: "sys".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            content_parts: None,
        },
        LlmChatMessage {
            role: "tool".to_string(),
            content: format!("iter0 tool payload {}", "a".repeat(1_000)),
            name: None,
            tool_calls: None,
            tool_call_id: Some("call_0".to_string()),
            content_parts: None,
        },
        LlmChatMessage {
            role: "tool".to_string(),
            content: format!("iter1 tool payload {}", "b".repeat(1_000)),
            name: None,
            tool_calls: None,
            tool_call_id: Some("call_1".to_string()),
            content_parts: None,
        },
    ];
    let ledger = vec![
        (1usize, 0usize, "vox_read_file".to_string()),
        (2usize, 1usize, "vox_git_diff".to_string()),
    ];
    // At iteration 2, iter 0 is >= 2 iterations old (masked), while iter 1 is recent (preserved).
    let (masked_count, saved_tokens) =
        mask_stale_intraturn_tool_messages(&mut messages, &ledger, 2);
    assert_eq!(masked_count, 1);
    assert!(saved_tokens > 100);
    assert!(
        messages[1].content.starts_with(
            "[INTRA-TURN TOOL OBSERVATION MASKED: tool=vox_read_file, iter=0, blake3="
        )
    );
    assert!(messages[2].content.starts_with("iter1 tool payload "));
}

#[test]
fn compact_tool_def_for_rank_reduces_secondary_schema_bytes_by_at_least_30_pct() {
    let mut full_bytes = 0usize;
    let mut tiered_bytes = 0usize;
    for (idx, entry) in TOOL_REGISTRY
        .iter()
        .take(DEFAULT_RANKED_MAX_TOOLS)
        .enumerate()
    {
        let schema_val = tool_input_schema(entry.name);
        let full_def = compact_tool_def_for_rank(0, entry, schema_val.clone());
        let tiered_def = compact_tool_def_for_rank(idx, entry, schema_val);
        full_bytes += serde_json::to_string(&full_def).unwrap().len();
        tiered_bytes += serde_json::to_string(&tiered_def).unwrap().len();
    }
    assert!(
        tiered_bytes < full_bytes,
        "two-tier tool schema must be smaller than full schema ({tiered_bytes} vs {full_bytes})"
    );
    // Also verify secondary rank (>= 8) specifically saves >= 30% on rich schemas.
    let mut secondary_full = 0usize;
    let mut secondary_compact = 0usize;
    for entry in TOOL_REGISTRY
        .iter()
        .skip(PRIMARY_FULL_SCHEMA_TOOLS_CAP)
        .take(12)
    {
        let schema_val = tool_input_schema(entry.name);
        secondary_full +=
            serde_json::to_string(&compact_tool_def_for_rank(0, entry, schema_val.clone()))
                .unwrap()
                .len();
        secondary_compact += serde_json::to_string(&compact_tool_def_for_rank(
            PRIMARY_FULL_SCHEMA_TOOLS_CAP,
            entry,
            schema_val,
        ))
        .unwrap()
        .len();
    }
    let savings_pct =
        ((secondary_full.saturating_sub(secondary_compact)) as f64 / secondary_full as f64) * 100.0;
    assert!(
        savings_pct >= 30.0,
        "secondary tool schemas (rank >= 8) must compress by >= 30%, got {savings_pct:.1}% ({secondary_compact} vs {secondary_full})"
    );
}

#[tokio::test]
async fn run_agent_turn_deduplicates_identical_intraturn_read_only_tool_calls_and_invalidates_on_write()
 {
    let server = MockServer::start().await;
    // Iteration 1: read-only `vox_repo_status` with `{}` -> executes and memoizes.
    // Iteration 2: identical `vox_repo_status` with `{}` -> served from `read_tool_memo` (`deduped_read_hit: true`).
    // Iteration 3: final answer.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-read-1",
            "model": "test-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_repo_1",
                        "type": "function",
                        "function": {
                            "name": "vox_repo_status",
                            "arguments": "{}",
                        },
                    }],
                },
                "finish_reason": "tool_calls",
            }],
            "usage": {"prompt_tokens": 14, "completion_tokens": 6},
        })))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_response_body("done with deduped read")),
        )
        .mount(&server)
        .await;

    let state = test_state();
    let config = test_config(format!("{}/chat/completions", server.uri()));
    let outcome = run_agent_turn(
        &state,
        Some("sess-read-dedup"),
        vec![],
        "system prompt".to_string(),
        "check repo status twice".to_string(),
        None,
        None,
        config,
        DEFAULT_MAX_ITERATIONS,
        false,
    )
    .await
    .expect("run_agent_turn should succeed");

    assert_eq!(outcome.tool_calls_made, 2);
    assert_eq!(outcome.prompt_tokens, 38);
    assert_eq!(outcome.completion_tokens, 17);
    assert_eq!(
        outcome.telemetry.get("intraturn_read_tools_deduped"),
        Some(&serde_json::json!(1)),
        "second identical read-only call must hit read_tool_memo"
    );
    let requests = server.received_requests().await.expect("received requests");
    let third_body: serde_json::Value =
        serde_json::from_slice(&requests[2].body).expect("third request body is JSON");
    let msgs = third_body["messages"].as_array().expect("messages array");
    let dedup_msg = msgs.iter().rev().find(|m| m["role"] == "tool").unwrap();
    assert!(
        dedup_msg["content"]
            .as_str()
            .is_some_and(|s| s.contains("\"deduped_read_hit\":true")),
        "second tool message must carry deduped_read_hit envelope: {dedup_msg}"
    );

    // Verify canonical path normalization and path-scoped write invalidation (Phase 160)
    assert_eq!(
        canonicalize_tool_args_json(&serde_json::json!({"path": "./crates/vox-cli/src/main.rs/"})),
        canonicalize_tool_args_json(&serde_json::json!({"path": "crates/vox-cli/src/main.rs"}))
    );
    let mut memo = std::collections::HashMap::new();
    memo.insert(
        (
            "vox_read_file".to_string(),
            canonicalize_tool_args_json(&serde_json::json!({"path": "crates/a.rs"})),
        ),
        (0usize, "h1".to_string(), "fn a() {}".to_string()),
    );
    memo.insert(
        (
            "vox_read_file".to_string(),
            canonicalize_tool_args_json(&serde_json::json!({"path": "crates/b.rs"})),
        ),
        (0usize, "h2".to_string(), "fn b() {}".to_string()),
    );
    let evicted =
        invalidate_read_memo_for_write(&mut memo, &serde_json::json!({"path": "./crates/b.rs"}));
    assert_eq!(evicted, 1, "only b.rs should be evicted");
    assert_eq!(memo.len(), 1, "a.rs must remain memoized");
    assert!(memo.contains_key(&(
        "vox_read_file".to_string(),
        canonicalize_tool_args_json(&serde_json::json!({"path": "crates/a.rs"})),
    )));
}

#[test]
fn phase_163_to_168_edit_recovery_prefix_kv_universal_salvage_and_deescalation_work() {
    // 1. Phase 163 (`SOTA-WRITE-PRIORITY-03..04`): `build_edit_failure_recovery_window` supports
    // `vox_apply_structured_edit` with `target_content` and `vox_generate_code` with `replacement_code`.
    let tmp_dir = std::env::temp_dir().join("vox_phase_163_recovery_probe");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let probe_file = tmp_dir.join("target_mod.rs");
    let mut content = String::new();
    for idx in 1..=50 {
        content.push_str(&format!("pub fn line_fn_{idx}() -> usize {{ {idx} }}\n"));
    }
    std::fs::write(&probe_file, &content).expect("write probe file");

    let window = build_edit_failure_recovery_window(
        Some(&tmp_dir),
        "vox_apply_structured_edit",
        &serde_json::json!({
            "path": "target_mod.rs",
            "target_content": "pub fn line_fn_30() -> usize { 30 }",
            "replacement_code": "pub fn line_fn_30() -> usize { 300 }"
        }),
    )
    .expect("window must be built for vox_apply_structured_edit");
    assert!(
        window.contains("[TARGET FILE CONTEXT WINDOW (`target_mod.rs`, lines 15..=50)]"),
        "window must center around target_content at line 30: {window}"
    );
    assert!(window.contains("  30 | pub fn line_fn_30() -> usize { 30 }"));

    let gen_window = build_edit_failure_recovery_window(
        Some(&tmp_dir),
        "vox_generate_code",
        &serde_json::json!({
            "file_path": "target_mod.rs",
            "replacement_code": "pub fn line_fn_20() -> usize { 20 }"
        }),
    )
    .expect("window must be built for vox_generate_code");
    assert!(gen_window.contains("  20 | pub fn line_fn_20() -> usize { 20 }"));
    let _ = std::fs::remove_dir_all(&tmp_dir);

    // 2. Phase 164 (`SOTA-PREFIX-IMMUTABLE-01..04`): `provider_uses_prefix_kv_cache`
    let mut anth_cfg = LlmConfig::openrouter("claude-sonnet-4-6");
    anth_cfg.provider = "anthropic".to_string();
    assert!(provider_uses_prefix_kv_cache(&anth_cfg));

    let or_gemini_cfg = LlmConfig::openrouter("google/gemini-3.1-pro");
    assert!(provider_uses_prefix_kv_cache(&or_gemini_cfg));

    let local_cfg = LlmConfig::ollama("vox-mens-qwen35-4b");
    assert!(!provider_uses_prefix_kv_cache(&local_cfg));
    assert_eq!(KV_CACHE_OVERFLOW_MASK_THRESHOLD_TOKENS, 24_000);

    // 3. Phase 165 (`SOTA-UNIVERSAL-SALVAGE-01..04`): `salvage_tool_calls_from_text`
    // across all 5 dialects, stringified JSON arguments, `<tool_call>` XML, and multi-call arrays.
    let offered = vec![
        LlmToolDef {
            name: "vox_apply_structured_edit".to_string(),
            description: Some("Apply structured edit".to_string()),
            parameters: serde_json::json!({"type": "object"}),
        },
        LlmToolDef {
            name: "vox_validate_file".to_string(),
            description: Some("Validate file".to_string()),
            parameters: serde_json::json!({"type": "object"}),
        },
    ];

    // 3a. OpenAI `{"type":"function","function":{"name":...,"arguments":"{...}"}}` with stringified JSON
    let openai_prose = r#"I will edit the file now:
```json
{"type":"function","function":{"name":"vox_apply_structured_edit","arguments":"{\"path\":\"src/lib.rs\",\"target_content\":\"old\",\"replacement_code\":\"new\"}"}}
```"#;
    let salvaged_openai = salvage_tool_calls_from_text(openai_prose, &offered);
    assert_eq!(salvaged_openai.len(), 1);
    assert_eq!(salvaged_openai[0]["name"], "vox_apply_structured_edit");
    assert_eq!(
        salvaged_openai[0]["arguments"]["path"],
        serde_json::json!("src/lib.rs")
    );

    // 3b. Anthropic `{"type":"tool_use","name":...,"input":{...}}` + wire alias (`edit_file` -> `vox_apply_structured_edit`)
    let anthropic_prose =
        r#"{"type":"tool_use","name":"edit_file","input":{"path":"src/main.vox"}}"#;
    let salvaged_anth = salvage_tool_calls_from_text(anthropic_prose, &offered);
    assert_eq!(salvaged_anth.len(), 1);
    assert_eq!(salvaged_anth[0]["name"], "vox_apply_structured_edit");
    assert_eq!(
        salvaged_anth[0]["arguments"]["path"],
        serde_json::json!("src/main.vox")
    );

    // 3c. `<tool_call>...</tool_call>` XML blocks + MENS `{"tool_name","arguments"}` + MCP JSON-RPC `tools/call`
    let xml_multi_prose = r#"<tool_call>
{"tool_name":"vox_apply_structured_edit","arguments":{"path":"a.vox"}}
</tool_call>
<tool_call>
{"jsonrpc":"2.0","method":"tools/call","params":{"name":"vox_validate_file","arguments":{"path":"a.vox"}}}
</tool_call>"#;
    let salvaged_xml = salvage_tool_calls_from_text(xml_multi_prose, &offered);
    assert_eq!(salvaged_xml.len(), 2);
    assert_eq!(salvaged_xml[0]["name"], "vox_apply_structured_edit");
    assert_eq!(salvaged_xml[1]["name"], "vox_validate_file");

    // 3d. JSON array `[{...}, {...}]`
    let array_prose = r#"[{"name":"vox_apply_structured_edit","arguments":{"path":"b.vox"}},{"name":"vox_validate_file","arguments":{"path":"b.vox"}}]"#;
    let salvaged_arr = salvage_tool_calls_from_text(array_prose, &offered);
    assert_eq!(salvaged_arr.len(), 2);

    // 4. Phase 168 (`SOTA-CASCADE-DEESC-01..04`): De-escalation & tool-call stream envelope detection
    assert!(should_deescalate_repair_cascade(true, true));
    assert!(!should_deescalate_repair_cascade(true, false));
    assert!(!should_deescalate_repair_cascade(false, true));
    assert!(looks_like_streamed_tool_call_envelope(
        r#"  <tool_call>{"name":"vox_validate_file"}"#
    ));
    assert!(looks_like_streamed_tool_call_envelope(
        r#"```json
{"type":"function","function":{"name":"vox_validate_file"}}"#
    ));
    assert!(!looks_like_streamed_tool_call_envelope(
        "Here is the explanation of how the actor model works in Vox:"
    ));
}

#[test]
fn evaluate_turn_quality_and_gate_dpo_admits_high_quality_repairs_and_rejects_degenerate_replies() {
    let prompt = "Implement a deterministic Flesch-Kincaid response quality analyzer in Rust with structured unit tests.";
    let chosen = "First, we implement `analyze_response_quality` in Rust to compute Flesch Reading Ease and keyword relevance.\n\n```rust\npub fn analyze_response_quality(prompt: &str, response: &str) -> f64 {\n    88.5\n}\n```\n\nTherefore, every response is deterministically scored without an extra LLM call, and furthermore unit tests verify all five quality buckets.";
    let rejected = "idk";

    let (metrics, admitted) =
        super::evaluate_turn_quality_and_gate_dpo(prompt, chosen, Some(rejected));
    assert!(
        metrics.quality_score >= 60.0,
        "structured response must score >= 60.0, got {}",
        metrics.quality_score
    );
    assert!(
        admitted,
        "high-quality chosen response over degenerate rejected response must pass DPO gate"
    );

    let (deg_metrics, deg_admitted) =
        super::evaluate_turn_quality_and_gate_dpo(prompt, rejected, Some(chosen));
    assert!(
        !deg_admitted,
        "degenerate chosen response ({}) must be rejected by DPO gate",
        deg_metrics.quality_score
    );
}
