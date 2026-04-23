use crate::PublisherConfig;
pub const API_BASE: &str = "https://api.twitter.com";
pub const POST_PATH: &str = "/2/tweets";
pub const TWEET_MAX_CHARS: usize = 280;
pub const SUMMARY_MARGIN: usize = 20;
pub const CANARY_PATH: &str = "/2/users/me";
use crate::types::UnifiedNewsItem;
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

pub async fn post(
    publisher_cfg: &PublisherConfig,
    token: &str,
    item: &UnifiedNewsItem,
    override_cfg: Option<&crate::types::TwitterOverride>,
    dry_run: bool,
) -> Result<String> {
    if dry_run {
        return Ok(format!("dry-run-twitter-{}", item.id));
    }

    let client = Client::new();
    let root = publisher_cfg
        .twitter_api_base
        .clone()
        .unwrap_or_else(|| API_BASE.to_string());
    let root = root.trim_end_matches('/').to_string();
    let url = format!("{}/2/tweets", root);
    let chunk_max = publisher_cfg
        .twitter_text_chunk_max
        .unwrap_or(TWEET_MAX_CHARS)
        .max(1);
    let truncation_suffix = publisher_cfg
        .twitter_truncation_suffix
        .as_deref()
        .unwrap_or("...");

    let primary_text = truncate_chars(&item.content_markdown, chunk_max, truncation_suffix);

    let should_thread = override_cfg
        .map(|c| c.thread)
        .unwrap_or_else(|| item.content_markdown.chars().count() > chunk_max);
    let mut texts = if should_thread {
        let full = item.content_markdown.clone();
        chunk_chars(&full, chunk_max)
    } else {
        vec![primary_text]
    };

    if texts.is_empty() {
        texts.push(String::new());
    }

    let mut last_id: Option<String> = None;
    for (i, text) in texts.iter().enumerate() {
        let body = if i == 0 {
            json!({ "text": text })
        } else {
            json!({
                "text": text,
                "reply": { "in_reply_to_tweet_id": last_id.as_deref().unwrap_or("") }
            })
        };

        let res = client
            .post(&url)
            .bearer_auth(token)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            if res.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let reset = res
                    .headers()
                    .get("x-rate-limit-reset")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown");
                return Err(anyhow!(
                    "Twitter rate limited (429); rate limit resets at Unix time {}. \
                     Retry budget will apply standard backoff.",
                    reset
                ));
            }
            let err_text = res.text().await?;
            return Err(anyhow!("Twitter API error: {}", err_text));
        }

        let data: serde_json::Value = res.json().await?;
        last_id = data["data"]["id"]
            .as_str()
            .map(std::string::ToString::to_string)
            .filter(|s| !s.is_empty());
        if last_id.is_none() {
            return Err(anyhow!(
                "Twitter API missing tweet id in response: {}",
                data
            ));
        }
    }

    Ok(last_id.unwrap_or_default())
}

fn truncate_chars(s: &str, max_chars: usize, suffix: &str) -> String {
    use unicode_segmentation::UnicodeSegmentation;
    let graphemes: Vec<&str> = s.graphemes(true).collect();
    if graphemes.len() <= max_chars {
        return s.to_string();
    }
    let suffix_graphemes: Vec<&str> = suffix.graphemes(true).collect();
    let take = max_chars.saturating_sub(suffix_graphemes.len());
    format!("{}{}", graphemes[..take].concat(), suffix)
}

fn chunk_chars(s: &str, max_chars: usize) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    if max_chars == 0 {
        return vec![s.to_string()];
    }
    let graphemes: Vec<&str> = s.graphemes(true).collect();
    if graphemes.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < graphemes.len() {
        let end = (i + max_chars).min(graphemes.len());
        out.push(graphemes[i..end].concat());
        i = end;
    }
    out
}
