use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, bearer_token};
use crate::adapters::reddit;
use crate::adapters::tests::{item_fixture, config_fixture};
use crate::types::{RedditConfig, RedditPostKind};

#[tokio::test]
async fn reddit_contract_success() {
    let mock_server = MockServer::start().await;
    let item = item_fixture();
    let _publisher_cfg = config_fixture(Some(mock_server.uri()));
    
    let auth = reddit::RedditAuthConfig {
        client_id: "id",
        client_secret: "secret",
        refresh_token: "ref",
        user_agent: "ua",
        api_base: Some(&mock_server.uri()),
    };

    let cfg = RedditConfig {
        subreddit: "rust".to_string(),
        kind: RedditPostKind::Link,
        title_override: None,
        text_override: None,
        url_override: None,
        nsfw: false,
        spoiler: false,
        send_replies: true,
    };

    // 1. Token refresh
    Mock::given(method("POST"))
        .and(path("/api/v1/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "valid-token"
        })))
        .mount(&mock_server)
        .await;

    // 2. Submit
    Mock::given(method("POST"))
        .and(path("/api/submit"))
        .and(bearer_token("valid-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "json": {
                "errors": [],
                "data": { "url": "https://reddit.com/r/rust/..." }
            }
        })))
        .mount(&mock_server)
        .await;

    let res: anyhow::Result<String> = reddit::submit(&auth, &item, &cfg, "https://example.com").await;
    let res = res.unwrap();
    assert!(res.contains("reddit.com"));
}

#[tokio::test]
async fn reddit_body_size_guard() {
    let item = item_fixture();
    let cfg = RedditConfig {
        subreddit: "rust".to_string(),
        kind: RedditPostKind::SelfPost,
        title_override: None,
        text_override: Some("x".repeat(crate::adapters::reddit::SELFPOST_BODY_MAX + 1)),
        url_override: None,
        nsfw: false,
        spoiler: false,
        send_replies: true,
    };

    let auth = reddit::RedditAuthConfig {
        client_id: "id",
        client_secret: "secret",
        refresh_token: "ref",
        user_agent: "ua",
        api_base: None,
    };

    let res: anyhow::Result<String> = reddit::submit(&auth, &item, &cfg, "https://example.com").await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("exceeds"));
}
