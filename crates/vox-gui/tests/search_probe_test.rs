#![allow(unused, dead_code)]

#[path = "../src/commands/mod.rs"]
pub mod commands;
#[path = "../src/config/mod.rs"]
pub mod config;

pub mod vox_gui {
    pub use crate::commands;
}

use vox_gui::commands::search_probe::{
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

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_unconfigured_searxng_returns_remediation() {
    let prev = std::env::var("VOX_SEARCH_SEARXNG_URL").ok();
    unsafe {
        std::env::remove_var("VOX_SEARCH_SEARXNG_URL");
    }
    let result = probe_search_provider("searxng".to_string(), "rust".to_string()).await;
    if let Some(val) = prev {
        unsafe {
            std::env::set_var("VOX_SEARCH_SEARXNG_URL", val);
        }
    }
    assert!(
        result.is_ok(),
        "Unconfigured SearXNG should return Ok(ProviderProbeResult)"
    );
    let probe = result.unwrap();
    assert_eq!(probe.provider, "searxng");
    assert_eq!(probe.http_status, 0);
    assert!(!probe.success);
    assert!(probe.error_message.is_some());
    assert!(
        probe
            .remediation_tip
            .as_deref()
            .unwrap_or("")
            .contains("VOX_SEARCH_SEARXNG_URL")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_probe_unconfigured_tavily_returns_remediation() {
    let prev = std::env::var("TAVILY_API_KEY").ok();
    unsafe {
        std::env::remove_var("TAVILY_API_KEY");
    }
    let result = probe_search_provider("tavily".to_string(), "rust".to_string()).await;
    if let Some(val) = prev {
        unsafe {
            std::env::set_var("TAVILY_API_KEY", val);
        }
    }
    assert!(
        result.is_ok(),
        "Unconfigured Tavily should return Ok(ProviderProbeResult)"
    );
    let probe = result.unwrap();
    assert_eq!(probe.provider, "tavily");
    assert_eq!(probe.http_status, 0);
    assert!(!probe.success);
    assert!(probe.error_message.is_some());
    assert!(
        probe
            .remediation_tip
            .as_deref()
            .unwrap_or("")
            .contains("Tavily API key")
    );
}

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
        let mut policy = vox_search::policy::SearchPolicy::default();
        policy.wikipedia_api_url = Some(format!("{}/w/api.php", self.wikipedia.uri()));
        policy.openalex_api_url = Some(self.openalex.uri());
        policy.arxiv_api_url = Some(format!("{}/api/query", self.arxiv.uri()));
        policy.tavily_api_url = Some(self.tavily.uri());
        policy.searxng_url = Some(self.searxng.uri());
        policy
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
async fn test_get_research_engine_status_payload() {
    let status = vox_gui::commands::search_probe::get_research_engine_status()
        .await
        .unwrap();
    assert!(
        status
            .providers
            .iter()
            .any(|p| p.id == "openalex" && p.is_keyless)
    );
    assert!(
        status
            .free_key_offers
            .iter()
            .any(|o| o.provider_id == "tavily" && o.signup_url.contains("tavily.com"))
    );
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

#[tokio::test(flavor = "multi_thread")]
async fn test_save_and_get_research_engine_config_roundtrip() {
    let current_dir = std::env::current_dir().unwrap_or_default();
    let original_content = vox_package_types::VoxManifest::discover(&current_dir)
        .ok()
        .and_then(|(_, path)| std::fs::read_to_string(&path).ok().map(|c| (path, c)));

    struct VoxTomlCleanup(Option<(std::path::PathBuf, String)>);
    impl Drop for VoxTomlCleanup {
        fn drop(&mut self) {
            if let Some((path, content)) = &self.0 {
                let _ = std::fs::write(path, content);
            }
        }
    }
    let _cleanup = VoxTomlCleanup(original_content);

    let cfg = vox_gui::commands::search_probe::ResearchEngineConfigDto {
        active_lane: Some("deep".into()),
        fast_timeout_ms: Some(2200),
        deep_timeout_ms: Some(5500),
        enabled_providers: Some(vec!["wikipedia".into(), "arxiv".into()]),
        provider_api_keys: None,
    };
    let save_res = vox_gui::commands::search_probe::save_research_engine_config(cfg).await;
    assert!(save_res.is_ok());

    let status = vox_gui::commands::search_probe::get_research_engine_status()
        .await
        .unwrap();
    assert_eq!(status.active_lane, "deep");
    assert_eq!(status.fast_timeout_ms, 2200);
    assert_eq!(status.deep_timeout_ms, 5500);
    let wiki = status
        .providers
        .iter()
        .find(|p| p.id == "wikipedia")
        .unwrap();
    assert!(wiki.is_enabled);
    let tavily = status.providers.iter().find(|p| p.id == "tavily").unwrap();
    assert!(!tavily.is_enabled);
}
