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
/// Conservative Reddit title cap.
pub const REDDIT_TITLE_MAX: usize = 300;
/// HN title cap.
pub const HACKER_NEWS_TITLE_MAX: usize = 80;
/// YouTube title cap.
pub const YOUTUBE_TITLE_MAX: usize = 100;
/// YouTube description cap.
pub const YOUTUBE_DESCRIPTION_MAX: usize = 5000;

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

/// Clamp text to `max_chars` preserving UTF-8 boundaries and adding `...` when truncated.
#[must_use]
pub fn clamp_text(input: &str, max_chars: usize) -> String {
    let normalized = input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let keep = max_chars.saturating_sub(3);
    format!("{}...", normalized.chars().take(keep).collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::clamp_text;

    #[test]
    fn clamp_text_preserves_short_strings() {
        let out = clamp_text("hello world", 20);
        assert_eq!(out, "hello world");
    }

    #[test]
    fn clamp_text_truncates_and_appends_ellipsis() {
        let out = clamp_text("alpha beta gamma delta epsilon", 12);
        assert!(out.ends_with("..."));
        assert!(out.chars().count() <= 12);
    }
}
