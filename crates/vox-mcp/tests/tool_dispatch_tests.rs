#![allow(unsafe_code)]

use serde_json::json;
use std::fs;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;
use tokio::net::TcpListener;
use vox_db::{
    DbConfig, PublicationManifestParams, QuestionEventParams, QuestionSessionCreateParams, VoxDb,
};
use vox_mcp::llm_bridge::infer_test_stub::{INFER_STUB_ACK_ENV, INFER_STUB_BODY_ENV};
use vox_mcp::{ServerState, tools};
use vox_orchestrator::{
    Orchestrator, OrchestratorConfig, orch_daemon, session_context_envelope_key, types::AgentId,
};

/// Serializes infer-stub env mutations (`set_var` / `remove_var` are `unsafe` in Rust 2024).
static INFER_STUB_TEST_LOCK: Mutex<()> = Mutex::new(());
static ORCH_DAEMON_ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

struct InferStubEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl InferStubEnvGuard {
    fn enter(stub_json: &str) -> Self {
        let lock = INFER_STUB_TEST_LOCK.lock().expect("infer stub test lock");
        // SAFETY: tests hold `INFER_STUB_TEST_LOCK`; no concurrent access to these env keys.
        unsafe {
            unsafe { std::env::set_var(INFER_STUB_BODY_ENV, stub_json) };
            unsafe { std::env::set_var(INFER_STUB_ACK_ENV, "1") };
        }
        Self { _lock: lock }
    }
}

impl Drop for InferStubEnvGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var(INFER_STUB_BODY_ENV);
            std::env::remove_var(INFER_STUB_ACK_ENV);
        }
    }
}

struct OrchDaemonEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl OrchDaemonEnvGuard {
    fn enter(socket: &str, writes_enabled: bool) -> Self {
        let lock = ORCH_DAEMON_ENV_TEST_LOCK.lock().expect("orch daemon env lock");
        // SAFETY: tests hold `ORCH_DAEMON_ENV_TEST_LOCK`; no concurrent access to these env keys.
        unsafe {
            unsafe { std::env::set_var("VOX_ORCHESTRATOR_DAEMON_SOCKET", socket) };
            std::env::set_var(
                "VOX_MCP_ORCHESTRATOR_RPC_WRITES",
                if writes_enabled { "1" } else { "0" },
            );
        }
        Self { _lock: lock }
    }
}

impl Drop for OrchDaemonEnvGuard {
    fn drop(&mut self) {
        // SAFETY: tests hold `ORCH_DAEMON_ENV_TEST_LOCK`; no concurrent access to these env keys.
        unsafe {
            std::env::remove_var("VOX_ORCHESTRATOR_DAEMON_SOCKET");
            std::env::remove_var("VOX_MCP_ORCHESTRATOR_RPC_WRITES");
        }
    }
}

#[tokio::test]
async fn orchestrator_write_backend_mode_daemon_when_repo_aligned() {
    let state = ServerState::new_test().await;
    let orch = Arc::new(Orchestrator::new(OrchestratorConfig::for_testing()));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let _env = OrchDaemonEnvGuard::enter(&addr.to_string(), true);
    let server = tokio::spawn(orch_daemon::serve_listener(
        listener,
        addr.to_string(),
        state.repository.repository_id.clone(),
        orch,
    ));
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    state
        .probe_external_orchestrator_daemon_if_configured()
        .await;
    assert_eq!(
        format!("{:?}", state.orchestrator_backend_mode_for_writes()),
        "DaemonAlignedTcp"
    );
    server.abort();
}

#[tokio::test]
async fn orchestrator_write_backend_mode_embedded_on_repo_mismatch() {
    let state = ServerState::new_test().await;
    let orch = Arc::new(Orchestrator::new(OrchestratorConfig::for_testing()));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");
    let _env = OrchDaemonEnvGuard::enter(&addr.to_string(), true);
    let server = tokio::spawn(orch_daemon::serve_listener(
        listener,
        addr.to_string(),
        "mismatch-repo".to_string(),
        orch,
    ));
    tokio::time::sleep(std::time::Duration::from_millis(30)).await;

    state
        .probe_external_orchestrator_daemon_if_configured()
        .await;
    assert_eq!(
        format!("{:?}", state.orchestrator_backend_mode_for_writes()),
        "Embedded"
    );
    server.abort();
}

#[tokio::test]
async fn a2a_inbox_rejects_invalid_source_parameter() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_a2a_inbox",
        json!({ "agent_id": 1, "source": "db" }),
    )
    .await
    .expect("tool returns JSON body");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let msg = v["error"].as_str().unwrap_or_default();
    assert!(
        msg.contains("merged|local|mesh"),
        "expected source parse error, got: {msg}"
    );
}

#[tokio::test]
async fn a2a_inbox_merged_default_matches_explicit_without_mesh_url() {
    let state = ServerState::new_test().await;
    let sender = state
        .orchestrator
        .spawn_agent("a2a-inbox-src")
        .expect("spawn sender");
    let receiver = state
        .orchestrator
        .spawn_agent("a2a-inbox-dst")
        .expect("spawn receiver");
    tools::handle_tool_call(
        &state,
        "vox_a2a_send",
        json!({
            "sender_id": sender.0,
            "receiver_id": receiver.0,
            "msg_type": "free_form",
            "payload": "solo-local",
        }),
    )
    .await
    .expect("send");

    let omitted =
        tools::handle_tool_call(&state, "vox_a2a_inbox", json!({ "agent_id": receiver.0 }))
            .await
            .expect("inbox default");
    let explicit = tools::handle_tool_call(
        &state,
        "vox_a2a_inbox",
        json!({ "agent_id": receiver.0, "source": "merged" }),
    )
    .await
    .expect("inbox merged");
    let a: serde_json::Value = serde_json::from_str(&omitted).unwrap();
    let b: serde_json::Value = serde_json::from_str(&explicit).unwrap();
    assert_eq!(a["success"], true);
    assert_eq!(b["success"], true);
    assert_eq!(a["data"]["source"], "merged");
    assert_eq!(b["data"]["source"], "merged");
    assert_eq!(a["data"]["remote_attempted"], false);
    assert_eq!(b["data"]["remote_attempted"], false);
    assert_eq!(a["data"]["remote_ok"], false);
    assert_eq!(b["data"]["remote_ok"], false);
    assert_eq!(a["data"]["unread_count"], b["data"]["unread_count"]);
    assert_eq!(a["data"]["unread_count"], 1);
}

#[tokio::test]
async fn submit_task_attention_requires_explicit_human_confirmation_marker() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.attention_enabled = true;
    let state = ServerState::new(orch);
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "deploy production migration",
            "files": []
        }),
    )
    .await
    .expect("submit tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("explicit human confirmation"),
        "expected confirmation gate error, got: {err}"
    );
}

