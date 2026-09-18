// crates/vox-search/tests/partial_harvest_resilience_test.rs
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::{WebSearchDispatcher, WebSearchDispatcherExt};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_partial_harvest_resilience_slow_provider_does_not_block_fast() {
    let slow_server = MockServer::start().await;
    let fast_server = MockServer::start().await;

    // Slow provider: delays by 4000ms (exceeds fast lane deadline of 1500ms)
    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(4000)))
        .mount(&slow_server)
        .await;

    // Fast provider: responds in 40ms
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(std::time::Duration::from_millis(40))
                .set_body_json(serde_json::json!({
                    "query": {
                        "search": [
                            {
                                "title": "Quantum Error Correction",
                                "pageid": 42,
                                "snippet": "Surface codes and fault-tolerant quantum computation"
                            }
                        ]
                    }
                })),
        )
        .mount(&fast_server)
        .await;

    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 1500;
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_arxiv = false;
    policy.openalex_api_url = Some(slow_server.uri());
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", fast_server.uri()));

    // Warm up OS networking/proxy stack to ensure deterministic timing on macOS
    let _ = vox_http_client::client()
        .get(format!("{}/w/api.php", fast_server.uri()))
        .send()
        .await;

    let start = std::time::Instant::now();
    let dispatcher = WebSearchDispatcher::new();
    let hits = dispatcher
        .search_with_lane("quantum error correction", ResearchLane::Fast, &policy)
        .await
        .expect("partial harvest search should succeed");
    let elapsed = start.elapsed();

    // Must return within the deadline (with small buffer for scheduling)
    assert!(
        elapsed < std::time::Duration::from_millis(2500),
        "Partial harvest must not hang on slow provider (took {:?})",
        elapsed
    );

    // Fast results must be harvested
    assert!(
        !hits.is_empty(),
        "Fast provider results must be harvested successfully"
    );
    assert_eq!(hits[0].title, "Quantum Error Correction");

    // Slow results must be excluded
    assert!(
        hits.iter()
            .all(|h| !h.provenance.iter().any(|p| p == "engine:openalex")),
        "Slow OpenAlex results must be pruned after deadline expiration"
    );
    assert!(
        hits.iter()
            .any(|h| h.provenance.iter().any(|p| p == "engine:wikipedia")),
        "Wikipedia fast result must be present in hits"
    );
}
