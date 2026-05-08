//! Inbound intelligence ingestion logic for `vox scientia ingest-*` and `vox scientia feed-source-*`.
//!
//! The actual crawl/dedup workflow is dispatched through `vox-plugin-publication`
//! so that `vox-cli` does not carry a direct dependency on `vox-scientia-ingest`
//! (and its transitive `feed-rs`/`fnv` deps).

use anyhow::Result;
use vox_db::VoxDb;

/// Run one batch of Scientist RSS/Atom crawling and deduplication tick.
///
/// Delegates to `vox_plugin_publication::ingest_tick`, which owns the
/// `FeedCrawler` / `IngestDeduplicator` logic and the `vox-scientia-ingest` dep.
pub async fn ingest_tick(feed_id: Option<&str>, limit: usize) -> Result<()> {
    vox_plugin_publication::ingest_tick(feed_id, limit).await
}

/// Register or update a feed source for inbound intelligence.
pub async fn feed_source_add(id: &str, url: &str, kind: &str, interval_ms: i64) -> Result<()> {
    let db = VoxDb::connect_default().await?;
    db.upsert_feed_source(id, url, kind, interval_ms).await?;
    println!("Upserted feed source '{}' ({}).", id, url);
    Ok(())
}

/// List registered feed sources.
pub async fn feed_source_list() -> Result<()> {
    let db = VoxDb::connect_default().await?;
    let sources = db.list_feed_sources().await?;
    if sources.is_empty() {
        println!("No feed sources registered.");
    } else {
        println!("{:<20} {:<10} {:<10} URL", "ID", "KIND", "INTERVAL");
        for s in sources {
            println!(
                "{:<20} {:<10} {:<10} {}",
                s.id, s.source_kind, s.crawl_interval_ms, s.url
            );
        }
    }
    Ok(())
}
