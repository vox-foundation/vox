use crate::PublisherConfig;
use crate::contract::{DEFAULT_TWITTER_API_BASE, TWITTER_TEXT_CHUNK_MAX};
use crate::types::{TwitterConfig, UnifiedNewsItem};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

pub async fn post(
    publisher_cfg: &PublisherConfig,
    token: &str,
    item: &UnifiedNewsItem,
    config: &TwitterConfig,
) -> Result<String> {
    let client = Client::new();
    let root = publisher_cfg
        .twitter_api_base
        .clone()
        .unwrap_or_else(|| DEFAULT_TWITTER_API_BASE.to_string());
    let root = root.trim_end_matches('/').to_string();
    let url = format!("{}/2/tweets", root);
    let chunk_max = publisher_cfg
        .twitter_text_chunk_max
        .unwrap_or(TWITTER_TEXT_CHUNK_MAX)
        .max(1);
    let truncation_suffix = publisher_cfg
        .twitter_truncation_suffix
        .as_deref()
        .unwrap_or("...");

    let primary_text = config.short_text.clone().unwrap_or_else(|| {
        truncate_chars(&item.content_markdown, chunk_max, truncation_suffix)
    });

    let mut texts = if config.thread {
        let full = config
            .short_text
            .clone()
            .unwrap_or_else(|| item.content_markdown.clone());
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
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let suffix_len = suffix.chars().count();
    let take = max_chars.saturating_sub(suffix_len);
    format!("{}{}", s.chars().take(take).collect::<String>(), suffix)
}

fn chunk_chars(s: &str, max_chars: usize) -> Vec<String> {
    if max_chars == 0 {
        return vec![s.to_string()];
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let end = (i + max_chars).min(chars.len());
        out.push(chars[i..end].iter().collect());
        i = end;
    }
    out
}
