use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path, bearer_token};
use crate::adapters::mastodon;
use crate::adapters::tests::{item_fixture, config_fixture};
use crate::types::MastodonOverride;

#[tokio::test]
async fn mastodon_contract_success()  {
    let mock_server = MockServer::start().await;
    let item = item_fixture();
    let mut publisher_cfg = config_fixture(None);
    publisher_cfg.mastodon_access_token = Some("tok".to_string());
    
    publisher_cfg.mastodon_domain = Some(mock_server.uri());
    
    let cfg = MastodonOverride {
                visibility: Some("public".to_string()),
                spoiler_text: None,
        language: None,
    };

    Mock::given(method("POST"))
        .and(path("/api/v1/statuses"))
        .and(bearer_token("tok"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "url": "https://social.test/posts/abc"
        })))
        .mount(&mock_server)
        .await;

    let res = mastodon::post(&publisher_cfg, &item, Some(&cfg), &"x".repeat(500), false).await.expect("post");
    assert_eq!(res, "https://social.test/posts/abc");
}

#[tokio::test]
async fn mastodon_dry_run_isolation()  {
    let mock_server = MockServer::start().await;
    let item = item_fixture();
    let mut publisher_cfg = config_fixture(None);
    publisher_cfg.mastodon_domain = Some(mock_server.uri());
    let cfg = MastodonOverride {
                visibility: Some("public".to_string()),
                spoiler_text: None,
        language: None,
    };

    // No mocks mounted. If it hits the server, it will 404/fail.
    let res = mastodon::post(&publisher_cfg, &item, Some(&cfg), "summary", true).await.unwrap();
    assert!(res.starts_with("dry-run-mastodon-"));
}

#[tokio::test]
async fn mastodon_overlong_content_guard()  {
    let item = item_fixture();
    let mut publisher_cfg = config_fixture(None);
    publisher_cfg.mastodon_access_token = Some("tok".to_string());
    publisher_cfg.mastodon_domain = Some("test.social".to_string());
    
    let cfg = MastodonOverride::default();
    

    let res = mastodon::post(&publisher_cfg, &item, Some(&cfg), &"x".repeat(501), false).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("exceeds 500 limit"));
}
