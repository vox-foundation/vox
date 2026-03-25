//! Integration-style tests: real HTTP to a local mock server (no production endpoints).

use std::time::Duration;

use axum::{routing::post, Json, Router};
use chrono::Utc;
use serde_json::json;
use tokio::net::TcpListener;
use vox_publisher::types::{
    OpenCollectiveConfig, SyndicationConfig, TwitterConfig, UnifiedNewsItem,
};
use vox_publisher::{Publisher, PublisherConfig};

#[tokio::test]
async fn twitter_and_opencollective_use_configured_bases_only() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = Router::new()
        .route(
            "/2/tweets",
            post(|Json(body): Json<serde_json::Value>| async move {
                assert!(body.get("text").is_some());
                Json(json!({ "data": { "id": "mocktweet1" } }))
            }),
        )
        .route(
            "/graphql/v2",
            post(|Json(body): Json<serde_json::Value>| async move {
                assert!(body.get("query").is_some());
                Json(json!({ "data": { "createUpdate": { "id": "mockoc1" } } }))
            }),
        );

    let _guard = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(80)).await;

    let base = format!("http://{}", addr);
    let item = UnifiedNewsItem {
        id: "mock-item".to_string(),
        title: "T".to_string(),
        author: "A".to_string(),
        published_at: Utc::now(),
        tags: vec![],
        content_markdown: "body".to_string(),
        syndication: SyndicationConfig {
            twitter: Some(TwitterConfig {
                short_text: Some("hello mock".to_string()),
                thread: false,
            }),
            github: None,
            open_collective: Some(OpenCollectiveConfig {
                is_private: false,
                collective_slug: "slug".to_string(),
            }),
            crates_io: None,
            rss: false,
            dry_run: false,
        },
    };

    let publisher = Publisher::new(PublisherConfig {
        twitter_bearer_token: Some("bearer".into()),
        open_collective_token: Some("oc-key".into()),
        dry_run: false,
        twitter_api_base: Some(base.clone()),
        opencollective_graphql_url: Some(format!("{}/graphql/v2", base)),
        ..Default::default()
    });

    let out = publisher.publish_all(&item).await.expect("publish");
    assert_eq!(out.twitter_id.as_deref(), Some("mocktweet1"));
    assert_eq!(out.oc_id.as_deref(), Some("mockoc1"));
}
