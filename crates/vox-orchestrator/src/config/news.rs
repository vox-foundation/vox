use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::defaults::default_true;

/// Unified news syndication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NewsConfig {
    /// Whether the background news monitor is active (default: false).
    pub enabled: bool,
    /// Relative path to watch for new Markdown news items (default: "docs/news").
    pub news_dir: String,
    /// When true, walk `news_dir` recursively (includes `drafts/` subfolders).
    #[serde(default = "default_true")]
    pub scan_recursive: bool,
    /// Personal access token for GitHub Releases (Octocrab).
    pub github_token: Option<String>,
    /// Bearer token for Twitter X API v2 (reqwest).
    pub twitter_token: Option<String>,
    /// API Key for Open Collective GraphQL v2 (reqwest).
    pub opencollective_token: Option<String>,
    /// Reddit OAuth app client id.
    pub reddit_client_id: Option<String>,
    /// Reddit OAuth app client secret.
    pub reddit_client_secret: Option<String>,
    /// Reddit OAuth refresh token for posting identity.
    pub reddit_refresh_token: Option<String>,
    /// Reddit API User-Agent string, e.g. platform:app:v1 (by /u/name).
    pub reddit_user_agent: Option<String>,
    /// YouTube OAuth client id.
    pub youtube_client_id: Option<String>,
    /// YouTube OAuth client secret.
    pub youtube_client_secret: Option<String>,
    /// YouTube OAuth refresh token.
    pub youtube_refresh_token: Option<String>,
    /// Bluesky PDS handle (canonical identifier).
    pub bluesky_handle: Option<String>,
    /// Bluesky specialized app-password (not main login).
    pub bluesky_password: Option<String>,
    /// Mastodon app instance access token.
    pub mastodon_token: Option<String>,
    /// Mastodon instance domain, e.g. "mastodon.social".
    pub mastodon_domain: Option<String>,
    /// LinkedIn OAuth access token.
    pub linkedin_token: Option<String>,
    /// Discord webhook URL for automated news delivery.
    pub discord_webhook: Option<String>,
    /// Hacker News routing mode (`manual_assist`).
    pub hacker_news_mode: Option<String>,
    /// Global flag to force local testing only without actually calling external publish endpoints.
    pub dry_run: bool,
    /// Must be true (or `VOX_NEWS_PUBLISH_ARMED=1`) before any **live** syndication attempt.
    #[serde(default)]
    pub publish_armed: bool,
    /// Override public site URL for RSS links (default: vox-publisher contract default).
    #[serde(default)]
    pub site_base_url: Option<String>,
    /// Path to `feed.xml` relative to repo root.
    #[serde(default)]
    pub rss_feed_path: Option<String>,
    #[serde(default)]
    pub opencollective_graphql_url: Option<String>,
    #[serde(default)]
    pub github_graphql_url: Option<String>,
    #[serde(default)]
    pub github_rest_base: Option<String>,
    #[serde(default)]
    pub twitter_api_base: Option<String>,
    /// Optional override for tweet chunk max chars (defaults to publisher contract constant).
    #[serde(default)]
    pub twitter_text_chunk_max: Option<usize>,
    /// Optional truncation suffix for non-thread tweet shortening (default "...").
    #[serde(default)]
    pub twitter_truncation_suffix: Option<String>,
    /// When true, block live fan-out unless worthiness decision is `Publish`.
    #[serde(default)]
    pub worthiness_enforce: bool,
    /// Optional minimum score floor before live fan-out is allowed.
    #[serde(default)]
    pub worthiness_score_min: Option<f64>,
    /// Optional per-channel worthiness floors (`channel -> floor`).
    #[serde(default)]
    pub channel_worthiness_floors: BTreeMap<String, f64>,
}

impl Default for NewsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            news_dir: "docs/news".to_string(),
            scan_recursive: true,
            github_token: None,
            twitter_token: None,
            opencollective_token: None,
            reddit_client_id: None,
            reddit_client_secret: None,
            reddit_refresh_token: None,
            reddit_user_agent: None,
            youtube_client_id: None,
            youtube_client_secret: None,
            youtube_refresh_token: None,
            bluesky_handle: None,
            bluesky_password: None,
            mastodon_token: None,
            mastodon_domain: None,
            linkedin_token: None,
            discord_webhook: None,
            hacker_news_mode: None,
            dry_run: true,
            publish_armed: false,
            site_base_url: None,
            rss_feed_path: None,
            opencollective_graphql_url: None,
            github_graphql_url: None,
            github_rest_base: None,
            twitter_api_base: None,
            twitter_text_chunk_max: None,
            twitter_truncation_suffix: None,
            worthiness_enforce: false,
            worthiness_score_min: None,
            channel_worthiness_floors: BTreeMap::new(),
        }
    }
}
