//! Tavily leg of `WebSearchDispatcher`, end to end against wiremock (Task 15).
//! No network, no real key: the client is built with a dummy key and the mock's URL.
#![cfg(feature = "tavily")]

// Task 9c: no real web host from any test (see tests/common/mod.rs).
mod common;

use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::search_circuit_breaker::{SearchProviderCircuitRegistry, SearchProviderId};
use vox_search::tavily::{TavilyClient, TavilySessionBudget};
use vox_search::web_dispatcher::{ProviderStatus, SearchReport, WebSearchDispatcher};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn tavily_only_policy() -> SearchPolicy {
    SearchPolicy {
        tavily_enabled: true,
        searxng_url: None,
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        // Not a timeout test: generous deadline so scheduling noise cannot flip Ok → Timeout.
        deep_timeout_ms: 10_000,
        ..SearchPolicy::default()
    }
}

fn tavily_status(r: &SearchReport) -> &ProviderStatus {
    &r.providers
        .iter()
        .find(|o| o.provider == "tavily")
        .expect("tavily outcome")
        .status
}

fn search_body(n: usize) -> serde_json::Value {
    let results: Vec<_> = (0..n)
        .map(|i| {
            serde_json::json!({
                "title": format!("Gemini 3.8 Flash release notes {i}"),
                "url": format!("https://example.test/gemini-{i}"),
                "content": "Gemini 3.8 Flash is the latest Gemini Flash model, released on OpenRouter with long context and low latency pricing.",
                "score": 0.9 - (i as f64) * 0.1,
            })
        })
        .collect();
    serde_json::json!({
        "query": "q", "answer": null, "images": [], "response_time": 0.3, "results": results
    })
}

async fn run(
    mock: &MockServer,
    registry: &SearchProviderCircuitRegistry,
    budget: &TavilySessionBudget,
) -> SearchReport {
    let client = TavilyClient::new("tvly-test", Some(&mock.uri())).expect("client");
    WebSearchDispatcher::search_with_tavily(
        "gemini flash latest",
        ResearchLane::Deep,
        &tavily_only_policy(),
        registry,
        Some(client),
        budget,
    )
    .await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tavily_hits_reach_the_report_with_engine_provenance() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        // drift-allow(bearer-header-inline): parses/matches/redacts a header, or a test fixture; not building one
        .and(header("authorization", "Bearer tvly-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_body(3)))
        .expect(1)
        .mount(&mock)
        .await;
    let budget = TavilySessionBudget::new(10);

    let r = run(&mock, &SearchProviderCircuitRegistry::new(), &budget).await;

    assert_eq!(tavily_status(&r), &ProviderStatus::Ok { hits: 3 });
    assert_eq!(r.hits.len(), 3);
    assert!(
        r.hits
            .iter()
            .all(|h| h.provenance.iter().any(|p| p == "engine:tavily")),
        "{:?}",
        r.hits
    );
    assert_eq!(
        budget.usage_and_remaining(),
        (1, 9),
        "basic search costs 1 credit"
    );
    assert_eq!(r.tavily_credits, Some((1, 9)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tavily_401_is_an_error_with_the_provider_message_not_ok_zero() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "detail": {"error": "Unauthorized: missing or invalid API key."}
        })))
        .mount(&mock)
        .await;

    let r = run(
        &mock,
        &SearchProviderCircuitRegistry::new(),
        &TavilySessionBudget::new(10),
    )
    .await;

    match tavily_status(&r) {
        ProviderStatus::Error { message } => {
            assert!(message.contains("401"), "{message}");
            assert!(message.contains("missing or invalid API key"), "{message}");
        }
        other => panic!("expected Error, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tavily_429_is_an_error_and_arms_a_rate_limit_cooldown() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "detail": {"error": "Too many requests"}
        })))
        // Exactly one request: the client must not retry-with-backoff inside the lane deadline.
        .expect(1)
        .mount(&mock)
        .await;
    let registry = SearchProviderCircuitRegistry::new();

    let r = run(&mock, &registry, &TavilySessionBudget::new(10)).await;

    assert!(
        matches!(tavily_status(&r), ProviderStatus::Error { message } if message.contains("429")),
        "{:?}",
        r.providers
    );
    // A plain failure cools down 10 s; a rate-limit failure 120 s.
    let cooldown = registry
        .cooldown_remaining(SearchProviderId::Tavily)
        .expect("breaker armed");
    assert!(
        cooldown > std::time::Duration::from_secs(60),
        "rate-limit cooldown expected, got {cooldown:?}"
    );

    // The open breaker is reported as such, not as "not configured".
    let again = run(&mock, &registry, &TavilySessionBudget::new(10)).await;
    assert_eq!(tavily_status(&again), &ProviderStatus::CircuitOpen);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tavily_call_past_the_session_budget_is_skipped_as_budget_exhausted() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_body(1)))
        .expect(2)
        .mount(&mock)
        .await;
    let registry = SearchProviderCircuitRegistry::new();
    let budget = TavilySessionBudget::new(2);

    for _ in 0..2 {
        let r = run(&mock, &registry, &budget).await;
        assert_eq!(tavily_status(&r), &ProviderStatus::Ok { hits: 1 });
    }
    let third = run(&mock, &registry, &budget).await;
    assert_eq!(tavily_status(&third), &ProviderStatus::BudgetExhausted);
    assert!(
        registry.is_available(SearchProviderId::Tavily),
        "an exhausted budget is not a provider failure"
    );
    // `.expect(2)` is verified when `mock` drops: the third call never reached Tavily.
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn thin_tavily_snippet_is_replaced_by_extract_content() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": "q", "answer": null, "images": [], "response_time": 0.2,
            "results": [{"title": "Gemini flash", "url": "https://example.test/thin", "content": "short", "score": 0.8}]
        })))
        .mount(&mock)
        .await;
    Mock::given(method("POST"))
        .and(path("/extract"))
        // drift-allow(bearer-header-inline): parses/matches/redacts a header, or a test fixture; not building one
        .and(header("authorization", "Bearer tvly-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "url": "https://example.test/thin",
                "raw_content": "Extracted body: Gemini 3.8 Flash is the newest Gemini Flash model available on OpenRouter."
            }],
            "failed_results": [],
            "response_time": 0.4
        })))
        .expect(1)
        .mount(&mock)
        .await;
    let budget = TavilySessionBudget::new(10);

    let r = run(&mock, &SearchProviderCircuitRegistry::new(), &budget).await;

    assert_eq!(r.hits.len(), 1);
    assert!(
        r.hits[0].content_snippet.starts_with("Extracted body:"),
        "{:?}",
        r.hits[0]
    );
    assert!(
        r.hits[0]
            .provenance
            .iter()
            .any(|p| p == "engine:tavily+tavily_extract"),
        "{:?}",
        r.hits[0].provenance
    );
    assert_eq!(
        budget.usage_and_remaining().0,
        2,
        "search (1) + extract (1) both charged to the session budget"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_json_error_body_is_truncated_in_the_status_message() {
    let mock = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(502).set_body_string("<html>".repeat(2000)))
        .mount(&mock)
        .await;

    let r = run(
        &mock,
        &SearchProviderCircuitRegistry::new(),
        &TavilySessionBudget::new(10),
    )
    .await;

    match tavily_status(&r) {
        ProviderStatus::Error { message } => {
            assert!(message.contains(":502:"), "{message}");
            assert!(message.chars().count() < 350, "{} chars", message.len());
        }
        other => panic!("expected Error, got {other:?}"),
    }
}