#[tokio::test]
async fn submit_task_explicit_retrieval_persists_context_envelope() {
    let state = ServerState::new_test().await;
    let session_id = "submit-retrieval-session";
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "Implement retrieval bridge",
            "files": [],
            "session_id": session_id,
            "retrieval": {
                "trigger": "explicit_tool_query",
                "retrieval_tier": "hybrid",
                "memory_hit_count": 2,
                "knowledge_hit_count": 1,
                "used_vector": true,
                "used_bm25": true,
                "used_lexical_fallback": false,
                "contradiction_count": 0
            }
        }),
    )
    .await
    .expect("submit task json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");

    let context_key = session_context_envelope_key(session_id);
    let ctx_handle = state.orchestrator.context_handle();
    let store = vox_orchestrator::sync_lock::rw_read(&*ctx_handle);
    let context_raw = store
        .get(&context_key)
        .expect("context envelope should be persisted");
    let context_json: serde_json::Value =
        serde_json::from_str(&context_raw).expect("context json parse");
    assert_eq!(context_json["envelope_type"], "retrieval_evidence");
}

#[tokio::test]
async fn submit_task_rejects_invalid_context_envelope_json() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "Implement retrieval bridge",
            "files": [],
            "context_envelope_json": "{not-json"
        }),
    )
    .await
    .expect("submit task json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("invalid context_envelope_json"),
        "expected context envelope validation failure, got: {err}"
    );
}

#[tokio::test]
async fn submit_task_rejects_invalid_harness_spec_json() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "Implement harness relay",
            "files": [],
            "harness_spec_json": "{not-json"
        }),
    )
    .await
    .expect("submit task json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("invalid harness_spec_json"),
        "expected harness validation failure, got: {err}"
    );
}

#[tokio::test]
async fn submit_task_rejects_structurally_incomplete_context_envelope_json() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "Implement retrieval bridge",
            "files": [],
            "context_envelope_json": "{}"
        }),
    )
    .await
    .expect("submit task json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("invalid context_envelope_json"),
        "expected structural context envelope validation failure, got: {err}"
    );
}

#[tokio::test]
async fn submit_task_whitespace_context_envelope_json_is_ignored() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "Implement retrieval bridge",
            "files": [],
            "context_envelope_json": "   "
        }),
    )
    .await
    .expect("submit task json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");
}

#[tokio::test]
async fn submit_task_context_envelope_json_persists_context_envelope() {
    let state = ServerState::new_test().await;
    let session_id = "submit-context-envelope-session";
    let retrieval = vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 1,
        knowledge_hit_count: 1,
        chunk_hit_count: 0,
        repo_hit_count: 0,
        rrf_fused_hit_count: 0,
        used_vector: true,
        used_bm25: true,
        used_lexical_fallback: false,
        contradiction_count: 0,
        source_diversity: 2,
        evidence_quality: 0.7,
        citation_coverage: 0.8,
        verification_performed: false,
        verification_reason: None,
        recommended_next_action: None,
    };
    let context = vox_orchestrator::ContextEnvelope::from_session_retrieval(
        "repo-submit",
        session_id,
        &retrieval,
    );
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "Implement retrieval bridge",
            "files": [],
            "session_id": session_id,
            "context_envelope_json": context_json
        }),
    )
    .await
    .expect("submit task json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");

    let context_key = session_context_envelope_key(session_id);
    let ctx_handle = state.orchestrator.context_handle();
    let store = vox_orchestrator::sync_lock::rw_read(&*ctx_handle);
    let context_raw = store
        .get(&context_key)
        .expect("context envelope should be persisted");
    let context_json: serde_json::Value =
        serde_json::from_str(&context_raw).expect("context json parse");
    assert_eq!(context_json["envelope_type"], "retrieval_evidence");
}

#[tokio::test]
async fn submit_task_rejects_retrieval_and_context_envelope_json_together() {
    let state = ServerState::new_test().await;
    let retrieval = vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 1,
        knowledge_hit_count: 1,
        chunk_hit_count: 0,
        repo_hit_count: 0,
        rrf_fused_hit_count: 0,
        used_vector: true,
        used_bm25: true,
        used_lexical_fallback: false,
        contradiction_count: 0,
        source_diversity: 2,
        evidence_quality: 0.7,
        citation_coverage: 0.8,
        verification_performed: false,
        verification_reason: None,
        recommended_next_action: None,
    };
    let context =
        vox_orchestrator::ContextEnvelope::from_session_retrieval("repo-submit", "sid", &retrieval);
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let raw = tools::handle_tool_call(
        &state,
        "vox_submit_task",
        json!({
            "description": "Implement retrieval bridge",
            "files": [],
            "context_envelope_json": context_json,
            "retrieval": {
                "trigger": "explicit_tool_query",
                "retrieval_tier": "hybrid",
                "memory_hit_count": 1,
                "knowledge_hit_count": 1,
                "used_vector": true,
                "used_bm25": true,
                "used_lexical_fallback": false,
                "contradiction_count": 0
            }
        }),
    )
    .await
    .expect("submit task json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("Provide only one of"),
        "expected mutual exclusion validation failure, got: {err}"
    );
}

#[tokio::test]
async fn vox_agent_handoff_rejects_invalid_context_envelope_json() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_agent_handoff",
        json!({
            "from_agent_id": 1,
            "to_agent_id": 2,
            "plan_summary": "handoff test",
            "context_envelope_json": "{not-json"
        }),
    )
    .await
    .expect("handoff tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("invalid context_envelope_json"),
        "expected context envelope validation failure, got: {err}"
    );
}

#[tokio::test]
async fn vox_agent_handoff_rejects_invalid_harness_spec_json() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_agent_handoff",
        json!({
            "from_agent_id": 1,
            "to_agent_id": 2,
            "plan_summary": "handoff test",
            "harness_spec_json": "{not-json"
        }),
    )
    .await
    .expect("handoff tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("invalid harness_spec_json"),
        "expected harness validation failure, got: {err}"
    );
}

#[tokio::test]
async fn vox_agent_handoff_accepts_valid_context_envelope_json() {
    let state = ServerState::new_test().await;
    let retrieval = vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 1,
        knowledge_hit_count: 1,
        chunk_hit_count: 0,
        repo_hit_count: 0,
        rrf_fused_hit_count: 0,
        used_vector: true,
        used_bm25: true,
        used_lexical_fallback: false,
        contradiction_count: 0,
        source_diversity: 2,
        evidence_quality: 0.7,
        citation_coverage: 0.8,
        verification_performed: false,
        verification_reason: None,
        recommended_next_action: None,
    };
    let context =
        vox_orchestrator::ContextEnvelope::from_session_retrieval("repo-mcp", "sid-mcp", &retrieval);
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let raw = tools::handle_tool_call(
        &state,
        "vox_agent_handoff",
        json!({
            "from_agent_id": 1,
            "to_agent_id": 2,
            "plan_summary": "handoff test",
            "context_envelope_json": context_json
        }),
    )
    .await
    .expect("handoff tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");
    let data = v["data"].as_str().unwrap_or_default();
    assert!(
        data.contains("Handoff initiated"),
        "expected handoff success message, got: {data}"
    );
}

#[tokio::test]
async fn vox_agent_handoff_rejects_structurally_incomplete_context_envelope_json() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_agent_handoff",
        json!({
            "from_agent_id": 1,
            "to_agent_id": 2,
            "plan_summary": "handoff test",
            "context_envelope_json": "{}"
        }),
    )
    .await
    .expect("handoff tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("invalid context_envelope_json"),
        "expected structural context envelope validation failure, got: {err}"
    );
}

