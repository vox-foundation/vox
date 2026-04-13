pub mod config;
pub use config::*;
pub mod heuristics;
pub use heuristics::*;

use anyhow::Result;
use tracing::{info, warn};
use crate::types::{HackerNewsConfig, HackerNewsMode, TwitterConfig, UnifiedNewsItem};
use crate::SyndicationResult;
use crate::ChannelOutcome;
use crate::adapters;
use crate::social_retry;

#[cfg(feature = "scientia-reddit")]
use crate::types::{RedditConfig, RedditPostKind};
#[cfg(feature = "scientia-youtube")]
use crate::types::YouTubeConfig;

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

        let derived_twitter: Option<TwitterConfig> =
            item.syndication.twitter.clone().map(|mut cfg| {
                if cfg
                    .short_text
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    let max_chars = twitter_effective_summary_max_chars(
                        item,
                        &self.config,
                        &mut result.decision_reasons,
                    );
                    cfg.short_text = Some(summarize_for_social(
                        item.content_markdown.as_str(),
                        max_chars,
                    ));
                }
                cfg
            });

        #[cfg(feature = "scientia-reddit")]
        let social_source = item
            .syndication
            .youtube
            .as_ref()
            .and_then(|yt| yt.description_override.as_deref())
            .unwrap_or(item.content_markdown.as_str());
        #[cfg(feature = "scientia-reddit")]
        let reddit_cap = social_text_cap_with_template_profile(
            item,
            "reddit",
            self.config
                .reddit_selfpost_summary_max
                .unwrap_or(crate::contract::REDDIT_SELFPOST_SUMMARY_MAX),
            &mut result.decision_reasons,
        );
        #[cfg(feature = "scientia-reddit")]
        let derived_summary = summarize_for_social(social_source, reddit_cap);
        #[cfg(feature = "scientia-reddit")]
        let derived_reddit: Option<RedditConfig> =
            item.syndication.reddit.clone().map(|mut cfg| {
                if cfg
                    .title_override
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    cfg.title_override = Some(crate::contract::clamp_text(
                        item.title.as_str(),
                        crate::contract::REDDIT_TITLE_MAX,
                    ));
                }
                if matches!(cfg.kind, RedditPostKind::SelfPost)
                    && cfg
                        .text_override
                        .as_deref()
                        .map(str::trim)
                        .unwrap_or("")
                        .is_empty()
                {
                    cfg.text_override = Some(derived_summary.clone());
                }
                cfg
            });
        let derived_hn: Option<HackerNewsConfig> =
            item.syndication.hacker_news.clone().map(|mut cfg| {
                if cfg
                    .title_override
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                    && item.syndication.youtube.is_some()
                {
                    cfg.title_override = Some(crate::contract::clamp_text(
                        format!("{} (video)", item.title).as_str(),
                        crate::contract::HACKER_NEWS_TITLE_MAX,
                    ));
                }
                if let Some(mode) = self.config.hacker_news_mode.as_deref()
                    && mode.trim().eq_ignore_ascii_case("manual_assist")
                {
                    cfg.mode = HackerNewsMode::ManualAssist;
                }
                cfg
            });
        #[cfg(feature = "scientia-youtube")]
        let derived_youtube: Option<YouTubeConfig> =
            item.syndication.youtube.clone().map(|mut cfg| {
                if cfg
                    .description_override
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    let yt_cap = social_text_cap_with_template_profile(
                        item,
                        "youtube",
                        crate::contract::YOUTUBE_DESCRIPTION_MAX,
                        &mut result.decision_reasons,
                    );
                    cfg.description_override =
                        Some(summarize_for_social(item.content_markdown.as_str(), yt_cap));
                }
                if cfg
                    .title_override
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    cfg.title_override = Some(crate::contract::clamp_text(
                        item.title.as_str(),
                        crate::contract::YOUTUBE_TITLE_MAX,
                    ));
                }
                if cfg
                    .category_id
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    cfg.category_id = self
                        .config
                        .youtube_default_category_id
                        .as_deref()
                        .map(str::trim)
                        .filter(|v| !v.is_empty())
                        .map(std::string::ToString::to_string);
                }
                cfg
            });

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
                        };
                    }
                }
            }
        }

        if let Some(twitter) = &derived_twitter {
            if let Some(reason) = policy_block_reason(item, "twitter", &self.config) {
                result.twitter = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("twitter".to_string(), reason);
            } else if is_dry_run {
                info!("[DRY RUN] Would post to Twitter: {:?}", twitter);
                result.twitter = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-tweet-{}", item.id)),
                };
            } else if let Some(token) = &self.config.twitter_bearer_token {
                match social_retry::run_with_retries(social_retry_budget, || {
                    adapters::twitter::post(&self.config, token.as_str(), item, twitter)
                })
                .await
                {
                    Ok(id) => {
                        result.twitter = ChannelOutcome::Success {
                            external_id: Some(id),
                        };
                        info!("Posted to Twitter.");
                    }
                    Err(e) => {
                        result.twitter = ChannelOutcome::Failed {
                            code: "twitter_post_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                        };
                    }
                }
            } else {
                warn!("Twitter config present but no API token.");
                result.twitter = ChannelOutcome::Failed {
                    code: "missing_twitter_token".to_string(),
                    message: "Twitter config present but no API token.".to_string(),
                    retryable: false,
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
                        };
                    }
                }
            } else {
                warn!("GitHub config present but no API token.");
                result.github = ChannelOutcome::Failed {
                    code: "missing_github_token".to_string(),
                    message: "GitHub config present but no API token.".to_string(),
                    retryable: false,
                };
            }
        }

        if let Some(oc) = &item.syndication.open_collective {
            if let Some(reason) = policy_block_reason(item, "open_collective", &self.config) {
                result.open_collective = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("open_collective".to_string(), reason);
            } else if is_dry_run {
                info!(
                    "[DRY RUN] Would post to Open Collective slug {}",
                    oc.collective_slug
                );
                result.open_collective = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-oc-{}", item.id)),
                };
            } else if let Some(token) = &self.config.open_collective_token {
                match social_retry::run_with_retries(social_retry_budget, || {
                    adapters::opencollective::post(&self.config, token, item, oc)
                })
                .await
                {
                    Ok(id) => {
                        result.open_collective = ChannelOutcome::Success {
                            external_id: Some(id),
                        };
                        info!("Posted to Open Collective.");
                    }
                    Err(e) => {
                        result.open_collective = ChannelOutcome::Failed {
                            code: "opencollective_post_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                        };
                    }
                }
            } else {
                warn!("Open Collective config present but no API token.");
                result.open_collective = ChannelOutcome::Failed {
                    code: "missing_opencollective_token".to_string(),
                    message: "Open Collective config present but no API token.".to_string(),
                    retryable: false,
                };
            }
        }

        #[cfg(feature = "scientia-reddit")]
        if let Some(reddit) = &derived_reddit {
            let canonical_link = self.config.site.news_item_link(&item.id);
            if let Some(reason) = policy_block_reason(item, "reddit", &self.config) {
                result.reddit = ChannelOutcome::Disabled;
                result.decision_reasons.insert("reddit".to_string(), reason);
            } else if is_dry_run {
                info!(
                    "[DRY RUN] Would post to Reddit subreddit {}",
                    reddit.subreddit
                );
                result.reddit = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-reddit-{}", item.id)),
                };
            } else if let (
                Some(client_id),
                Some(client_secret),
                Some(refresh_token),
                Some(user_agent),
            ) = (
                self.config.reddit_client_id.as_deref(),
                self.config.reddit_client_secret.as_deref(),
                self.config.reddit_refresh_token.as_deref(),
                self.config.reddit_user_agent.as_deref(),
            ) {
                let auth = adapters::reddit::RedditAuthConfig {
                    client_id,
                    client_secret,
                    refresh_token,
                    user_agent,
                };
                match social_retry::run_with_retries(social_retry_budget, || {
                    adapters::reddit::submit(&auth, item, reddit, canonical_link.as_str())
                })
                .await
                {
                    Ok(id) => {
                        result.reddit = ChannelOutcome::Success {
                            external_id: Some(id),
                        };
                        info!("Posted to Reddit.");
                    }
                    Err(e) => {
                        result.reddit = ChannelOutcome::Failed {
                            code: "reddit_post_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                        };
                    }
                }
            } else {
                result.reddit = ChannelOutcome::Failed {
                    code: "missing_reddit_credentials".to_string(),
                    message: "Reddit config present but OAuth credentials are incomplete."
                        .to_string(),
                    retryable: false,
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

        if let Some(hn) = &derived_hn {
            let canonical_link = self.config.site.news_item_link(&item.id);
            if let Some(reason) = policy_block_reason(item, "hacker_news", &self.config) {
                result.hacker_news = ChannelOutcome::Disabled;
                result
                    .decision_reasons
                    .insert("hacker_news".to_string(), reason);
            } else {
                match adapters::hacker_news::post_manual_assist(item, hn, canonical_link.as_str())
                    .await
                {
                    Ok(url) => {
                        result.hacker_news = if is_dry_run {
                            ChannelOutcome::DryRun {
                                external_id: Some(url),
                            }
                        } else {
                            ChannelOutcome::Success {
                                external_id: Some(url),
                            }
                        };
                        info!("Generated Hacker News submit link.");
                    }
                    Err(e) => {
                        result.hacker_news = ChannelOutcome::Failed {
                            code: "hacker_news_prepare_failed".to_string(),
                            message: e.to_string(),
                            retryable: false,
                        };
                    }
                }
            }
        }

        #[cfg(feature = "scientia-youtube")]
        if let Some(yt) = &derived_youtube {
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
                let mut missing_asset = false;
                let resolved = if std::path::Path::new(yt.video_asset_ref.as_str()).is_absolute() {
                    std::path::PathBuf::from(yt.video_asset_ref.as_str())
                } else if let Some(root) = self.config.youtube_repo_root.as_deref() {
                    root.join(yt.video_asset_ref.as_str())
                } else {
                    std::path::PathBuf::from(yt.video_asset_ref.as_str())
                };
                if !resolved.is_file() {
                    result.youtube = ChannelOutcome::Disabled;
                    result.decision_reasons.insert(
                        "youtube".to_string(),
                        format!("missing_video_asset:{}", resolved.display()),
                    );
                    info!(
                        "Skipping YouTube publish for {}: missing payload {}",
                        item.id,
                        resolved.display()
                    );
                    missing_asset = true;
                }
                let mut youtube_precheck_failed = false;
                if !missing_asset
                    && let Err(e) = adapters::youtube::precheck_video_upload(&resolved)
                {
                    result.youtube = ChannelOutcome::Failed {
                        code: "youtube_precheck_failed".to_string(),
                        message: e.to_string(),
                        retryable: false,
                    };
                    youtube_precheck_failed = true;
                }
                if !missing_asset
                    && !youtube_precheck_failed
                    && let (Some(client_id), Some(client_secret), Some(refresh_token)) = (
                        self.config.youtube_client_id.as_deref(),
                        self.config.youtube_client_secret.as_deref(),
                        self.config.youtube_refresh_token.as_deref(),
                    )
                {
                    let auth = adapters::youtube::YouTubeAuthConfig {
                        client_id,
                        client_secret,
                        refresh_token,
                    };
                    match social_retry::run_with_retries(social_retry_budget, || {
                        adapters::youtube::upload_video(
                            &auth,
                            yt,
                            item,
                            self.config.youtube_repo_root.as_deref(),
                        )
                    })
                    .await
                    {
                        Ok(external_id) => {
                            result.youtube = ChannelOutcome::Success {
                                external_id: Some(external_id),
                            };
                            info!("Uploaded video to YouTube.");
                        }
                        Err(e) => {
                            result.youtube = ChannelOutcome::Failed {
                                code: "youtube_upload_failed".to_string(),
                                message: e.to_string(),
                                retryable: true,
                            };
                        }
                    }
                } else if !missing_asset && !youtube_precheck_failed {
                    result.youtube = ChannelOutcome::Failed {
                        code: "missing_youtube_credentials".to_string(),
                        message: "YouTube config present but OAuth credentials are incomplete."
                            .to_string(),
                        retryable: false,
                    };
                }
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
                };
            }
        }

        Ok(result)
    }
}
