//! Single source of truth for news publishing defaults and validation (no I/O).

/// Default public site base (GitHub Pages) for generated links and RSS.
pub const DEFAULT_SITE_BASE_URL: &str = "https://vox-foundation.github.io/vox";

/// Default path to RSS feed relative to repository root.
pub const DEFAULT_RSS_FEED_PATH: &str = "docs/src/feed.xml";

/// Default canonical GitHub repo for foundation news when templates omit overrides.
pub const DEFAULT_GITHUB_REPO: &str = "vox-foundation/vox";

/// Default Open Collective slug.
pub const DEFAULT_OPENCOLLECTIVE_SLUG: &str = "vox-foundation";

/// Production Twitter API v2 base URL.
pub const DEFAULT_TWITTER_API_BASE: &str = "https://api.twitter.com";

/// Production GitHub REST API base.
pub const DEFAULT_GITHUB_REST_BASE: &str = "https://api.github.com";

/// Production Open Collective GraphQL endpoint.
pub const DEFAULT_OPENCOLLECTIVE_GRAPHQL_URL: &str = "https://api.opencollective.com/graphql/v2";

/// Production GitHub GraphQL endpoint.
pub const DEFAULT_GITHUB_GRAPHQL_URL: &str = "https://api.github.com/graphql";

/// Max tweet body length used for conservative chunking (legacy limit; adjust when tier supports longer).
pub const TWITTER_TEXT_CHUNK_MAX: usize = 280;

/// Site configuration for RSS links and feed file location.
#[derive(Debug, Clone)]
pub struct NewsSiteConfig {
    /// e.g. `https://vox-foundation.github.io/vox` (no trailing slash).
    pub base_url: String,
    /// Path to `feed.xml` under repo root.
    pub rss_feed_path: std::path::PathBuf,
}

impl Default for NewsSiteConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_SITE_BASE_URL.trim_end_matches('/').to_string(),
            rss_feed_path: std::path::PathBuf::from(DEFAULT_RSS_FEED_PATH),
        }
    }
}

impl NewsSiteConfig {
    #[must_use]
    pub fn news_item_link(&self, news_id: &str) -> String {
        format!("{}/news/{}.html", self.base_url, news_id)
    }

    #[must_use]
    pub fn feed_self_link(&self) -> String {
        format!("{}/feed.xml", self.base_url)
    }
}

/// Validate `owner/repo` format.
pub fn validate_github_repo(repo: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = repo.split('/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        anyhow::bail!("Invalid GitHub repo, expected owner/name, got {:?}", repo);
    }
    Ok(())
}

/// Reject path traversal in logical news ids (filename stems).
pub fn validate_news_id(id: &str) -> anyhow::Result<()> {
    if id.is_empty() {
        anyhow::bail!("news id must not be empty");
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        anyhow::bail!("news id must not contain path segments or '..': {:?}", id);
    }
    Ok(())
}
