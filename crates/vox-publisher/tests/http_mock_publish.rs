//! Integration tests for local `Publisher::publish_all` and `SyndicationHandoffPayloadV1` handoff (`SOCIAL-03`, `SOCIAL-04`).

use chrono::Utc;
use vox_publisher::handoff::build_syndication_handoff_payload;
use vox_publisher::types::{OpenCollectiveConfig, SyndicationConfig, UnifiedNewsItem};
use vox_publisher::{ChannelOutcome, Publisher, PublisherConfig};

#[tokio::test]
async fn twitter_and_opencollective_queue_handoff_to_vox_ops_syndication() {
    let item = UnifiedNewsItem {
        id: "mock-item".to_string(),
        title: "T".to_string(),
        author: "A".to_string(),
        published_at: Utc::now(),
        tags: vec![],
        content_markdown: "body".to_string(),
        syndication: SyndicationConfig {
            social: vec![vox_publisher::types::SocialChannel::Twitter],
            forge: None,
            open_collective: Some(OpenCollectiveConfig {
                is_private: false,
                scheduled_publish_at: None,
            }),
            crates_io: None,
            rss: false,
            dry_run: false,
            ..Default::default()
        },
        topic_pack: None,
    };

    let publisher = Publisher::new(PublisherConfig {
        twitter_bearer_token: Some("bearer".into()),
        open_collective_token: Some("oc-key".into()),
        dry_run: false,
        ..Default::default()
    });

    let out = publisher.publish_all(&item).await.expect("publish");
    assert_eq!(out.twitter_id(), Some("queued:vox-ops-syndication"));
    assert_eq!(out.oc_id(), Some("queued:vox-ops-syndication"));

    let handoff = build_syndication_handoff_payload(&item, None).expect("build handoff");
    assert_eq!(handoff.item_id, "mock-item");
    assert_eq!(
        handoff.channels,
        vec!["twitter".to_string(), "opencollective".to_string()]
    );
}

#[tokio::test]
async fn partial_channel_failure_is_reported_without_short_circuiting() {
    let item = UnifiedNewsItem {
        id: "mock-item-failure".to_string(),
        title: "T".to_string(),
        author: "A".to_string(),
        published_at: Utc::now(),
        tags: vec![],
        content_markdown: "body".to_string(),
        syndication: SyndicationConfig {
            social: vec![vox_publisher::types::SocialChannel::Twitter],
            forge: None,
            open_collective: Some(OpenCollectiveConfig {
                is_private: false,
                scheduled_publish_at: None,
            }),
            crates_io: None,
            rss: false,
            dry_run: false,
            ..Default::default()
        },
        topic_pack: None,
    };

    // Provide twitter token so twitter queues, omit open_collective token so open_collective fails
    let publisher = Publisher::new(PublisherConfig {
        twitter_bearer_token: Some("bearer".into()),
        open_collective_token: None,
        dry_run: false,
        ..Default::default()
    });
    let out = publisher.publish_all(&item).await.expect("publish");
    assert_eq!(out.twitter_id(), Some("queued:vox-ops-syndication"));
    assert!(matches!(out.open_collective, ChannelOutcome::Failed { .. }));
    assert!(out.has_failures());
}
