#![allow(unused, dead_code)]

#[path = "../src/commands/mod.rs"]
pub mod commands;
#[path = "../src/config/mod.rs"]
pub mod config;

pub mod vox_gui {
    pub use crate::commands;
}

use vox_gui::commands::search_probe::{probe_all_search_providers, probe_search_provider};

#[tokio::test]
async fn test_probe_search_provider_rejects_empty_query() {
    let result = probe_search_provider("duckduckgo".to_string(), "  ".to_string()).await;
    assert!(result.is_err(), "Empty query must return Err");
    assert_eq!(result.err().unwrap(), "Query cannot be empty");
}

#[tokio::test]
async fn test_probe_search_provider_rejects_overlong_query() {
    let long_query = "a".repeat(1001);
    let result = probe_search_provider("duckduckgo".to_string(), long_query).await;
    assert!(result.is_err(), "Overlong query must return Err");
    assert_eq!(
        result.err().unwrap(),
        "Query exceeds maximum allowed length of 1000 characters"
    );
}

#[tokio::test]
async fn test_probe_search_provider_rejects_unknown_provider() {
    let result =
        probe_search_provider("nonexistent_search_engine".to_string(), "test".to_string()).await;
    assert!(result.is_err(), "Unknown provider must return Err");
    assert!(result.err().unwrap().contains("Unknown provider"));
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
async fn test_probe_wikipedia_search_returns_result_shape() {
    let result =
        probe_search_provider("wikipedia".to_string(), "Rust programming".to_string()).await;
    assert!(
        result.is_ok(),
        "Probe wikipedia should return Ok(ProviderProbeResult)"
    );
    let probe = result.unwrap();
    assert_eq!(probe.provider, "wikipedia");
    if probe.success {
        assert_eq!(probe.http_status, 200);
        assert!(probe.hit_count > 0);
    } else {
        assert_eq!(probe.http_status, 500);
        assert!(probe.error_message.is_some());
    }
}

#[tokio::test]
async fn test_probe_all_search_providers_returns_batch_results() {
    let result = probe_all_search_providers("Rust language".to_string()).await;
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
}

#[tokio::test]
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

#[tokio::test]
async fn test_probe_openalex_and_arxiv_accepted() {
    let res_oa =
        vox_gui::commands::search_probe::probe_search_provider("openalex".into(), "rust".into())
            .await;
    assert!(res_oa.is_ok());
    let res_ax =
        vox_gui::commands::search_probe::probe_search_provider("arxiv".into(), "rust".into()).await;
    assert!(res_ax.is_ok());
}
