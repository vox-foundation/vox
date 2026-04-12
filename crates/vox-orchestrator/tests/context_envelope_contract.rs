use std::path::PathBuf;

#[test]
fn context_envelope_projection_validates_against_contract_schema() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = root.join("../../contracts/communication/context-envelope.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path).expect("read schema");
    let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("compile schema validator");

    let retrieval = vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 2,
        knowledge_hit_count: 1,
        chunk_hit_count: 0,
        repo_hit_count: 0,
        rrf_fused_hit_count: 1,
        used_vector: true,
        used_bm25: true,
        used_lexical_fallback: false,
        contradiction_count: 0,
        source_diversity: 2,
        evidence_quality: 0.87,
        citation_coverage: 0.92,
        verification_performed: true,
        verification_reason: Some("contradiction_detected".to_string()),
        recommended_next_action: Some("retry_hybrid".to_string()),
    };
    let envelope = vox_orchestrator::ContextEnvelope::from_session_retrieval(
        "repo-contract-test",
        "session-contract-test",
        &retrieval,
    );
    let instance = serde_json::to_value(&envelope).expect("serialize context envelope");
    validator
        .validate(&instance)
        .expect("validate against schema");
}

#[test]
fn context_envelope_sign_and_verify_obo_token() {
    let retrieval = vox_orchestrator::SessionRetrievalEnvelope {
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 0,
        knowledge_hit_count: 0,
        chunk_hit_count: 0,
        repo_hit_count: 0,
        rrf_fused_hit_count: 0,
        used_vector: false,
        used_bm25: false,
        used_lexical_fallback: false,
        contradiction_count: 0,
        source_diversity: 0,
        evidence_quality: 0.0,
        citation_coverage: 0.0,
        verification_performed: false,
        verification_reason: None,
        recommended_next_action: None,
    };
    let envelope =
        vox_orchestrator::ContextEnvelope::from_session_retrieval("repo", "session", &retrieval);
    let key = b"my_super_secret_session_key_12345";

    assert!(envelope.obo_token.is_none());
    assert!(!envelope.verify(key));

    let signed = envelope.sign(key);
    assert!(signed.obo_token.is_some());
    assert!(signed.verify(key));

    assert!(!signed.verify(b"wrong_key"));
}
