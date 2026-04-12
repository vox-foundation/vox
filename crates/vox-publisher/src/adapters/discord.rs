use crate::types::{DiscordConfig, UnifiedNewsItem};
use crate::PublisherConfig;
use anyhow::{Result, anyhow};

pub async fn post(
    _publisher_cfg: &PublisherConfig,
    item: &UnifiedNewsItem,
    cfg: &DiscordConfig,
    dry_run: bool,
) -> Result<String> {
    let webhook_url = cfg.webhook_url_override.clone().or_else(|| {
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxSocialDiscordWebhook)
            .expose()
            .map(|s| s.to_string())
    }).ok_or_else(|| anyhow!("Missing Discord webhook URL"))?;

    let message_content = cfg.message.clone().unwrap_or_else(|| item.title.clone());

    let mut payload = serde_json::json!({
        "content": message_content,
        "tts": cfg.tts,
    });

    if cfg.embed_title.is_some() || cfg.embed_description.is_some() || cfg.embed_url.is_some() {
        let mut embed = serde_json::json!({});
        if let Some(t) = &cfg.embed_title { embed["title"] = serde_json::json!(t); }
        if let Some(d) = &cfg.embed_description { embed["description"] = serde_json::json!(d); }
        if let Some(u) = &cfg.embed_url { embed["url"] = serde_json::json!(u); }
        if let Some(c) = cfg.embed_color { embed["color"] = serde_json::json!(c); }
        
        payload["embeds"] = serde_json::json!([embed]);
    }

    if dry_run {
        return Ok(format!("dry-run-discord-{}", item.id));
    }

    let client = reqwest::Client::new();
    let res = client.post(&webhook_url)
        .json(&payload)
        .send()
        .await?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(anyhow!("Discord API error: {} - {}", status, text));
    }

    Ok(format!("discord-{}", item.id))
}
