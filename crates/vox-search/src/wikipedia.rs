use crate::searxng::SearxngResult;
use serde::Deserialize;
use tracing::debug;

#[derive(Deserialize)]
struct WikiSearchResponse {
    query: Option<WikiQuery>,
}

#[derive(Deserialize)]
struct WikiQuery {
    search: Option<Vec<WikiSearchItem>>,
}

#[derive(Deserialize)]
struct WikiSearchItem {
    title: String,
    pageid: u64,
    snippet: String,
}

pub struct WikipediaClient;

impl WikipediaClient {
    pub fn parse_search_json(json_str: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        let parsed: WikiSearchResponse = serde_json::from_str(json_str)?;
        let items = parsed.query.and_then(|q| q.search).unwrap_or_default();
        let results = items
            .into_iter()
            .take(limit)
            .map(|item| {
                // Strip HTML tags like <span class="searchmatch"> and decode common entities
                let clean_snippet = item
                    .snippet
                    .replace("<span class=\"searchmatch\">", "")
                    .replace("</span>", "")
                    .replace("&quot;", "\"")
                    .replace("&#039;", "'")
                    .replace("&apos;", "'")
                    .replace("&lt;", "<")
                    .replace("&gt;", ">")
                    .replace("&nbsp;", " ")
                    .replace("&amp;", "&");
                let url = if !item.title.trim().is_empty() {
                    format!(
                        "https://en.wikipedia.org/wiki/{}",
                        urlencoding::encode(&item.title.replace(' ', "_"))
                    )
                } else {
                    format!("https://en.wikipedia.org/?curid={}", item.pageid)
                };
                SearxngResult {
                    url,
                    title: item.title,
                    content: clean_snippet,
                    engine: Some("wikipedia".to_string()),
                    score: Some(0.85),
                }
            })
            .collect();
        Ok(results)
    }

    pub async fn search(query: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let client = vox_http_client::client();
        let srlimit = limit.clamp(1, 50);
        let url = format!(
            "https://en.wikipedia.org/w/api.php?action=query&list=search&srsearch={}&utf8=&format=json&srlimit={}",
            urlencoding::encode(query.trim()),
            srlimit
        );
        debug!(url = %url, query = query, "Firing Wikipedia encyclopedic fallback");
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Wikipedia API returned status {}",
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
    fn parses_wikipedia_api_response() {
        let sample_json = r#"{
            "query": {
                "search": [
                    {
                        "ns": 0,
                        "title": "Accessibility",
                        "pageid": 1475,
                        "size": 34102,
                        "wordcount": 3421,
                        "snippet": "Accessibility is the design of products, devices, services, or environments for people with disabilities.",
                        "timestamp": "2026-01-01T00:00:00Z"
                    }
                ]
            }
        }"#;
        let hits = WikipediaClient::parse_search_json(sample_json, 5).expect("parse json");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Accessibility");
        assert_eq!(hits[0].url, "https://en.wikipedia.org/wiki/Accessibility");
        assert!(hits[0].content.contains("Accessibility is the design"));
        assert_eq!(hits[0].engine.as_deref(), Some("wikipedia"));
        assert_eq!(hits[0].score, Some(0.85));
    }

    #[test]
    fn falls_back_to_curid_url_when_title_is_empty() {
        let sample_json = r#"{
            "query": {
                "search": [
                    {
                        "title": "",
                        "pageid": 1475,
                        "snippet": "No title"
                    }
                ]
            }
        }"#;
        let hits = WikipediaClient::parse_search_json(sample_json, 5).expect("parse json");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].url, "https://en.wikipedia.org/?curid=1475");
    }

    #[test]
    fn parses_wikipedia_snippet_strips_searchmatch_and_entities() {
        let sample_json = r#"{
            "query": {
                "search": [
                    {
                        "title": "Rust",
                        "pageid": 42,
                        "snippet": "<span class=\"searchmatch\">Rust</span> is &quot;fast&quot; &amp; safe. It&#039;s &lt;great&gt; &amp; &apos;cool&apos;&nbsp;!"
                    }
                ]
            }
        }"#;
        let hits = WikipediaClient::parse_search_json(sample_json, 5).expect("parse json");
        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].content,
            "Rust is \"fast\" & safe. It's <great> & 'cool' !"
        );
    }

    #[test]
    fn respects_limit_and_handles_empty() {
        let sample_json = r#"{
            "query": {
                "search": [
                    { "title": "Page 1", "pageid": 1, "snippet": "Snippet 1" },
                    { "title": "Page 2", "pageid": 2, "snippet": "Snippet 2" },
                    { "title": "Page 3", "pageid": 3, "snippet": "Snippet 3" }
                ]
            }
        }"#;
        let hits = WikipediaClient::parse_search_json(sample_json, 2).expect("parse json");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].title, "Page 1");
        assert_eq!(hits[0].url, "https://en.wikipedia.org/wiki/Page_1");
        assert_eq!(hits[1].title, "Page 2");
        assert_eq!(hits[1].url, "https://en.wikipedia.org/wiki/Page_2");

        let empty_json = r#"{"query": {"search": []}}"#;
        let hits_empty = WikipediaClient::parse_search_json(empty_json, 5).expect("parse json");
        assert!(hits_empty.is_empty());

        let null_query = r#"{}"#;
        let hits_null = WikipediaClient::parse_search_json(null_query, 5).expect("parse json");
        assert!(hits_null.is_empty());
    }

    #[tokio::test]
    async fn search_returns_empty_immediately_on_blank_query_or_zero_limit() {
        let hits_blank = WikipediaClient::search("   ", 5)
            .await
            .expect("search blank");
        assert!(hits_blank.is_empty());

        let hits_zero = WikipediaClient::search("valid query", 0)
            .await
            .expect("search zero");
        assert!(hits_zero.is_empty());
    }
}
