use crate::types::{LinkedInConfig, UnifiedNewsItem};
use crate::PublisherConfig;
use anyhow::{Result, anyhow};

pub async fn post(
    _publisher_cfg: &PublisherConfig,
    _item: &UnifiedNewsItem,
    _cfg: &LinkedInConfig,
    _dry_run: bool,
) -> Result<String> {
    // TOESTUB: stub/unimplemented - LinkedIn adapter awaiting App approval
    Err(anyhow!("LinkedIn adapter not implemented"))
}
