pub mod config;
pub use config::*;
pub mod heuristics;
pub use heuristics::*;

use crate::ChannelOutcome;
use crate::SyndicationResult;
use crate::adapters;
use crate::social_retry;
use crate::types::UnifiedNewsItem;
use anyhow::Result;
use tracing::{info, warn};

pub struct Publisher {
    config: PublisherConfig,
}

impl Publisher {
    pub fn new(config: PublisherConfig) -> Self {
        Self { config }
    }

    pub async fn publish_all(&self, item: &UnifiedNewsItem) -> Result<SyndicationResult> {
        item.validate()?;
        info!("Starting syndication for news item: {}", item.id);
        let mut result = SyndicationResult::default();
        let dist_report = crate::distribution_compile::compile_for_publish(item);
        for w in &dist_report.warnings {
            warn!(target: "vox.publisher.distribution_compile", "{}", w);
        }
        result.decision_reasons.insert(
            "distribution_derivation_digest".to_string(),
            dist_report.derivation_digest_hex.clone(),
        );
        if let Ok(json) = serde_json::to_string(&dist_report.channel_plans) {
            result
                .decision_reasons
                .insert("distribution_compile_channel_plans".to_string(), json);
        }
        if let Some(ref rp) = item.syndication.distribution_policy.retry_profile {
            let t = rp.trim();
            if !t.is_empty() {
                result
                    .decision_reasons
                    .insert("retry_profile".to_string(), t.to_string());
            }
        }
        if let Some(ref rp) = item.syndication.distribution_policy.rate_limit_profile {
            let t = rp.trim();
            if !t.is_empty() {
                result
                    .decision_reasons
                    .insert("rate_limit_profile".to_string(), t.to_string());
            }
        }

        let social_retry_budget = social_retry::budget_from_distribution_policy(item);
        result.decision_reasons.insert(
            "social_retry_max_attempts".to_string(),
            social_retry_budget.max_attempts.to_string(),
        );

        let is_dry_run = self.config.dry_run
            || item.syndication.dry_run
            || item
                .syndication
                .distribution_policy
                .dry_run
                .unwrap_or(false);
        if is_dry_run {
            info!("DRY RUN MODE ENABLED. API requests will be constructed but not sent.");
        }

        if item.syndication.rss {
            if let Some(reason) = policy_block_reason(item, "rss", &self.config) {
                result.rss = ChannelOutcome::Disabled;
                result.decision_reasons.insert("rss".to_string(), reason);
            } else if is_dry_run {
                info!("[DRY RUN] Would update RSS feed for {}", item.id);
                result.rss = ChannelOutcome::DryRun { external_id: None };
            } else {
                match social_retry::run_with_retries(social_retry_budget, || {
                    adapters::rss::update_feed(item, &self.config.site)
                })
                .await
                {
                    Ok(()) => {
                        result.rss = ChannelOutcome::Success { external_id: None };
                        info!("RSS feed updated.");
                    }
                    Err(e) => {
                        result.rss = ChannelOutcome::Failed {
                            code: "rss_update_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                            failure_class: None,
                        };
                    }
                }
            }
        }

        if item
            .syndication
            .is_active(crate::types::SocialChannel::Twitter)
        {
            if let Some(reason) = policy_block_reason(item, "twitter", &self.config) {
                result.twitter = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("twitter".to_string(), reason);
            } else if is_dry_run {
                info!("[DRY RUN] Would post to Twitter");
                result.twitter = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-tweet-{}", item.id)),
                };
            } else if self.config.twitter_bearer_token.is_some() {
                result.twitter = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            } else {
                warn!("Twitter config present but no API token.");
                result.twitter = ChannelOutcome::Failed {
                    code: "missing_twitter_token".to_string(),
                    message: "Twitter config present but no API token.".to_string(),
                    retryable: false,
                    failure_class: None,
                };
            }
        }

