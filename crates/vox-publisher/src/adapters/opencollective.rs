use crate::PublisherConfig;
pub const GRAPHQL_URL: &str = "https://api.opencollective.com/graphql/v2";
pub const AUTH_HEADER: &str = "Personal-Token";
use crate::types::{OpenCollectiveConfig, UnifiedNewsItem};
use anyhow::{Result, anyhow};
use pulldown_cmark::{Options, Parser, html};
use reqwest::Client;
use serde_json::json;

pub async fn post(
    publisher_cfg: &PublisherConfig,
    token: &str,
    item: &UnifiedNewsItem,
    config: &OpenCollectiveConfig,
    dry_run: bool,
) -> Result<String> {
    if dry_run {
        return Ok(format!("dry-run-opencollective-{}", item.id));
    }

    let client = Client::new();
    let endpoint = publisher_cfg
        .opencollective_graphql_url
        .as_deref()
        .unwrap_or(GRAPHQL_URL)
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
            "html": markdown_to_html(&item.content_markdown),
            "isPrivate": config.is_private,
            "makePublicOn": config.scheduled_publish_at.map(|dt| dt.to_rfc3339()),
            "account": {
                "slug": &config.collective_slug
            }
        }
    });

    let res = client
        .post(&endpoint)
        .header("Personal-Token", token)
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

fn markdown_to_html(markdown: &str) -> String {
    let parser = Parser::new_ext(markdown, Options::all());
    let mut out = String::new();
    html::push_html(&mut out, parser);
    out
}
