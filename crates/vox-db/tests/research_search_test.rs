use vox_db::{DbConfig, VoxDb};

#[tokio::test]
async fn test_fts5_research_search_and_deduplication() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("connect");
    let session_id = db
        .create_research_session("test:sess1", "Rust memory safety guarantees")
        .await
        .expect("create");

    db.store_claim(
        session_id,
        12345,
        "Rust prevents data races at compile time",
        false,
        false,
        false,
    )
    .await
    .expect("claim");
    db.store_claim_verdict(12345, "Supported", 0.95, "mock-verifier")
        .await
        .expect("verdict");
    db.store_research_artifact(
        session_id,
        "{}",
        "# Rust Invariants\nOwnership guarantees no data races.",
    )
    .await
    .expect("artifact");

    // 1. Test FTS search
    let search_hits = db
        .search_research_artifacts("data races", 5)
        .await
        .expect("search");
    assert!(
        !search_hits.is_empty(),
        "must find matching research artifact"
    );
    assert_eq!(search_hits[0].session_id, session_id);

    // 2. Test Claim Cache lookup
    let cached = db.get_cached_claim_verdict(12345, 0).await.expect("lookup");
    assert!(cached.is_some(), "must find recently verified claim");
    let c = cached.unwrap();
    assert_eq!(c.verdict, "Supported");
    assert!(c.confidence >= 0.90);

    // 3. Test Claim Cache with max_age_ms
    let fresh = db
        .get_cached_claim_verdict(12345, 60_000)
        .await
        .expect("lookup fresh");
    assert!(fresh.is_some(), "must find claim within 60s window");

    // Non-existent claim lookup
    let missing = db
        .get_cached_claim_verdict(99999, 0)
        .await
        .expect("lookup missing");
    assert!(missing.is_none(), "must return None for unknown claim");
}