        if let Some(forge_cfg) = &item.syndication.forge {
            if let Some(reason) = policy_block_reason(item, "github", &self.config) {
                result.github = ChannelOutcome::Disabled;
                result.decision_reasons.insert("github".to_string(), reason);
            } else if is_dry_run {
                info!(
                    "[DRY RUN] Would post to GitHub repository {} as {:?}",
                    forge_cfg.repo, forge_cfg.post_type
                );
                result.github = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-github-{}", item.id)),
                };
            } else if let Some(token) = &self.config.forge_token {
                match social_retry::run_with_retries(social_retry_budget, || {
                    adapters::forge::post(&self.config, token, item, forge_cfg)
                })
                .await
                {
                    Ok(id) => {
                        result.github = ChannelOutcome::Success {
                            external_id: Some(id),
                        };
                        info!("Posted to GitHub.");
                    }
                    Err(e) => {
                        result.github = ChannelOutcome::Failed {
                            code: "github_post_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                            failure_class: None,
                        };
                    }
                }
            } else {
                warn!("GitHub config present but no API token.");
                result.github = ChannelOutcome::Failed {
                    code: "missing_github_token".to_string(),
                    message: "GitHub config present but no API token.".to_string(),
                    retryable: false,
                    failure_class: None,
                };
            }
        }

        if item.syndication.open_collective.is_some() {
            if let Some(reason) = policy_block_reason(item, "open_collective", &self.config) {
                result.open_collective = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("open_collective".to_string(), reason);
            } else if is_dry_run {
                result.open_collective = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-oc-{}", item.id)),
                };
            } else if self.config.open_collective_token.is_some() {
                result.open_collective = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            } else {
                result.open_collective = ChannelOutcome::Failed {
                    code: "missing_opencollective_token".to_string(),
                    message: "Open Collective config present but no API token.".to_string(),
                    retryable: false,
                    failure_class: None,
                };
            }
        }

        #[cfg(feature = "scientia-reddit")]
        if let Some(reddit) = &item.syndication.reddit {
            if let Some(reason) = policy_block_reason(item, "reddit", &self.config) {
                result.reddit = ChannelOutcome::Disabled;
                result.decision_reasons.insert("reddit".to_string(), reason);
            } else if is_dry_run {
                info!(
                    "[DRY RUN] Would post to Reddit subreddit {}",
                    reddit.subreddit.as_deref().unwrap_or("unknown")
                );
                result.reddit = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-reddit-{}", item.id)),
                };
            } else {
                result.reddit = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            }
        }

        #[cfg(not(feature = "scientia-reddit"))]
        if item.syndication.reddit.is_some() {
            result.reddit = ChannelOutcome::Disabled;
            result.decision_reasons.insert(
                "reddit".to_string(),
                "feature_disabled:scientia-reddit".to_string(),
            );
        }

        if item.syndication.hacker_news {
            if let Some(reason) = policy_block_reason(item, "hacker_news", &self.config) {
                result.hacker_news = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("hacker_news".to_string(), reason);
            } else {
                let canonical_link = self.config.site.news_item_link(&item.id);
                let mut submit =
                    reqwest::Url::parse("https://news.ycombinator.com/submitlink").unwrap();
                submit
                    .query_pairs_mut()
                    .append_pair("u", canonical_link.as_str())
                    .append_pair("t", item.title.as_str());
                let url = submit.to_string();
                result.hacker_news = if is_dry_run {
                    ChannelOutcome::DryRun {
                        external_id: Some(url),
                    }
                } else {
                    ChannelOutcome::Success {
                        external_id: Some(url),
                    }
                };
            }
        }

        #[cfg(feature = "scientia-youtube")]
        if let Some(yt) = &item.syndication.youtube {
            if let Some(reason) = policy_block_reason(item, "youtube", &self.config) {
                result.youtube = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("youtube".to_string(), reason);
            } else if is_dry_run {
                result.youtube = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-youtube-{}", item.id)),
                };
                info!(
                    "[DRY RUN] Would upload video payload {} to YouTube",
                    yt.video_asset_ref
                );
            } else {
                result.youtube = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            }
        }

        #[cfg(not(feature = "scientia-youtube"))]
        if item.syndication.youtube.is_some() {
            result.youtube = ChannelOutcome::Disabled;
            result.decision_reasons.insert(
                "youtube".to_string(),
                "feature_disabled:scientia-youtube".to_string(),
            );
        }

        if item.syndication.crates_io.is_some() {
            if let Some(reason) = policy_block_reason(item, "crates_io", &self.config) {
                result.crates_io = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("crates_io".to_string(), reason);
            } else if is_dry_run {
                result.crates_io = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-crates-io-{}", item.id)),
                };
                info!("[DRY RUN] Would apply crates.io update metadata.");
            } else {
                result.crates_io = ChannelOutcome::Failed {
                    code: "crates_io_not_implemented".to_string(),
                    message:
                        "crates_io publishing is modeled in policy but adapter implementation is not wired yet."
                            .to_string(),
                    retryable: false,
                    failure_class: None,
                };
            }
        }

        if item
            .syndication
            .is_active(crate::types::SocialChannel::Bluesky)
        {
            if let Some(reason) = policy_block_reason(item, "bluesky", &self.config) {
                result.bluesky = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("bluesky".to_string(), reason);
            } else if is_dry_run {
                result.bluesky = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-bluesky-{}", item.id)),
                };
            } else {
                result.bluesky = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            }
        }

        if item
            .syndication
            .is_active(crate::types::SocialChannel::Mastodon)
        {
            if let Some(reason) = policy_block_reason(item, "mastodon", &self.config) {
                result.mastodon = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("mastodon".to_string(), reason);
            } else if is_dry_run {
                result.mastodon = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-mastodon-{}", item.id)),
                };
            } else {
                result.mastodon = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            }
        }

        if item.syndication.linkedin {
            if let Some(reason) = policy_block_reason(item, "linkedin", &self.config) {
                result.linkedin = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("linkedin".to_string(), reason);
            } else if is_dry_run {
                result.linkedin = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-linkedin-{}", item.id)),
                };
            } else {
                result.linkedin = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            }
        }

        if item
            .syndication
            .is_active(crate::types::SocialChannel::Discord)
        {
            if let Some(reason) = policy_block_reason(item, "discord", &self.config) {
                result.discord = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("discord".to_string(), reason);
            } else if is_dry_run {
                result.discord = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-discord-{}", item.id)),
                };
            } else {
                result.discord = ChannelOutcome::DryRun {
                    external_id: Some("queued:vox-ops-syndication".to_string()),
                };
            }
        }

        if item.syndication.researchgate {
            if let Some(reason) = policy_block_reason(item, "researchgate", &self.config) {
                result.researchgate = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("researchgate".to_string(), reason);
            } else {
                match adapters::researchgate::post(item, is_dry_run).await {
                    Ok(outcome) => {
                        result.researchgate = outcome;
                        if matches!(
                            result.researchgate,
                            ChannelOutcome::Failed {
                                failure_class: Some(
                                    crate::syndication_outcome::FailureClass::ManualActionRequired
                                ),
                                ..
                            }
                        ) {
                            info!("ResearchGate post requires manual action (no public API).");
                        }
                    }
                    Err(e) => {
                        result.researchgate = ChannelOutcome::Failed {
                            code: "researchgate_adapter_error".to_string(),
                            message: e.to_string(),
                            retryable: false,
                            failure_class: None,
                        };
                    }
                }
            }
        }

        Ok(result)
    }
}
