use crate::types::{HackerNewsConfig, UnifiedNewsItem};
use anyhow::Result;

pub const TITLE_MAX: usize = 80;

/// Hacker News native path is manual-assist via submitlink URL.
pub async fn post_manual_assist(
    item: &UnifiedNewsItem,
    config: &HackerNewsConfig,
    default_url: &str,
) -> Result<String> {
    let title = config
        .title_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(item.title.as_str());
    let url = config
        .url_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(default_url);
    let mut submit = reqwest::Url::parse("https://news.ycombinator.com/submitlink")?;
    submit
        .query_pairs_mut()
        .append_pair("u", url)
        .append_pair("t", title);
    Ok(submit.to_string())
}
