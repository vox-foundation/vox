#![allow(unused, dead_code)]

#[path = "../src/commands/mod.rs"]
pub mod commands;
#[path = "../src/config/mod.rs"]
pub mod config;
#[path = "../src/remote/mod.rs"]
pub mod remote;

pub mod vox_gui {
    pub use crate::commands;
}

use vox_search::probe::{
    probe_all_search_providers, probe_all_search_providers_with_policy, probe_search_provider,
    probe_search_provider_with_policy,
};

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_search_provider_rejects_empty_query() {
    let result = probe_search_provider("duckduckgo".to_string(), "  ".to_string()).await;
    assert!(result.is_err(), "Empty query must return Err");
    assert_eq!(result.err().unwrap(), "Query cannot be empty");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_search_provider_rejects_overlong_query() {
    let long_query = "a".repeat(1001);
    let result = probe_search_provider("duckduckgo".to_string(), long_query).await;
    assert!(result.is_err(), "Overlong query must return Err");
    assert_eq!(
        result.err().unwrap(),
        "Query exceeds maximum allowed length of 1000 characters"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_search_provider_rejects_unknown_provider() {
    let result =
        probe_search_provider("nonexistent_search_engine".to_string(), "test".to_string()).await;
    assert!(result.is_err(), "Unknown provider must return Err");
    assert!(result.err().unwrap().contains("Unknown provider"));
}

// The unconfigured-SearXNG / unconfigured-Tavily remediation paths are covered where the probe
// now lives (`vox_search::probe::tests::test_unconfigured_{searxng,tavily}_returns_helpful_remediation`).
// They were dropped here: clearing the env var does not unconfigure a provider whose value is
// also held in the Clavis vault, so on such a machine they probed the live endpoint and failed.

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

struct MockSearchCluster {
    wikipedia: MockServer,
    openalex: MockServer,
    arxiv: MockServer,
    tavily: MockServer,
    searxng: MockServer,
}

impl MockSearchCluster {
    async fn start() -> Self {
        let wikipedia = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w/api.php"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "query": {
                    "search": [
                        {
                            "title": "Rust (programming language)",
                            "pageid": 12345,
                            "snippet": "Rust is a multi-paradigm, general-purpose programming language."
                        }
                    ]
                }
            })))
            .mount(&wikipedia)
            .await;

        let openalex = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/works"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "id": "https://openalex.org/W123",
                        "title": "Rust Memory Safety Analysis",
                        "abstract_inverted_index": { "Rust": [0], "safety": [1] },
                        "primary_location": { "landing_page_url": "https://example.com/rust-safety" }
                    }
                ]
            })))
            .mount(&openalex)
            .await;

        let arxiv = MockServer::start().await;
        let atom_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title type="html">arXiv Query</title>
  <entry>
    <id>http://arxiv.org/abs/2301.00001v1</id>
    <title>Rust Type System</title>
    <summary>A study of affine types in Rust.</summary>
    <link href="http://arxiv.org/abs/2301.00001v1" rel="alternate"/>
  </entry>
</feed>"#;
        Mock::given(method("GET"))
            .and(path("/api/query"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(atom_xml, "application/atom+xml"))
            .mount(&arxiv)
            .await;

        let tavily = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "title": "Rust Documentation",
                        "url": "https://rust-lang.org",
                        "content": "Official Rust lang docs."
                    }
                ]
            })))
            .mount(&tavily)
            .await;

        let searxng = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [
                    {
                        "title": "SearXNG Rust",
                        "url": "https://searx.example/rust"
                    }
                ]
            })))
            .mount(&searxng)
            .await;

        Self {
            wikipedia,
            openalex,
            arxiv,
            tavily,
            searxng,
        }
    }

    fn policy(&self) -> vox_search::policy::SearchPolicy {
        vox_search::policy::SearchPolicy {
            wikipedia_api_url: Some(format!("{}/w/api.php", self.wikipedia.uri())),
            openalex_api_url: Some(self.openalex.uri()),
            arxiv_api_url: Some(format!("{}/api/query", self.arxiv.uri())),
            tavily_api_url: Some(self.tavily.uri()),
            searxng_url: Some(self.searxng.uri()),
            ..Default::default()
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_wikipedia_search_returns_result_shape() {
    let cluster = MockSearchCluster::start().await;
    let result = probe_search_provider_with_policy(
        "wikipedia".to_string(),
        "Rust programming".to_string(),
        cluster.policy(),
    )
    .await;
    assert!(
        result.is_ok(),
        "Probe wikipedia should return Ok(ProviderProbeResult)"
    );
    let probe = result.unwrap();
    assert_eq!(probe.provider, "wikipedia");
    assert_eq!(probe.http_status, 200);
    assert!(probe.success);
    assert!(probe.hit_count > 0);
    assert_eq!(probe.sample_titles[0], "Rust (programming language)");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_all_search_providers_returns_batch_results() {
    let cluster = MockSearchCluster::start().await;
    let result =
        probe_all_search_providers_with_policy("Rust language".to_string(), cluster.policy()).await;
    assert!(
        result.is_ok(),
        "Probe all should succeed with Ok results vector"
    );
    let probes = result.unwrap();
    assert_eq!(probes.len(), 5);
    let names: Vec<_> = probes.iter().map(|p| p.provider.as_str()).collect();
    assert_eq!(
        names,
        vec!["searxng", "tavily", "openalex", "arxiv", "wikipedia"]
    );
    let wiki = probes.iter().find(|p| p.provider == "wikipedia").unwrap();
    assert!(wiki.success, "wiki failed: {:?}", wiki.error_message);
    let oa = probes.iter().find(|p| p.provider == "openalex").unwrap();
    assert!(oa.success, "oa failed: {:?}", oa.error_message);
    let ax = probes.iter().find(|p| p.provider == "arxiv").unwrap();
    assert!(ax.success, "ax failed: {:?}", ax.error_message);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_openalex_and_arxiv_accepted() {
    let cluster = MockSearchCluster::start().await;
    let res_oa =
        probe_search_provider_with_policy("openalex".into(), "rust".into(), cluster.policy()).await;
    assert!(res_oa.is_ok());
    let probe_oa = res_oa.unwrap();
    assert!(
        probe_oa.success,
        "openalex failed: {:?}",
        probe_oa.error_message
    );
    assert_eq!(probe_oa.http_status, 200);

    let res_ax =
        probe_search_provider_with_policy("arxiv".into(), "rust".into(), cluster.policy()).await;
    assert!(res_ax.is_ok());
    let probe_ax = res_ax.unwrap();
    assert!(
        probe_ax.success,
        "arxiv failed: {:?}",
        probe_ax.error_message
    );
    assert_eq!(probe_ax.http_status, 200);
}
