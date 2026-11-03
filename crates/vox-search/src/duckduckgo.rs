use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DdgResult {
    #[serde(rename = "FirstURL")]
    pub url: String,
    #[serde(rename = "Text")]
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DdgTopicItem {
    Single(DdgResult),
    Category {
        #[serde(rename = "Name")]
        name: Option<String>,
        #[serde(rename = "Topics")]
        topics: Vec<DdgResult>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DdgResponse {
    #[serde(rename = "RelatedTopics", default)]
    pub related_topics: Vec<DdgTopicItem>,
}

pub struct DuckDuckGoClient;

impl DuckDuckGoClient {
    /// Pruned DuckDuckGo search stub.
    ///
    /// DuckDuckGo fallback has been retired in favor of parallel multi-source
    /// keyless retrieval (arXiv, OpenAlex, Wikipedia).
    pub async fn search(
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<crate::searxng::SearxngResult>> {
        let _ = (query, limit);
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ddg_search_stub_returns_empty() {
        let res = DuckDuckGoClient::search("test", 5)
            .await
            .expect("stub search");
        assert!(res.is_empty());
    }
}
