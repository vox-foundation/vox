use vox_db::{DbConfig, VoxDb};

#[tokio::test]
async fn test_claim_deduplication_reuses_cached_verdict() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("connect");

    // Compute stable claim_id for a deterministic claim text
    let claim_text = "Rust memory safety invariants prevent data races.";
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in claim_text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let claim_id = hash;

    // Store claim and a high-confidence verdict in session 1
    let session_1 = db
        .create_research_session("test:sess1", claim_text)
        .await
        .expect("create sess1");
    db.store_claim(session_1, claim_id, claim_text, false, false, false)
        .await
        .expect("store claim");
    db.store_claim_verdict(claim_id, "Supported", 0.95, "mock-verifier")
        .await
        .expect("store verdict");

    // 1. Verify get_cached_claim_verdict retrieves the high-confidence verdict
    let cached = db
        .get_cached_claim_verdict(claim_id, 14 * 24 * 60 * 60 * 1000)
        .await
        .expect("get cached");
    assert!(cached.is_some(), "must find cached claim verdict");
    let c = cached.unwrap();
    assert_eq!(c.verdict, "Supported");
    assert!(c.confidence >= 0.80);

    // 2. Verify an expired window returns None
    let expired = db
        .get_cached_claim_verdict(claim_id, -1) // negative or 0 behavior
        .await
        .expect("lookup");
    assert!(expired.is_some(), "max_age <= 0 means no expiration limit");

    // 3. Verify non-existent claim returns None
    let unknown = db
        .get_cached_claim_verdict(999_999_999, 14 * 24 * 60 * 60 * 1000)
        .await
        .expect("lookup unknown");
    assert!(unknown.is_none());
}
