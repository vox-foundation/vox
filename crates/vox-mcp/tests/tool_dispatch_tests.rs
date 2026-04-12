#![allow(unsafe_code)]

mod common;
use common::tool_dispatch_env::OrchDaemonEnvGuard;

use serde_json::json;
use std::sync::Arc;
use tokio::net::TcpListener;
use vox_db::{
    DbConfig, QuestionEventParams, QuestionSessionCreateParams, VoxDb,
};
use vox_mcp::{ServerState, tools};
use vox_orchestrator::{
    Orchestrator, OrchestratorConfig, orch_daemon, session_context_envelope_key, types::AgentId,
};
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
    let context = vox_orchestrator::ContextEnvelope::from_session_retrieval(
        "repo-mcp", "sid-mcp", &retrieval,
    );
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
    let payload =
        vox_orchestrator::HandoffPayload::new(AgentId(1), Some(AgentId(2)), "accept handoff")
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

