use vox_research_shim::research::{ResearchConfig, ResearchQuery, ResearchScope, run_research};

#[tokio::test]
async fn test_empty_web_retrieval_halts_without_synthesis() {
    let mut config = ResearchConfig::default();
    config.claim_detection_enabled = true;

    // An obscure query with web scope that yields zero hits
    let query = ResearchQuery {
        query: "x89q_gibberish_term_guaranteed_zero_hits_2026".to_string(),
        scope: ResearchScope::Web,
        max_sources: 5,
        persist_to_docs: false,
        verify_claims: true,
        site_scope: None,
        waves: 1,
        domain_mode: vox_research_shim::research::ResearchDomainMode::General,
    };

    let result = run_research(query, None, &config).await;
    assert!(
        result.is_err(),
        "Pipeline must halt with Err on zero retrieval hits"
    );
    let err_str = result.err().unwrap().to_string();
    assert!(
        err_str.contains("Zero evidence sources retrieved"),
        "Error must clearly cite zero retrieval hits: {err_str}"
    );
}
