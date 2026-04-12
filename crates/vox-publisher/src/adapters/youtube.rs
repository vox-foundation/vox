use crate::types::{YouTubeConfig, YouTubePrivacyStatus};
use anyhow::{Context, Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use std::path::{Path, PathBuf};

fn resolve_video_path(video_asset_ref: &str, repo_root: Option<&Path>) -> PathBuf {
    let p = PathBuf::from(video_asset_ref);
    if p.is_absolute() {
        p
    } else if let Some(root) = repo_root {
        root.join(p)
    } else {
        p
    }
}

pub struct YouTubeAuthConfig<'a> {
    pub client_id: &'a str,
    pub client_secret: &'a str,
    pub refresh_token: &'a str,
}

/// Conservative cap per uploaded file (64 GiB — below YouTube API extremes; catches mistakes early).
pub const YOUTUBE_ASSET_MAX_BYTES: u64 = 64 * 1024 * 1024 * 1024;

/// Size, non-empty, extension heuristics before OAuth + upload I/O.
pub fn precheck_video_upload(path: &Path) -> Result<(), anyhow::Error> {
    if !path.is_file() {
        return Err(anyhow!("youtube video payload missing: {}", path.display()));
    }
    let meta = std::fs::metadata(path)
        .with_context(|| format!("youtube video metadata {}", path.display()))?;
    let len = meta.len();
    if len == 0 {
        return Err(anyhow!("youtube payload is empty: {}", path.display()));
    }
    if len > YOUTUBE_ASSET_MAX_BYTES {
        return Err(anyhow!(
            "youtube payload too large (max {} bytes): {}",
            YOUTUBE_ASSET_MAX_BYTES,
            path.display()
        ));
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "mp4" | "mov" | "webm" | "mkv" | "mpeg" | "mpg" | "m4v" | "avi" => {}
        "" => {
            return Err(anyhow!(
                "youtube payload should use a known video extension (.mp4, .mov, .webm, ...): {}",
                path.display()
            ));
        }
        other => {
            return Err(anyhow!(
                "youtube payload extension {:?} is unusual for video upload (expected mp4/mov/webm/mkv/...): {}",
                other,
                path.display()
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct YouTubeTokenResponse {
    access_token: String,
}

async fn refresh_access_token(auth: &YouTubeAuthConfig<'_>) -> Result<String> {
    let client = Client::new();
    let body = [
        ("client_id", auth.client_id),
        ("client_secret", auth.client_secret),
        ("refresh_token", auth.refresh_token),
        ("grant_type", "refresh_token"),
    ];
    let res = client
        .post("https://oauth2.googleapis.com/token")
        .form(&body)
        .send()
        .await
        .context("youtube oauth refresh request")?;
    if !res.status().is_success() {
        let txt = res.text().await.unwrap_or_default();
        return Err(anyhow!("youtube oauth refresh failed: {txt}"));
    }
    let tok: YouTubeTokenResponse = res.json().await.context("youtube oauth parse")?;
    if tok.access_token.trim().is_empty() {
        return Err(anyhow!("youtube oauth refresh returned empty access_token"));
    }
    Ok(tok.access_token)
}

fn privacy_status(cfg: &YouTubeConfig) -> &'static str {
    match cfg.privacy_status {
        YouTubePrivacyStatus::Private => "private",
        YouTubePrivacyStatus::Unlisted => "unlisted",
        YouTubePrivacyStatus::Public => "public",
    }
}

pub async fn upload_video(
    auth: &YouTubeAuthConfig<'_>,
    cfg: &YouTubeConfig,
    item: &crate::types::UnifiedNewsItem,
    repo_root: Option<&Path>,
) -> Result<String> {
    let path = resolve_video_path(&cfg.video_asset_ref, repo_root);
    precheck_video_upload(&path)?;
    let mime = mime_guess::from_path(&path)
        .first_raw()
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = tokio::fs::read(&path)
        .await
        .with_context(|| format!("read youtube payload {}", path.display()))?;
    let size = bytes.len();
    if size == 0 {
        return Err(anyhow!("youtube payload is empty: {}", path.display()));
    }

    let access_token = refresh_access_token(auth).await?;
    let title = cfg
        .title_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(item.title.as_str());
    let description = cfg
        .description_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(item.content_markdown.as_str());
    let body = serde_json::json!({
        "snippet": {
            "title": title,
            "description": description,
            "tags": cfg.tags,
            "categoryId": cfg
                .category_id
                .clone()
                .unwrap_or_else(|| crate::contract::YOUTUBE_DEFAULT_CATEGORY_ID.to_string())
        },
        "status": {
            "privacyStatus": privacy_status(cfg)
        }
    });

    let client = Client::new();
    let start_url = reqwest::Url::parse_with_params(
        "https://www.googleapis.com/upload/youtube/v3/videos",
        &[
            ("part", "snippet,status"),
            ("uploadType", "resumable"),
            (
                "notifySubscribers",
                if cfg.notify_subscribers {
                    "true"
                } else {
                    "false"
                },
            ),
        ],
    )?;
    let start = client
        .post(start_url)
        .bearer_auth(&access_token)
        .header("X-Upload-Content-Type", mime.as_str())
        .header("X-Upload-Content-Length", size.to_string())
        .json(&body)
        .send()
        .await
        .context("youtube resumable session init")?;
    if !start.status().is_success() {
        let txt = start.text().await.unwrap_or_default();
        return Err(anyhow!("youtube resumable init failed: {txt}"));
    }
    let location = start
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|h| h.to_str().ok())
        .map(std::string::ToString::to_string)
        .ok_or_else(|| anyhow!("youtube resumable init missing Location header"))?;

    let upload = client
        .put(location)
        .bearer_auth(&access_token)
        .header(reqwest::header::CONTENT_TYPE, mime)
        .header(reqwest::header::CONTENT_LENGTH, size.to_string())
        .body(bytes)
        .send()
        .await
        .context("youtube resumable upload")?;
    if !upload.status().is_success() {
        let txt = upload.text().await.unwrap_or_default();
        return Err(anyhow!("youtube upload failed: {txt}"));
    }
    let v: serde_json::Value = upload.json().await.context("youtube upload parse")?;
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| anyhow!("youtube upload response missing id"))?;
    Ok(format!("https://www.youtube.com/watch?v={id}"))
}
