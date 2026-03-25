use crate::PublisherConfig;
use crate::contract::DEFAULT_OPENCOLLECTIVE_GRAPHQL_URL;
use crate::types::{OpenCollectiveConfig, UnifiedNewsItem};
use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

pub async fn post(
    publisher_cfg: &PublisherConfig,
    token: &str,
    item: &UnifiedNewsItem,
    config: &OpenCollectiveConfig,
) -> Result<String> {
    let client = Client::new();
    let endpoint = publisher_cfg
        .opencollective_graphql_url
        .as_deref()
        .unwrap_or(DEFAULT_OPENCOLLECTIVE_GRAPHQL_URL)
        .to_string();

    let mutation = r#"
      mutation CreateUpdate($update: UpdateCreateInput!) {
        createUpdate(update: $update) {
          id
          slug
          title
        }
      }
    "#;

    let variables = json!({
        "update": {
            "title": &item.title,
            "html": &item.content_markdown,
            "isPrivate": config.is_private,
            "makePublicOn": null,
            "account": {
                "slug": &config.collective_slug
            }
        }
    });

    let res = client
        .post(&endpoint)
        .header("Api-Key", token)
        .header("Content-Type", "application/json")
        .json(&json!({
            "query": mutation,
            "variables": variables
        }))
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        return Err(anyhow!("Open Collective API error: {}", err_text));
    }

    let body: serde_json::Value = res.json().await?;
    if let Some(errors) = body.get("errors") {
        return Err(anyhow!("Open Collective GraphQL error: {}", errors));
    }

    let update_id = body["data"]["createUpdate"]["id"]
        .as_str()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    tracing::info!("Created Open Collective Update successfully: {}", update_id);
    Ok(update_id)
}
