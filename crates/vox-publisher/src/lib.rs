pub mod adapters;
mod bounded_fs;
pub mod contract;
pub mod gate;
pub mod publication;
pub mod publication_preflight;
pub mod publication_worthiness;
pub mod scholarly;
pub mod scientia_evidence;
pub mod scientific_metadata;
pub mod templates;
pub mod topic_packs;
pub mod types;
pub mod zenodo_metadata;

pub use topic_packs::{apply_topic_pack_from_metadata_json, hydrate_syndication_from_pack_id};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tracing::{info, warn};
#[cfg(feature = "scientia-youtube")]
use types::YouTubeConfig;
use types::{HackerNewsConfig, HackerNewsMode, TwitterConfig, UnifiedNewsItem};
#[cfg(feature = "scientia-reddit")]
use types::{RedditConfig, RedditPostKind};

pub use contract::NewsSiteConfig;

fn summarize_for_social(raw: &str, max_chars: usize) -> String {
    contract::clamp_text(raw, max_chars)
}

fn normalized_tags(item: &UnifiedNewsItem) -> Vec<String> {
    item.tags.iter().map(|t| t.trim().to_lowercase()).collect()
}

fn topic_score(item: &UnifiedNewsItem, include_tags: &[String]) -> f64 {
    if include_tags.is_empty() {
        return 1.0;
    }
    let tags = normalized_tags(item);
    let include_norm: Vec<String> = include_tags
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    if include_norm.is_empty() {
        return 1.0;
    }
    let matched = include_norm
        .iter()
        .filter(|needle| tags.iter().any(|t| t == *needle))
        .count();
    matched as f64 / include_norm.len() as f64
}

fn policy_block_reason(
    item: &UnifiedNewsItem,
    channel: &str,
    cfg: &PublisherConfig,
) -> Option<String> {
    let p = item
        .syndication
        .distribution_policy
        .channel_policy
        .get(channel)?;
    if p.enabled == Some(false) {
        return Some("policy_disabled".to_string());
    }
    if let Some(filters) = p.topic_filters.as_ref() {
        let tags = normalized_tags(item);
        let include_norm: Vec<String> = filters
            .include_tags
            .iter()
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        if !include_norm.is_empty()
            && !include_norm
                .iter()
                .any(|needle| tags.iter().any(|t| t == needle))
        {
            return Some("topic_filtered_out".to_string());
        }
        let exclude_norm: Vec<String> = filters
            .exclude_tags
            .iter()
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty())
            .collect();
        if exclude_norm
            .iter()
            .any(|needle| tags.iter().any(|t| t == needle))
        {
            return Some("topic_excluded".to_string());
        }
        if let Some(min) = filters.min_topic_score {
            let score = topic_score(item, &filters.include_tags);
            if score < min {
                return Some(format!("topic_score_below_min:{score:.3}<{min:.3}"));
            }
        }
    }
    if let Some(min_floor) = p.worthiness_floor {
        if let Some(actual) = cfg.worthiness_score {
            if actual < min_floor {
                return Some(format!("worthiness_below_floor:{actual:.3}<{min_floor:.3}"));
            }
        } else {
            return Some("worthiness_unavailable".to_string());
        }
    }
    None
}

#[derive(Clone)]
pub struct PublisherConfig {
    pub twitter_bearer_token: Option<String>,
    pub github_token: Option<String>,
    pub open_collective_token: Option<String>,
    pub dry_run: bool,
    pub site: NewsSiteConfig,
    pub twitter_api_base: Option<String>,
    pub github_rest_base: Option<String>,
    pub github_graphql_url: Option<String>,
    pub opencollective_graphql_url: Option<String>,
    pub twitter_text_chunk_max: Option<usize>,
    pub twitter_truncation_suffix: Option<String>,
    pub reddit_client_id: Option<String>,
    pub reddit_client_secret: Option<String>,
    pub reddit_refresh_token: Option<String>,
    pub reddit_user_agent: Option<String>,
    pub youtube_client_id: Option<String>,
    pub youtube_client_secret: Option<String>,
    pub youtube_refresh_token: Option<String>,
    pub youtube_repo_root: Option<std::path::PathBuf>,
    pub hacker_news_mode: Option<String>,
    pub worthiness_score: Option<f64>,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            twitter_bearer_token: None,
            github_token: None,
            open_collective_token: None,
            dry_run: true,
            site: NewsSiteConfig::default(),
            twitter_api_base: None,
            github_rest_base: None,
            github_graphql_url: None,
            opencollective_graphql_url: None,
            twitter_text_chunk_max: None,
            twitter_truncation_suffix: None,
            reddit_client_id: None,
            reddit_client_secret: None,
            reddit_refresh_token: None,
            reddit_user_agent: None,
            youtube_client_id: None,
            youtube_client_secret: None,
            youtube_refresh_token: None,
            youtube_repo_root: None,
            hacker_news_mode: None,
            worthiness_score: None,
        }
    }
}

