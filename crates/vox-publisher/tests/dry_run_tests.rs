use chrono::Utc;
use vox_publisher::types::{ForgeConfig, ForgePostType, OpenCollectiveConfig, UnifiedNewsItem};
use vox_publisher::{Publisher, PublisherConfig};

#[tokio::test]
async fn test_dry_run_zero_web_leakage() {
    let test_item = UnifiedNewsItem {
        id: "test-offline-123".to_string(),
        title: "Test Offline".to_string(),
        author: "Vox Test".to_string(),
        published_at: Utc::now(),
        tags: vec![],
        content_markdown: "Offline content".to_string(),
        syndication: vox_publisher::types::SyndicationConfig {
            social: vec![vox_publisher::types::SocialChannel::Twitter],
            forge: Some(ForgeConfig {
                repo: "vox/fake".to_string(),
                post_type: ForgePostType::Release,
                release_tag: Some("test-offline-123".to_string()),
                draft: true,
                discussion_category: None,
            }),
            open_collective: Some(OpenCollectiveConfig {
                is_private: false,

                scheduled_publish_at: None,
            }),
            crates_io: None,
            rss: true,
            dry_run: true,
            ..Default::default()
        },
        topic_pack: None,
    };

    let publisher = Publisher::new(PublisherConfig {
        twitter_bearer_token: Some("secret1".to_string()),
        forge_token: Some("secret2".to_string()),
        open_collective_token: Some("secret3".to_string()),
        dry_run: false,
        ..Default::default()
    });

    let out = publisher.publish_all(&test_item).await.unwrap();

    assert_eq!(out.twitter_id(), Some("dry-run-tweet-test-offline-123"));
    assert_eq!(out.github_id(), Some("dry-run-github-test-offline-123"));
    assert_eq!(out.oc_id(), Some("dry-run-oc-test-offline-123"));
}
