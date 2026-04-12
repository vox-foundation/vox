//! Inbound intelligence ingestion logic for `vox scientia ingest-*` and `vox scientia feed-source-*`.

use anyhow::Result;
use vox_db::VoxDb;
use vox_runtime::llm::LlmConfig;
use vox_search::embeddings::EmbeddingService;
use vox_scientia_ingest::{FeedCrawler, IngestDeduplicator};
use std::sync::Arc;

/// Run one batch of Scientist RSS/Atom crawling and deduplication tick.
pub async fn ingest_tick(feed_id: Option<&str>, limit: usize) -> Result<()> {
    let db = VoxDb::connect_default().await?;
    let sources = db.list_feed_sources().await?;
    let filtered: Vec<_> = if let Some(id) = feed_id {
        sources.into_iter().filter(|s| s.id == id).collect()
    } else {
        sources
    };

    if filtered.is_empty() {
        println!("No matching feed sources found for ingest-tick.");
        return Ok(());
    }

    let db_arc = Arc::new(db);
    // Use a default embedding model for deduplication
    let embedding_config = LlmConfig::openai("text-embedding-3-small");
    let embedder = EmbeddingService::new(db_arc.clone(), embedding_config);
    
    let deduplicator = IngestDeduplicator::new(&db_arc, Some(&embedder));
    let crawler = FeedCrawler::new()?;

    for src in filtered {
        println!("Crawling feed '{}' ({}) ...", src.id, src.url);
        match crawler.crawl_url(&src.url, &src.source_kind).await {
            Ok(items) => {
                let mut count = 0;
                for item in items.into_iter().take(limit) {
                    if deduplicator.is_duplicate(&item, 0.85).await? {
                        continue;
                    }

                    db_arc.upsert_external_intelligence_pending(
                        &item.source_url,
                        &item.source_kind,
                        &item.title,
                        item.abstract_text.as_deref(),
                    ).await?;
                    count += 1;
                }
                println!("  -> Ingested {} new items from '{}'.", count, src.id);
            }
            Err(e) => {
                eprintln!("  !! Failed to crawl feed '{}': {}", src.id, e);
            }
        }
    }

    Ok(())
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
        println!("{:<20} {:<10} {:<10} {}", "ID", "KIND", "INTERVAL", "URL");
        for s in sources {
            println!("{:<20} {:<10} {:<10} {}", s.id, s.source_kind, s.crawl_interval_ms, s.url);
        }
    }
    Ok(())
}
