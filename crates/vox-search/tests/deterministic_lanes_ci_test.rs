// crates/vox-search/tests/deterministic_lanes_ci_test.rs
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::searxng::SearxngResult;
use vox_search::web_dispatcher::{WebSearchDispatcher, WebSearchDispatcherExt, true_rrf_fuse};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_fast_lane_deadline_enforcement() {
    let slow_server = MockServer::start().await;
    let fast_server = MockServer::start().await;

    // Slow provider: OpenAlex responds in 2500ms (exceeds 500ms fast timeout)
    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(2500)))
        .mount(&slow_server)
        .await;

    // Fast provider: Wikipedia responds in ~10ms
    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {
                "search": [
                    {
                        "title": "Fast result",
                        "pageid": 1,
                        "snippet": "Wikipedia returned fast"
                    }
                ]
            }
        })))
        .mount(&fast_server)
        .await;

    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 500;
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

    let dispatcher = WebSearchDispatcher::new();
    let hits = dispatcher
        .search_with_lane("test query", ResearchLane::Fast, &policy)
        .await
        .expect("search");

    // Must have results from the fast Wikipedia provider
    assert!(
        !hits.is_empty(),
        "Fast lane must harvest from fast providers within the deadline"
    );
    // Must NOT have results from the slow OpenAlex provider (timed out)
    assert!(
        hits.iter()
            .all(|h| !h.provenance.iter().any(|p| p == "engine:openalex")),
        "OpenAlex exceeded deadline; results must be excluded from Fast lane hits"
    );
    assert!(
        hits.iter()
            .any(|h| h.provenance.iter().any(|p| p == "engine:wikipedia")),
        "Wikipedia fast result must be present in hits"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_true_rrf_multi_source_fusion() {
    let server_oa = MockServer::start().await;
    let server_wiki = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/works"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "id": "https://openalex.org/W1",
                    "title": "Paper 1",
                    "primary_location": {"landing_page_url": "https://example.com/p1"},
                    "abstract_inverted_index": {"test": [0]}
                }
            ]
        })))
        .mount(&server_oa)
        .await;

    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {
                "search": [
                    {
                        "title": "Wiki 1",
                        "pageid": 2,
                        "snippet": "Wikipedia snippet"
                    }
                ]
            }
        })))
        .mount(&server_wiki)
        .await;

    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 4000;
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_arxiv = false;
    policy.openalex_api_url = Some(server_oa.uri());
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", server_wiki.uri()));

    // Warm up OS networking/proxy stack to avoid parallel runner cold-start delays
    let _ = vox_http_client::client()
        .get(format!("{}/works", server_oa.uri()))
        .send()
        .await;
    let _ = vox_http_client::client()
        .get(format!("{}/w/api.php", server_wiki.uri()))
        .send()
        .await;

    let dispatcher = WebSearchDispatcher::new();
    let hits = dispatcher
        .search_with_lane("test", ResearchLane::Fast, &policy)
        .await
        .expect("search");
    assert!(!hits.is_empty());
    assert!(
        hits.iter()
            .any(|h| h.provenance.iter().any(|p| p == "engine:openalex"))
    );
    assert!(
        hits.iter()
            .any(|h| h.provenance.iter().any(|p| p == "engine:wikipedia"))
    );
}

