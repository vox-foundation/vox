//! Inbound intelligence ingestion logic for `vox scientia ingest-*` and `vox scientia feed-source-*`.
//!
//! The actual crawl/dedup workflow is dispatched through the `publication`
//! plugin via the `vox-plugin-host` ABI surface (the `Publication` extension
//! point), so that `vox-cli` does not carry a direct dependency on
//! `vox-plugin-publication` (or its transitive `feed-rs` / `vox-publisher`
//! deps).

use abi_stable::std_types::{ROption, RString};
use anyhow::{anyhow, Result};
use vox_db::VoxDb;

/// Run one batch of Scientist RSS/Atom crawling and deduplication tick.
///
/// Loads the `publication` plugin (one-time, cached process-wide), grabs its
/// `Publication` extension trait object, and dispatches `ingest_tick` over
/// the abi_stable boundary. The plugin internally runs the async workflow
/// on a small per-call current-thread runtime; we wrap the sync call in
/// `spawn_blocking` so this function stays cleanly `async` for callers.
pub async fn ingest_tick(feed_id: Option<&str>, limit: usize) -> Result<()> {
    let feed_id_owned: Option<String> = feed_id.map(|s| s.to_string());
    let limit_u32: u32 = limit.try_into().unwrap_or(u32::MAX);
    tokio::task::spawn_blocking(move || -> Result<()> {
        let plugin = vox_plugin_host::cached_code_plugin("publication")
            .map_err(|e| anyhow!("publication plugin: {e}"))?;
        let publication = plugin
            .plugin
            .as_publication()
            .into_option()
            .ok_or_else(|| anyhow!(
                "publication plugin loaded but does not expose the Publication extension"
            ))?;
        let feed_arg = match feed_id_owned {
            Some(s) => ROption::RSome(RString::from(s)),
            None => ROption::RNone,
        };
        publication
            .ingest_tick(feed_arg, limit_u32)
            .into_result()
            .map_err(|e| anyhow!("publication.ingest_tick: {e}"))
    })
    .await
    .map_err(|e| anyhow!("publication ingest task join error: {e}"))?
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
