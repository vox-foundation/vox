use vox_research_shim::research::{ResearchConfig, ResearchQuery, ResearchScope, run_research};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_empty_web_retrieval_halts_without_synthesis() {
    let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
        .await
        .expect("in-memory db");

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
        lane: vox_search::policy::ResearchLane::Fast,
    };

    let result = run_research(query, Some(&db), &config).await;
    assert!(
        result.is_err(),
        "Pipeline must halt with Err on zero retrieval hits"
    );
    let err_str = result.err().unwrap().to_string();
    assert!(
        err_str.contains("Zero research hits retrieved")
            || err_str.contains("Zero evidence sources retrieved"),
        "Error must clearly cite zero retrieval hits: {err_str}"
    );

    // Verify ResearchStage::Failed was recorded in Codex DB
    let session = db
        .get_research_session(1)
        .await
        .expect("query session")
        .expect("session record exists");
    assert_eq!(
        session.status, "failed",
        "Session status must be set to failed on zero hits"
    );
}
