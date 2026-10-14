//! TRUST-03 / RET-02 regression tests: a chat turn must never write model output into the legacy
//! VoxKB tables, and legacy KB rows must never reach a chat prompt. Each test seeds the legacy
//! tables first (`vox_db::LEGACY_VOXKB_DDL`), because with no legacy table present both
//! assertions would pass vacuously. A child module of `message` (same shape as
//! `message_turn_trace_tests`).

use std::sync::Arc;
use std::time::Duration;

use vox_db::VoxDb;
use vox_orchestrator::models::ProviderType;
use vox_orchestrator::models::spec::PricingSource;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::super::agent_loop::CHAT_MESSAGE_ENV_LOCK;
use super::super::turn_events::test_spec;
use super::chat_message;
use crate::chat_tools::params::ChatMessageParams;
use crate::server_state::ServerState;

const MODEL_ID: &str = "acme/widget-5.5";
/// The stub assistant reply: long enough to be a snippet and containing the seeded rule keyword.
const REPLY: &str = "Widgets are a fine topic to discuss at considerable length today.";
const MARKER: &str = "ZX9-LEGACY-KB-MARKER-4471";

/// Seed one knowledge base, a matching keyword rule and two entries. Entry `e2` embeds the
/// canonicalized `prompt` verbatim: the retired KB-lines enrichment searched with a substring
/// match on that text, so without `e2` a regression of it would never find a row to leak.
async fn seeded_db(prompt: &str) -> Arc<VoxDb> {
    let canonical = vox_actor_runtime::prompt_canonical::canonicalize_prompt(prompt, true, true)
        .expect("canonicalize")
        .text
        .replace('\'', "''");
    let db = VoxDb::open_memory().await.expect("open memory db");
    // Seed with `execute_batch`: the sql-surface guard flags only the raw query/execute calls.
    db.connection()
        .execute_batch(vox_db::LEGACY_VOXKB_DDL)
        .await
        .expect("legacy ddl");
    db.connection()
        .execute_batch(format!(
            "INSERT INTO knowledge_bases (id, name, created_at_ms, updated_at_ms) \
               VALUES ('kb1','mykb',1,1);
             INSERT INTO kb_routing_rules (id, kb_id, rule_type, pattern, created_at_ms) \
               VALUES ('r1','kb1','keyword','widgets',1);
             INSERT INTO kb_entries (id, kb_id, content, source_signal, created_at_ms) \
               VALUES ('e1','kb1','widgets seeded entry {MARKER}','manual',1);
             INSERT INTO kb_entries (id, kb_id, content, source_signal, created_at_ms) \
               VALUES ('e2','kb1','{canonical} {MARKER}','manual',2);"
        ))
        .await
        .expect("seed legacy kb");
    Arc::new(db)
}

fn reply_body(content: &str) -> serde_json::Value {
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

/// Run one plain chat turn for `prompt` against `db` with OpenRouter pointed at wiremock.
/// Returns the turn's JSON response, every request body the model server received, and the
/// `kb_findings` row count taken after DB attach (and the attach-time legacy migration) settled
/// but before the turn ran, so a caller can tell migrated rows from rows the turn wrote.
#[allow(unsafe_code)]
async fn run_turn(db: Arc<VoxDb>, prompt: &str) -> (serde_json::Value, Vec<String>, i64) {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(reply_body(REPLY)))
        .mount(&server)
        .await;
    let prev = (
        std::env::var("OPENROUTER_BASE_URL").ok(),
        std::env::var("OPENROUTER_API_KEY").ok(),
    );
    // SAFETY: callers hold CHAT_MESSAGE_ENV_LOCK, the crate-wide lock for these two vars.
    unsafe {
        std::env::set_var("OPENROUTER_BASE_URL", server.uri());
        std::env::set_var("OPENROUTER_API_KEY", "test-key");
    }
    vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

    let state = ServerState::new_test()
        .await
        .with_db_initialized(db.clone())
        .await;
    // `with_db_initialized` spawns the RET-03 legacy-row migration into `kb_findings` and does not
    // await it. Run it here to completion (idempotent per legacy id, so the spawned copy is a
    // no-op or loses the dedupe race): every seeded legacy row is then present before the
    // baseline is read, and the turn below cannot be blamed for it.
    crate::kb_tools::findings::migrate_legacy_kb_entries(&db, &state.repository.root)
        .await
        .expect("legacy kb migration");
    let findings_before_turn = db
        .kb_findings_counts()
        .await
        .expect("finding counts before the turn")
        .total;
    {
        let handle = state.orchestrator.models_handle();
        handle
            .write()
            .expect("models registry lock")
            .register(test_spec(
                MODEL_ID,
                ProviderType::OpenRouter,
                PricingSource::OpenRouter,
            ));
    }
    *state.mcp_chat_model_override.write() = Some(MODEL_ID.to_string());

    let params: ChatMessageParams = serde_json::from_value(serde_json::json!({
        "prompt": prompt,
        "session_id": "kb-leak-session",
    }))
    .expect("chat message params");
    let response = chat_message(&state, params).await;
    // Spawned post-turn work (the old writeback) gets a bounded window to run.
    tokio::time::sleep(Duration::from_secs(2)).await;

    // SAFETY: as above.
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

    let bodies = server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .map(|r| String::from_utf8_lossy(&r.body).into_owned())
        .collect();
    (
        serde_json::from_str(&response).expect("chat_message returns JSON"),
        bodies,
        findings_before_turn,
    )
}

