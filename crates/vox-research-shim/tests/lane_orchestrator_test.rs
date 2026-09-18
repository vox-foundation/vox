use vox_research_shim::research::types::{ResearchMetadata, ResearchQuery, ResearchScope};
use vox_search::policy::ResearchLane;

#[test]
fn test_query_lane_defaults_and_metadata_serialization() {
    let q = ResearchQuery {
        query: "What is quantum annealing?".to_string(),
        scope: ResearchScope::Both, // AMENDED: #1 — uses real ResearchScope::Both
        max_sources: 5,
        persist_to_docs: false,
        verify_claims: true,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: ResearchLane::Fast,
    };
    assert_eq!(q.lane, ResearchLane::Fast);

    let q_default = ResearchQuery::default();
    assert_eq!(q_default.lane, ResearchLane::Fast);

    let meta = ResearchMetadata {
        session_id: 1,
        duration_ms: 10,
        provider: "test".to_string(),
        routing_tier: vox_research_shim::research::types::RoutingTier::Direct,
        confidence: 0.2,
        subquery_count: 1,
        source_count: 0,
        claim_verdicts: vec![],
        retrieval_diagnostics: vox_research_shim::research::types::RetrievalDiagnostics::default(),
        quality_score: 50,
        planner_degraded: false,
        competence: None,
        self_verification: None,
        citation_audit: None,
        corroboration_counts: vec![],
        wave_count: 1,
        wave_stability: None,
        low_grounding_evidence: true,
    };
    let json = serde_json::to_value(&meta).expect("serialize metadata");
    assert_eq!(json["low_grounding_evidence"], true);
    let back: ResearchMetadata = serde_json::from_value(json).expect("deserialize metadata");
    assert!(back.low_grounding_evidence);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_epistemic_zero_hit_halt_fails_cleanly() {
    // AMENDED: A-3 — ResearchConfig is re-exported at `vox_research_shim::research` root,
    // NOT under the non-existent `vox_research_shim::research::config` path.
    // AMENDED: B-3 — Use a SearchPolicy with all providers disabled so run_research
    // exits immediately with zero hits without touching the live network.
    let q = ResearchQuery {
        query: "xyznonexistent999query".to_string(),
        scope: ResearchScope::Web,
        max_sources: 5,
        persist_to_docs: false,
        verify_claims: true,
        site_scope: None,
        domain_mode: Default::default(),
        waves: 1,
        lane: ResearchLane::Fast,
    };
    // Use the re-exported ResearchConfig (not vox_research_shim::research::config::ResearchConfig)
    let mut cfg = vox_research_shim::research::ResearchConfig::default();
    // Inject a policy with all provider endpoints disabled so no live network is needed
    cfg.search_policy.wikipedia_fallback_enabled = false;
    cfg.search_policy.enable_wikipedia = false;
    cfg.search_policy.enable_openalex = false;
    cfg.search_policy.enable_arxiv = false;
    cfg.search_policy.duckduckgo_fallback_enabled = false;
    cfg.search_policy.tavily_enabled = false;
    cfg.search_policy.searxng_url = None;
    let res =
        vox_research_shim::research::orchestrator::pipeline::run_research(q, None, &cfg).await;
    assert!(
        res.is_err(),
        "Zero hits must trigger hard failure instead of internal knowledge fallback"
    );
    let err_str = res.err().unwrap().to_string();
    assert!(
        err_str.contains("Zero research hits retrieved"),
        "Error message must cite zero research hits: {err_str}"
    );
}
