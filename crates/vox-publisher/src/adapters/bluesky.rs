use crate::PublisherConfig;
use crate::types::{BlueskyConfig, UnifiedNewsItem};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct CreateSessionRequest<'a> {
    identifier: &'a str,
    password: &'a str,
}

#[derive(Deserialize)]
struct CreateSessionResponse {
    access_token: String,
    did: String,
}

#[derive(Serialize)]
struct PostRecord {
    #[serde(rename = "$type")]
    type_: &'static str,
    text: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Serialize)]
struct CreateRecordRequest<'a> {
    repo: &'a str,
    collection: &'static str,
    record: PostRecord,
}

pub async fn post(
    _publisher_cfg: &PublisherConfig,
    handle: &str,
    password: &str,
    item: &UnifiedNewsItem,
    config: &BlueskyConfig,
) -> Result<String> {
    let client = Client::new();
    
    // 1. Create Session
    let session_resp = client
        .post("https://bsky.social/xrpc/com.atproto.server.createSession")
        .json(&CreateSessionRequest {
            identifier: handle,
            password,
        })
        .send()
        .await?;

    if !session_resp.status().is_success() {
        let err_body = session_resp.text().await.unwrap_or_default();
        return Err(anyhow!("Bluesky session creation failed: {}", err_body));
    }

    let session: CreateSessionResponse = session_resp.json().await?;

    // 2. Create Post
    let text = config
        .text
        .clone()
        .unwrap_or_else(|| item.content_markdown.clone());
        
    if text.is_empty() {
        return Err(anyhow!("Bluesky post text cannot be empty"));
    }

    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let post_resp = client
        .post("https://bsky.social/xrpc/app.bsky.feed.post")
        .bearer_auth(&session.access_token)
        .json(&CreateRecordRequest {
            repo: &session.did,
            collection: "app.bsky.feed.post",
            record: PostRecord {
                type_: "app.bsky.feed.post",
                text,
                created_at,
            },
        })
        .send()
        .await?;

    if !post_resp.status().is_success() {
        let err_body = post_resp.text().await.unwrap_or_default();
        return Err(anyhow!("Bluesky post creation failed: {}", err_body));
    }

    Ok("bluesky_post_success".to_string())
}
