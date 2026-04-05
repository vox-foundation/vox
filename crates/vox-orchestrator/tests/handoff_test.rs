use vox_orchestrator::{
    AgentId, HandoffPayload, Orchestrator, OrchestratorConfig,
    handoff::{CONTEXT_ENVELOPE_JSON_METADATA_KEY, HARNESS_SPEC_JSON_METADATA_KEY},
    types::TaskId,
};

#[test]
fn test_handoff_payload_serialization() {
    let handoff = HandoffPayload::new(AgentId(1), Some(AgentId(2)), "Initial Setup")
        .with_pending(vec![TaskId(5)])
        .with_metadata("db_url", "local");

    let json = handoff.to_json();
    assert!(json.contains("Initial Setup"));
    assert!(json.contains("local"));

    let parsed = HandoffPayload::from_json(&json).unwrap();
    assert_eq!(parsed.from_agent, AgentId(1));
    assert_eq!(parsed.to_agent, Some(AgentId(2)));
    assert_eq!(parsed.pending_tasks.len(), 1);
    assert_eq!(parsed.metadata.len(), 1);
}

#[test]
fn accept_handoff_rejects_invalid_context_envelope_metadata() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let payload = HandoffPayload::new(AgentId(1), Some(AgentId(2)), "bad context")
        .with_metadata(CONTEXT_ENVELOPE_JSON_METADATA_KEY, "{not-json");
    let err = orch
        .accept_handoff(payload)
        .expect_err("should reject handoff");
    assert!(
        err.to_string().contains("Handoff invariant failed"),
        "expected invariant failure, got: {err}"
    );
}

#[test]
fn accept_handoff_accepts_valid_context_envelope_metadata() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
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
        "repo-handoff",
        "sid-handoff",
        &retrieval,
    );
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let payload = HandoffPayload::new(AgentId(1), Some(AgentId(2)), "good context")
        .with_metadata(CONTEXT_ENVELOPE_JSON_METADATA_KEY, context_json);
    let accepted = orch
        .accept_handoff(payload)
        .expect("handoff should succeed");
    assert_eq!(accepted, AgentId(1));
    assert_eq!(orch.status().agent_count, 1);
}

#[test]
fn accept_handoff_persists_context_envelope_by_session_id() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let mut rx = orch.event_bus().subscribe();
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
        "repo-handoff",
        "sid-handoff-persist",
        &retrieval,
    );
    let key = vox_orchestrator::session_context_envelope_key("sid-handoff-persist");
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let payload = HandoffPayload::new(AgentId(1), Some(AgentId(2)), "persist context")
        .with_metadata(CONTEXT_ENVELOPE_JSON_METADATA_KEY, context_json.clone());
    orch.accept_handoff(payload)
        .expect("handoff should succeed");
    let mut saw_handoff = false;
    for _ in 0..8 {
        let evt = rx.try_recv().expect("handoff event");
        if let vox_orchestrator::AgentEventKind::AgentHandoffAccepted {
            has_context_envelope,
            session_id,
            ..
        } = evt.kind
        {
            assert!(has_context_envelope);
            assert_eq!(session_id.as_deref(), Some("sid-handoff-persist"));
            saw_handoff = true;
            break;
        }
    }
    assert!(saw_handoff, "expected AgentHandoffAccepted event");
    let handle = orch.context_handle();
    let stored = vox_orchestrator::sync_lock::rw_read(&*handle)
        .get(&key)
        .expect("context envelope should be persisted");
    assert_eq!(stored, context_json);
}

#[test]
fn accept_handoff_does_not_persist_when_context_session_id_missing() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
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
    let mut context = vox_orchestrator::ContextEnvelope::from_session_retrieval(
        "repo-handoff",
        "sid-missing",
        &retrieval,
    );
    context.subject.session_id = None;
    let context_json = serde_json::to_string(&context).expect("serialize context");
    let payload = HandoffPayload::new(AgentId(1), Some(AgentId(2)), "no session id")
        .with_metadata(CONTEXT_ENVELOPE_JSON_METADATA_KEY, context_json);
    orch.accept_handoff(payload)
        .expect("handoff should succeed");
    let handle = orch.context_handle();
    let entries = vox_orchestrator::sync_lock::rw_read(&*handle).entries();
    assert!(
        entries.keys().all(|k| !k.starts_with("context_envelope:")),
        "no session-scoped context key should be persisted when session_id is absent"
    );
}

#[test]
fn accept_handoff_emits_harness_metadata_fields() {
    let orch = Orchestrator::new(OrchestratorConfig::for_testing());
    let mut rx = orch.event_bus().subscribe();
    let harness = vox_orchestrator::AgentHarnessSpec::minimal_contract_first(
        "repo-handoff",
        "handoff harness accept",
        Some("sid-handoff-harness"),
        Some("thread-handoff-harness"),
        &["artifacts/out.md".to_string()],
    );
    let payload = HandoffPayload::new(AgentId(1), Some(AgentId(2)), "harness accept")
        .with_metadata(
            HARNESS_SPEC_JSON_METADATA_KEY,
            serde_json::to_string(&harness).expect("serialize harness"),
        );
    orch.accept_handoff(payload)
        .expect("handoff should succeed");
    let mut saw_handoff = false;
    for _ in 0..8 {
        let evt = rx.try_recv().expect("handoff event");
        if let vox_orchestrator::AgentEventKind::AgentHandoffAccepted {
            has_context_envelope,
            has_harness_spec,
            session_id,
            thread_id,
            ..
        } = evt.kind
        {
            assert!(!has_context_envelope);
            assert!(has_harness_spec);
            assert_eq!(session_id.as_deref(), Some("sid-handoff-harness"));
            assert_eq!(thread_id.as_deref(), Some("thread-handoff-harness"));
            saw_handoff = true;
            break;
        }
    }
    assert!(saw_handoff, "expected AgentHandoffAccepted event");
}
