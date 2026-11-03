// Task 9c: no real web host from any test (see tests/common/mod.rs).
mod common;

use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::{ProviderStatus, WebSearchDispatcher};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn status_of<'a>(r: &'a vox_search::web_dispatcher::SearchReport, p: &str) -> &'a ProviderStatus {
    &r.providers
        .iter()
        .find(|o| o.provider == p)
        .unwrap_or_else(|| panic!("no outcome for {p}"))
        .status
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_distinguishes_ok_error_timeout_disabled_and_unconfigured() {
    let wiki = MockServer::start().await;
    Mock::given(method("GET")).and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {"search": [{"title": "Gemini (language model)", "pageid": 7, "snippet": "Gemini Flash"}]}
        })))
        .mount(&wiki).await;
    let openalex = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&openalex)
        .await;

    let policy = SearchPolicy {
        // This test asserts status *classification* (Ok/Error/Disabled/NotConfigured),
        // not timing — the mocks below carry no artificial delay, so a short deadline
        // just races an undelayed local wiremock against host scheduling noise and
        // occasionally misclassifies a provider as Timeout under load. Give it a
        // generous deadline instead of a tight one.
        deep_timeout_ms: 10_000,
        tavily_enabled: false,
        searxng_url: None,
        enable_arxiv: false,
        wikipedia_api_url: Some(format!("{}/w/api.php", wiki.uri())),
        openalex_api_url: Some(openalex.uri()),
        ..Default::default()
    };

    let r =
        WebSearchDispatcher::search_with_report("gemini flash", ResearchLane::Deep, &policy).await;

    assert_eq!(status_of(&r, "wikipedia"), &ProviderStatus::Ok { hits: 1 });
    assert!(
        matches!(status_of(&r, "openalex"), ProviderStatus::Error { .. }),
        "{:?}",
        r.providers
    );
    assert_eq!(status_of(&r, "arxiv"), &ProviderStatus::Disabled);
    assert_eq!(status_of(&r, "searxng"), &ProviderStatus::NotConfigured);
    assert_eq!(
        r.providers.len(),
        5,
        "every provider is reported, including tavily"
    );
    assert_eq!(r.hits.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_marks_slow_provider_as_timeout() {
    let slow = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(3000)))
        .mount(&slow)
        .await;
    let policy = SearchPolicy {
        fast_timeout_ms: 300,
        tavily_enabled: false,
        searxng_url: None,
        enable_arxiv: false,
        enable_openalex: false,
        wikipedia_api_url: Some(format!("{}/w/api.php", slow.uri())),
        ..Default::default()
    };

    let r = WebSearchDispatcher::search_with_report("x y z", ResearchLane::Fast, &policy).await;
    assert_eq!(status_of(&r, "wikipedia"), &ProviderStatus::Timeout);
    assert!(r.hits.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fast_lane_timeout_does_not_open_breaker_but_deep_lane_timeout_does() {
    use vox_search::search_circuit_breaker::{SearchProviderCircuitRegistry, SearchProviderId};

    let searxng = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(800))
                .set_body_json(serde_json::json!({
                    "results": [
                        {"url": "https://openrouter.ai/x", "title": "t", "content": "c", "engine": "searxng", "score": 1.0}
                    ]
                })),
        )
        .mount(&searxng)
        .await;

    let policy = SearchPolicy {
        searxng_url: Some(searxng.uri()),
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        fast_timeout_ms: 200,
        deep_timeout_ms: 3000,
        ..Default::default()
    };

    let registry = SearchProviderCircuitRegistry::new();

    // 1. A Fast-lane search times out against the 800ms-delayed mock (200ms deadline).
    let r1 = WebSearchDispatcher::search_with_report_and_registry(
        "q",
        ResearchLane::Fast,
        &policy,
        &registry,
    )
    .await;
    assert_eq!(status_of(&r1, "searxng"), &ProviderStatus::Timeout);

    // 2. Repeat Fast-lane timeouts past the breaker's failure threshold (1 failure already
    //    arms a cooldown per ProviderCircuitBreaker::record_failure — see search_circuit_breaker.rs).
    //    Run a couple more to be robust to any future threshold change.
    for _ in 0..2 {
        let r = WebSearchDispatcher::search_with_report_and_registry(
            "q",
            ResearchLane::Fast,
            &policy,
            &registry,
        )
        .await;
        assert_eq!(status_of(&r, "searxng"), &ProviderStatus::Timeout);
    }
    assert!(
        registry.is_available(SearchProviderId::Searxng),
        "Fast-lane timeouts must NOT arm the circuit breaker"
    );

    // 3. A Deep-lane search on the same registry must still reach SearXNG (not CircuitOpen)
    //    and get real hits, since the 3000ms deadline comfortably covers the 800ms delay.
    let r3 = WebSearchDispatcher::search_with_report_and_registry(
        "q",
        ResearchLane::Deep,
        &policy,
        &registry,
    )
    .await;
    match status_of(&r3, "searxng") {
        ProviderStatus::Ok { hits } => assert!(*hits > 0, "expected at least one hit"),
        other => panic!("expected Ok, got {other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deep_lane_timeout_still_opens_breaker() {
    use vox_search::search_circuit_breaker::{SearchProviderCircuitRegistry, SearchProviderId};

    let searxng = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(2000)))
        .mount(&searxng)
        .await;

    let policy = SearchPolicy {
        searxng_url: Some(searxng.uri()),
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        fast_timeout_ms: 200,
        deep_timeout_ms: 300, // shorter than the 2000ms mock delay
        ..Default::default()
    };

    let registry = SearchProviderCircuitRegistry::new();

    let r = WebSearchDispatcher::search_with_report_and_registry(
        "q",
        ResearchLane::Deep,
        &policy,
        &registry,
    )
    .await;
    assert_eq!(status_of(&r, "searxng"), &ProviderStatus::Timeout);
    assert!(
        !registry.is_available(SearchProviderId::Searxng),
        "Deep-lane timeouts must still arm the circuit breaker"
    );
}

