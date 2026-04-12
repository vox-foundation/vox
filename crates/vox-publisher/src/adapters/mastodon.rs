use crate::types::{MastodonConfig, UnifiedNewsItem};
use crate::PublisherConfig;
use anyhow::{Result, anyhow};

pub async fn post(
    _publisher_cfg: &PublisherConfig,
    _item: &UnifiedNewsItem,
    _cfg: &MastodonConfig,
    _dry_run: bool,
) -> Result<String> {
    // TOESTUB: stub/unimplemented - Mastodon adapter awaiting provider policy
    Err(anyhow!("Mastodon adapter not implemented"))
}
