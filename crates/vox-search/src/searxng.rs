use serde::{Deserialize, Serialize};
use tracing::debug;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearxngResult {
    pub url: String,
    pub title: String,
    pub content: String,
    /// Raw sub-engine/source label. For SearXNG hits this is whatever
    /// SearXNG's own response says (e.g. "brave", "yahoo", "yep") — kept
    /// for display provenance. Not a stable "who produced this" key; use
    /// `provider` for that (Task 8c).
    pub engine: Option<String>,
    pub score: Option<f64>,
    /// The top-level provider that fetched this hit (arxiv / openalex /
    /// wikipedia / tavily / searxng), set once by the provider task in
    /// `web_dispatcher::search_core` — never inferred from `engine`, since
    /// SearXNG's sub-engine names vary and don't identify the provider.
    /// `None` for hits built outside that path (e.g. raw SearXNG API
    /// deserialization before tagging, or test fixtures that don't need it).
    #[serde(default)]
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearxngResponse {
    pub results: Vec<SearxngResult>,
}

pub struct SearxngSearchClient {
    pub base_url: String,
}

impl SearxngSearchClient {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }

    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        engines_csv: &str,
        language: &str,
    ) -> anyhow::Result<Vec<SearxngResult>> {
        let client = vox_http_client::client();
        let url = format!(
            "{}/search?q={}&format=json&engines={}&language={}",
            self.base_url.trim_end_matches('/'),
            urlencoding::encode(query),
            urlencoding::encode(engines_csv),
            urlencoding::encode(language),
        );

        debug!(
            url = %url,
            query = query,
            engines = engines_csv,
            language = language,
            "Firing SearXNG search"
        );

        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "SearXNG search failed with status: {}",
                resp.status()
            ));
        }

        let body: SearxngResponse = resp.json().await?;
        let mut results = body.results;
        results.truncate(limit);

        Ok(results)
    }
}
