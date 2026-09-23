//! Wiremock stub for Tavily `/extract` uplift (`tavily_extract`).

use vox_search::policy::SearchPolicy;
use vox_search::tavily::{TavilyClient, TavilySessionBudget};
use vox_search::tavily_extract::{snippet_quality_low, uplift_low_quality_snippets};
use vox_search::web_dispatcher::WebSearchDispatcher;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn snippet_quality_heuristic() {
    assert!(snippet_quality_low("tiny"));
    assert!(!snippet_quality_low(
        "A sufficiently long alphanumeric snippet that should not trigger Tavily extract uplift because it has enough grounding text."
    ));
}

#[tokio::test]
async fn extract_client_maps_wiremock_response() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/extract"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "url": "https://example.test/page",
                "raw_content": "Expanded markdown body from Tavily extract with enough detail for grounding and citation."
            }],
            "failed_results": [],
            "response_time": 0.42
        })))
        .mount(&mock)
        .await;

    let client = TavilyClient::new("test-key", Some(&mock.uri())).expect("client");
    let hits = client
        .extract(&[String::from("https://example.test/page")])
        .await
        .expect("extract");

    assert_eq!(hits.len(), 1);
    assert!(hits[0].content.contains("Expanded markdown"));
}

#[tokio::test]
async fn extract_uplift_replaces_thin_snippet_content() {
    // Exercises the real `uplift_low_quality_snippets`, not a copy of its loop.
    let tavily = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/extract"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "url": "https://example.test/thin",
                "raw_content": "This uplifted extract body is long enough to pass snippet quality checks and provide useful grounding for the research pipeline."
            }],
            "failed_results": [],
            "response_time": 0.5
        })))
        .mount(&tavily)
        .await;

    let mut rows = [vox_search::searxng::SearxngResult {
        url: "https://example.test/thin".to_string(),
        title: "Thin page".to_string(),
        content: "short".to_string(),
        engine: Some("google".to_string()),
        score: Some(0.7),
    }];
    assert!(snippet_quality_low(&rows[0].content));

    let client = TavilyClient::new("test-key", Some(&tavily.uri())).expect("client");
    let budget = TavilySessionBudget::new(5);
    uplift_low_quality_snippets(&mut rows, 3, &client, &budget).await;
    assert_eq!(rows[0].engine.as_deref(), Some("google+tavily_extract"));
    assert_eq!(budget.usage_and_remaining(), (1, 4));
    assert!(!snippet_quality_low(&rows[0].content));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn web_dispatcher_maps_searxng_without_extract_when_snippet_ok() {
    let searx = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "url": "https://example.test/page",
                "title": "Title",
                "content": "A sufficiently long alphanumeric snippet that should not trigger Tavily extract uplift because it has enough grounding text for the pipeline.",
                "engine": "google",
                "score": 0.9
            }]
        })))
        .mount(&searx)
        .await;

    let policy = SearchPolicy {
        searxng_url: Some(searx.uri()),
        duckduckgo_fallback_enabled: false,
        tavily_enabled: false,
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        fast_timeout_ms: 4000,
        ..SearchPolicy::default()
    };

    let _ = vox_http_client::client()
        .get(format!("{}/search", searx.uri()))
        .send()
        .await;

    let hits = WebSearchDispatcher::search("query text", &policy)
        .await
        .expect("dispatcher");

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "https://example.test/page");
}
