//! Contract-backed **topic packs** (`contracts/scientia/distribution.topic-packs.yaml`) merged into
//! [`crate::types::SyndicationConfig`] when `metadata_json` contains `"topic_pack": "<id>"`.

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;
use tracing::warn;

use crate::types::SyndicationConfig;

static EMBEDDED_TOPIC_PACKS_YAML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../contracts/scientia/distribution.topic-packs.yaml"
));

#[derive(Debug, Deserialize)]
pub struct TopicPacksFile {
    pub version: u32,
    #[serde(default)]
    pub packs: BTreeMap<String, DistributionTopicPack>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DistributionTopicPack {
    pub description: String,
    #[serde(default)]
    pub channels: Vec<String>,
    #[serde(default)]
    pub template_profile: BTreeMap<String, String>,
    #[serde(default)]
    pub min_worthiness_score: BTreeMap<String, f64>,
}

pub fn load_topic_packs_embedded() -> anyhow::Result<TopicPacksFile> {
    load_topic_packs_from_str(EMBEDDED_TOPIC_PACKS_YAML)
}

pub fn load_topic_packs_from_str(yaml: &str) -> anyhow::Result<TopicPacksFile> {
    Ok(serde_yaml::from_str(yaml)?)
}

pub fn merge_topic_pack_into_syndication(
    syn: &mut SyndicationConfig,
    pack: &DistributionTopicPack,
) {
    if !pack.channels.is_empty() {
        let allow: HashSet<String> = pack
            .channels
            .iter()
            .map(|c| c.trim().to_lowercase())
            .filter(|c| !c.is_empty())
            .collect();
        if !allow.contains("rss") {
            syn.rss = false;
        }
        if !allow.contains("twitter") {
            syn.twitter = None;
        }
        if !allow.contains("github") {
            syn.forge = None;
        }
        if !allow.contains("open_collective") {
            syn.open_collective = None;
        }
        if !allow.contains("reddit") {
            syn.reddit = None;
        }
        if !allow.contains("hacker_news") {
            syn.hacker_news = None;
        }
        if !allow.contains("youtube") {
            syn.youtube = None;
        }
        if !allow.contains("crates_io") {
            syn.crates_io = None;
        }
        if !allow.contains("discord") {
            syn.discord = None;
        }
        if !allow.contains("bluesky") {
            syn.bluesky = None;
        }
        if !allow.contains("linkedin") {
            syn.linkedin = None;
        }
        if !allow.contains("mastodon") {
            syn.mastodon = None;
        }
    }
    for (ch, score) in &pack.min_worthiness_score {
        let key = ch.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        let entry = syn
            .distribution_policy
            .channel_policy
            .entry(key)
            .or_default();
        entry.worthiness_floor = Some(match entry.worthiness_floor {
            Some(existing) => existing.max(*score),
            None => *score,
        });
    }
    for (ch, profile) in &pack.template_profile {
        let key = ch.trim().to_lowercase();
        if key.is_empty() {
            continue;
        }
        let entry = syn
            .distribution_policy
            .channel_policy
            .entry(key)
            .or_default();
        let replace = entry
            .template_profile
            .as_deref()
            .map(str::trim)
            .map(str::is_empty)
            .unwrap_or(true);
        if replace {
            entry.template_profile = Some(profile.clone());
        }
    }
}

/// Load the embedded topic-pack contract and merge the named pack into `syn`.
///
/// Unknown ids log a warning and leave `syn` unchanged (non-blocking).
pub fn hydrate_syndication_from_pack_id(
    syn: &mut SyndicationConfig,
    pack_id: &str,
) -> anyhow::Result<()> {
    let pack_id = pack_id.trim();
    if pack_id.is_empty() {
        return Ok(());
    }
    let doc = load_topic_packs_embedded()?;
    let Some(pack) = doc.packs.get(pack_id) else {
        warn!(
            target: "vox.publisher.topic_packs",
            pack_id,
            "unknown topic_pack; skipping policy merge"
        );
        return Ok(());
    };
    merge_topic_pack_into_syndication(syn, pack);
    Ok(())
}

/// Reads optional `topic_pack` from publication `metadata_json` and merges pack policy into `syn`.
///
/// Unknown pack ids log a warning and leave `syn` unchanged (non-blocking).
pub fn apply_topic_pack_from_metadata_json(
    syn: &mut SyndicationConfig,
    metadata_json: Option<&str>,
) -> anyhow::Result<()> {
    let Some(raw) = metadata_json.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|e| anyhow::anyhow!("metadata_json: {e}"))?;
    let Some(pack_name) = v
        .get("topic_pack")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return Ok(());
    };
    hydrate_syndication_from_pack_id(syn, pack_name)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ChannelPolicyConfig, SyndicationConfig};

    #[test]
    fn embedded_contract_loads_research_breakthrough() {
        let doc = load_topic_packs_embedded().expect("embedded yaml");
        assert_eq!(doc.version, 1);
        let p = doc
            .packs
            .get("research_breakthrough")
            .expect("research_breakthrough pack");
        assert!(!p.description.is_empty());
        assert!(
            p.min_worthiness_score
                .get("github")
                .is_some_and(|s| *s > 0.8_f64)
        );
    }

    #[test]
    fn channel_allowlist_removes_unlisted_targets() {
        let doc = load_topic_packs_embedded().expect("embedded yaml");
        let pack = doc.packs.get("research_breakthrough").expect("pack");
        let mut syn = SyndicationConfig {
            twitter: Some(crate::types::TwitterConfig {
                short_text: None,
                thread: false,
            }),
            rss: true,
            ..Default::default()
        };
        merge_topic_pack_into_syndication(&mut syn, pack);
        assert!(syn.twitter.is_none());
        assert!(syn.rss);
    }

    #[test]
    fn apply_from_metadata_json_uses_topic_pack_field() {
        let mut syn = SyndicationConfig::default();
        apply_topic_pack_from_metadata_json(
            &mut syn,
            Some(r#"{"topic_pack":"benchmark","tags":[]}"#),
        )
        .expect("apply");
        let yt = syn
            .distribution_policy
            .channel_policy
            .get("youtube")
            .expect("youtube floor from benchmark pack");
        assert!((yt.worthiness_floor.unwrap() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn merge_applies_worthiness_floor_and_respects_existing_higher_floor() {
        let doc = load_topic_packs_embedded().expect("embedded yaml");
        let pack = doc.packs.get("benchmark").expect("benchmark");
        let mut syn = SyndicationConfig::default();
        syn.distribution_policy.channel_policy.insert(
            "twitter".to_string(),
            ChannelPolicyConfig {
                worthiness_floor: Some(0.9),
                ..Default::default()
            },
        );
        merge_topic_pack_into_syndication(&mut syn, pack);
        let tw = syn
            .distribution_policy
            .channel_policy
            .get("twitter")
            .expect("twitter policy");
        assert!(
            tw.worthiness_floor.unwrap() >= 0.9,
            "should keep higher manifest floor"
        );
        let yt = syn
            .distribution_policy
            .channel_policy
            .get("youtube")
            .expect("youtube from pack");
        assert!((yt.worthiness_floor.unwrap() - 0.8).abs() < f64::EPSILON);
    }
}
