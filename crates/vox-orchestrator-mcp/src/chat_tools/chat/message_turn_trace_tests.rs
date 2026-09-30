//! Turn-trace tests that drive `try_run_agent_turn` (the default `vox_chat_message` path)
//! against a wiremock model server. A child module of `message` so it can call the private
//! `try_run_agent_turn` and read `AgentTurnResult` without widening either.

use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;
use vox_orchestrator::models::ProviderType;
use vox_orchestrator::models::spec::PricingSource;
use vox_orchestrator::{
    AffinityGroupRegistry, Orchestrator, OrchestratorConfig, SessionConfig, SessionManager,
};
use vox_repository::{RepoCapabilities, RepositoryContext};
use vox_skills::new_registry_arc;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::agent_loop::CHAT_MESSAGE_ENV_LOCK;
use super::super::turn_events::test_spec;
use super::try_run_agent_turn;
use crate::server_state::ServerState;

const MODEL_ID: &str = "acme/widget-5.5";

fn test_state() -> ServerState {
    let cfg = OrchestratorConfig::for_testing();
    let orch_cfg = cfg.clone();
    let groups = AffinityGroupRegistry::new(vec![]);
    let session_cfg = SessionConfig {
        persist: false,
        sessions_dir: std::env::temp_dir().join("vox-mcp-turn-trace-test-sessions"),
        ..SessionConfig::default()
    };
    let session_manager = SessionManager::new(session_cfg).expect("session manager");
    let repository = RepositoryContext {
        root: PathBuf::from("."),
        git_root: None,
        repository_id: "turn-trace-test".into(),
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

/// Register a catalog-priced OpenRouter spec and pin it as the process-global chat model.
fn register_pinned(state: &ServerState, id: &str) {
    {
        let handle = state.orchestrator.models_handle();
        let mut registry = handle.write().expect("models registry lock");
        registry.register(test_spec(
            id,
            ProviderType::OpenRouter,
            PricingSource::OpenRouter,
        ));
    }
    *state.mcp_chat_model_override.write() = Some(id.to_string());
}

fn plain_body(content: &str) -> serde_json::Value {
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

/// Point OpenRouter at `uri`; returns the previous values. Callers hold `CHAT_MESSAGE_ENV_LOCK`.
#[allow(unsafe_code)]
fn point_openrouter_at(uri: &str) -> (Option<String>, Option<String>) {
    let prev = (
        std::env::var("OPENROUTER_BASE_URL").ok(),
        std::env::var("OPENROUTER_API_KEY").ok(),
    );
    // SAFETY: callers hold CHAT_MESSAGE_ENV_LOCK, the crate-wide lock for these two vars.
    unsafe {
        std::env::set_var("OPENROUTER_BASE_URL", uri);
        std::env::set_var("OPENROUTER_API_KEY", "test-key");
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);
    prev
}

#[allow(unsafe_code)]
fn restore_openrouter(prev: (Option<String>, Option<String>)) {
    // SAFETY: as in `point_openrouter_at`.
    unsafe {
        match prev.0 {
            Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
            None => std::env::remove_var("OPENROUTER_BASE_URL"),
        }
        match prev.1 {
            Some(v) => std::env::set_var("OPENROUTER_API_KEY", v),
            None => std::env::remove_var("OPENROUTER_API_KEY"),
        }
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);
}

/// Run one plain (no-tool) default-path turn with `clutch`; returns the turn's events.
async fn plain_turn_events(clutch: Option<&str>) -> Vec<serde_json::Value> {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(plain_body("no tools needed")))
        .mount(&server)
        .await;
    let prev = point_openrouter_at(&server.uri());
    let state = test_state();
    register_pinned(&state, MODEL_ID);
    let result = try_run_agent_turn(
        &state,
        "system prompt",
        "hello there",
        "hello there",
        "trace-session",
        None,
        false,
        None,
        None,
        None,
        None,
        clutch,
        None,
        false,
    )
    .await;
    restore_openrouter(prev);
    result
        .expect("an OpenRouter pick runs the agent loop")
        .expect("the turn succeeds")
        .events
}

/// The composer's mode flows into the event (a hard-coded default would say "efficiency").
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn a_default_path_turn_starts_with_exactly_one_routing_decision() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let events = plain_turn_events(Some("genius")).await;
    let routing = events
        .iter()
        .filter(|e| e["kind"] == "routing_decision")
        .count();
    assert_eq!(
        routing, 1,
        "exactly one routing_decision per turn: {events:?}"
    );
    let first = &events[0];
    assert_eq!(first["kind"], "routing_decision");
    assert_eq!(first["resolved_id"], MODEL_ID);
    assert_eq!(first["family"], "acme/widget");
    assert_eq!(first["resolved_from"], "catalog");
    assert_eq!(first["mode"], "genius");
    assert_eq!(first["objective"], "Highest available intelligence");
    assert_eq!(first["reason"], "Pinned by the global model override");
}

/// No composer mode: the resolver used its own default axes, so the event claims no mode.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn a_turn_without_a_composer_mode_claims_no_mode() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let events = plain_turn_events(None).await;
    let first = &events[0];
    assert_eq!(first["kind"], "routing_decision");
    assert!(first.get("mode").is_none(), "no mode was sent: {first}");
    assert!(
        first.get("objective").is_none(),
        "no mode, no promise: {first}"
    );
    assert_eq!(first["resolved_id"], MODEL_ID);
}

