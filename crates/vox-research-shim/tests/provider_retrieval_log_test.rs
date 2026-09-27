//! Task 9 review round 1 (B1, m2): the deep trace's provider table is the
//! registry's log of EVERY search it ran — a later search's failure must show
//! up next to an earlier search's success.
//!
//! Hermetic in every build: the hit provider is a wiremock SearXNG whose result
//! URL points back at the same mock, so when `vox-search/web-scrape` is
//! feature-unified on (any workspace-wide build), `fetch_and_extract` scrapes
//! the mock — never a real site. (A wiremock *Wikipedia* hit would not do: the
//! Wikipedia client hard-codes `https://en.wikipedia.org/wiki/…` hit URLs.)

use vox_research_shim::research::provider::ProviderRegistry;
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::ProviderStatus;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A SearXNG mock returning one hit whose URL is served by the same mock.
async fn local_searxng() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{
                "url": format!("{}/page", s.uri()),
                "title": "Gemini Flash release notes",
                "content": "Gemini Flash model release notes and pricing details",
                "engine": "mock",
                "score": 1.0
            }]
        })))
        .mount(&s)
        .await;
    Mock::given(method("GET"))
        .and(path("/page"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<html><head><title>Gemini Flash</title></head><body><p>Gemini Flash model release notes and pricing details for the local mock page.</p></body></html>",
        ))
        .mount(&s)
        .await;
    s
}

fn offline_policy(searxng: &MockServer) -> SearchPolicy {
    SearchPolicy {
        deep_timeout_ms: 10_000,
        tavily_enabled: false,
        searxng_url: Some(searxng.uri()),
        enable_wikipedia: false,
        wikipedia_fallback_enabled: false,
        enable_arxiv: false,
        enable_openalex: false,
        duckduckgo_fallback_enabled: false,
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retrieval_log_covers_every_search_including_a_later_failure() {
    let searxng = local_searxng().await;
    let openalex = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&openalex)
        .await;

    let registry = ProviderRegistry::default();
    // Wave 1: searxng only.
    let (hits, _) = registry
        .search_with_lane(
            "gemini flash",
            ResearchLane::Deep,
            &offline_policy(&searxng),
        )
        .await;
    assert_eq!(hits.len(), 1);
    assert!(
        hits[0].url.starts_with(&searxng.uri()),
        "the hit (and any scrape of it) stays on the mock: {}",
        hits[0].url
    );
    // A later wave: openalex now enabled and failing.
    let later = SearchPolicy {
        enable_openalex: true,
        openalex_api_url: Some(openalex.uri()),
        ..offline_policy(&searxng)
    };
    registry
        .search_with_lane("gemini flash disambiguation", ResearchLane::Deep, &later)
        .await;

    let (rows, _credits) = registry.into_retrieval_log();
    let hit_row = rows
        .iter()
        .find(|r| r.provider == "searxng" && matches!(r.status, ProviderStatus::Ok { .. }))
        .unwrap_or_else(|| panic!("searxng ok row: {rows:?}"));
    assert_eq!(hit_row.calls, 2);
    assert_eq!(hit_row.status, ProviderStatus::Ok { hits: 2 });
    let failed = rows
        .iter()
        .find(|r| r.provider == "openalex" && matches!(r.status, ProviderStatus::Error { .. }))
        .unwrap_or_else(|| panic!("the later wave's openalex error must be logged: {rows:?}"));
    assert_eq!(failed.calls, 1);
    // Wave 1 had openalex disabled — that outcome is its own row, not folded away.
    assert!(
        rows.iter()
            .any(|r| r.provider == "openalex" && r.status == ProviderStatus::Disabled),
        "{rows:?}"
    );
}
