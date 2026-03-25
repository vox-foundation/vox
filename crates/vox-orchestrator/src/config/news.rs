use serde::{Deserialize, Serialize};

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
        }
    }
}
