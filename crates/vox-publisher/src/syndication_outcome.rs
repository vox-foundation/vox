//! Per-channel syndication status for [`crate::Publisher::publish_all`].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::types::UnifiedNewsItem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    AuthExpired,
    AuthInvalid,
    RateLimited,
    PayloadRejected,
    ServiceDown,
    ContractMismatch,
    /// Platforms with no API support (e.g. ResearchGate) that require user login/upload.
    ManualActionRequired,
    Unknown,
}

impl FailureClass {
    pub fn from_http_status(status: reqwest::StatusCode) -> Self {
        match status.as_u16() {
            401 => Self::AuthExpired,
            403 => Self::AuthInvalid,
            429 => Self::RateLimited,
            400 | 422 => Self::PayloadRejected,
            500..=599 => Self::ServiceDown,
            _ => Self::Unknown,
        }
    }
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        failure_class: Option<FailureClass>,
    },
}

impl ChannelOutcome {
    pub fn fail_from_error(code: &str, err: anyhow::Error) -> Self {
        Self::Failed {
            code: code.to_string(),
            message: err.to_string(),
            retryable: true,
            failure_class: None,
        }
    }

    pub fn fail_from_res_text(code: &str, status: reqwest::StatusCode, text: String) -> Self {
        Self::Failed {
            code: code.to_string(),
            message: text,
            retryable: status.as_u16() >= 500 || status.as_u16() == 429,
            failure_class: Some(FailureClass::from_http_status(status)),
        }
    }
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
    pub bluesky: ChannelOutcome,
    #[serde(default)]
    pub mastodon: ChannelOutcome,
    #[serde(default)]
    pub linkedin: ChannelOutcome,
    #[serde(default)]
    pub discord: ChannelOutcome,
    #[serde(default)]
    pub researchgate: ChannelOutcome,
    /// True when a Zenodo DOI was minted, which triggers ResearchGate to ingest
    /// the record automatically within 3–14 days via DOI/CrossRef feeds.
    /// This is NOT a channel outcome — ResearchGate has no public API.
    /// Author must manually confirm authorship at researchgate.net after DOI appears.
    #[serde(default)]
    pub researchgate_doi_queued: bool,
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
            &self.bluesky,
            &self.mastodon,
            &self.linkedin,
            &self.discord,
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
        let twitter_ok = !item.syndication.social.contains(&crate::types::SocialChannel::Twitter) || ok(&self.twitter);
        let github_ok = item.syndication.forge.is_none() || ok(&self.github);
        let oc_ok = item.syndication.open_collective.is_none() || ok(&self.open_collective);
        let reddit_ok = item.syndication.reddit.is_none() || ok(&self.reddit);
        let hn_ok = !item.syndication.hacker_news || ok(&self.hacker_news);
        let yt_ok = item.syndication.youtube.is_none() || ok(&self.youtube);
        let crates_ok = item.syndication.crates_io.is_none() || ok(&self.crates_io);
        let bsky_ok = !item.syndication.social.contains(&crate::types::SocialChannel::Bluesky) || ok(&self.bluesky);
        let masto_ok = !item.syndication.social.contains(&crate::types::SocialChannel::Mastodon) || ok(&self.mastodon);
        let linkedin_ok = !item.syndication.linkedin || ok(&self.linkedin);
        let discord_ok = !item.syndication.social.contains(&crate::types::SocialChannel::Discord) || ok(&self.discord);
        let rg_ok = !item.syndication.researchgate || ok(&self.researchgate);
        rss_ok
            && twitter_ok
            && github_ok
            && oc_ok
            && reddit_ok
            && hn_ok
            && yt_ok
            && crates_ok
            && bsky_ok
            && masto_ok
            && linkedin_ok
            && discord_ok
            && rg_ok
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

    #[must_use]
    pub fn reddit_id(&self) -> Option<&str> {
        match &self.reddit {
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
    pub fn discord_id(&self) -> Option<&str> {
        match &self.discord {
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
    pub fn bluesky_id(&self) -> Option<&str> {
        match &self.bluesky {
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
    pub fn mastodon_id(&self) -> Option<&str> {
        match &self.mastodon {
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
    pub fn linkedin_id(&self) -> Option<&str> {
        match &self.linkedin {
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
