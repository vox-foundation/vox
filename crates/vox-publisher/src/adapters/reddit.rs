use crate::types::{RedditConfig, RedditPostKind, UnifiedNewsItem};
use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;

pub const AUTH_BASE: &str = "https://www.reddit.com";
pub const OAUTH_BASE: &str = "https://oauth.reddit.com";
pub const TITLE_MAX: usize = 300;
pub const SELFPOST_BODY_MAX: usize = 40_000;
pub const SELFPOST_SUMMARY_MAX: usize = 700;

#[derive(Debug, Deserialize)]
struct RedditAccessTokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
struct RedditSubmitResponse {
    json: Option<RedditJsonWrapper>,
}

#[derive(Debug, Deserialize)]
struct RedditJsonWrapper {
    errors: Vec<(String, String, String)>,
    data: Option<RedditSubmitData>,
}

#[derive(Debug, Deserialize)]
struct RedditSubmitData {
    url: Option<String>,
}

pub struct RedditAuthConfig<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub refresh_token: &'a str,
    pub user_agent: &'a str,
    pub api_base: Option<&'a str>,
}

async fn refresh_access_token(auth: &RedditAuthConfig<'_>) -> Result<String> {
    let client = Client::new();
    let mut body = HashMap::new();
    body.insert("grant_type", "refresh_token");
    body.insert("refresh_token", auth.refresh_token);
    let base = auth
        .api_base
        .unwrap_or("https://www.reddit.com")
        .trim_end_matches('/');
    let url = format!("{}/api/v1/access_token", base);
    let res = client
        .post(&url)
        .basic_auth(auth.client_id, Some(auth.client_secret))
        .header("User-Agent", auth.user_agent)
        .form(&body)
        .send()
        .await
        .context("reddit oauth refresh request")?;
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(anyhow!("reddit oauth refresh failed: {t}"));
    }
    let parsed: RedditAccessTokenResponse = res.json().await.context("reddit oauth parse")?;
    if parsed.access_token.trim().is_empty() {
        return Err(anyhow!("reddit oauth refresh returned empty access_token"));
    }
    Ok(parsed.access_token)
}

pub async fn submit(
    auth: &RedditAuthConfig<'_>,
    item: &UnifiedNewsItem,
    cfg: &RedditConfig,
    default_url: &str,
) -> Result<String> {
    if let RedditPostKind::SelfPost = cfg.kind {
        let text = cfg
            .text_override
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(item.content_markdown.as_str());
        if text.chars().count() > SELFPOST_BODY_MAX {
            return Err(anyhow!(
                "Reddit self-post body ({} chars) exceeds {} char server limit",
                text.chars().count(),
                SELFPOST_BODY_MAX
            ));
        }
    }

    let token = refresh_access_token(auth).await?;
    let title = cfg
        .title_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(item.title.as_str());
    let mut form: HashMap<&str, String> = HashMap::new();
    form.insert("api_type", "json".to_string());
    form.insert(
        "sr",
        cfg.subreddit
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string(),
    );
    form.insert("title", title.to_string());
    form.insert("nsfw", cfg.nsfw.to_string());
    form.insert("spoiler", cfg.spoiler.to_string());
    form.insert("sendreplies", cfg.send_replies.to_string());

    match cfg.kind {
        RedditPostKind::Link => {
            let url = cfg
                .url_override
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(default_url);
            form.insert("kind", "link".to_string());
            form.insert("url", url.to_string());
        }
        RedditPostKind::SelfPost => {
            form.insert("kind", "self".to_string());
            let text = cfg
                .text_override
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(item.content_markdown.as_str());
            form.insert("text", text.to_string());
        }
    }

    let client = Client::new();
    let base = auth
        .api_base
        .unwrap_or("https://oauth.reddit.com")
        .trim_end_matches('/');
    let url = format!("{}/api/submit", base);
    let res = client
        .post(&url)
        .bearer_auth(token)
        .header("User-Agent", auth.user_agent)
        .form(&form)
        .send()
        .await
        .context("reddit submit request")?;
    if !res.status().is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(anyhow!("reddit submit failed: {t}"));
    }
    let parsed: RedditSubmitResponse = res.json().await.context("reddit submit parse")?;
    if let Some(wrapper) = parsed.json {
        if !wrapper.errors.is_empty() {
            return Err(anyhow!("reddit submit errors: {:?}", wrapper.errors));
        }
        if let Some(url) = wrapper.data.and_then(|d| d.url)
            && !url.trim().is_empty()
        {
            return Ok(url);
        }
    }
    Ok("reddit_submitted".to_string())
}
