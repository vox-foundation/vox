//! Shared switching helpers used by CLI/MCP/orchestrator publication surfaces.

use chrono::Utc;
use serde::Deserialize;

use crate::types::{SyndicationConfig, UnifiedNewsItem};
use crate::{ChannelOutcome, SyndicationResult};

#[derive(Deserialize, Default)]
struct MetaEnvelope {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    syndication: Option<SyndicationConfig>,
    #[serde(default)]
    topic_pack: Option<String>,
}

/// Build a runtime item from manifest row fields and optional `metadata_json`.
pub fn unified_news_item_from_manifest_parts(
    publication_id: &str,
    title: &str,
    author: &str,
    body_markdown: &str,
    metadata_json: Option<&str>,
) -> anyhow::Result<UnifiedNewsItem> {
    let meta: MetaEnvelope = metadata_json
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| anyhow::anyhow!("parse metadata_json for route simulation/publish: {e}"))?
        .unwrap_or_default();
    let topic_pack = meta
        .topic_pack
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);
    let mut item = UnifiedNewsItem {
        id: publication_id.to_string(),
        title: title.to_string(),
        author: author.to_string(),
        published_at: Utc::now(),
        tags: meta.tags,
        content_markdown: body_markdown.to_string(),
        syndication: meta.syndication.unwrap_or_default(),
        topic_pack,
    };
    item.hydrate_topic_pack_if_set()?;
    if item.syndication.distribution_policy.dry_run == Some(true) {
        item.syndication.dry_run = true;
    }
    Ok(item)
}

/// Parse and normalize channel CSV from CLI flags.
#[must_use]
pub fn parse_channels_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(normalize_channel_name)
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

/// Normalize channel names from API payloads/tool params.
#[must_use]
pub fn normalize_channels(raw: &[String]) -> Vec<String> {
    raw.iter()
        .map(|s| normalize_channel_name(s))
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

#[must_use]
pub fn normalize_channel_name(raw: &str) -> String {
    raw.trim().to_lowercase()
}

/// Apply operator allowlist for selective publication.
pub fn apply_channel_allowlist(item: &mut UnifiedNewsItem, allowed: &[String]) {
    let has = |name: &str| allowed.iter().any(|x| x == name);
    if !has("rss") {
        item.syndication.rss = false;
    }
    if !has("twitter") {
        item.syndication.twitter = None;
    }
    if !has("github") {
        item.syndication.github = None;
    }
    if !has("open_collective") {
        item.syndication.open_collective = None;
    }
    if !has("reddit") {
        item.syndication.reddit = None;
    }
    if !has("hacker_news") {
        item.syndication.hacker_news = None;
    }
    if !has("youtube") {
        item.syndication.youtube = None;
    }
    if !has("crates_io") {
        item.syndication.crates_io = None;
    }
}

/// Return channel ids that failed in a publication result.
#[must_use]
pub fn failed_channels(result: &SyndicationResult) -> Vec<String> {
    let mut out = Vec::new();
    let mut maybe = |name: &str, channel_outcome: &ChannelOutcome| {
        if matches!(channel_outcome, ChannelOutcome::Failed { .. }) {
            out.push(name.to_string());
        }
    };
    maybe("rss", &result.rss);
    maybe("twitter", &result.twitter);
    maybe("github", &result.github);
    maybe("open_collective", &result.open_collective);
    maybe("reddit", &result.reddit);
    maybe("hacker_news", &result.hacker_news);
    maybe("youtube", &result.youtube);
    maybe("crates_io", &result.crates_io);
    out
}

pub struct AttemptOutcome<'a> {
    pub content_sha3_256: &'a str,
    pub outcome_json: &'a str,
}

/// Parse the latest attempt matching `digest` and return its failed channel list.
///
/// `attempts` must be ordered **newest-first** (as returned by [`vox_db`] publication attempt queries).
/// Parsing failures are surfaced as errors (no silent skip).
pub fn failed_channels_from_latest_digest_attempt(
    attempts: &[AttemptOutcome<'_>],
    digest: &str,
) -> anyhow::Result<Option<Vec<String>>> {
    let Some(latest) = attempts.iter().find(|a| a.content_sha3_256 == digest) else {
        return Ok(None);
    };
    let parsed: SyndicationResult = serde_json::from_str(latest.outcome_json).map_err(|e| {
        anyhow::anyhow!("malformed syndication outcome_json for digest {digest}: {e}")
    })?;
    Ok(Some(failed_channels(&parsed)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_channels_csv_normalizes() {
        let out = parse_channels_csv(" twitter, reddit ,YOUTUBE ");
        assert_eq!(
            out,
            vec![
                "twitter".to_string(),
                "reddit".to_string(),
                "youtube".to_string()
            ]
        );
    }

    #[test]
    fn distribution_policy_dry_run_true_forces_syndication_dry_run() {
        let meta = r#"{
            "syndication": {
                "dry_run": false,
                "rss": false,
                "distribution_policy": { "dry_run": true }
            }
        }"#;
        let item =
            unified_news_item_from_manifest_parts("p", "t", "a", "b", Some(meta)).expect("item");
        assert!(item.syndication.dry_run);
    }

    #[test]
    fn latest_digest_attempt_parse_errors_are_not_swallowed() {
        let ok_json = serde_json::to_string(&SyndicationResult::default()).expect("json");
        let attempts = [
            AttemptOutcome {
                content_sha3_256: "d1",
                outcome_json: "{not-json",
            },
            AttemptOutcome {
                content_sha3_256: "d1",
                outcome_json: ok_json.as_str(),
            },
        ];
        let err = failed_channels_from_latest_digest_attempt(&attempts, "d1").unwrap_err();
        assert!(
            err.to_string()
                .contains("malformed syndication outcome_json"),
            "{err}"
        );
    }
}
