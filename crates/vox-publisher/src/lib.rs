pub mod adapters;
mod bounded_fs;
pub mod citation_cff;
pub mod contract;
pub mod crossref_metadata;
pub mod gate;
pub mod openreview_api_types;
pub mod publication;
pub mod publication_preflight;
pub mod publication_worthiness;
pub mod scholarly;
#[cfg(feature = "scholarly-external-jobs")]
pub mod scholarly_external_jobs;
#[cfg(feature = "scholarly-external-jobs")]
pub mod scholarly_remote_status;
pub mod scientia_evidence;
pub mod scientific_metadata;
pub mod submission_package;
pub mod switching;
pub mod templates;
pub mod topic_packs;
pub mod types;
pub mod zenodo_api_types;
pub mod zenodo_metadata;

mod social_retry;
mod syndication_outcome;

pub use syndication_outcome::{ChannelOutcome, SyndicationResult};
pub use topic_packs::{apply_topic_pack_from_metadata_json, hydrate_syndication_from_pack_id};

use anyhow::Result;
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

#[must_use]
fn syndication_template_profile_enabled() -> bool {
    std::env::var("VOX_SYNDICATION_TEMPLATE_PROFILE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn channel_template_profile_label(item: &UnifiedNewsItem, channel: &str) -> Option<String> {
    let map = &item.syndication.distribution_policy.channel_policy;
    let p = map.get(channel).or_else(|| {
        map.iter()
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(channel))
            .map(|(_, v)| v)
    })?;
    p.template_profile
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string)
}

fn note_template_profile_inert(
    item: &UnifiedNewsItem,
    channel: &str,
    decision_reasons: &mut BTreeMap<String, String>,
) {
    if channel_template_profile_label(item, channel).is_some()
        && !syndication_template_profile_enabled()
        && !decision_reasons.contains_key("template_profile_inert")
    {
        decision_reasons.insert(
            "template_profile_inert".to_string(),
            "VOX_SYNDICATION_TEMPLATE_PROFILE is not enabled; channel template_profile keys are ignored"
                .to_string(),
        );
    }
}

fn twitter_effective_summary_max_chars(
    item: &UnifiedNewsItem,
    cfg: &PublisherConfig,
    decision_reasons: &mut BTreeMap<String, String>,
) -> usize {
    let margin_base = cfg
        .twitter_summary_margin_chars
        .unwrap_or(contract::TWITTER_SUMMARY_MARGIN_CHARS);
    note_template_profile_inert(item, "twitter", decision_reasons);
    if !syndication_template_profile_enabled() {
        return contract::TWITTER_TEXT_CHUNK_MAX.saturating_sub(margin_base);
    }
    let Some(ref p) = channel_template_profile_label(item, "twitter") else {
        return contract::TWITTER_TEXT_CHUNK_MAX.saturating_sub(margin_base);
    };
    let p_low = p.to_ascii_lowercase();
    let margin_adj = match p_low.as_str() {
        "brief" | "tight" | "compact" => margin_base.saturating_sub(16).max(4),
        "roomy" | "spacious" | "narrative" => (margin_base.saturating_add(24))
            .min(contract::TWITTER_TEXT_CHUNK_MAX.saturating_div(3)),
        _ => {
            decision_reasons.insert(
                "template_profile_fallback_twitter".to_string(),
                format!("unknown template_profile {p:?}; using default twitter margin"),
            );
            margin_base
        }
    };
    decision_reasons.insert(
        "template_profile_resolved_twitter".to_string(),
        format!("{p}:margin_chars={margin_adj}"),
    );
    contract::TWITTER_TEXT_CHUNK_MAX.saturating_sub(margin_adj)
}

