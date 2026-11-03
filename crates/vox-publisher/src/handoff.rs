//! Strongly-typed `SyndicationHandoffPayloadV1` contract (`SOCIAL-04`).
//!
//! Public `vox-publisher` prepares scholarly artifacts and syndication plans locally,
//! then exports a digest-bound `SyndicationHandoffPayloadV1` JSON payload for server-side
//! OAuth/webhook execution in `vox-ops-syndication`.

use crate::types::{SocialChannel, UnifiedNewsItem};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use sha3::Sha3_256;
use vox_scientia::review::ApprovalToken;

/// Canonical JSON Schema for [`SyndicationHandoffPayloadV1`].
pub const SYNDICATION_HANDOFF_SCHEMA_JSON: &str =
    include_str!("../../../contracts/scientia/syndication-handoff.v1.schema.json");

/// Server-bound handoff payload emitted by `vox scientia syndication-export` and consumed by
/// `vox-ops-syndication` (`POST /v1/syndication/handoff`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyndicationHandoffPayloadV1 {
    pub schema_version: u32,
    pub idempotency_key: String,
    pub item_id: String,
    pub content_sha3_256: String,
    pub approval_token_digest: Option<String>,
    pub channels: Vec<String>,
    pub item: UnifiedNewsItem,
    pub generated_at_ms: i64,
}

/// Compute the lowercase hex `SHA3-256` digest of a news item's markdown body.
#[must_use]
pub fn compute_content_sha3_256(content_markdown: &str) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(content_markdown.as_bytes());
    hex::encode(hasher.finalize())
}

/// Collect the active server-bound social channels configured on a [`UnifiedNewsItem`].
#[must_use]
pub fn active_handoff_channels(item: &UnifiedNewsItem) -> Vec<String> {
    let mut channels = Vec::new();
    if item.syndication.is_active(SocialChannel::Twitter) {
        channels.push("twitter".to_string());
    }
    if item.syndication.reddit.is_some() {
        channels.push("reddit".to_string());
    }
    if item.syndication.youtube.is_some() {
        channels.push("youtube".to_string());
    }
    if item.syndication.is_active(SocialChannel::Discord) {
        channels.push("discord".to_string());
    }
    if item.syndication.is_active(SocialChannel::Bluesky) {
        channels.push("bluesky".to_string());
    }
    if item.syndication.is_active(SocialChannel::Mastodon) {
        channels.push("mastodon".to_string());
    }
    if item.syndication.linkedin {
        channels.push("linkedin".to_string());
    }
    if item.syndication.hacker_news {
        channels.push("hacker_news".to_string());
    }
    if item.syndication.open_collective.is_some() {
        channels.push("opencollective".to_string());
    }
    channels
}

/// Build and schema-validate a [`SyndicationHandoffPayloadV1`] from a [`UnifiedNewsItem`] and optional [`ApprovalToken`].
pub fn build_syndication_handoff_payload(
    item: &UnifiedNewsItem,
    approval_token: Option<&ApprovalToken>,
) -> Result<SyndicationHandoffPayloadV1> {
    item.validate()?;
    let content_sha3_256 = compute_content_sha3_256(&item.content_markdown);
    let approval_token_digest = if let Some(tok) = approval_token {
        if tok.bound_digest() != content_sha3_256 {
            bail!(
                "ApprovalToken bound_digest ({}) does not match item content_sha3_256 ({content_sha3_256})",
                tok.bound_digest()
            );
        }
        Some(tok.bound_digest().to_string())
    } else {
        None
    };

    let channels = active_handoff_channels(item);
    let mut idemp_hasher = Sha256::new();
    idemp_hasher.update(item.id.as_bytes());
    idemp_hasher.update(b":");
    idemp_hasher.update(content_sha3_256.as_bytes());
    idemp_hasher.update(b":");
    idemp_hasher.update(channels.join(",").as_bytes());
    let idempotency_key = hex::encode(idemp_hasher.finalize());

    let payload = SyndicationHandoffPayloadV1 {
        schema_version: 1,
        idempotency_key,
        item_id: item.id.clone(),
        content_sha3_256,
        approval_token_digest,
        channels,
        item: item.clone(),
        generated_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    validate_handoff_payload_schema(&payload)?;
    Ok(payload)
}

/// Validate a [`SyndicationHandoffPayloadV1`] against `contracts/scientia/syndication-handoff.v1.schema.json`.
pub fn validate_handoff_payload_schema(payload: &SyndicationHandoffPayloadV1) -> Result<()> {
    let instance_val = serde_json::to_value(payload)?;
    let validator = vox_jsonschema_util::compile_validator_from_utf8(
        SYNDICATION_HANDOFF_SCHEMA_JSON,
        std::path::Path::new("contracts/scientia/syndication-handoff.v1.schema.json"),
    )?;
    vox_jsonschema_util::validate(&instance_val, &validator, "SyndicationHandoffPayloadV1")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OpenCollectiveConfig, SyndicationConfig};
    use chrono::Utc;

    #[test]
    fn builds_and_validates_syndication_handoff_payload_against_schema() {
        let item = UnifiedNewsItem {
            id: "vox-release-1-7".to_string(),
            title: "Vox v1.7 Sovereign Operations".to_string(),
            author: "Vox Team".to_string(),
            published_at: Utc::now(),
            tags: vec!["release".to_string()],
            content_markdown:
                "Vox v1.7 introduces fenced PostgreSQL telemetry and zero-cheating leaderboards."
                    .to_string(),
            syndication: SyndicationConfig {
                social: vec![SocialChannel::Twitter, SocialChannel::Bluesky],
                open_collective: Some(OpenCollectiveConfig {
                    is_private: false,
                    scheduled_publish_at: None,
                }),
                rss: true,
                dry_run: true,
                ..Default::default()
            },
            topic_pack: None,
        };

        let handoff = build_syndication_handoff_payload(&item, None).expect("valid handoff");
        assert_eq!(handoff.schema_version, 1);
        assert_eq!(
            handoff.channels,
            vec![
                "twitter".to_string(),
                "bluesky".to_string(),
                "opencollective".to_string()
            ]
        );

        // Verify #[serde(deny_unknown_fields)] rejects unknown fields
        let mut raw: serde_json::Value = serde_json::to_value(&handoff).unwrap();
        raw.as_object_mut()
            .unwrap()
            .insert("rogue_field".to_string(), serde_json::json!(true));
        assert!(serde_json::from_value::<SyndicationHandoffPayloadV1>(raw).is_err());
    }
}
