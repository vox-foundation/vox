use std::sync::Arc;
use vox_orchestrator_mcp::memory_tools::{ResearchSearchParams, research_search};
use vox_orchestrator_mcp::server_state::ServerState;

#[tokio::test]
async fn test_research_search_mcp_tool_execution() {
    let db = Arc::new(vox_db::VoxDb::open_memory().await.unwrap());

    // Populate a test research session, claim, verdict, and artifact
    let session_id = db
        .create_research_session("test:sess-mcp", "Rust memory safety guarantees")
        .await
        .expect("create session");

    db.store_claim(
        session_id,
        12345,
        "Rust prevents data races at compile time",
        false,
        false,
        false,
    )
    .await
    .expect("store claim");

    db.store_claim_verdict(12345, "Supported", 0.95, "mock-verifier")
        .await
        .expect("store verdict");

    db.store_research_artifact(
        session_id,
        "{}",
        "# Rust Invariants\nOwnership guarantees no data races.",
    )
    .await
    .expect("store artifact");

    let state = ServerState::new_full(vox_orchestrator_mcp::load_config())
        .with_db_initialized(db)
        .await;

    let params = ResearchSearchParams {
        query: "data races".to_string(),
        limit: Some(5),
        domain: None,
        min_confidence: Some(0.80),
        verified_only: Some(true),
    };

    let result_json = research_search(&state, params).await;
    assert!(
        result_json.contains("\"success\": true"),
        "Result should be success: {result_json}"
    );
    assert!(
        result_json.contains("Rust prevents data races at compile time"),
        "Result should contain verified claim: {result_json}"
    );
    assert!(
        result_json.contains("Supported"),
        "Result should contain Supported verdict: {result_json}"
    );
}
