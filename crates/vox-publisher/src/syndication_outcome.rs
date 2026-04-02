//! Per-channel syndication status for [`crate::Publisher::publish_all`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::types::UnifiedNewsItem;

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

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyndicationResult {
    pub rss: ChannelOutcome,
    pub twitter: ChannelOutcome,
    pub github: ChannelOutcome,
    pub open_collective: ChannelOutcome,
    pub reddit: ChannelOutcome,
    pub hacker_news: ChannelOutcome,
    pub youtube: ChannelOutcome,
    pub crates_io: ChannelOutcome,
    #[serde(default)]
    pub decision_reasons: BTreeMap<String, String>,
}

impl SyndicationResult {
    #[must_use]
    pub fn has_failures(&self) -> bool {
        [
            &self.rss,
            &self.twitter,
            &self.github,
            &self.open_collective,
            &self.reddit,
            &self.hacker_news,
            &self.youtube,
            &self.crates_io,
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
        let github_ok = item.syndication.forge.is_none() || ok(&self.github);
        let oc_ok = item.syndication.open_collective.is_none() || ok(&self.open_collective);
        let reddit_ok = item.syndication.reddit.is_none() || ok(&self.reddit);
        let hn_ok = item.syndication.hacker_news.is_none() || ok(&self.hacker_news);
        let yt_ok = item.syndication.youtube.is_none() || ok(&self.youtube);
        let crates_ok = item.syndication.crates_io.is_none() || ok(&self.crates_io);
        rss_ok && twitter_ok && github_ok && oc_ok && reddit_ok && hn_ok && yt_ok && crates_ok
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
