use crate::types::{LinkedInConfig, UnifiedNewsItem};
use crate::PublisherConfig;
use anyhow::{Result, anyhow};

pub const API_BASE: &str = "https://api.linkedin.com";
pub const API_VERSION: &str = "202504";
pub const POST_PATH: &str = "/rest/posts";
pub const CANARY_PATH: &str = "/v2/userinfo";

pub async fn post(
    publisher_cfg: &PublisherConfig,
    item: &UnifiedNewsItem,
    cfg: &LinkedInConfig,
    dry_run: bool,
) -> Result<String> {
    if dry_run {
        return Ok(format!("dry-run-linkedin-{}", item.id));
    }

    let token = publisher_cfg
        .linkedin_access_token
        .as_deref()
        .ok_or_else(|| anyhow!("LinkedIn config present but missing access token"))?;

    let text = cfg.text.clone().unwrap_or_else(|| item.content_markdown.clone());

    let visibility = cfg.visibility.clone().unwrap_or_else(|| "PUBLIC".to_string());
    
    let payload = serde_json::json!({
        "author": cfg.author_urn,
        "commentary": text,
        "visibility": visibility,
        "distribution": {
            "feedDistribution": "MAIN_FEED",
            "targetEntities": [],
            "thirdPartyDistributionChannels": []
        },
        "lifecycleState": "PUBLISHED",
        "isReshareDisabledByAuthor": false
    });

    let client = reqwest::Client::new();
    let base = publisher_cfg
        .linkedin_api_base
        .as_deref()
        .unwrap_or("https://api.linkedin.com")
        .trim_end_matches('/');
    let url = format!("{}/rest/posts", base);
    let res = client
        .post(&url)
        .bearer_auth(token)
        .header("Linkedin-Version", API_VERSION)
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(anyhow!("LinkedIn API failed: {}", err_text));
    }

    let created_urn = res.headers()
        .get("x-restli-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or(&item.id)
        .to_string();

    Ok(created_urn)
}
