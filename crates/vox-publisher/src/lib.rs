pub mod adapters;
pub mod contract;
pub mod templates;
pub mod types;

use anyhow::Result;
use tracing::{info, warn};
use types::UnifiedNewsItem;

pub use contract::NewsSiteConfig;

pub struct PublisherConfig {
    pub twitter_bearer_token: Option<String>,
    pub github_token: Option<String>,
    pub open_collective_token: Option<String>,
    pub dry_run: bool,
    pub site: NewsSiteConfig,
    pub twitter_api_base: Option<String>,
    pub github_rest_base: Option<String>,
    pub github_graphql_url: Option<String>,
    pub opencollective_graphql_url: Option<String>,
}

impl Default for PublisherConfig {
    fn default() -> Self {
        Self {
            twitter_bearer_token: None,
            github_token: None,
            open_collective_token: None,
            dry_run: true,
            site: NewsSiteConfig::default(),
            twitter_api_base: None,
            github_rest_base: None,
            github_graphql_url: None,
            opencollective_graphql_url: None,
        }
    }
}

pub struct Publisher {
    config: PublisherConfig,
}

#[derive(Debug, Default)]
pub struct SyndicationResult {
    pub twitter_id: Option<String>,
    pub github_id: Option<String>,
    pub oc_id: Option<String>,
}

impl Publisher {
    pub fn new(config: PublisherConfig) -> Self {
        Self { config }
    }

    pub async fn publish_all(&self, item: &UnifiedNewsItem) -> Result<SyndicationResult> {
        item.validate()?;
        info!("Starting syndication for news item: {}", item.id);
        let mut result = SyndicationResult::default();

        let is_dry_run = self.config.dry_run || item.syndication.dry_run;
        if is_dry_run {
            info!("DRY RUN MODE ENABLED. API requests will be constructed but not sent.");
        }

        if item.syndication.rss {
            if is_dry_run {
                info!("[DRY RUN] Would update RSS feed for {}", item.id);
            } else {
                adapters::rss::update_feed(item, &self.config.site).await?;
                info!("RSS feed updated.");
            }
        }

        if let Some(twitter) = &item.syndication.twitter {
            if is_dry_run {
                info!("[DRY RUN] Would post to Twitter: {:?}", twitter);
                result.twitter_id = Some(format!("dry-run-tweet-{}", item.id));
            } else if let Some(token) = &self.config.twitter_bearer_token {
                let id = adapters::twitter::post(&self.config, token, item, twitter).await?;
                result.twitter_id = Some(id);
                info!("Posted to Twitter.");
            } else {
                warn!("Twitter config present but no API token.");
            }
        }

        if let Some(github) = &item.syndication.github {
            if is_dry_run {
                info!(
                    "[DRY RUN] Would post to GitHub repository {} as {:?}",
                    github.repo, github.post_type
                );
                result.github_id = Some(format!("dry-run-github-{}", item.id));
            } else if let Some(token) = &self.config.github_token {
                let id = adapters::github::post(&self.config, token, item, github).await?;
                result.github_id = Some(id);
                info!("Posted to GitHub.");
            } else {
                warn!("GitHub config present but no API token.");
            }
        }

        if let Some(oc) = &item.syndication.open_collective {
            if is_dry_run {
                info!(
                    "[DRY RUN] Would post to Open Collective slug {}",
                    oc.collective_slug
                );
                result.oc_id = Some(format!("dry-run-oc-{}", item.id));
            } else if let Some(token) = &self.config.open_collective_token {
                let id = adapters::opencollective::post(&self.config, token, item, oc).await?;
                result.oc_id = Some(id);
                info!("Posted to Open Collective.");
            } else {
                warn!("Open Collective config present but no API token.");
            }
        }

        Ok(result)
    }
}