#[tokio::test]
async fn vox_agent_handoff_whitespace_context_envelope_json_is_ignored() {
    let state = ServerState::new_test().await;
    let raw = tools::handle_tool_call(
        &state,
        "vox_agent_handoff",
        json!({
            "from_agent_id": 1,
            "to_agent_id": 2,
            "plan_summary": "handoff test",
            "context_envelope_json": "   "
        }),
    )
    .await
    .expect("handoff tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");
}

#[tokio::test]
async fn vox_agent_handoff_context_metadata_reaches_ludus_plan_handoff_payload() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let state = ServerState::new_test().await.with_db(db);
    let retrieval = vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 1,
        knowledge_hit_count: 1,
        chunk_hit_count: 0,
        repo_hit_count: 0,
        rrf_fused_hit_count: 0,
        used_vector: true,
        used_bm25: true,
        used_lexical_fallback: false,
        contradiction_count: 0,
        source_diversity: 2,
        evidence_quality: 0.7,
        citation_coverage: 0.8,
        verification_performed: false,
        verification_reason: None,
        recommended_next_action: None,
    };
    let session_id = "sid-ludus-handoff";
    let context = vox_orchestrator::ContextEnvelope::from_session_retrieval(
        "repo-ludus",
        session_id,
        &retrieval,
    );
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let raw = tools::handle_tool_call(
        &state,
        "vox_agent_handoff",
        json!({
            "from_agent_id": 1,
            "to_agent_id": 2,
            "plan_summary": "handoff to validate event payload metadata",
            "context_envelope_json": context_json
        }),
    )
    .await
    .expect("handoff tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");

    let db_ref = state.db.as_ref().expect("db attached");
    let mut saw_plan_handoff = false;
    for _ in 0..40 {
        for agent_bucket in ["0", "1", "2"] {
            let events = vox_ludus::db::get_events(db_ref, agent_bucket, Some(200))
                .await
                .expect("list agent events");
            for ev in &events {
                let Some(raw_payload) = ev.payload.as_deref() else {
                    continue;
                };
                let Ok(payload) = serde_json::from_str::<serde_json::Value>(raw_payload) else {
                    continue;
                };
                let has_meta = payload["has_context_envelope"] == serde_json::json!(true)
                    && payload["session_id"] == serde_json::json!(session_id);
                if !has_meta {
                    continue;
                }
                if ev.event_type == "plan_handoff" {
                    saw_plan_handoff = true;
                }
            }
        }

        if saw_plan_handoff {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert!(
        saw_plan_handoff,
        "expected plan_handoff Ludus payload with context metadata"
    );
}

#[tokio::test]
async fn vox_agent_handoff_harness_metadata_reaches_ludus_plan_handoff_payload() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let state = ServerState::new_test().await.with_db(db);
    let harness = vox_orchestrator::AgentHarnessSpec::minimal_contract_first(
        state.repository.repository_id.as_str(),
        "handoff harness payload",
        Some("sid-harness-ludus"),
        Some("thread-harness-ludus"),
        &["artifacts/out.md".to_string()],
    );
    let harness_json = serde_json::to_string(&harness).expect("serialize harness");
    let raw = tools::handle_tool_call(
        &state,
        "vox_agent_handoff",
        json!({
            "from_agent_id": 1,
            "to_agent_id": 2,
            "plan_summary": "handoff to validate harness payload metadata",
            "harness_spec_json": harness_json
        }),
    )
    .await
    .expect("handoff tool json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");

    let db_ref = state.db.as_ref().expect("db attached");
    let mut saw_plan_handoff = false;
    for _ in 0..40 {
        for agent_bucket in ["0", "1", "2"] {
            let events = vox_ludus::db::get_events(db_ref, agent_bucket, Some(200))
                .await
                .expect("list agent events");
            for ev in &events {
                let Some(raw_payload) = ev.payload.as_deref() else {
                    continue;
                };
                let Ok(payload) = serde_json::from_str::<serde_json::Value>(raw_payload) else {
                    continue;
                };
                let has_meta = payload["has_harness_spec"] == serde_json::json!(true);
                if !has_meta {
                    continue;
                }
                if ev.event_type == "plan_handoff" {
                    saw_plan_handoff = true;
                }
            }
        }

        if saw_plan_handoff {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert!(
        saw_plan_handoff,
        "expected plan_handoff Ludus payload with harness metadata"
    );
}

#[tokio::test]
async fn orchestrator_accept_handoff_context_metadata_reaches_ludus_accept_payload() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let state = ServerState::new_test().await.with_db(db);
    let retrieval = vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 1,
        knowledge_hit_count: 1,
        chunk_hit_count: 0,
        repo_hit_count: 0,
        rrf_fused_hit_count: 0,
        used_vector: true,
        used_bm25: true,
        used_lexical_fallback: false,
        contradiction_count: 0,
        source_diversity: 2,
        evidence_quality: 0.7,
        citation_coverage: 0.8,
        verification_performed: false,
        verification_reason: None,
        recommended_next_action: None,
    };
    let session_id = "sid-ludus-handoff-accept";
    let context = vox_orchestrator::ContextEnvelope::from_session_retrieval(
        "repo-ludus",
        session_id,
        &retrieval,
    );
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let payload = vox_orchestrator::HandoffPayload::new(AgentId(1), Some(AgentId(2)), "accept handoff")
        .with_metadata(
            vox_orchestrator::handoff::CONTEXT_ENVELOPE_JSON_METADATA_KEY,
            context_json,
        );
    state
        .orchestrator
        .accept_handoff(payload)
        .expect("accept_handoff should succeed");

    let db_ref = state.db.as_ref().expect("db attached");
    let mut saw_handoff_accepted = false;
    for _ in 0..60 {
        for agent_bucket in ["0", "1", "2"] {
            let events = vox_ludus::db::get_events(db_ref, agent_bucket, Some(200))
                .await
                .expect("list agent events");
            for ev in &events {
                if ev.event_type != "agent_handoff_accepted" {
                    continue;
                }
                let Some(raw_payload) = ev.payload.as_deref() else {
                    continue;
                };
                let Ok(payload) = serde_json::from_str::<serde_json::Value>(raw_payload) else {
                    continue;
                };
                if payload["has_context_envelope"] == serde_json::json!(true)
                    && payload["session_id"] == serde_json::json!(session_id)
                {
                    saw_handoff_accepted = true;
                    break;
                }
            }
            if saw_handoff_accepted {
                break;
            }
        }
        if saw_handoff_accepted {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(
        saw_handoff_accepted,
        "expected agent_handoff_accepted Ludus payload with context metadata"
    );
}

#[tokio::test]
async fn a2a_send_attention_require_human_blocks_escalation() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.attention_enabled = true;
    let state = ServerState::new(orch);
    let sender = state.orchestrator.spawn_agent("a2a-src").expect("spawn");
    let receiver = state.orchestrator.spawn_agent("a2a-dst").expect("spawn");
    let raw = tools::handle_tool_call(
        &state,
        "vox_a2a_send",
        json!({
            "sender_id": sender.0,
            "receiver_id": receiver.0,
            "msg_type": "error_report",
            "payload": "security breach with credential leak in production deploy path",
        }),
    )
    .await
    .expect("a2a send json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("blocked pending human review"),
        "expected require-human block, got: {err}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a2a_send_attention_defer_returns_normalized_decision_and_suppresses_delivery() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.attention_enabled = true;
    orch.interruption_calibration
        .a2a_escalation_gain_offset_bits = -0.09;
    let state = ServerState::new(orch);
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let session_id = "a2a-defer-session";
    let repo = state.repository.repository_id.clone();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let qsid = db
        .create_question_session(QuestionSessionCreateParams {
            session_id,
            repository_id: &repo,
            task_id: None,
            policy_version: "test",
            started_at_ms: now_ms,
        })
        .await
        .expect("create question session");
    db.insert_question_event(QuestionEventParams {
        question_session_id: qsid,
        question_id: "q-a2a-defer-1",
        turn_index: 0,
        actor: "assistant",
        question_kind: "open_ended",
        prompt: "need more detail",
        expected_information_gain_bits: 0.2,
        expected_user_cost: 0.2,
        utility_bits_per_cost: 1.0,
        answer_text: None,
        answer_type: None,
        answered_at_ms: None,
        created_at_ms: now_ms,
    })
    .await
    .expect("insert pending question");
    let state = state.with_db(db);
    let sender = state.orchestrator.spawn_agent("a2a-src").expect("spawn");
    let receiver = state.orchestrator.spawn_agent("a2a-dst").expect("spawn");
    let payload = "x".repeat(1600);
    let raw = tools::handle_tool_call(
        &state,
        "vox_a2a_send",
        json!({
            "sender_id": sender.0,
            "receiver_id": receiver.0,
            "msg_type": "help_request",
            "payload": payload,
            "sender_session_id": session_id,
        }),
    )
    .await
    .expect("a2a send json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");
    assert_eq!(v["data"]["deferred"], true, "{raw}");
    assert_eq!(v["data"]["decision"], "DeferUntilCheckpoint", "{raw}");
    assert_eq!(v["data"]["surface"], "a2a_send");
    assert_eq!(v["data"]["channel"], "a2a_escalation");
    assert!(v["data"]["timestamp_ms"].as_u64().unwrap_or(0) > 0);
    assert!(v["data"]["message_id"].is_null());

    let inbox_raw =
        tools::handle_tool_call(&state, "vox_a2a_inbox", json!({ "agent_id": receiver.0 }))
            .await
            .expect("inbox");
    let inbox: serde_json::Value = serde_json::from_str(&inbox_raw).expect("json");
    assert_eq!(inbox["success"], true, "{inbox_raw}");
    assert_eq!(inbox["data"]["unread_count"], 0, "{inbox_raw}");
}

#[tokio::test]
async fn a2a_send_attention_exhausted_gate_blocks_before_policy() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.attention_enabled = true;
    orch.attention_budget_ms = 0;
    let state = ServerState::new(orch);
    let sender = state.orchestrator.spawn_agent("a2a-src").expect("spawn");
    let receiver = state.orchestrator.spawn_agent("a2a-dst").expect("spawn");
    let raw = tools::handle_tool_call(
        &state,
        "vox_a2a_send",
        json!({
            "sender_id": sender.0,
            "receiver_id": receiver.0,
            "msg_type": "help_request",
            "payload": "need quick context",
        }),
    )
    .await
    .expect("a2a send json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], false, "{raw}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("Attention budget exhausted"),
        "expected exhaustion gate error, got: {err}"
    );

    let inbox_raw =
        tools::handle_tool_call(&state, "vox_a2a_inbox", json!({ "agent_id": receiver.0 }))
            .await
            .expect("inbox");
    let inbox: serde_json::Value = serde_json::from_str(&inbox_raw).expect("json");
    assert_eq!(inbox["success"], true, "{inbox_raw}");
    assert_eq!(inbox["data"]["unread_count"], 0, "{inbox_raw}");
}

#[tokio::test]
async fn test_mcp_tool_dispatch_list_queues() {
    let state = ServerState::new_test().await;

    // Test a basic orchestrator tool
    let result = tools::handle_tool_call(&state, "vox_orchestrator_status", json!({})).await;

    assert!(
        result.is_ok(),
        "Tool call should succeed, got: {:?}",
        result.err()
    );
    let json_str = result.unwrap();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");

    assert_eq!(val["success"], true);
}

#[tokio::test]
async fn orchestrator_persistence_outbox_lifecycle_tool_returns_typed_payload() {
    let state = ServerState::new_test().await;
    let lifecycle = serde_json::json!({
        "queued": 3,
        "pruned_last_run": 1,
        "retried_last_run": 2,
        "replayed_last_run": 2,
        "replay_failed_last_run": 1,
        "replay_failed_by_op": {"record_trust_observation": 1},
        "last_run_unix_ms": 123456u64
    });
    let store = state.orchestrator.context_store();
    vox_orchestrator::sync_lock::rw_read(&*store).set(
        AgentId(0),
        "orchestrator/persistence_outbox_lifecycle".to_string(),
        lifecycle.to_string(),
        0,
    );

    let raw = tools::handle_tool_call(
        &state,
        "vox_orchestrator_persistence_outbox_lifecycle",
        json!({}),
    )
    .await
    .expect("lifecycle tool");
    let val: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(val["success"], true, "{raw}");
    assert_eq!(
        val["data"]["context_key"],
        "orchestrator/persistence_outbox_lifecycle"
    );
    assert_eq!(val["data"]["lifecycle"]["queued"], 3);
    assert_eq!(
        val["data"]["lifecycle"]["replay_failed_by_op"]["record_trust_observation"],
        1
    );
}

#[tokio::test]
async fn orchestrator_persistence_outbox_queue_tool_filters_and_redacts_replay() {
    let state = ServerState::new_test().await;
    let queue = serde_json::json!([
        {
            "lane": "lineage/task_failed",
            "error": "lineage failed",
            "first_seen_unix_ms": 1000u64,
            "retry_count": 1u64,
            "replay": {"op":"append_orchestration_lineage_event","task_id":1}
        },
        {
            "lane": "trust/observation",
            "error": "trust failed",
            "first_seen_unix_ms": 2000u64,
            "retry_count": 0u64,
            "replay": {"op":"record_trust_observation","entity_id":"7"}
        }
    ]);
    let store = state.orchestrator.context_store();
    vox_orchestrator::sync_lock::rw_read(&*store).set(
        AgentId(0),
        "orchestrator/persistence_outbox".to_string(),
        queue.to_string(),
        0,
    );

    let filtered = tools::handle_tool_call(
        &state,
        "vox_orchestrator_persistence_outbox_queue",
        json!({
            "lane": "trust/observation",
            "limit": 10,
            "include_replay": false
        }),
    )
    .await
    .expect("queue tool");
    let val: serde_json::Value = serde_json::from_str(&filtered).expect("json");
    assert_eq!(val["success"], true, "{filtered}");
    assert_eq!(val["data"]["lane_filter"], "trust/observation");
    assert_eq!(val["data"]["total_after_filter"], 1);
    assert_eq!(val["data"]["returned"], 1);
    assert_eq!(val["data"]["rows"][0]["lane"], "trust/observation");
    assert!(
        val["data"]["rows"][0].get("replay").is_none(),
        "include_replay=false should redact replay payload"
    );
}

#[tokio::test]
async fn test_mcp_tool_dispatch_invalid_tool() {
    let state = ServerState::new_test().await;

    let result = tools::handle_tool_call(&state, "non_existent_tool", json!({})).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_mcp_tool_dispatch_skill_list() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(&state, "vox_skill_list", json!({})).await;

    assert!(
        result.is_ok(),
        "Skill list should succeed, got: {:?}",
        result.err()
    );
    let json_str = result.unwrap();
    let val: serde_json::Value = serde_json::from_str(&json_str).expect("Valid JSON");

    assert_eq!(val["success"], true);
    assert!(val["data"].is_array());
}

#[tokio::test]
async fn oratio_status_includes_runtime_diagnostic_object() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(&state, "vox_oratio_status", json!({}))
        .await
        .expect("oratio status");
    let val: serde_json::Value = serde_json::from_str(&result).expect("Valid JSON");
    assert!(
        val.get("runtime").is_some(),
        "status should embed runtime config snapshot"
    );
    assert!(val.get("candle").is_some());
}