#[cfg(any(feature = "scientia-reddit", feature = "scientia-youtube"))]
fn social_text_cap_with_template_profile(
    item: &UnifiedNewsItem,
    channel: &str,
    base_cap: usize,
    decision_reasons: &mut BTreeMap<String, String>,
) -> usize {
    note_template_profile_inert(item, channel, decision_reasons);
    if !syndication_template_profile_enabled() {
        return base_cap;
    }
    let Some(ref p) = channel_template_profile_label(item, channel) else {
        return base_cap;
    };
    let p_low = p.to_ascii_lowercase();
    let scaled = match p_low.as_str() {
        "brief" | "tight" | "compact" => base_cap.saturating_mul(88).saturating_div(100).max(120),
        "roomy" | "spacious" | "narrative" => base_cap
            .saturating_mul(114)
            .saturating_div(100)
            .min(base_cap.saturating_add(900)),
        _ => {
            decision_reasons.insert(
                format!("template_profile_fallback_{channel}"),
                format!("unknown template_profile {p:?}; using default cap"),
            );
            base_cap
        }
    };
    decision_reasons.insert(
        format!("template_profile_resolved_{channel}"),
        format!("{p}:cap={scaled}"),
    );
    scaled
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
    let map = &item.syndication.distribution_policy.channel_policy;
    let p = map.get(channel).or_else(|| {
        map.iter()
            .find(|(k, _)| k.trim().eq_ignore_ascii_case(channel))
            .map(|(_, v)| v)
    })?;
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
    pub twitter_summary_margin_chars: Option<usize>,
    pub reddit_selfpost_summary_max: Option<usize>,
    pub reddit_client_id: Option<String>,
    pub reddit_client_secret: Option<String>,
    pub reddit_refresh_token: Option<String>,
    pub reddit_user_agent: Option<String>,
    pub youtube_client_id: Option<String>,
    pub youtube_client_secret: Option<String>,
    pub youtube_refresh_token: Option<String>,
    pub youtube_repo_root: Option<std::path::PathBuf>,
    pub hacker_news_mode: Option<String>,
    pub youtube_default_category_id: Option<String>,
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
            twitter_summary_margin_chars: None,
            reddit_selfpost_summary_max: None,
            reddit_client_id: None,
            reddit_client_secret: None,
            reddit_refresh_token: None,
            reddit_user_agent: None,
            youtube_client_id: None,
            youtube_client_secret: None,
            youtube_refresh_token: None,
            youtube_repo_root: None,
            hacker_news_mode: None,
            youtube_default_category_id: None,
            worthiness_score: None,
        }
    }
}

/// Environment variable names read by [`PublisherConfig::from_operator_environment`].
///
/// Used by [`PublisherConfig::clear_route_simulation_env_overrides`] so route-simulation tests stay
/// deterministic when the shell exports news/social tokens.
pub const ROUTE_SIMULATION_ENV_KEYS: &[&str] = &[
    "VOX_NEWS_SITE_BASE_URL",
    "VOX_NEWS_RSS_FEED_PATH",
    "VOX_NEWS_TWITTER_TOKEN",
    "VOX_NEWS_GITHUB_TOKEN",
    "VOX_NEWS_OPENCOLLECTIVE_TOKEN",
    "VOX_NEWS_TWITTER_TEXT_CHUNK_MAX",
    "VOX_NEWS_TWITTER_TRUNCATION_SUFFIX",
    "VOX_SOCIAL_REDDIT_CLIENT_ID",
    "VOX_SOCIAL_REDDIT_CLIENT_SECRET",
    "VOX_SOCIAL_REDDIT_REFRESH_TOKEN",
    "VOX_SOCIAL_REDDIT_USER_AGENT",
    "VOX_SOCIAL_YOUTUBE_CLIENT_ID",
    "VOX_SOCIAL_YOUTUBE_CLIENT_SECRET",
    "VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN",
    "VOX_SOCIAL_HN_MODE",
    "VOX_SOCIAL_TWITTER_SUMMARY_MARGIN_CHARS",
    "VOX_SOCIAL_REDDIT_SELFPOST_SUMMARY_MAX",
    "VOX_SOCIAL_YOUTUBE_DEFAULT_CATEGORY_ID",
    "VOX_SCHOLARLY_ADAPTER",
    "VOX_SCHOLARLY_DISABLE",
    "VOX_SCHOLARLY_DISABLE_LIVE",
    "VOX_SCHOLARLY_DISABLE_ZENODO",
    "VOX_SCHOLARLY_DISABLE_OPENREVIEW",
    "VOX_OPENREVIEW_API_BASE",
    "VOX_OPENREVIEW_INVITATION",
    "VOX_OPENREVIEW_SIGNATURE",
    "VOX_OPENREVIEW_ACCESS_TOKEN",
    "OPENREVIEW_API_BASE",
    "OPENREVIEW_INVITATION",
    "OPENREVIEW_SIGNATURE",
    "OPENREVIEW_ACCESS_TOKEN",
    "OPENREVIEW_EMAIL",
    "OPENREVIEW_PASSWORD",
    "VOX_ZENODO_SANDBOX",
    "VOX_ZENODO_API_BASE",
    "VOX_ZENODO_HTTP_MAX_ATTEMPTS",
    "VOX_ZENODO_ATTACH_MANIFEST_BODY",
    "VOX_ZENODO_PUBLISH_DEPOSITION",
    "VOX_ZENODO_DRAFT_ONLY",
    "VOX_ZENODO_PUBLISH_NOW",
    "VOX_ZENODO_STAGING_DIR",
    "VOX_ZENODO_UPLOAD_ALLOWLIST",
    "VOX_ZENODO_VERIFY_STAGING_CHECKSUMS",
    "VOX_ZENODO_REQUIRE_METADATA_PARITY",
    "VOX_OPENREVIEW_HTTP_MAX_ATTEMPTS",
    "VOX_SOCIAL_WORTHINESS_ENFORCE",
    "VOX_SOCIAL_WORTHINESS_SCORE_MIN",
];

