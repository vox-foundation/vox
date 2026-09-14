use serde::{Deserialize, Serialize};
use tracing::debug;

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

impl DdgResponse {
    pub fn flatten_topics(self, limit: usize) -> Vec<DdgResult> {
        let mut out = Vec::new();
        for item in self.related_topics {
            match item {
                DdgTopicItem::Single(r) => {
                    out.push(r);
                    if out.len() >= limit {
                        break;
                    }
                }
                DdgTopicItem::Category { topics, .. } => {
                    for r in topics {
                        out.push(r);
                        if out.len() >= limit {
                            break;
                        }
                    }
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
        out
    }
}

pub struct DuckDuckGoClient;

impl DuckDuckGoClient {
    pub async fn search(
        query: &str,
        limit: usize,
    ) -> anyhow::Result<Vec<crate::searxng::SearxngResult>> {
        let client = vox_http_client::client();
        // DuckDuckGo Instant Answer API is limited but free and no auth.
        // For actual web search, they have a different endpoint but it's often scraper-blocked.
        // We'll use the RelatedTopics as a factual fallback.
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json",
            urlencoding::encode(query)
        );

        debug!(url = %url, query = query, "Firing DuckDuckGo fallback search");

        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "DuckDuckGo search failed with status: {}",
                resp.status()
            ));
        }

        let body: DdgResponse = resp.json().await?;
        let results = body
            .flatten_topics(limit)
            .into_iter()
            .map(|r| crate::searxng::SearxngResult {
                url: r.url,
                title: r.text.clone(),
                content: r.text,
                engine: Some("duckduckgo".to_string()),
                score: None,
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_heterogeneous_ddg_response() {
        let raw_json = r#"{
            "RelatedTopics": [
                {
                    "FirstURL": "https://duckduckgo.com/c/Rust_(programming_language)",
                    "Text": "Rust (programming language)"
                },
                {
                    "Name": "Other Languages",
                    "Topics": [
                        {
                            "FirstURL": "https://duckduckgo.com/c/Go_(programming_language)",
                            "Text": "Go (programming language)"
                        }
                    ]
                }
            ]
        }"#;

        let resp: DdgResponse =
            serde_json::from_str(raw_json).expect("deserialize heterogeneous ddg");
        let results = resp.flatten_topics(10);
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].url,
            "https://duckduckgo.com/c/Rust_(programming_language)"
        );
        assert_eq!(results[0].text, "Rust (programming language)");
        assert_eq!(
            results[1].url,
            "https://duckduckgo.com/c/Go_(programming_language)"
        );
        assert_eq!(results[1].text, "Go (programming language)");
    }

    #[test]
    fn flatten_topics_respects_limit() {
        let raw_json = r#"{
            "RelatedTopics": [
                {
                    "FirstURL": "https://example.com/1",
                    "Text": "One"
                },
                {
                    "Name": "Category",
                    "Topics": [
                        {
                            "FirstURL": "https://example.com/2",
                            "Text": "Two"
                        },
                        {
                            "FirstURL": "https://example.com/3",
                            "Text": "Three"
                        }
                    ]
                }
            ]
        }"#;

        let resp: DdgResponse = serde_json::from_str(raw_json).unwrap();
        let results = resp.flatten_topics(2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].url, "https://example.com/1");
        assert_eq!(results[1].url, "https://example.com/2");
    }
}
