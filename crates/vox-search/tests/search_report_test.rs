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

    let mut policy = SearchPolicy::default();
    policy.deep_timeout_ms = 1500;
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_arxiv = false;
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", wiki.uri()));
    policy.openalex_api_url = Some(openalex.uri());

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
    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 300;
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_arxiv = false;
    policy.enable_openalex = false;
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", slow.uri()));

    let r = WebSearchDispatcher::search_with_report("x y z", ResearchLane::Fast, &policy).await;
    assert_eq!(status_of(&r, "wikipedia"), &ProviderStatus::Timeout);
    assert!(r.hits.is_empty());
}
