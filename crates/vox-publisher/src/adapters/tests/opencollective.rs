use wiremock::matchers::{method, header, body_json};
use wiremock::{Mock, MockServer, ResponseTemplate};
use crate::adapters::opencollective;
use crate::PublisherConfig;
use crate::types::{OpenCollectiveConfig, UnifiedNewsItem};
use serde_json::json;

#[tokio::test]
async fn test_opencollective_post_success() {
    let mock_server = MockServer::start().await;
    let token = "test-token";
    let item = UnifiedNewsItem {
        id: "test-item".to_string(),
        title: "Test Title".to_string(),
        content_markdown: "Test Content".to_string(),
        ..Default::default()
    };
    let config = OpenCollectiveConfig {
        collective_slug: "test-collective".to_string(),
        is_private: false,
        ..Default::default()
    };

    let publisher_cfg = PublisherConfig {
        open_collective_token: Some("test-token".to_string()),
        opencollective_graphql_url: Some(mock_server.uri()),
        ..Default::default()
    };

    Mock::given(method("POST"))
        .and(header("Personal-Token", "test-token"))
        .and(body_json(json!({
            "query": "\n      mutation CreateUpdate($update: UpdateCreateInput!) {\n        createUpdate(update: $update) {\n          id\n          slug\n          title\n        }\n      }\n    ",
            "variables": {
                "update": {
                    "title": "Test Title",
                    "html": "<p>Test Content</p>\n",
                    "isPrivate": false,
                    "makePublicOn": null,
                    "account": {
                        "slug": "test-collective"
                    }
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
                "createUpdate": {
                    "id": "upd-123",
                    "slug": "test-update",
                    "title": "Test Title"
                }
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result: anyhow::Result<String> = opencollective::post(&publisher_cfg, token, &item, &config, false).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "upd-123");
}

#[tokio::test]
async fn test_opencollective_post_dry_run() {
    let mock_server = MockServer::start().await;
    let token = "test-token";
    let item = UnifiedNewsItem {
        id: "test-item".to_string(),
        ..Default::default()
    };
    let config = OpenCollectiveConfig::default();
    let publisher_cfg = PublisherConfig {
        opencollective_graphql_url: Some(mock_server.uri()),
        ..Default::default()
    };

    let result: anyhow::Result<String> = opencollective::post(&publisher_cfg, token, &item, &config, true).await;
    assert!(result.is_ok());
    assert!(result.unwrap().contains("dry-run-opencollective"));
}