/// Task 15 Step 3b: the per-provider candidate pool is deeper than the kept
/// output, so the relevance rerank can promote a hit SearXNG ranked #8.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_matching_hit_ranked_eighth_survives_the_final_cap() {
    let results: Vec<_> = (1..=10)
        .map(|i| {
            let (title, content) = if i == 8 {
                (
                    "Gemini 3.8 Flash on OpenRouter".to_string(),
                    "Gemini 3.8 Flash is the latest Gemini Flash model on OpenRouter.".to_string(),
                )
            } else {
                (
                    format!("Unrelated page {i}"),
                    "Cooking recipes and garden tips for the weekend.".to_string(),
                )
            };
            serde_json::json!({
                "url": format!("https://example.test/{i}"),
                "title": title, "content": content, "engine": "searxng",
                "score": 1.0 - (i as f64) * 0.05
            })
        })
        .collect();
    let searxng = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "results": results })),
        )
        .mount(&searxng)
        .await;

    let policy = SearchPolicy {
        searxng_url: Some(searxng.uri()),
        enable_wikipedia: false,
        enable_openalex: false,
        enable_arxiv: false,
        tavily_enabled: false,
        deep_timeout_ms: 10_000,
        searxng_max_results: 5,
        searxng_max_urls_to_scrape: 3,
        candidate_depth: 10,
        ..SearchPolicy::default()
    };

    let r = WebSearchDispatcher::search_with_report_and_registry(
        "latest gemini flash model openrouter",
        ResearchLane::Deep,
        &policy,
        &vox_search::search_circuit_breaker::SearchProviderCircuitRegistry::new(),
    )
    .await;

    assert_eq!(status_of(&r, "searxng"), &ProviderStatus::Ok { hits: 10 });
    assert_eq!(r.hits.len(), 5, "final kept count is unchanged");
    assert_eq!(r.hits[0].path, "https://example.test/8", "{:?}", r.hits);
}
