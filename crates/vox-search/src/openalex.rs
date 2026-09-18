use std::collections::HashMap;

use serde::Deserialize;
use tracing::debug;

use crate::searxng::SearxngResult;

const DEFAULT_OPENALEX_BASE_URL: &str = "https://api.openalex.org";

#[derive(Deserialize)]
struct OpenAlexResponse {
    results: Option<Vec<OpenAlexWork>>,
}

#[derive(Deserialize)]
struct OpenAlexWork {
    id: Option<String>,
    doi: Option<String>,
    title: Option<String>,
    abstract_inverted_index: Option<HashMap<String, Vec<usize>>>,
    primary_location: Option<OpenAlexLocation>,
    open_access: Option<OpenAccessInfo>,
}

#[derive(Deserialize)]
struct OpenAlexLocation {
    landing_page_url: Option<String>,
}

#[derive(Deserialize)]
struct OpenAccessInfo {
    oa_url: Option<String>,
}

/// Reconstruct plain-text abstract from an OpenAlex inverted index and truncate cleanly.
pub fn reconstruct_abstract(inverted: &HashMap<String, Vec<usize>>, max_chars: usize) -> String {
    if inverted.is_empty() {
        return String::new();
    }
    let mut tokens: Vec<(usize, &str)> = Vec::new();
    for (word, positions) in inverted {
        for &pos in positions {
            tokens.push((pos, word.as_str()));
        }
    }
    tokens.sort_by_key(|&(pos, _)| pos);
    let full = tokens
        .into_iter()
        .map(|(_, w)| w)
        .collect::<Vec<_>>()
        .join(" ");
    if full.chars().count() <= max_chars {
        full
    } else if max_chars < 3 {
        full.chars().take(max_chars).collect()
    } else {
        let keep = max_chars.saturating_sub(3);
        let prefix: String = full.chars().take(keep).collect();
        format!("{}...", prefix.trim_end())
    }
}

pub struct OpenAlexClient;

impl OpenAlexClient {
    /// Parse OpenAlex `/works` JSON response into `SearxngResult` items up to `limit`.
    pub fn parse_search_json(json_str: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        let parsed: OpenAlexResponse = serde_json::from_str(json_str)?;
        let items = parsed.results.unwrap_or_default();
        let results = items
            .into_iter()
            .take(limit)
            .map(|item| {
                let url = item
                    .open_access
                    .as_ref()
                    .and_then(|oa| oa.oa_url.as_ref())
                    .filter(|u| !u.trim().is_empty())
                    .or_else(|| {
                        item.primary_location
                            .as_ref()
                            .and_then(|loc| loc.landing_page_url.as_ref())
                            .filter(|u| !u.trim().is_empty())
                    })
                    .or_else(|| item.doi.as_ref().filter(|u| !u.trim().is_empty()))
                    .or_else(|| item.id.as_ref().filter(|u| !u.trim().is_empty()))
                    .cloned()
                    .unwrap_or_default();

                let title = item.title.unwrap_or_default();
                let content = item
                    .abstract_inverted_index
                    .as_ref()
                    .map(|inv| reconstruct_abstract(inv, 2000))
                    .unwrap_or_default();

                SearxngResult {
                    url,
                    title,
                    content,
                    engine: Some("openalex".to_string()),
                    score: Some(0.85),
                }
            })
            .collect();
        Ok(results)
    }

    /// Search OpenAlex works endpoint with optional base URL and API key injection.
    pub async fn search(
        query: &str,
        limit: usize,
        base_url: Option<&str>,
        api_key: Option<&str>,
    ) -> anyhow::Result<Vec<SearxngResult>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let base = base_url.unwrap_or(DEFAULT_OPENALEX_BASE_URL);
        let url = format!(
            "{}/works?search={}&per-page={}&mailto=research@vox.computer",
            base.trim_end_matches('/'),
            urlencoding::encode(query.trim()),
            limit
        );

        debug!(url = %url, query = query, "Firing OpenAlex search");

        let client = vox_http_client::client();
        let mut req = client.get(&url);
        if let Some(key) = api_key {
            let trimmed_key = key.trim();
            if !trimmed_key.is_empty() {
                req = req.header("api-key", trimmed_key);
            }
        }

        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "OpenAlex API returned status {}",
                resp.status()
            ));
        }

        let text = resp.text().await?;
        Self::parse_search_json(&text, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconstruct_abstract_basic() {
        let mut inverted = HashMap::new();
        inverted.insert("Hello".to_string(), vec![0]);
        inverted.insert("world.".to_string(), vec![1]);
        let res = reconstruct_abstract(&inverted, 50);
        assert_eq!(res, "Hello world.");
    }

    #[test]
    fn test_parse_search_json_basic() {
        let json = r#"{
            "results": [
                {
                    "id": "https://openalex.org/W1",
                    "title": "Test Title",
                    "primary_location": { "landing_page_url": "https://example.com/paper" }
                }
            ]
        }"#;
        let hits = OpenAlexClient::parse_search_json(json, 5).expect("parse json");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Test Title");
        assert_eq!(hits[0].url, "https://example.com/paper");
    }

    #[tokio::test]
    async fn test_search_short_circuit_empty() {
        let hits = OpenAlexClient::search("   ", 5, None, None)
            .await
            .expect("search");
        assert!(hits.is_empty());
    }
}
