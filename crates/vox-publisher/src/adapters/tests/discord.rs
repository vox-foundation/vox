use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};
use crate::adapters::discord;
use crate::adapters::tests::{item_fixture, config_fixture};
use crate::types::DiscordConfig;

#[tokio::test]
async fn discord_contract_success() {
    let mock_server = MockServer::start().await;
    let item = item_fixture();
    let mut publisher_cfg = config_fixture(None);
    publisher_cfg.discord_webhook_url = Some(format!("{}/webhook/123", mock_server.uri()));
    
    let cfg = DiscordConfig {
        message: Some("hello".to_string()),
        tts: false,
        embed_title: None,
        embed_url: None,
        embed_description: None,
        embed_color: None,
    };

    Mock::given(method("POST"))
        .and(path("/webhook/123"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&mock_server)
        .await;

    let res = discord::post(&publisher_cfg, &item, &cfg, false).await.unwrap();
    assert!(res.starts_with("discord-"));
}

#[tokio::test]
async fn discord_overlong_content_guard() {
    let item = item_fixture();
    let mut publisher_cfg = config_fixture(None);
    publisher_cfg.discord_webhook_url = Some("http://localhost/dummy".to_string());
    let mut cfg = DiscordConfig::default();
    cfg.message = Some("x".repeat(2001));

    let res = discord::post(&publisher_cfg, &item, &cfg, false).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("exceeds 2000 char limit"));
}