#[tokio::test]
async fn test_news_gate_simulation_returns_structured_reason_codes() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(
        &state,
        "vox_news_simulate_publish_gate",
        json!({
            "news_id": "example-news",
            "content": "not-frontmatter"
        }),
    )
    .await
    .expect("tool call succeeds");

    let val: serde_json::Value = serde_json::from_str(&result).expect("Valid JSON");
    assert_eq!(val["success"], true);
    let reasons = val["data"]["blocking_reasons"]
        .as_array()
        .expect("blocking_reasons array");
    assert!(
        reasons
            .iter()
            .any(|r| r["code"].as_str() == Some("parse_error")),
        "expected parse_error reason code"
    );
}

#[tokio::test]
async fn test_scientia_scholarly_staging_export_writes_files() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "mcp-staging-export",
        content_type: "scientia",
        source_ref: None,
        title: "Staging tool test",
        author: "Vox",
        abstract_text: Some("Abstract"),
        body_markdown: "# Hello",
        citations_json: None,
        metadata_json: Some(r#"{"syndication":{"dry_run":true,"rss":false}}"#),
        content_sha3_256: "digest-staging-mcp",
        state: "draft",
    })
    .await
    .expect("upsert");
    let out = tempfile::tempdir().expect("tempdir");
    let state = ServerState::new_test().await.with_db(db);
    let result = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_scholarly_staging_export",
        json!({
            "publication_id": "mcp-staging-export",
            "output_dir": out.path().to_string_lossy(),
            "venue": "openreview",
        }),
    )
    .await
    .expect("staging export");
    let val: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(val["success"], true);
    let written = val["data"]["written"].as_array().expect("written array");
    let names: Vec<&str> = written.iter().filter_map(|v| v.as_str()).collect();
    assert!(names.contains(&"body.md"));
    assert!(names.contains(&"CITATION.cff"));
    assert!(!names.contains(&"zenodo.json"));
    assert!(out.path().join("body.md").is_file());
}

