use serde::{Deserialize, Serialize};

pub mod rss_crawler;
pub mod deduplicator;

pub use rss_crawler::FeedCrawler;
pub use deduplicator::IngestDeduplicator;

/// A registry feed source endpoint (e.g. RSS URL) for inbound research scoping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSource {
    pub id: String,
    pub url: String,
    pub source_kind: String,
    pub crawl_interval_ms: i64,
}

/// A raw scraped/received item before semantic evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundItem {
    pub source_url: String,
    pub source_kind: String,
    pub title: String,
    pub abstract_text: Option<String>,
}