/// Environment variable names read by [`PublisherConfig::from_operator_environment`].
///
/// Used by [`PublisherConfig::clear_route_simulation_env_overrides`] so route-simulation tests stay
/// deterministic when the shell exports news/social tokens.
pub const ROUTE_SIMULATION_ENV_KEYS: &[&str] = &[
    "VOX_NEWS_TWITTER_TOKEN",
    "VOX_NEWS_GITHUB_TOKEN",
    "VOX_NEWS_OPENCOLLECTIVE_TOKEN",
    "VOX_SOCIAL_REDDIT_CLIENT_ID",
    "VOX_SOCIAL_REDDIT_CLIENT_SECRET",
    "VOX_SOCIAL_REDDIT_REFRESH_TOKEN",
    "VOX_SOCIAL_REDDIT_USER_AGENT",
    "VOX_SOCIAL_YOUTUBE_CLIENT_ID",
    "VOX_SOCIAL_YOUTUBE_CLIENT_SECRET",
    "VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN",
    "VOX_SOCIAL_HN_MODE",
];

impl PublisherConfig {
    /// Unset process env vars that influence [`Self::from_operator_environment`] (for tests).
    pub fn clear_route_simulation_env_overrides() {
        for &key in ROUTE_SIMULATION_ENV_KEYS {
            // SAFETY: test helper; must not run concurrently with other threads reading the same env keys.
            unsafe {
                std::env::remove_var(key);
            }
        }
    }

    /// Build a publisher config for operator surfaces (`vox db publication-*`, MCP Scientia tools).
    ///
    /// Loads optional credentials from the standard `VOX_NEWS_*` / `VOX_SOCIAL_*` environment keys
    /// and applies the given [`NewsSiteConfig`] (CLI typically uses [`NewsSiteConfig::default`];
    /// MCP may override `base_url` from orchestrator config).
    #[must_use]
    pub fn from_operator_environment(
        dry_run: bool,
        youtube_repo_root: Option<std::path::PathBuf>,
        site: NewsSiteConfig,
    ) -> Self {
        let env_opt = |k: &str| {
            std::env::var(k)
                .ok()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())
        };
        Self {
            twitter_bearer_token: env_opt("VOX_NEWS_TWITTER_TOKEN"),
            github_token: env_opt("VOX_NEWS_GITHUB_TOKEN"),
            open_collective_token: env_opt("VOX_NEWS_OPENCOLLECTIVE_TOKEN"),
            reddit_client_id: env_opt("VOX_SOCIAL_REDDIT_CLIENT_ID"),
            reddit_client_secret: env_opt("VOX_SOCIAL_REDDIT_CLIENT_SECRET"),
            reddit_refresh_token: env_opt("VOX_SOCIAL_REDDIT_REFRESH_TOKEN"),
            reddit_user_agent: env_opt("VOX_SOCIAL_REDDIT_USER_AGENT"),
            youtube_client_id: env_opt("VOX_SOCIAL_YOUTUBE_CLIENT_ID"),
            youtube_client_secret: env_opt("VOX_SOCIAL_YOUTUBE_CLIENT_SECRET"),
            youtube_refresh_token: env_opt("VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN"),
            hacker_news_mode: env_opt("VOX_SOCIAL_HN_MODE"),
            youtube_repo_root,
            dry_run,
            site,
            ..Default::default()
        }
    }
}

