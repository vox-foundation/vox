//! Async integration tests for crawl and dedup surfaces (`vox-scientia-ingest`).
//!
//! Uses in-memory `VoxDb` only; no HTTP feeds and no embedding provider.

use vox_db::{DbConfig, VoxDb};
use vox_scientia_ingest::{FeedCrawler, IngestDeduplicator, InboundItem};

#[tokio::test]
async fn feed_crawler_crawl_all_empty_sources_yields_no_items() {
    let crawler = FeedCrawler::new().expect("build FeedCrawler");
    let out = crawler.crawl_all(&[]).await;
    assert!(out.is_empty());
}

#[tokio::test]
async fn deduplicator_without_embedder_short_circuits_false() {
    let db = VoxDb::connect(DbConfig::Memory)
        .await
        .expect("memory VoxDb");
    let dedup = IngestDeduplicator::new(&db, None);
    let item = InboundItem {
        source_url: "https://example.test/paper/1".into(),
        source_kind: "rss".into(),
        title: "Some title".into(),
        abstract_text: Some("Abstract body.".into()),
    };
    let dup = dedup
        .is_duplicate(&item, 0.99)
        .await
        .expect("is_duplicate without embedder");
    assert!(
        !dup,
        "without an embedder, dedup must not flag duplicates (no vector search)"
    );
}