/// TRUST-03: the assistant reply matches a legacy routing rule, yet no row is written.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn kb_leak_chat_turn_writes_no_kb_entry_or_finding() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let db = seeded_db("tell me about gadgets").await;
    let before = db.legacy_kb_counts().await.expect("counts before");
    assert_eq!(before.kb_entries, 2, "seed is in place");

    let (response, bodies, findings_before_turn) =
        run_turn(db.clone(), "tell me about gadgets").await;

    assert_eq!(
        response["success"], true,
        "the turn must succeed: {response}"
    );
    assert!(!bodies.is_empty(), "the stub model was actually called");
    let after = db.legacy_kb_counts().await.expect("counts after");
    assert_eq!(
        after.kb_entries, before.kb_entries,
        "a chat turn must not write a legacy kb_entries row"
    );
    // The attach-time RET-03 migration copies the 2 seeded legacy rows into the inbox. That is
    // expected; it must not make this guard vacuous (baseline is the migrated rows) nor be
    // mistaken for a turn write.
    assert_eq!(
        findings_before_turn, 2,
        "the seeded legacy rows are migrated before the turn, so the baseline is meaningful"
    );
    let counts = db.kb_findings_counts().await.expect("finding counts");
    assert_eq!(
        counts.total, findings_before_turn,
        "a chat turn must not write a finding (baseline taken after the attach migration)"
    );
    assert_eq!(
        counts
            .by_origin
            .get("legacy_kb_entry")
            .copied()
            .unwrap_or(0),
        counts.total,
        "every finding present must be a migrated legacy row, none from the turn: {:?}",
        counts.by_origin
    );
}

/// RET-02: no request the model server received carries legacy KB text or its section headers.
fn assert_no_legacy_kb_in_prompt(bodies: &[String]) {
    assert!(!bodies.is_empty(), "the stub model was actually called");
    let all = bodies.join("\n");
    for needle in [
        MARKER,
        "[AUTONOMOUS RESEARCH — KNOWLEDGE BASE]",
        "[MENTIONED KNOWLEDGE BASES]",
    ] {
        assert!(
            !all.contains(needle),
            "legacy KB text reached the model prompt ({needle}): {all}"
        );
    }
}

/// RET-02: a prompt whose keywords match a legacy entry gets no KB-lines enrichment.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn kb_leak_keyword_prompt_carries_no_legacy_kb_text() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let (response, bodies, _) = run_turn(
        seeded_db("tell me about widgets").await,
        "tell me about widgets",
    )
    .await;
    assert_eq!(
        response["success"], true,
        "the turn must succeed: {response}"
    );
    assert_no_legacy_kb_in_prompt(&bodies);
}

/// D-05: `@mykb` still parses but injects nothing until the Phase 7 Library-scope remap.
#[tokio::test]
#[allow(clippy::await_holding_lock)]
async fn kb_leak_at_mention_injects_nothing() {
    let _env_guard = CHAT_MESSAGE_ENV_LOCK.lock().expect("env lock");
    let (response, bodies, _) = run_turn(
        seeded_db("summarise @mykb please").await,
        "summarise @mykb please",
    )
    .await;
    assert_eq!(
        response["success"], true,
        "the turn must succeed: {response}"
    );
    assert_no_legacy_kb_in_prompt(&bodies);
}
