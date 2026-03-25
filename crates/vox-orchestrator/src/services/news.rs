use std::path::Path;
use std::fs;
use crate::Orchestrator;
use vox_publisher::types::UnifiedNewsItem;
use vox_publisher::{Publisher, PublisherConfig};

/// Service for monitoring docs/news/ and synchronizing with external platforms.
pub struct NewsService;

impl NewsService {
    /// Perform one synchronization tick of the news system.
    pub async fn tick(orch: &Orchestrator) -> anyhow::Result<()> {
        let config = orch.config.read().unwrap().clone();
        if !config.news.enabled {
            return Ok(());
        }

        let news_dir = Path::new(&config.news.news_dir);
        if !news_dir.exists() {
            return Ok(());
        }

        let publisher_config = PublisherConfig {
            twitter_bearer_token: config.news.twitter_token.clone(),
            github_token: config.news.github_token.clone(),
            open_collective_token: config.news.opencollective_token.clone(),
            dry_run: config.news.dry_run,
        };
        let publisher = Publisher::new(publisher_config);

        // 1. Scan for eligible markdown files
        let entries = match fs::read_dir(news_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!("Failed to read news directory {}: {}", news_dir.display(), e);
                return Ok(());
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                let id = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
                if id.is_empty() {
                    continue;
                }

                // 2. Check DB
                let published = {
                    let db_opt = orch.db();
                    if let Some(db) = db_opt {
                        db.is_news_published(id).await.unwrap_or(false)
                    } else {
                        false
                    }
                };

                if published {
                    continue;
                }

                // 3. Parse NewsItem
                let content = fs::read_to_string(&path)?;
                let item = match UnifiedNewsItem::parse(&content, id) {
                    Ok(it) => it,
                    Err(e) => {
                        tracing::error!("Failed to parse news item {}: {}", path.display(), e);
                        continue;
                    }
                };

                tracing::info!("Publishing new news item: {}", id);

                // 4. Syndiate
                let result = match publisher.publish_all(&item).await {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::error!("News syndication failed for {}: {}", id, e);
                        continue;
                    }
                };

                // 5. Update DB
                if let Some(db) = orch.db() {
                    let _ = db.mark_news_published(
                        id,
                        result.twitter_id,
                        result.github_id,
                        result.oc_id
                    ).await;
                }
            }
        }

        Ok(())
    }
}