const CONTRACT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/gui/turn-event-kinds.v1.json"
));

/// Receipt ids are server-minted UUIDv7s; the golden carries a fixed stand-in.
const GOLDEN_RECEIPT_ID: &str = "01920000-aaaa-7bbb-8ccc-000000000001";

fn tool_call_body() -> Value {
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
                    "function": {"name": "vox_git_status", "arguments": "{}"},
                }],
            },
            "finish_reason": "tool_calls",
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5},
    })
}

/// The golden seam: what the real default chat path emits for one tool-calling turn equals the
/// contract's `golden_turn`, which the GUI renders in `TurnTrace.test.tsx`.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn the_golden_turn_matches_the_contract() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let server = MockServer::start().await;
    // Streaming attempts get an empty SSE stream (the tool-call signature), so they fall back to
    // plain requests and never consume the tool-call response below. Mounted first: wiremock
    // answers with the first matching mock.
    Mock::given(method("POST"))
        .and(header("accept", "text/event-stream"))
        .respond_with(
            ResponseTemplate::new(200).set_body_raw("data: [DONE]\n\n", "text/event-stream"),
        )
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(tool_call_body()))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(plain_body("done, saw the tool result")),
        )
        .mount(&server)
        .await;
    let prev = point_openrouter_at(&server.uri());
    let state = test_state();
    register_pinned(&state, MODEL_ID);

    let result = try_run_agent_turn(
        &state,
        "system prompt",
        "what's the git status?",
        "what's the git status?",
        "golden-session",
        None,
        false,
        None,
        None,
        None,
        None,
        Some("efficiency"),
        None,
        false,
    )
    .await;
    restore_openrouter(prev);

    let turn = result
        .expect("an OpenRouter pick runs the agent loop")
        .expect("the turn succeeds");
    let mut events = turn.events.clone();
    for ev in &mut events {
        if ev.get("receipt_id").is_some() {
            ev["receipt_id"] = Value::String(GOLDEN_RECEIPT_ID.to_string());
        }
    }
    let actual = Value::Array(events);
    let contract: Value = serde_json::from_str(CONTRACT).expect("contract is valid JSON");
    assert_eq!(
        &actual, &contract["golden_turn"]["events"],
        "the golden turn differs from what try_run_agent_turn emits (left: actual, right: contract)"
    );
}
