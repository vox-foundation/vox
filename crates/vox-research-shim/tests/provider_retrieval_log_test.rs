//! Task 9 review round 1 (B1, m2): the deep trace's provider table is the
//! registry's log of EVERY search it ran — a later search's failure must show
//! up next to an earlier search's success. Offline: wiremock providers only.

use vox_research_shim::research::provider::ProviderRegistry;
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::ProviderStatus;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn offline_policy(wiki: &MockServer) -> SearchPolicy {
    SearchPolicy {
        deep_timeout_ms: 10_000,
        tavily_enabled: false,
        searxng_url: None,
        enable_arxiv: false,
        enable_openalex: false,
        wikipedia_api_url: Some(format!("{}/w/api.php", wiki.uri())),
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn retrieval_log_covers_every_search_including_a_later_failure() {
    let wiki = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {"search": [{"title": "Gemini (language model)", "pageid": 7, "snippet": "Gemini Flash"}]}
        })))
        .mount(&wiki)
        .await;
    let openalex = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&openalex)
        .await;

    let registry = ProviderRegistry::default();
    // Wave 1: wikipedia only.
    let (hits, _) = registry
        .search_with_lane("gemini flash", ResearchLane::Deep, &offline_policy(&wiki))
        .await;
    assert_eq!(hits.len(), 1);
    // A later wave: openalex now enabled and failing.
    let later = SearchPolicy {
        enable_openalex: true,
        openalex_api_url: Some(openalex.uri()),
        ..offline_policy(&wiki)
    };
    registry
        .search_with_lane("gemini flash disambiguation", ResearchLane::Deep, &later)
        .await;

    let (rows, _credits) = registry.into_retrieval_log();
    let wiki_row = rows
        .iter()
        .find(|r| r.provider == "wikipedia" && matches!(r.status, ProviderStatus::Ok { .. }))
        .unwrap_or_else(|| panic!("wikipedia ok row: {rows:?}"));
    assert_eq!(wiki_row.calls, 2);
    assert_eq!(wiki_row.status, ProviderStatus::Ok { hits: 2 });
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