#[tokio::test]
async fn scholarly_staging_mcp_written_matches_submission_package() {
    use vox_publisher::publication::PublicationManifest;
    use vox_publisher::submission_package::{self, ScholarlyVenue};

    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let meta = r#"{"syndication":{"dry_run":true,"rss":false}}"#;
    let publication_id = "parity-staging-export";
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id,
        content_type: "scientia",
        source_ref: None,
        title: "Parity title",
        author: "Parity author",
        abstract_text: Some("Abs"),
        body_markdown: "# Parity body",
        citations_json: None,
        metadata_json: Some(meta),
        content_sha3_256: "parity-staging-digest",
        state: "draft",
    })
    .await
    .expect("upsert");

    let manifest = PublicationManifest {
        publication_id: publication_id.to_string(),
        content_type: "scientia".to_string(),
        source_ref: None,
        title: "Parity title".to_string(),
        author: "Parity author".to_string(),
        abstract_text: Some("Abs".to_string()),
        body_markdown: "# Parity body".to_string(),
        citations_json: None,
        metadata_json: Some(meta.to_string()),
    };

    let dir_direct = tempfile::tempdir().expect("tempdir");
    let mut w_direct = submission_package::write_scholarly_staging(
        &manifest,
        ScholarlyVenue::OpenReview,
        dir_direct.path(),
    )
    .expect("direct write");
    submission_package::validate_scholarly_staging(
        dir_direct.path(),
        ScholarlyVenue::OpenReview,
        &manifest,
    )
    .expect("direct validate");

    let dir_mcp = tempfile::tempdir().expect("tempdir2");
    let state = ServerState::new_test().await.with_db(db);
    let result = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_scholarly_staging_export",
        json!({
            "publication_id": "parity-staging-export",
            "output_dir": dir_mcp.path().to_string_lossy(),
            "venue": "openreview",
        }),
    )
    .await
    .expect("mcp staging export");
    let val: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(val["success"], true);
    let mut w_mcp: Vec<String> = val["data"]["written"]
        .as_array()
        .expect("written array")
        .iter()
        .filter_map(|x| x.as_str().map(std::string::ToString::to_string))
        .collect();

    w_direct.sort();
    w_mcp.sort();
    assert_eq!(w_direct, w_mcp);
}

#[tokio::test]
async fn scholarly_staging_mcp_written_matches_submission_package_zenodo() {
    use vox_publisher::publication::PublicationManifest;
    use vox_publisher::scientific_metadata::{ScientificAuthor, ScientificPublicationMetadata};
    use vox_publisher::submission_package::{self, ScholarlyVenue};

    let sci = ScientificPublicationMetadata {
        authors: vec![ScientificAuthor {
            name: "Author".to_string(),
            orcid: None,
            affiliation: None,
        }],
        license_spdx: Some("MIT".to_string()),
        ..Default::default()
    };
    let meta = vox_publisher::scientific_metadata::build_scientia_metadata_json(
        "p",
        None,
        Some(&sci),
        None,
    )
    .expect("meta");
    let meta_s = meta.clone();
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let publication_id = "parity-staging-zenodo";
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id,
        content_type: "scientia",
        source_ref: None,
        title: "Zenodo parity",
        author: "Author",
        abstract_text: Some("A"),
        body_markdown: "body",
        citations_json: None,
        metadata_json: Some(&meta_s),
        content_sha3_256: "digest-zenodo-parity",
        state: "draft",
    })
    .await
    .expect("upsert");

    let manifest = PublicationManifest {
        publication_id: publication_id.to_string(),
        content_type: "scientia".to_string(),
        source_ref: None,
        title: "Zenodo parity".to_string(),
        author: "Author".to_string(),
        abstract_text: Some("A".to_string()),
        body_markdown: "body".to_string(),
        citations_json: None,
        metadata_json: Some(meta),
    };

    let dir_direct = tempfile::tempdir().expect("tempdir");
    let mut w_direct = submission_package::write_scholarly_staging(
        &manifest,
        ScholarlyVenue::Zenodo,
        dir_direct.path(),
    )
    .expect("direct write");
    submission_package::validate_scholarly_staging(
        dir_direct.path(),
        ScholarlyVenue::Zenodo,
        &manifest,
    )
    .expect("direct validate");

    let dir_mcp = tempfile::tempdir().expect("tempdir2");
    let state = ServerState::new_test().await.with_db(db);
    let result = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_scholarly_staging_export",
        json!({
            "publication_id": publication_id,
            "output_dir": dir_mcp.path().to_string_lossy(),
            "venue": "zenodo",
        }),
    )
    .await
    .expect("mcp staging export");
    let val: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(val["success"], true);
    let mut w_mcp: Vec<String> = val["data"]["written"]
        .as_array()
        .expect("written array")
        .iter()
        .filter_map(|x| x.as_str().map(std::string::ToString::to_string))
        .collect();

    w_direct.sort();
    w_mcp.sort();
    assert_eq!(w_direct, w_mcp);
    assert!(w_mcp.iter().any(|f| f == "zenodo.json"));
}

