use chrono::Utc;
use std::collections::BTreeMap;
use vox_publisher::types::{
    ChannelPolicyConfig, DistributionPolicyConfig, SyndicationConfig, TopicFiltersConfig,
    TwitterConfig, UnifiedNewsItem,
};
use vox_publisher::{ChannelOutcome, Publisher, PublisherConfig};

fn base_item() -> UnifiedNewsItem {
    UnifiedNewsItem {
        id: "policy-item-1".to_string(),
        title: "Policy Item".to_string(),
        author: "Vox".to_string(),
        published_at: Utc::now(),
        tags: vec!["benchmark".to_string(), "release".to_string()],
        content_markdown: "A benchmark release with practical impact.".to_string(),
        syndication: SyndicationConfig {
            twitter: Some(TwitterConfig {
                short_text: None,
                thread: true,
            }),
            rss: true,
            dry_run: true,
            ..Default::default()
        },
        topic_pack: None,
    }
}

#[tokio::test]
async fn channel_policy_enabled_false_disables_channel_with_reason() {
    let mut item = base_item();
    let mut policy = BTreeMap::new();
    policy.insert(
        "twitter".to_string(),
        ChannelPolicyConfig {
            enabled: Some(false),
            ..Default::default()
        },
    );
    item.syndication.distribution_policy = DistributionPolicyConfig {
        channel_policy: policy,
        ..Default::default()
    };

    let out = Publisher::new(PublisherConfig::default())
        .publish_all(&item)
        .await
        .expect("publish");

    assert!(matches!(out.twitter, ChannelOutcome::Disabled));
    assert_eq!(
        out.decision_reasons.get("twitter").map(String::as_str),
        Some("policy_disabled")
    );
}

#[tokio::test]
async fn topic_filter_excludes_channel_when_tag_missing() {
    let mut item = base_item();
    let mut policy = BTreeMap::new();
    policy.insert(
        "twitter".to_string(),
        ChannelPolicyConfig {
            topic_filters: Some(TopicFiltersConfig {
                include_tags: vec!["video_demo".to_string()],
                exclude_tags: vec![],
                min_topic_score: Some(0.5),
            }),
            ..Default::default()
        },
    );
    item.syndication.distribution_policy = DistributionPolicyConfig {
        channel_policy: policy,
        ..Default::default()
    };

    let out = Publisher::new(PublisherConfig::default())
        .publish_all(&item)
        .await
        .expect("publish");

    assert!(matches!(out.twitter, ChannelOutcome::Disabled));
    assert_eq!(
        out.decision_reasons.get("twitter").map(String::as_str),
        Some("topic_filtered_out")
    );
}

#[tokio::test]
async fn worthiness_floor_blocks_channel_when_score_too_low() {
    let mut item = base_item();
    let mut policy = BTreeMap::new();
    policy.insert(
        "twitter".to_string(),
        ChannelPolicyConfig {
            worthiness_floor: Some(0.9),
            ..Default::default()
        },
    );
    item.syndication.distribution_policy = DistributionPolicyConfig {
        channel_policy: policy,
        ..Default::default()
    };
    let cfg = PublisherConfig {
        worthiness_score: Some(0.72),
        ..Default::default()
    };

    let out = Publisher::new(cfg)
        .publish_all(&item)
        .await
        .expect("publish");
    assert!(matches!(out.twitter, ChannelOutcome::Disabled));
    let reason = out
        .decision_reasons
        .get("twitter")
        .cloned()
        .unwrap_or_default();
    assert!(reason.starts_with("worthiness_below_floor"));
}
