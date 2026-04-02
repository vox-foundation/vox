use std::path::PathBuf;

use vox_mcp::memory::{RetrievalEvidenceEnvelope, RetrievalTriggerMode};

#[test]
fn retrieval_evidence_projection_validates_against_context_envelope_schema() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let schema_path = root.join("../../contracts/communication/context-envelope.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path).expect("read schema");
    let schema: serde_json::Value = serde_json::from_str(&schema_text).expect("parse schema");
    let validator = jsonschema::validator_for(&schema).expect("compile schema validator");

    let retrieval = RetrievalEvidenceEnvelope {
        trigger: RetrievalTriggerMode::ExplicitToolQuery,
        retrieval_tier: "hybrid".to_string(),
        memory_hit_count: 2,
        knowledge_hit_count: 1,
        chunk_hit_count: 1,
        repo_hit_count: 0,
        used_vector: true,
        used_bm25: true,
        used_lexical_fallback: false,
        contradiction_count: 0,
        top_score: Some(0.91),
        search_intent: "verification".to_string(),
        selected_mode: "hybrid".to_string(),
        backend_mix: vec!["bm25".to_string(), "vector".to_string()],
        source_diversity: 2,
        evidence_quality: 0.88,
        citation_coverage: 0.9,
        verification_performed: true,
        verification_reason: Some("contradiction_detected".to_string()),
        verification_query: Some("verify retrieval".to_string()),
        recommended_next_action: Some("retry_hybrid".to_string()),
        search_plan: serde_json::json!({ "intent": "verification" }),
        search_diagnostics: serde_json::json!({ "quality": 0.88 }),
        sqlite_journal_mode: Some("wal".to_string()),
        sqlite_fts5_reported: Some(true),
        sqlite_foreign_keys_on: Some(true),
        rrf_fused_hit_count: 1,
    };
    let envelope = retrieval.to_context_envelope("repo-mcp-contract", Some("mcp-session"));
    let instance = serde_json::to_value(&envelope).expect("serialize envelope");
    validator
        .validate(&instance)
        .expect("validate against context-envelope schema");
    assert_eq!(instance["envelope_type"], "retrieval_evidence");
}
