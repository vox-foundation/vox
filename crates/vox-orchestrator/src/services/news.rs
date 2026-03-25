use std::fs;
use std::path::{Path, PathBuf};

use crate::Orchestrator;
use vox_publisher::types::UnifiedNewsItem;
use vox_publisher::{NewsSiteConfig, Publisher, PublisherConfig};
use walkdir::WalkDir;

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

        let mut site = NewsSiteConfig::default();
        if let Some(ref u) = config.news.site_base_url {
            site.base_url = u.trim_end_matches('/').to_string();
        }
        if let Some(ref p) = config.news.rss_feed_path {
            site.rss_feed_path = PathBuf::from(p);
        }

        let publisher_config = PublisherConfig {
            twitter_bearer_token: config.news.twitter_token.clone(),
            github_token: config.news.github_token.clone(),
            open_collective_token: config.news.opencollective_token.clone(),
            dry_run: config.news.dry_run,
            site,
            twitter_api_base: config.news.twitter_api_base.clone(),
            github_rest_base: config.news.github_rest_base.clone(),
            github_graphql_url: config.news.github_graphql_url.clone(),
            opencollective_graphql_url: config.news.opencollective_graphql_url.clone(),
            ..Default::default()
        };
        let publisher = Publisher::new(publisher_config);

        let paths = collect_news_markdown_paths(news_dir, config.news.scan_recursive);
        for path in paths {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if id.is_empty() {
                continue;
            }

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

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to read news file {}: {}", path.display(), e);
                    continue;
                }
            };

            let item = match UnifiedNewsItem::parse(&content, id) {
                Ok(it) => it,
                Err(e) => {
                    tracing::error!("Failed to parse news item {}: {}", path.display(), e);
                    continue;
                }
            };

            let syndicate_live = !config.news.dry_run && !item.syndication.dry_run;
            if syndicate_live {
                let env_armed = std::env::var("VOX_NEWS_PUBLISH_ARMED")
                    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                if !config.news.publish_armed && !env_armed {
                    tracing::warn!(
                        "Skipping live syndication for {}: set news.publish_armed=true or VOX_NEWS_PUBLISH_ARMED=1",
                        id
                    );
                    continue;
                }
                let Some(db) = orch.db() else {
                    tracing::error!(
                        "Skipping live syndication for {}: VoxDb is required for dual-approval gate",
                        id
                    );
                    continue;
                };
                match db.has_dual_news_approval(id).await {
                    Ok(true) => {}
                    Ok(false) => {
                        tracing::warn!(
                            "Skipping live syndication for {}: need two distinct approvers (MCP vox_news_approve or DB news_publish_approvals)",
                            id
                        );
                        continue;
                    }
                    Err(e) => {
                        tracing::error!("Approval check failed for {}: {}", id, e);
                        continue;
                    }
                }
            }

            tracing::info!("Publishing new news item: {}", id);

            let result = match publisher.publish_all(&item).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("News syndication failed for {}: {}", id, e);
                    continue;
                }
            };

            if let Some(db) = orch.db() {
                let _ = db
                    .mark_news_published(
                        id,
                        result.github_id.as_deref(),
                        result.twitter_id.as_deref(),
                        result.oc_id.as_deref(),
                    )
                    .await;
            }
        }

        Ok(())
    }
}

fn collect_news_markdown_paths(news_dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if recursive {
        for entry in WalkDir::new(news_dir).into_iter().filter_map(std::result::Result::ok) {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("md") {
                out.push(p.to_path_buf());
            }
        }
    } else if let Ok(entries) = fs::read_dir(news_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("md") {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}
