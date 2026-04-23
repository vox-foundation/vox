use std::fs;
use std::path::{Path, PathBuf};

use crate::Orchestrator;
use vox_publisher::gate::{evaluate_publish_gate, publish_gate_inputs_for_orchestrator};
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

        let publisher_config_base = PublisherConfig {
            twitter_bearer_token: config.news.twitter_token.clone(),
            forge_token: config.news.github_token.clone(),
            open_collective_token: config.news.opencollective_token.clone(),
            reddit_client_id: config.news.reddit_client_id.clone(),
            reddit_client_secret: config.news.reddit_client_secret.clone(),
            reddit_refresh_token: config.news.reddit_refresh_token.clone(),
            reddit_user_agent: config.news.reddit_user_agent.clone(),
            youtube_client_id: config.news.youtube_client_id.clone(),
            youtube_client_secret: config.news.youtube_client_secret.clone(),
            youtube_refresh_token: config.news.youtube_refresh_token.clone(),
            twitter_api_base: config.news.twitter_api_base.clone(),
            forge_rest_base: config.news.github_rest_base.clone(),
            forge_graphql_url: config.news.github_graphql_url.clone(),
            opencollective_graphql_url: config.news.opencollective_graphql_url.clone(),
            twitter_summary_margin_chars: None,
            reddit_selfpost_summary_max: None,
            dry_run: config.news.dry_run,
            site,
            twitter_text_chunk_max: config.news.twitter_text_chunk_max,
            twitter_truncation_suffix: config.news.twitter_truncation_suffix.clone(),
            youtube_repo_root: Some(vox_repository::resolve_repo_root_for_ci()),
            hacker_news_mode: config.news.hacker_news_mode.clone(),
            youtube_default_category_id: None,
            worthiness_score: None,
            ..Default::default()
        };

        let paths = collect_news_markdown_paths(news_dir, config.news.scan_recursive);
        for path in paths {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            if id.is_empty() {
                continue;
            }

            let content = match vox_bounded_fs::read_utf8_path_capped(&path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to read news file {}: {}", path.display(), e);
                    continue;
                }
            };

            let mut item = match UnifiedNewsItem::parse(&content, id) {
                Ok(it) => it,
                Err(e) => {
                    tracing::error!("Failed to parse news item {}: {}", path.display(), e);
                    continue;
                }
            };
            for (channel, floor) in &config.news.channel_worthiness_floors {
                let entry = item
                    .syndication
                    .distribution_policy
                    .channel_policy
                    .entry(channel.trim().to_lowercase())
                    .or_default();
                entry.worthiness_floor = Some(*floor);
            }
            let content_digest = item.content_sha3_256();

            let already_published = {
                let db_opt = orch.db();
                if let Some(db) = db_opt {
                    db.is_news_published_for_content(id, &content_digest)
                        .await
                        .unwrap_or(false)
                } else {
                    false
                }
            };
            if already_published {
                continue;
            }

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
            let gate = evaluate_publish_gate(publish_gate_inputs_for_orchestrator(
                config.news.dry_run,
                config.news.publish_armed,
                db_opt.is_some(),
                dual_approval_met,
                &item,
            ));
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
                    "syndication": item.syndication,
                    "topic_pack": item.topic_pack,
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
                        revision_history_json: None,
                        content_sha3_256: &content_digest,
                        state: "approved",
                    })
                    .await;
            }

            let worthiness_score = compute_news_worthiness_score(&item)
                .inspect_err(|e| {
                    tracing::warn!("Worthiness score probe failed for {}: {}", id, e);
                })
                .ok();
            if config.news.worthiness_enforce
                && !config.news.dry_run
                && !item.syndication.dry_run
                && let Some(score) = worthiness_score
            {
                let floor = config.news.worthiness_score_min.unwrap_or(0.85);
                if score < floor {
                    tracing::warn!(
                        "Skipping live syndication for {} due to worthiness floor: {:.3} < {:.3}",
                        id,
                        score,
                        floor
                    );
                    continue;
                }
            }

            let mut publisher_config = publisher_config_base.clone();
            publisher_config.worthiness_score = worthiness_score;
            let publisher = Publisher::new(publisher_config);

            tracing::info!("Publishing new news item: {}", id);

            let result = match publisher.publish_all(&item).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!("News syndication failed for {}: {}", id, e);
                    continue;
                }
            };

            if let Some(db) = db_opt {
                if let Some(yt_cfg) = &item.syndication.youtube {
                    let (status, storage_uri) = match &result.youtube {
                        vox_publisher::ChannelOutcome::Success { external_id } => {
                            ("uploaded", external_id.clone())
                        }
                        vox_publisher::ChannelOutcome::DryRun { external_id } => {
                            ("dry_run", external_id.clone())
                        }
                        vox_publisher::ChannelOutcome::Failed { .. } => ("failed", None),
                        vox_publisher::ChannelOutcome::Disabled => ("disabled", None),
                    };
                    let _ = db
                        .upsert_publication_media_asset(vox_db::PublicationMediaAssetParams {
                            publication_id: id,
                            asset_ref: yt_cfg.video_asset_ref.as_str(),
                            media_type: "video",
                            storage_uri: storage_uri.as_deref(),
                            status,
                            metadata_json: None,
                        })
                        .await;
                }
                if let Ok(result_json) = serde_json::to_string(&result) {
                    // Canonical: `publication_attempts` (+ optional channel). `news_publish_attempts` is legacy-only.
                    let _ = db
                        .record_publication_attempt(
                            id,
                            &content_digest,
                            "community_syndication",
                            &result_json,
                        )
                        .await;
                }
                if gate.live_publish_allowed && result.all_enabled_channels_succeeded(&item) {
                    let _ = db
                        .mark_news_published(
                            id,
                            &content_digest,
                            result.github_id(),
                            result.twitter_id(),
                            result.oc_id(),
                        )
                        .await;
                    let _ = db
                        .set_publication_state(
                            id,
                            "published",
                            Some(
                                &serde_json::json!({
                                    "channel_group": "community_syndication"
                                })
                                .to_string(),
                            ),
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
                            Some(
                                &serde_json::json!({
                                    "channel_group": "community_syndication"
                                })
                                .to_string(),
                            ),
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

fn compute_news_worthiness_score(item: &UnifiedNewsItem) -> anyhow::Result<f64> {
    let manifest = vox_publisher::publication::PublicationManifest::from(item.clone());
    let root = vox_repository::resolve_repo_root_for_ci();
    vox_publisher::publication_worthiness::worthiness_score_for_publication_manifest(
        &manifest, &root,
    )
}