#[tokio::test]
async fn test_scientia_route_simulate_tool_is_registered_and_returns_json() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_route_simulate",
        json!({ "publication_id": "missing-id" }),
    )
    .await
    .expect("tool call should return structured json");
    let val: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert!(val.get("success").is_some());
}

#[tokio::test]
async fn test_scientia_publish_and_retry_tools_are_registered() {
    let state = ServerState::new_test().await;
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": "missing-id", "dry_run": true }),
    )
    .await
    .expect("publish tool json");
    let retry = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_retry_failed",
        json!({ "publication_id": "missing-id", "dry_run": true }),
    )
    .await
    .expect("retry tool json");
    let p: serde_json::Value = serde_json::from_str(&publish).expect("valid json");
    let r: serde_json::Value = serde_json::from_str(&retry).expect("valid json");
    assert!(p.get("success").is_some());
    assert!(r.get("success").is_some());
}

#[tokio::test]
async fn test_scientia_publish_compact_json_is_single_line() {
    let state = ServerState::new_test().await;
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": "missing-id", "dry_run": true, "json": true }),
    )
    .await
    .expect("publish tool json");
    assert!(
        !publish.contains('\n'),
        "compact tool envelope should be one line, got: {publish:?}"
    );
}

#[tokio::test]
async fn test_scientia_retry_failed_uses_current_manifest_digest() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "retry-digest-case",
        content_type: "scientia",
        source_ref: None,
        title: "Retry digest test",
        author: "Vox",
        abstract_text: None,
        body_markdown: "Body",
        citations_json: None,
        metadata_json: Some("{}"),
        content_sha3_256: "digest-current",
        state: "draft",
    })
    .await
    .expect("upsert");
    let stale_outcome = serde_json::json!({
        "rss": {"status":"failed","code":"x","message":"x","retryable":true},
        "twitter": {"status":"disabled"},
        "github": {"status":"disabled"},
        "open_collective": {"status":"disabled"},
        "reddit": {"status":"disabled"},
        "hacker_news": {"status":"disabled"},
        "youtube": {"status":"disabled"},
        "crates_io": {"status":"disabled"},
        "decision_reasons": {}
    });
    db.record_publication_attempt(
        "retry-digest-case",
        "digest-old",
        "manual_test",
        &serde_json::to_string(&stale_outcome).expect("json"),
    )
    .await
    .expect("record attempt");

    let state = ServerState::new_test().await.with_db(db);
    let retry = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_retry_failed",
        json!({ "publication_id": "retry-digest-case", "dry_run": true }),
    )
    .await
    .expect("retry tool json");
    let val: serde_json::Value = serde_json::from_str(&retry).expect("valid json");
    assert_eq!(val["success"], false);
    let err = val["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("current manifest digest"),
        "expected digest-scoped retry error, got: {err}"
    );
}

#[tokio::test]
async fn test_scientia_live_publish_honors_digest_gate() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.news.dry_run = false;
    let state = ServerState::new(orch);
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id: "gate-live-mcp",
        content_type: "scientia",
        source_ref: None,
        title: "Title",
        author: "Author",
        abstract_text: None,
        body_markdown: "Body",
        citations_json: None,
        metadata_json: Some(r#"{"syndication":{"dry_run":false,"rss":false}}"#),
        content_sha3_256: "digest-gate-mcp",
        state: "approved",
    })
    .await
    .expect("upsert");
    let state = state.with_db(db);
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": "gate-live-mcp", "dry_run": false }),
    )
    .await
    .expect("publish tool");
    let val: serde_json::Value = serde_json::from_str(&publish).expect("valid json");
    assert_eq!(val["success"], false);
    let err = val["error"].as_str().expect("error string");
    assert!(
        err.contains("live publish blocked"),
        "expected gate failure, got: {err}"
    );
}

#[tokio::test]
async fn test_scientia_live_publish_honors_worthiness_floor_when_gate_passes() {
    let mut orch = OrchestratorConfig::for_testing();
    orch.news.dry_run = false;
    orch.news.publish_armed = true;
    orch.news.worthiness_enforce = true;
    orch.news.worthiness_score_min = Some(0.99);
    let state = ServerState::new(orch);
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let publication_id = "worthiness-floor-mcp";
    let digest = "digest-worthiness-mcp";
    db.upsert_publication_manifest(PublicationManifestParams {
        publication_id,
        content_type: "scientia",
        source_ref: None,
        title: "Title",
        author: "Author",
        abstract_text: None,
        body_markdown: "Body",
        citations_json: None,
        metadata_json: Some(r#"{"syndication":{"dry_run":false,"rss":false}}"#),
        content_sha3_256: digest,
        state: "approved",
    })
    .await
    .expect("upsert");
    db.record_publication_approval_for_digest(publication_id, digest, "alice")
        .await
        .expect("approve alice");
    db.record_publication_approval_for_digest(publication_id, digest, "bob")
        .await
        .expect("approve bob");
    let state = state.with_db(db);
    let publish = tools::handle_tool_call(
        &state,
        "vox_scientia_publication_publish",
        json!({ "publication_id": publication_id, "dry_run": false }),
    )
    .await
    .expect("publish tool");
    let val: serde_json::Value = serde_json::from_str(&publish).expect("valid json");
    assert_eq!(val["success"], false);
    let err = val["error"].as_str().expect("error string");
    assert!(
        err.contains("worthiness"),
        "expected worthiness floor failure, got: {err}"
    );
}

#[tokio::test]
async fn poll_events_succeeds_without_db_using_transient_buffer() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(&state, "vox_poll_events", json!({ "limit": 10 }))
        .await
        .expect("poll_events");
    let val: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(val["success"], true);
    let data = val["data"].as_array().expect("data array");
    assert!(data.is_empty());
}

#[tokio::test]
async fn cost_history_accepts_buckets_alias() {
    let state = ServerState::new_test().await;
    let result = tools::handle_tool_call(&state, "vox_budget_history", json!({ "buckets": 5 }))
        .await
        .expect("alias call");
    let val: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(val["success"], false);
    let err = val["error"].as_str().unwrap_or("");
    assert!(
        err.contains("Database not configured") || err.contains("not configured"),
        "expected db-missing error, got: {err}"
    );
}

#[tokio::test]
async fn spawn_and_retire_agent_round_trip() {
    let state = ServerState::new_test().await;
    let spawn = tools::handle_tool_call(
        &state,
        "vox_spawn_agent",
        json!({ "name": "mcp-test-agent" }),
    )
    .await
    .expect("spawn");
    let s: serde_json::Value = serde_json::from_str(&spawn).expect("json");
    assert_eq!(s["success"], true);
    let id = s["data"]["agent_id"].as_u64().expect("agent_id");
    let retire = tools::handle_tool_call(&state, "vox_retire_agent", json!({ "agent_id": id }))
        .await
        .expect("retire");
    let r: serde_json::Value = serde_json::from_str(&retire).expect("json");
    assert_eq!(r["success"], true);
}

