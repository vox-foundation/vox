use wiremock::matchers::{method, path, body_json, header};
use wiremock::{Mock, MockServer, ResponseTemplate};
use crate::adapters::bluesky;
use crate::PublisherConfig;
use crate::types::{BlueskyConfig, UnifiedNewsItem};
use serde_json::json;

#[tokio::test]
async fn test_bluesky_post_success() {
    let mock_server = MockServer::start().await;
    let handle = "test.bsky.social";
    let password = "test-password";
    let pds_base = mock_server.uri();
    let item = UnifiedNewsItem {
        id: "test-item".to_string(),
        content_markdown: "Test Content".to_string(),
        ..Default::default()
    };
    let config = BlueskyConfig::default();
    let publisher_cfg = PublisherConfig::default();

    // 1. Mock createSession
    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.server.createSession"))
        .and(body_json(json!({
            "identifier": handle,
            "password": password
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "accessJwt": "test-access-jwt",
            "refreshJwt": "test-refresh-jwt",
            "did": "did:plc:test"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    // 2. Mock createRecord
    Mock::given(method("POST"))
        .and(path("/xrpc/com.atproto.repo.createRecord"))
        .and(header("Authorization", "Bearer test-access-jwt"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "uri": "at://did:plc:test/app.bsky.feed.post/123",
            "cid": "abc"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result: anyhow::Result<String> = bluesky::post(&publisher_cfg, handle, password, &pds_base, &item, &config, false).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "bluesky_post_success");
}
