use crate::PublisherConfig;
use crate::types::{BlueskyConfig, UnifiedNewsItem};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use tokio::sync::Mutex;
use std::collections::HashMap;

pub const DEFAULT_PDS: &str = "https://bsky.social";
pub const SESSION_PATH: &str = "/xrpc/com.atproto.server.createSession";
pub const RECORD_PATH: &str = "/xrpc/com.atproto.repo.createRecord";
pub const DESCRIBE_PATH: &str = "/xrpc/com.atproto.server.describeServer";
pub const FEED_COLLECTION: &str = "app.bsky.feed.post";
pub const SESSION_TTL_SECS: u64 = 110 * 60;

#[derive(Serialize)]
struct CreateSessionRequest<'a> {
    identifier: &'a str,
    password: &'a str,
}

#[derive(Deserialize, Clone)]
struct CreateSessionResponse {
    #[serde(rename = "accessJwt")]
    access_jwt: String,
    #[serde(rename = "refreshJwt")]
    refresh_jwt: String,
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

struct SessionCacheEntry {
    session: CreateSessionResponse,
    expires_at: std::time::Instant,
}

static BLUESKY_SESSION_CACHE: OnceLock<Mutex<HashMap<String, SessionCacheEntry>>> = OnceLock::new();

fn session_cache() -> &'static Mutex<HashMap<String, SessionCacheEntry>> {
    BLUESKY_SESSION_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub async fn post(
    _publisher_cfg: &PublisherConfig,
    handle: &str,
    password: &str,
    pds_base: &str,
    item: &UnifiedNewsItem,
    config: &BlueskyConfig,
    dry_run: bool,
) -> Result<String> {
    if dry_run {
        return Ok(format!("dry-run-bluesky-{}", item.id));
    }
    let client = Client::new();
    
    let cache_key = format!("{}::{}", pds_base.trim_end_matches('/'), handle);
    let mut cache_guard = session_cache().lock().await;

    let mut maybe_session = None;
    if let Some(entry) = cache_guard.get(&cache_key) {
        if std::time::Instant::now() < entry.expires_at {
            maybe_session = Some(entry.session.clone());
        }
    }

    let session = if let Some(s) = maybe_session {
        s
    } else {
        let session_resp = client
            .post(format!("{}/xrpc/com.atproto.server.createSession", pds_base.trim_end_matches('/')))
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

        let parsed: CreateSessionResponse = session_resp.json().await?;
        
        let entry = SessionCacheEntry {
            session: parsed.clone(),
            // AT Protocol accessJwt usually lasts 2 hours, be conservative with 1 hr 50 mins
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(110 * 60),
        };
        cache_guard.insert(cache_key, entry);
        parsed
    };
    
    // We can confidently drop the lock now
    drop(cache_guard);

    let text = config
        .text
        .clone()
        .unwrap_or_else(|| item.content_markdown.clone());
        
    if text.is_empty() {
        return Err(anyhow!("Bluesky post text cannot be empty"));
    }

    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let post_resp = client
        .post(format!("{}/xrpc/com.atproto.repo.createRecord", pds_base.trim_end_matches('/')))
        .bearer_auth(&session.access_jwt)
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