#[tokio::test]
async fn ludus_notifications_list_requires_db() {
    let state = ServerState::new_test().await;
    let out = tools::handle_tool_call(
        &state,
        "vox_ludus_notifications_list",
        json!({ "limit": 5 }),
    )
    .await
    .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], false);
}

#[tokio::test]
async fn ludus_progress_snapshot_requires_db() {
    let state = ServerState::new_test().await;
    let out = tools::handle_tool_call(
        &state,
        "vox_ludus_progress_snapshot",
        json!({ "notification_limit": 3, "policy_limit": 5, "policy_days": 7 }),
    )
    .await
    .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], false);
}

#[tokio::test]
async fn ludus_progress_snapshot_with_db_returns_structure() {
    let db = VoxDb::open_memory().await.expect("db");
    let state = ServerState::new_test().await.with_db(db);
    let out = tools::handle_tool_call(
        &state,
        "vox_ludus_progress_snapshot",
        json!({ "notification_limit": 3, "policy_limit": 5, "policy_days": 7 }),
    )
    .await
    .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], true);
    let data = &val["data"];
    assert!(data["kpi"].is_object());
    assert!(data["notifications"].is_array());
    assert!(data["policy_snapshots_recent"].is_array());
}

#[tokio::test]
async fn ludus_notification_ack_with_db_marks_read() {
    let db = VoxDb::open_memory().await.expect("db");
    let uid = vox_ludus::db::canonical_user_id();
    let n = vox_ludus::notifications::Notification::new(
        &uid,
        vox_ludus::notifications::NotificationType::LevelUp,
        "title",
        "body",
    );
    let nid = n.id.clone();
    vox_ludus::db::insert_notification(&db, &n)
        .await
        .expect("insert");

    let state = ServerState::new_test().await.with_db(db);
    let db_ref = state.db.as_ref().expect("codex");

    let out = tools::handle_tool_call(
        &state,
        "vox_ludus_notification_ack",
        json!({ "notification_id": nid }),
    )
    .await
    .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], true);

    let unread = vox_ludus::db::list_unread_notifications(db_ref, &uid, 10)
        .await
        .expect("list");
    assert!(unread.is_empty());
}

#[tokio::test]
async fn ludus_notification_ack_other_user_is_rejected() {
    let db = VoxDb::open_memory().await.expect("db");
    let n = vox_ludus::notifications::Notification::new(
        "other-mcp-notif-user",
        vox_ludus::notifications::NotificationType::LevelUp,
        "t",
        "m",
    );
    let nid = n.id.clone();
    vox_ludus::db::insert_notification(&db, &n)
        .await
        .expect("insert");
    let state = ServerState::new_test().await.with_db(db);
    let out = tools::handle_tool_call(
        &state,
        "vox_ludus_notification_ack",
        json!({ "notification_id": nid }),
    )
    .await
    .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], false);
}

#[tokio::test]
async fn ludus_notifications_ack_all_clears_unread() {
    let db = VoxDb::open_memory().await.expect("db");
    let uid = vox_ludus::db::canonical_user_id();
    for _ in 0..2 {
        let n = vox_ludus::notifications::Notification::new(
            &uid,
            vox_ludus::notifications::NotificationType::QuestCompleted,
            "q",
            "m",
        );
        vox_ludus::db::insert_notification(&db, &n)
            .await
            .expect("insert");
    }
    let state = ServerState::new_test().await.with_db(db);
    let db_ref = state.db.as_ref().expect("codex");
    let before = vox_ludus::db::list_unread_notifications(db_ref, &uid, 10)
        .await
        .expect("list");
    assert_eq!(before.len(), 2);

    let out = tools::handle_tool_call(&state, "vox_ludus_notifications_ack_all", json!({}))
        .await
        .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], true);

    let after = vox_ludus::db::list_unread_notifications(db_ref, &uid, 10)
        .await
        .expect("list");
    assert!(after.is_empty());
}

#[tokio::test]
async fn repo_catalog_list_resolves_local_entries() {
    let root = tempdir().expect("tempdir");
    let local_repo = root.path().join("shared-sdk");
    fs::create_dir_all(local_repo.join(".git")).expect("git dir");
    fs::write(
        root.path().join(".vox").join("repositories.yaml"),
        r#"schema_version: 1
repositories:
  - display_name: shared-sdk
    repository_id: null
    root_path: shared-sdk
    access_mode: local
    capabilities: [read_file, text_search]
"#,
    )
    .unwrap_or_else(|_| {
        fs::create_dir_all(root.path().join(".vox")).expect("dot vox");
        fs::write(
            root.path().join(".vox").join("repositories.yaml"),
            r#"schema_version: 1
repositories:
  - display_name: shared-sdk
    repository_id: null
    root_path: shared-sdk
    access_mode: local
    capabilities: [read_file, text_search]
"#,
        )
        .expect("catalog write");
    });

    let state = ServerState::new_test()
        .await
        .with_workspace_root(root.path().to_path_buf());
    let out = tools::handle_tool_call(&state, "vox_repo_catalog_list", json!({}))
        .await
        .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], true, "{out}");
    assert_eq!(val["data"]["repositories"][0]["display_name"], "shared-sdk");
    assert_eq!(
        val["data"]["repositories"][0]["resolution_status"],
        "resolved_local"
    );
}

#[tokio::test]
async fn repo_query_text_returns_grouped_matches() {
    let root = tempdir().expect("tempdir");
    let local_repo = root.path().join("shared-sdk");
    fs::create_dir_all(local_repo.join(".git")).expect("git dir");
    fs::write(local_repo.join("lib.rs"), "fn alpha() {}\nfn beta() {}\n").expect("source");
    fs::create_dir_all(root.path().join(".vox")).expect("dot vox");
    fs::write(
        root.path().join(".vox").join("repositories.yaml"),
        r#"schema_version: 1
repositories:
  - display_name: shared-sdk
    repository_id: null
    root_path: shared-sdk
    access_mode: local
    capabilities: [read_file, text_search]
"#,
    )
    .expect("catalog write");

    let state = ServerState::new_test()
        .await
        .with_workspace_root(root.path().to_path_buf());
    let out = tools::handle_tool_call(
        &state,
        "vox_repo_query_text",
        json!({ "query": "alpha", "max_matches_per_repo": 5 }),
    )
    .await
    .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], true, "{out}");
    assert_eq!(val["data"]["result_count"], 1, "{out}");
    assert_eq!(val["data"]["trace"]["source_plane"], "mcp");
    assert_eq!(val["data"]["hits"][0]["display_name"], "shared-sdk");
}