#[test]
fn test_true_rrf_source_weights_and_k_clamping() {
    let arxiv_item = SearxngResult {
        url: "https://arxiv.org/abs/2301.00001".to_string(),
        title: "ArXiv Paper".to_string(),
        content: "ArXiv snippet".to_string(),
        engine: Some("arxiv".to_string()),
        score: None,
    };
    let openalex_item = SearxngResult {
        url: "https://openalex.org/W123".to_string(),
        title: "OpenAlex Paper".to_string(),
        content: "OpenAlex snippet".to_string(),
        engine: Some("openalex".to_string()),
        score: None,
    };
    let wiki_item = SearxngResult {
        url: "https://en.wikipedia.org/wiki/Science".to_string(),
        title: "Wikipedia Science".to_string(),
        content: "Wikipedia snippet".to_string(),
        engine: Some("wikipedia".to_string()),
        score: None,
    };

    let lists = vec![vec![arxiv_item], vec![openalex_item], vec![wiki_item]];

    // With k = 60.0 (default):
    // arXiv rank 1: (1.0 / 61.0) * 1.20 = 1.20 / 61.0 ≈ 0.019672
    // OpenAlex rank 1: (1.0 / 61.0) * 1.10 = 1.10 / 61.0 ≈ 0.018033
    // Wikipedia rank 1: (1.0 / 61.0) * 1.00 = 1.00 / 61.0 ≈ 0.016393
    let fused = true_rrf_fuse(lists.clone(), 60.0);
    assert_eq!(fused.len(), 3);
    assert_eq!(fused[0].engine.as_deref(), Some("arxiv"));
    assert_eq!(fused[1].engine.as_deref(), Some("openalex"));
    assert_eq!(fused[2].engine.as_deref(), Some("wikipedia"));

    let s0 = fused[0].score.unwrap();
    let s1 = fused[1].score.unwrap();
    let s2 = fused[2].score.unwrap();
    assert!(s0 > s1, "arXiv weight (1.20) must outrank OpenAlex (1.10)");
    assert!(
        s1 > s2,
        "OpenAlex weight (1.10) must outrank Wikipedia (1.00)"
    );

    // Test k clamping: negative or zero k must clamp to 1.0
    let fused_clamped = true_rrf_fuse(lists, -10.0);
    // With k clamped to 1.0, rank 1: rank + k = 2.0. arXiv: 1.20 / 2.0 = 0.60
    assert_eq!(fused_clamped[0].engine.as_deref(), Some("arxiv"));
    let clamped_score = fused_clamped[0].score.unwrap();
    assert!((clamped_score - 0.60).abs() < 1e-6);
}

#[test]
fn test_true_rrf_deduplication_score_summing() {
    // Both arXiv and OpenAlex find the same URL
    let item1 = SearxngResult {
        url: "https://arxiv.org/abs/2206.05503".to_string(),
        title: "ArXiv Title".to_string(),
        content: "ArXiv snippet".to_string(),
        engine: Some("arxiv".to_string()),
        score: None,
    };
    let item2 = SearxngResult {
        url: "https://arxiv.org/abs/2206.05503".to_string(),
        title: "OpenAlex Title".to_string(),
        content: "OpenAlex snippet".to_string(),
        engine: Some("openalex".to_string()),
        score: None,
    };

    let lists = vec![vec![item1], vec![item2]];
    let fused = true_rrf_fuse(lists, 60.0);

    // Should dedup into a single hit whose score is the sum of both contributions
    assert_eq!(fused.len(), 1);
    let expected = (1.0 / 61.0) * 1.20 + (1.0 / 61.0) * 1.10;
    assert!((fused[0].score.unwrap() - expected).abs() < 1e-6);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_associated_function_parity() {
    let fast_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/w/api.php"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "query": {
                "search": [
                    {
                        "title": "Fast result",
                        "pageid": 1,
                        "snippet": "Wikipedia returned fast"
                    }
                ]
            }
        })))
        .mount(&fast_server)
        .await;

    let mut policy = SearchPolicy::default();
    policy.fast_timeout_ms = 4000;
    policy.tavily_enabled = false;
    policy.searxng_url = None;
    policy.enable_arxiv = false;
    policy.enable_openalex = false;
    policy.wikipedia_api_url = Some(format!("{}/w/api.php", fast_server.uri()));

    // Warm up OS networking/proxy stack to ensure deterministic timing on macOS
    let _ = vox_http_client::client()
        .get(format!("{}/w/api.php", fast_server.uri()))
        .send()
        .await;

    // Verify associated function call WebSearchDispatcher::search_with_lane(...) without instantiation
    let hits = WebSearchDispatcher::search_with_lane("test query", ResearchLane::Fast, &policy)
        .await
        .expect("search");
    assert!(!hits.is_empty());
}
