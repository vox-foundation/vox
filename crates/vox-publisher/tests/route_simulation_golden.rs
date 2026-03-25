//! Golden snapshot: stable `SyndicationResult` for route simulation (hourglass — few integration tests).

use chrono::Utc;
use serde::Deserialize;
use vox_publisher::types::{SyndicationConfig, UnifiedNewsItem};
use vox_publisher::{Publisher, PublisherConfig};

#[derive(Deserialize)]
struct MetadataFixture {
    tags: Vec<String>,
    syndication: SyndicationConfig,
}

fn golden_item() -> UnifiedNewsItem {
    let meta: MetadataFixture =
        serde_json::from_str(include_str!("fixtures/golden_route_metadata.json"))
            .expect("golden_route_metadata.json");
    UnifiedNewsItem {
        id: "golden-route-001".to_string(),
        title: "Golden route item".to_string(),
        author: "Vox".to_string(),
        published_at: Utc::now(),
        tags: meta.tags,
        content_markdown: "Hello world for golden route.".to_string(),
        syndication: meta.syndication,
        topic_pack: None,
    }
}

#[tokio::test]
async fn route_simulation_matches_golden_snapshot() {
    let item = golden_item();
    let out = Publisher::new(PublisherConfig::default())
        .publish_all(&item)
        .await
        .expect("publish");
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/route_simulation_golden.json"))
            .expect("route_simulation_golden.json");
    let actual = serde_json::to_value(&out).expect("serialize result");
    assert_eq!(actual, expected);
}