#[tokio::test]
async fn repo_query_text_records_benchmark_metric_when_db_attached() {
    let root = tempdir().expect("tempdir");
    let local_repo = root.path().join("shared-sdk");
    fs::create_dir_all(local_repo.join(".git")).expect("git dir");
    fs::write(local_repo.join("lib.rs"), "fn alpha() {}\n").expect("source");
    fs::create_dir_all(root.path().join(".vox")).expect("dot vox");
    fs::write(
        root.path().join(".vox").join("repositories.yaml"),
        r#"schema_version: 1
repositories:
  - display_name: shared-sdk
    repository_id: null
    root_path: shared-sdk
    access_mode: local
    capabilities: [read_file, text_search]
"#,
    )
    .expect("catalog write");

    let db = VoxDb::open_memory().await.expect("db");
    let state = ServerState::new_test()
        .await
        .with_workspace_root(root.path().to_path_buf())
        .with_db(db);
    let out = tools::handle_tool_call(
        &state,
        "vox_repo_query_text",
        json!({ "query": "alpha", "max_matches_per_repo": 5 }),
    )
    .await
    .expect("dispatch");
    let val: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(val["success"], true, "{out}");
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let rows = state
        .db
        .as_ref()
        .expect("db")
        .list_research_metrics_by_type(
            "benchmark_event",
            &format!("bench:{}", state.repository.repository_id),
            20,
        )
        .await
        .expect("metrics");
    assert!(
        !rows.is_empty(),
        "expected cross_repo_query benchmark metric"
    );
}

/// `VOX_MCP_TEST_INFER_STUB_BODY` + `VOX_MCP_INFER_STUB_ACK` force a fake `vox_plan` completion.
#[tokio::test]
async fn vox_plan_infer_stub_enforce_blocks_thin_plan() {
    let stub = r#"{"summary":"thin stub","tasks":[{"id":1,"description":"do the work","files":[],"estimated_complexity":8,"depends_on":[]}]}"#;
    let _stub_env = InferStubEnvGuard::enter(stub);
    let out = {
        let mut orch = OrchestratorConfig::for_testing();
        orch.plan_adequacy_enforce = true;
        let state = ServerState::new(orch);
        tools::handle_tool_call(
            &state,
            "vox_plan",
            json!({
                "goal": "migrate authentication across crates/vox-auth, crates/vox-mcp, and update docs; add regression tests",
                "auto_expand_thin_plan": false,
                "max_refine_rounds": 0,
                "write_to_disk": false,
            }),
        )
        .await
        .expect("tool returns json")
    };
    let v: serde_json::Value = serde_json::from_str(&out).expect("json");
    assert_eq!(v["success"], false, "{out}");
    let err = v["error"].as_str().unwrap_or_default();
    assert!(
        err.contains("Plan adequacy") && err.contains("thin"),
        "unexpected error: {err}"
    );
    assert!(
        v["remediation"]
            .as_str()
            .unwrap_or_default()
            .contains("VOX_ORCHESTRATOR_PLAN_ADEQUACY_ENFORCE"),
        "{out}"
    );
}

#[tokio::test]
async fn vox_plan_infer_stub_succeeds_when_enforce_off() {
    let stub = r#"{"summary":"thin stub","tasks":[{"id":1,"description":"do the work","files":[],"estimated_complexity":8,"depends_on":[]}]}"#;
    let _stub_env = InferStubEnvGuard::enter(stub);
    let out = {
        let mut orch = OrchestratorConfig::for_testing();
        orch.plan_adequacy_enforce = false;
        let state = ServerState::new(orch);
        tools::handle_tool_call(
            &state,
            "vox_plan",
            json!({
                "goal": "migrate authentication across crates/vox-auth, crates/vox-mcp, and update docs; add regression tests",
                "auto_expand_thin_plan": false,
                "max_refine_rounds": 0,
                "write_to_disk": false,
            }),
        )
        .await
        .expect("tool returns json")
    };
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["success"], true, "{out}");
}

static MCP_TOOL_ARGS_STORAGE_TEST_LOCK: Mutex<()> = Mutex::new(());

struct McpToolArgsStorageGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl McpToolArgsStorageGuard {
    fn enter(mode: &'static str) -> Self {
        let lock = MCP_TOOL_ARGS_STORAGE_TEST_LOCK
            .lock()
            .expect("mcp tool args storage lock");
        // SAFETY: tests hold `MCP_TOOL_ARGS_STORAGE_TEST_LOCK`; no concurrent access to these env keys.
        unsafe {
            unsafe { std::env::set_var("VOX_LUDUS_EMERGENCY_OFF", "1") };
            unsafe { std::env::set_var("VOX_LUDUS_MCP_TOOL_ARGS", mode) };
        }
        Self { _lock: lock }
    }
}

impl Drop for McpToolArgsStorageGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("VOX_LUDUS_EMERGENCY_OFF");
            std::env::remove_var("VOX_LUDUS_MCP_TOOL_ARGS");
        }
    }
}

#[tokio::test]
async fn handle_tool_call_respects_mcp_tool_args_omit_for_stored_payloads() {
    let _g = McpToolArgsStorageGuard::enter("omit");
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let state = ServerState::new_test().await.with_db(db);
    let raw = tools::handle_tool_call(&state, "vox_skill_list", json!({}))
        .await
        .expect("skill_list json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");

    let db_ref = state.db.as_ref().expect("db");
    let mut found = false;
    for _ in 0..40 {
        let events = vox_ludus::db::get_events(db_ref, "0", Some(200))
            .await
            .expect("list");
        for ev in &events {
            let Some(payload) = ev.payload.as_deref() else {
                continue;
            };
            let Ok(p) = serde_json::from_str::<serde_json::Value>(payload) else {
                continue;
            };
            if p.get("type").and_then(|t| t.as_str()) != Some("tool_call") {
                continue;
            }
            if p.get("tool").and_then(|t| t.as_str()) != Some("vox_skill_list") {
                continue;
            }
            assert!(
                p.get("args").is_none() || p["args"].is_null(),
                "expected omitted args, got: {p}"
            );
            found = true;
            break;
        }
        if found {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(found, "expected tool_call event with null args");
}

#[tokio::test]
async fn handle_tool_call_respects_mcp_tool_args_hash_for_stored_payloads() {
    let _g = McpToolArgsStorageGuard::enter("hash");
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let state = ServerState::new_test().await.with_db(db);
    let raw = tools::handle_tool_call(&state, "vox_skill_list", json!({}))
        .await
        .expect("skill_list json");
    let v: serde_json::Value = serde_json::from_str(&raw).expect("json");
    assert_eq!(v["success"], true, "{raw}");

    let db_ref = state.db.as_ref().expect("db");
    let mut found = false;
    for _ in 0..40 {
        let events = vox_ludus::db::get_events(db_ref, "0", Some(200))
            .await
            .expect("list");
        for ev in &events {
            let Some(payload) = ev.payload.as_deref() else {
                continue;
            };
            let Ok(p) = serde_json::from_str::<serde_json::Value>(payload) else {
                continue;
            };
            if p.get("type").and_then(|t| t.as_str()) != Some("tool_call") {
                continue;
            }
            if p.get("tool").and_then(|t| t.as_str()) != Some("vox_skill_list") {
                continue;
            }
            let args = p
                .get("args")
                .and_then(|a| a.as_str())
                .unwrap_or_default();
            assert!(
                args.starts_with("xxh3:"),
                "expected hashed args, got args field: {:?}",
                p.get("args")
            );
            found = true;
            break;
        }
        if found {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    assert!(found, "expected tool_call event with xxh3 args");
}
