//! Offline smoke tests for ingest types and constructors (`vox-scientia-ingest`).
//!
//! `IngestDeduplicator` needs a live [`vox_db::VoxDb`] handle; skipped here to avoid DB fixtures.

use vox_scientia_ingest::{FeedCrawler, FeedSource, InboundItem};

#[test]
fn feed_source_and_item_json_roundtrip() {
    let src = FeedSource {
        id: "arxiv".into(),
        url: "https://example.test/feed.xml".into(),
        source_kind: "rss".into(),
        crawl_interval_ms: 60_000,
    };
    let json = serde_json::to_string(&src).unwrap();
    let back: FeedSource = serde_json::from_str(&json).unwrap();
    assert_eq!(back.id, src.id);

    let item = InboundItem {
        source_url: "https://paper.test/1".into(),
        source_kind: "rss".into(),
        title: "Title".into(),
        abstract_text: Some("Abstract.".into()),
    };
    let j2 = serde_json::to_string(&item).unwrap();
    let item2: InboundItem = serde_json::from_str(&j2).unwrap();
    assert_eq!(item2.title, item.title);
}

#[test]
fn feed_crawler_new_builds_client() {
    let crawler = FeedCrawler::new().expect("build FeedCrawler client");
    drop(crawler);
}
