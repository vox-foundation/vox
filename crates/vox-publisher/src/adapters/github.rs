use crate::contract::{DEFAULT_GITHUB_GRAPHQL_URL, DEFAULT_GITHUB_REST_BASE};
use crate::types::{GitHubConfig, GitHubPostType, UnifiedNewsItem};
use crate::PublisherConfig;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::json;

pub async fn post(
    publisher_cfg: &PublisherConfig,
    token: &str,
    item: &UnifiedNewsItem,
    config: &GitHubConfig,
) -> Result<String> {
    match config.post_type {
        GitHubPostType::Discussion => post_discussion(publisher_cfg, token, item, config).await,
        GitHubPostType::Release => post_release(publisher_cfg, token, item, config).await,
    }
}

async fn post_release(
    publisher_cfg: &PublisherConfig,
    token: &str,
    item: &UnifiedNewsItem,
    config: &GitHubConfig,
) -> Result<String> {
    let rest_base = publisher_cfg
        .github_rest_base
        .clone()
        .unwrap_or_else(|| DEFAULT_GITHUB_REST_BASE.to_string());
    let base = format!("{}/", rest_base.trim_end_matches('/'));

    let octocrab = octocrab::Octocrab::builder()
        .personal_token(token.to_string())
        .base_uri(base.as_str())
        .map_err(|e| anyhow!("invalid github rest base: {e}"))?
        .build()?;

    let parts: Vec<&str> = config.repo.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid target repo format, expected owner/repo, got {}",
            config.repo
        ));
    }
    let owner = parts[0];
    let repo = parts[1];

    let tag_name = config
        .release_tag
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(item.id.as_str());

    let release = octocrab
        .repos(owner, repo)
        .releases()
        .create(tag_name)
        .name(&item.title)
        .body(&item.content_markdown)
        .draft(config.draft)
        .send()
        .await?;
    tracing::info!("Created GitHub Release: {}", release.html_url);
    Ok(release.html_url.to_string())
}

async fn post_discussion(
    publisher_cfg: &PublisherConfig,
    token: &str,
    item: &UnifiedNewsItem,
    config: &GitHubConfig,
) -> Result<String> {
    let gql_url = publisher_cfg
        .github_graphql_url
        .clone()
        .unwrap_or_else(|| DEFAULT_GITHUB_GRAPHQL_URL.to_string());

    let parts: Vec<&str> = config.repo.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid target repo format, expected owner/repo, got {}",
            config.repo
        ));
    }
    let owner = parts[0];
    let repo = parts[1];
    let category_name = config
        .discussion_category
        .as_deref()
        .map(str::trim)
        .ok_or_else(|| anyhow!("discussion_category required"))?;

    let client = Client::new();

    let q_repo = json!({
        "query": r#"query($o:String!,$n:String!){
            repository(owner:$o,name:$n){
                id
                discussionCategories(first:25){
                    nodes{ id name }
                }
            }
        }"#,
        "variables": { "o": owner, "n": repo }
    });

    let res = client
        .post(&gql_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "vox-publisher")
        .json(&q_repo)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!("GitHub GraphQL (repo query) HTTP {}", res.status()));
    }
    let body: serde_json::Value = res.json().await?;
    if let Some(errs) = body.get("errors") {
        return Err(anyhow!("GitHub GraphQL errors: {}", errs));
    }

    let repo_id = body["data"]["repository"]["id"]
        .as_str()
        .ok_or_else(|| anyhow!("missing repository id"))?;
    let nodes = body["data"]["repository"]["discussionCategories"]["nodes"]
        .as_array()
        .ok_or_else(|| anyhow!("missing discussionCategories"))?;

    let cat_lower = category_name.to_lowercase();
    let category_id = nodes
        .iter()
        .find(|n| {
            n["name"]
                .as_str()
                .map(|s| s.to_lowercase() == cat_lower)
                .unwrap_or(false)
        })
        .and_then(|n| n["id"].as_str())
        .ok_or_else(|| {
            anyhow!(
                "No discussion category matching {:?} on {}/{}",
                category_name,
                owner,
                repo
            )
        })?;

    let mutation = json!({
        "query": r#"mutation($input:CreateDiscussionInput!){
            createDiscussion(input:$input){
                discussion{ id url }
            }
        }"#,
        "variables": {
            "input": {
                "repositoryId": repo_id,
                "categoryId": category_id,
                "title": item.title,
                "body": item.content_markdown
            }
        }
    });

    let res2 = client
        .post(&gql_url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "vox-publisher")
        .json(&mutation)
        .send()
        .await?;

    if !res2.status().is_success() {
        return Err(anyhow!(
            "GitHub GraphQL (createDiscussion) HTTP {}",
            res2.status()
        ));
    }
    let body2: serde_json::Value = res2.json().await?;
    if let Some(errs) = body2.get("errors") {
        return Err(anyhow!("GitHub GraphQL errors: {}", errs));
    }

    let url = body2["data"]["createDiscussion"]["discussion"]["url"]
        .as_str()
        .map(std::string::ToString::to_string)
        .unwrap_or_default();
    if url.is_empty() {
        return Err(anyhow!(
            "createDiscussion returned empty url: {}",
            body2
        ));
    }
    tracing::info!("Created GitHub Discussion: {}", url);
    Ok(url)
}
