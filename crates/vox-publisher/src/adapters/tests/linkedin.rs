use wiremock::matchers::{method, path, header, body_json};
use wiremock::{Mock, MockServer, ResponseTemplate};
use crate::adapters::linkedin;
use crate::PublisherConfig;
use crate::types::{LinkedInConfig, UnifiedNewsItem};
use serde_json::json;

#[tokio::test]
async fn test_linkedin_post_success() {
    let mock_server = MockServer::start().await;
    let item = UnifiedNewsItem {
        id: "test-item".to_string(),
        title: "Test Title".to_string(),
        content_markdown: "Test Content".to_string(),
        ..Default::default()
    };
    let config = LinkedInConfig {
        author_urn: "urn:li:person:123".to_string(),
        ..Default::default()
    };

    let publisher_cfg = PublisherConfig {
        linkedin_access_token: Some("test-token".to_string()),
        linkedin_api_base: Some(mock_server.uri()),
        ..Default::default()
    };

    Mock::given(method("POST"))
        .and(path("/rest/posts"))
        .and(header("Authorization", "Bearer test-token"))
        .and(header("Linkedin-Version", "202504"))
        .and(body_json(json!({
            "author": "urn:li:person:123",
            "commentary": "Test Content",
            "visibility": "PUBLIC",
            "distribution": {
                "feedDistribution": "MAIN_FEED",
                "targetEntities": [],
                "thirdPartyDistributionChannels": []
            },
            "lifecycleState": "PUBLISHED",
            "isReshareDisabledByAuthor": false
        })))
        .respond_with(ResponseTemplate::new(201).insert_header("x-restli-id", "urn:li:share:456"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let result: anyhow::Result<String> = linkedin::post(&publisher_cfg, &item, &config, false).await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "urn:li:share:456");
}