pub struct Publisher {
    config: PublisherConfig,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyndicationResult {
    pub rss: ChannelOutcome,
    pub twitter: ChannelOutcome,
    pub github: ChannelOutcome,
    pub open_collective: ChannelOutcome,
    pub reddit: ChannelOutcome,
    pub hacker_news: ChannelOutcome,
    pub youtube: ChannelOutcome,
    #[serde(default)]
    pub decision_reasons: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
#[derive(Default)]
pub enum ChannelOutcome {
    #[default]
    Disabled,
    DryRun {
        external_id: Option<String>,
    },
    Success {
        external_id: Option<String>,
    },
    Failed {
        code: String,
        message: String,
        retryable: bool,
    },
}

impl SyndicationResult {
    #[must_use]
    pub fn has_failures(&self) -> bool {
        [
            &self.rss,
            &self.twitter,
            &self.github,
            &self.open_collective,
            &self.reddit,
            &self.hacker_news,
            &self.youtube,
        ]
        .iter()
        .any(|o| matches!(o, ChannelOutcome::Failed { .. }))
    }

    #[must_use]
    pub fn all_enabled_channels_succeeded(&self, item: &UnifiedNewsItem) -> bool {
        fn ok(outcome: &ChannelOutcome) -> bool {
            matches!(
                outcome,
                ChannelOutcome::Success { .. }
                    | ChannelOutcome::Disabled
                    | ChannelOutcome::DryRun { .. }
            )
        }

        let rss_ok = !item.syndication.rss || ok(&self.rss);
        let twitter_ok = item.syndication.twitter.is_none() || ok(&self.twitter);
        let github_ok = item.syndication.github.is_none() || ok(&self.github);
        let oc_ok = item.syndication.open_collective.is_none() || ok(&self.open_collective);
        let reddit_ok = item.syndication.reddit.is_none() || ok(&self.reddit);
        let hn_ok = item.syndication.hacker_news.is_none() || ok(&self.hacker_news);
        let yt_ok = item.syndication.youtube.is_none() || ok(&self.youtube);
        rss_ok && twitter_ok && github_ok && oc_ok && reddit_ok && hn_ok && yt_ok
    }

    #[must_use]
    pub fn github_id(&self) -> Option<&str> {
        match &self.github {
            ChannelOutcome::Success {
                external_id: Some(v),
            }
            | ChannelOutcome::DryRun {
                external_id: Some(v),
            } => Some(v.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn twitter_id(&self) -> Option<&str> {
        match &self.twitter {
            ChannelOutcome::Success {
                external_id: Some(v),
            }
            | ChannelOutcome::DryRun {
                external_id: Some(v),
            } => Some(v.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn oc_id(&self) -> Option<&str> {
        match &self.open_collective {
            ChannelOutcome::Success {
                external_id: Some(v),
            }
            | ChannelOutcome::DryRun {
                external_id: Some(v),
            } => Some(v.as_str()),
            _ => None,
        }
    }
}

impl Publisher {
    pub fn new(config: PublisherConfig) -> Self {
        Self { config }
    }

    pub async fn publish_all(&self, item: &UnifiedNewsItem) -> Result<SyndicationResult> {
        item.validate()?;
        info!("Starting syndication for news item: {}", item.id);
        let mut result = SyndicationResult::default();

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
                    cfg.short_text = Some(summarize_for_social(
                        item.content_markdown.as_str(),
                        contract::TWITTER_TEXT_CHUNK_MAX.saturating_sub(20),
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
        let derived_summary = summarize_for_social(social_source, 700);
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
                    cfg.title_override = Some(contract::clamp_text(
                        item.title.as_str(),
                        contract::REDDIT_TITLE_MAX,
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
                    cfg.title_override = Some(contract::clamp_text(
                        format!("{} (video)", item.title).as_str(),
                        contract::HACKER_NEWS_TITLE_MAX,
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
                    cfg.description_override = Some(summarize_for_social(
                        item.content_markdown.as_str(),
                        contract::YOUTUBE_DESCRIPTION_MAX,
                    ));
                }
                if cfg
                    .title_override
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    cfg.title_override = Some(contract::clamp_text(
                        item.title.as_str(),
                        contract::YOUTUBE_TITLE_MAX,
                    ));
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
                match adapters::rss::update_feed(item, &self.config.site).await {
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
                match adapters::twitter::post(&self.config, token, item, twitter).await {
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

        if let Some(github) = &item.syndication.github {
            if let Some(reason) = policy_block_reason(item, "github", &self.config) {
                result.github = ChannelOutcome::Disabled;
                result.decision_reasons.insert("github".to_string(), reason);
            } else if is_dry_run {
                info!(
                    "[DRY RUN] Would post to GitHub repository {} as {:?}",
                    github.repo, github.post_type
                );
                result.github = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-github-{}", item.id)),
                };
            } else if let Some(token) = &self.config.github_token {
                match adapters::github::post(&self.config, token, item, github).await {
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
                match adapters::opencollective::post(&self.config, token, item, oc).await {
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
                match adapters::reddit::submit(&auth, item, reddit, canonical_link.as_str()).await {
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
            result.reddit = ChannelOutcome::Failed {
                code: "reddit_feature_disabled".to_string(),
                message: "reddit publishing requires vox-publisher feature `scientia-reddit`."
                    .to_string(),
                retryable: false,
            };
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
            } else if let (Some(client_id), Some(client_secret), Some(refresh_token)) = (
                self.config.youtube_client_id.as_deref(),
                self.config.youtube_client_secret.as_deref(),
                self.config.youtube_refresh_token.as_deref(),
            ) {
                let auth = adapters::youtube::YouTubeAuthConfig {
                    client_id,
                    client_secret,
                    refresh_token,
                };
                match adapters::youtube::upload_video(
                    &auth,
                    yt,
                    item,
                    self.config.youtube_repo_root.as_deref(),
                )
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
            } else {
                result.youtube = ChannelOutcome::Failed {
                    code: "missing_youtube_credentials".to_string(),
                    message: "YouTube config present but OAuth credentials are incomplete."
                        .to_string(),
                    retryable: false,
                };
            }
        }

        #[cfg(not(feature = "scientia-youtube"))]
        if item.syndication.youtube.is_some() {
            result.youtube = ChannelOutcome::Failed {
                code: "youtube_feature_disabled".to_string(),
                message: "youtube support requires vox-publisher feature `scientia-youtube`."
                    .to_string(),
                retryable: false,
            };
        }

        Ok(result)
    }
}
