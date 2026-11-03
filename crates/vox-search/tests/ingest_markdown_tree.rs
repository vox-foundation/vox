//! `ingest_markdown_tree` round-trip into in-memory VoxDb.

// Task 9c: no real web host from any test (see tests/common/mod.rs).
mod common;

use vox_db::{ContextTrustTier, VoxDb};
use vox_search::ingest::ingest_markdown_tree;

#[tokio::test]
async fn ingest_markdown_tree_inserts_at_least_one_doc() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("page.md"), "# Section\n\nbody text here").expect("write md");

    let db = VoxDb::open_memory().await.expect("open_memory");
    let n = ingest_markdown_tree(&db, dir.path(), "test-ingest:", ContextTrustTier::Untrusted)
        .await
        .expect("ingest");

    assert!(n >= 1, "expected ≥1 markdown file ingested, got {n}");

    let tiers = db.search_document_tiers().await.expect("tiers");
    assert_eq!(
        tiers,
        vec![(
            "test-ingest:page.md".to_string(),
            String::new(),
            ContextTrustTier::Untrusted
        )]
    );
}
