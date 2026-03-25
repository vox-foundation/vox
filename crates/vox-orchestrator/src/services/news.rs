use std::fs;
use std::path::{Path, PathBuf};

use crate::Orchestrator;
use vox_publisher::gate::{PublishGateInputs, evaluate_publish_gate};
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
            twitter_text_chunk_max: config.news.twitter_text_chunk_max,
            twitter_truncation_suffix: config.news.twitter_truncation_suffix.clone(),
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
            let content_digest = item.content_sha3_256();

            let db_opt = orch.db();
            let dual_approval_met = if let Some(db) = &db_opt {
                match db
                    .has_dual_publication_approval_for_digest(id, &content_digest)
                    .await
                {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("Approval check failed for {}: {}", id, e);
                        false
                    }
                }
            } else {
                false
            };
            let env_armed = std::env::var("VOX_NEWS_PUBLISH_ARMED")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let gate = evaluate_publish_gate(PublishGateInputs {
                orchestrator_dry_run: config.news.dry_run,
                item_dry_run: item.syndication.dry_run,
                publish_armed_config: config.news.publish_armed,
                publish_armed_env: env_armed,
                db_present: db_opt.is_some(),
                dual_approval_met,
            });
            if gate.has_blockers() {
                let reason_codes: Vec<&str> = gate
                    .blocking_reasons
                    .iter()
                    .map(|r| r.code.as_str())
                    .collect();
                tracing::warn!(
                    "Skipping syndication for {} due to gate blockers: {:?}",
                    id,
                    reason_codes
                );
                continue;
            }

            if let Some(db) = &db_opt {
                let source_ref = path.to_string_lossy().to_string();
                let metadata_json = serde_json::json!({
                    "tags": item.tags,
                    "syndication": item.syndication
                })
                .to_string();
                let _ = db
                    .upsert_publication_manifest(vox_db::PublicationManifestParams {
                        publication_id: id,
                        content_type: "news",
                        source_ref: Some(source_ref.as_str()),
                        title: &item.title,
                        author: &item.author,
                        abstract_text: None,
                        body_markdown: &item.content_markdown,
                        citations_json: None,
                        metadata_json: Some(metadata_json.as_str()),
                        content_sha3_256: &content_digest,
                        state: "approved",
                    })
                    .await;
            }

            tracing::info!("Publishing new news item: {}", id);

            let result = match publisher.publish_all(&item).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("News syndication failed for {}: {}", id, e);
                    continue;
                }
            };

            if let Some(db) = db_opt {
                if let Ok(result_json) = serde_json::to_string(&result) {
                    let _ = db
                        .record_news_publish_attempt(id, &content_digest, &result_json)
                        .await;
                    let _ = db
                        .record_publication_attempt(id, &content_digest, "community_syndication", &result_json)
                        .await;
                }
                if gate.live_publish_allowed && result.all_enabled_channels_succeeded(&item) {
                    let _ = db
                        .mark_news_published(id, result.github_id(), result.twitter_id(), result.oc_id())
                        .await;
                    let _ = db
                        .set_publication_state(
                            id,
                            "published",
                            Some(&serde_json::json!({
                                "channel_group": "community_syndication"
                            }).to_string()),
                        )
                        .await;
                } else if result.has_failures() {
                    tracing::warn!(
                        "Publish attempt for {} had channel failures; not marking as published.",
                        id
                    );
                    let _ = db
                        .set_publication_state(
                            id,
                            "publish_failed",
                            Some(&serde_json::json!({
                                "channel_group": "community_syndication"
                            }).to_string()),
                        )
                        .await;
                }
            }
        }

        Ok(())
    }
}

fn collect_news_markdown_paths(news_dir: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if recursive {
        for entry in WalkDir::new(news_dir)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
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
