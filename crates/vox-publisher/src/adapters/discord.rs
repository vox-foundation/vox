use crate::types::{DiscordConfig, UnifiedNewsItem};
use crate::PublisherConfig;
use anyhow::{Result, anyhow};

pub async fn post(
    _publisher_cfg: &PublisherConfig,
    _item: &UnifiedNewsItem,
    _cfg: &DiscordConfig,
    _dry_run: bool,
) -> Result<String> {
    Err(anyhow!("Discord adapter not implemented"))
}