impl PublisherConfig {
    #[inline]
    fn syndication_secret(id: vox_clavis::SecretId) -> Option<String> {
        vox_clavis::resolve_secret(id)
            .expose()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

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
    /// Resolves syndication credentials through Clavis (`vox_clavis::resolve_secret`) for the standard
    /// `VOX_NEWS_*` / `VOX_SOCIAL_*` secret specs (see `vox_publisher::contract` for precedence).
    /// Applies the given [`NewsSiteConfig`]. CLI typically uses
    /// [`NewsSiteConfig::from_default_with_operator_env`]. MCP builds from `[orchestrator.news]` then
    /// [`NewsSiteConfig::merge_operator_env_overrides`].
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
        let env_usize = |k: &str| {
            env_opt(k).and_then(|v| match v.parse::<usize>() {
                Ok(n) => Some(n),
                Err(_) => {
                    warn!(
                        target: "vox.publisher.config",
                        key = k,
                        value = v,
                        "invalid usize env override; ignoring"
                    );
                    None
                }
            })
        };
        Self {
            twitter_bearer_token: Self::syndication_secret(
                vox_clavis::SecretId::VoxNewsTwitterBearer,
            ),
            github_token: Self::syndication_secret(vox_clavis::SecretId::GitHubToken),
            open_collective_token: Self::syndication_secret(
                vox_clavis::SecretId::VoxNewsOpenCollectiveToken,
            ),
            twitter_summary_margin_chars: env_usize("VOX_SOCIAL_TWITTER_SUMMARY_MARGIN_CHARS"),
            reddit_selfpost_summary_max: env_usize("VOX_SOCIAL_REDDIT_SELFPOST_SUMMARY_MAX"),
            reddit_client_id: Self::syndication_secret(
                vox_clavis::SecretId::VoxSocialRedditClientId,
            ),
            reddit_client_secret: Self::syndication_secret(
                vox_clavis::SecretId::VoxSocialRedditClientSecret,
            ),
            reddit_refresh_token: Self::syndication_secret(
                vox_clavis::SecretId::VoxSocialRedditRefreshToken,
            ),
            reddit_user_agent: Self::syndication_secret(
                vox_clavis::SecretId::VoxSocialRedditUserAgent,
            ),
            youtube_client_id: Self::syndication_secret(
                vox_clavis::SecretId::VoxSocialYoutubeClientId,
            ),
            youtube_client_secret: Self::syndication_secret(
                vox_clavis::SecretId::VoxSocialYoutubeClientSecret,
            ),
            youtube_refresh_token: Self::syndication_secret(
                vox_clavis::SecretId::VoxSocialYoutubeRefreshToken,
            ),
            hacker_news_mode: env_opt("VOX_SOCIAL_HN_MODE"),
            youtube_default_category_id: env_opt("VOX_SOCIAL_YOUTUBE_DEFAULT_CATEGORY_ID"),
            twitter_text_chunk_max: env_usize("VOX_NEWS_TWITTER_TEXT_CHUNK_MAX"),
            twitter_truncation_suffix: env_opt("VOX_NEWS_TWITTER_TRUNCATION_SUFFIX"),
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

impl Publisher {
    pub fn new(config: PublisherConfig) -> Self {
        Self { config }
    }

    pub async fn publish_all(&self, item: &UnifiedNewsItem) -> Result<SyndicationResult> {
        item.validate()?;
        info!("Starting syndication for news item: {}", item.id);
        let mut result = SyndicationResult::default();
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
            self
                .config
                .reddit_selfpost_summary_max
                .unwrap_or(contract::REDDIT_SELFPOST_SUMMARY_MAX),
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
                    let yt_cap = social_text_cap_with_template_profile(
                        item,
                        "youtube",
                        contract::YOUTUBE_DESCRIPTION_MAX,
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
                    cfg.title_override = Some(contract::clamp_text(
                        item.title.as_str(),
                        contract::YOUTUBE_TITLE_MAX,
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
                match social_retry::run_with_retries(social_retry_budget, || {
                    adapters::github::post(&self.config, token, item, github)
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
                    && let Err(e) = adapters::youtube::precheck_video_upload(&resolved) {
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
