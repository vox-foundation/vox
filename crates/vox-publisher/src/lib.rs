pub mod adapters;
pub mod contract;
pub mod gate;
pub mod publication;
pub mod publication_preflight;
pub mod scholarly;
pub mod scientific_metadata;
pub mod templates;
pub mod types;
pub mod zenodo_metadata;

use anyhow::Result;
use serde::{Deserialize, Serialize};
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
    pub twitter_text_chunk_max: Option<usize>,
    pub twitter_truncation_suffix: Option<String>,
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
            twitter_text_chunk_max: None,
            twitter_truncation_suffix: None,
        }
    }
}

pub struct Publisher {
    config: PublisherConfig,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyndicationResult {
    pub rss: ChannelOutcome,
    pub twitter: ChannelOutcome,
    pub github: ChannelOutcome,
    pub open_collective: ChannelOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
#[derive(Default)]
pub enum ChannelOutcome {
    #[default]
    Disabled,
    DryRun {
        external_id: Option<String>,
    },
    Success {
        external_id: Option<String>,
    },
    Failed {
        code: String,
        message: String,
        retryable: bool,
    },
}

impl SyndicationResult {
    #[must_use]
    pub fn has_failures(&self) -> bool {
        [
            &self.rss,
            &self.twitter,
            &self.github,
            &self.open_collective,
        ]
        .iter()
        .any(|o| matches!(o, ChannelOutcome::Failed { .. }))
    }

    #[must_use]
    pub fn all_enabled_channels_succeeded(&self, item: &UnifiedNewsItem) -> bool {
        fn ok(outcome: &ChannelOutcome) -> bool {
            matches!(
                outcome,
                ChannelOutcome::Success { .. }
                    | ChannelOutcome::Disabled
                    | ChannelOutcome::DryRun { .. }
            )
        }

        let rss_ok = !item.syndication.rss || ok(&self.rss);
        let twitter_ok = item.syndication.twitter.is_none() || ok(&self.twitter);
        let github_ok = item.syndication.github.is_none() || ok(&self.github);
        let oc_ok = item.syndication.open_collective.is_none() || ok(&self.open_collective);
        rss_ok && twitter_ok && github_ok && oc_ok
    }

    #[must_use]
    pub fn github_id(&self) -> Option<&str> {
        match &self.github {
            ChannelOutcome::Success {
                external_id: Some(v),
            }
            | ChannelOutcome::DryRun {
                external_id: Some(v),
            } => Some(v.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn twitter_id(&self) -> Option<&str> {
        match &self.twitter {
            ChannelOutcome::Success {
                external_id: Some(v),
            }
            | ChannelOutcome::DryRun {
                external_id: Some(v),
            } => Some(v.as_str()),
            _ => None,
        }
    }

    #[must_use]
    pub fn oc_id(&self) -> Option<&str> {
        match &self.open_collective {
            ChannelOutcome::Success {
                external_id: Some(v),
            }
            | ChannelOutcome::DryRun {
                external_id: Some(v),
            } => Some(v.as_str()),
            _ => None,
        }
    }
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
                result.rss = ChannelOutcome::DryRun { external_id: None };
            } else {
                match adapters::rss::update_feed(item, &self.config.site).await {
                    Ok(()) => {
                        result.rss = ChannelOutcome::Success { external_id: None };
                        info!("RSS feed updated.");
                    }
                    Err(e) => {
                        result.rss = ChannelOutcome::Failed {
                            code: "rss_update_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                        };
                    }
                }
            }
        }

        if let Some(twitter) = &item.syndication.twitter {
            if is_dry_run {
                info!("[DRY RUN] Would post to Twitter: {:?}", twitter);
                result.twitter = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-tweet-{}", item.id)),
                };
            } else if let Some(token) = &self.config.twitter_bearer_token {
                match adapters::twitter::post(&self.config, token, item, twitter).await {
                    Ok(id) => {
                        result.twitter = ChannelOutcome::Success {
                            external_id: Some(id),
                        };
                        info!("Posted to Twitter.");
                    }
                    Err(e) => {
                        result.twitter = ChannelOutcome::Failed {
                            code: "twitter_post_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                        };
                    }
                }
            } else {
                warn!("Twitter config present but no API token.");
                result.twitter = ChannelOutcome::Failed {
                    code: "missing_twitter_token".to_string(),
                    message: "Twitter config present but no API token.".to_string(),
                    retryable: false,
                };
            }
        }

        if let Some(github) = &item.syndication.github {
            if is_dry_run {
                info!(
                    "[DRY RUN] Would post to GitHub repository {} as {:?}",
                    github.repo, github.post_type
                );
                result.github = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-github-{}", item.id)),
                };
            } else if let Some(token) = &self.config.github_token {
                match adapters::github::post(&self.config, token, item, github).await {
                    Ok(id) => {
                        result.github = ChannelOutcome::Success {
                            external_id: Some(id),
                        };
                        info!("Posted to GitHub.");
                    }
                    Err(e) => {
                        result.github = ChannelOutcome::Failed {
                            code: "github_post_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                        };
                    }
                }
            } else {
                warn!("GitHub config present but no API token.");
                result.github = ChannelOutcome::Failed {
                    code: "missing_github_token".to_string(),
                    message: "GitHub config present but no API token.".to_string(),
                    retryable: false,
                };
            }
        }

        if let Some(oc) = &item.syndication.open_collective {
            if is_dry_run {
                info!(
                    "[DRY RUN] Would post to Open Collective slug {}",
                    oc.collective_slug
                );
                result.open_collective = ChannelOutcome::DryRun {
                    external_id: Some(format!("dry-run-oc-{}", item.id)),
                };
            } else if let Some(token) = &self.config.open_collective_token {
                match adapters::opencollective::post(&self.config, token, item, oc).await {
                    Ok(id) => {
                        result.open_collective = ChannelOutcome::Success {
                            external_id: Some(id),
                        };
                        info!("Posted to Open Collective.");
                    }
                    Err(e) => {
                        result.open_collective = ChannelOutcome::Failed {
                            code: "opencollective_post_failed".to_string(),
                            message: e.to_string(),
                            retryable: true,
                        };
                    }
                }
            } else {
                warn!("Open Collective config present but no API token.");
                result.open_collective = ChannelOutcome::Failed {
                    code: "missing_opencollective_token".to_string(),
                    message: "Open Collective config present but no API token.".to_string(),
                    retryable: false,
                };
            }
        }

        Ok(result)
    }
}
