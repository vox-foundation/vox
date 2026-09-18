#![allow(unused, dead_code)]

#[path = "../src/commands/mod.rs"]
pub mod commands;
#[path = "../src/config/mod.rs"]
pub mod config;

pub mod vox_gui {
    pub use crate::commands;
}

use vox_gui::commands::search_probe::probe_search_provider;

#[tokio::test]
async fn test_probe_search_provider_rejects_empty_query() {
    let result = probe_search_provider("duckduckgo".to_string(), "  ".to_string()).await;
    assert!(result.is_err(), "Empty query must return Err");
    assert_eq!(result.err().unwrap(), "Query cannot be empty");
}

#[tokio::test]
async fn test_probe_search_provider_rejects_unknown_provider() {
    let result =
        probe_search_provider("nonexistent_search_engine".to_string(), "test".to_string()).await;
    assert!(result.is_err(), "Unknown provider must return Err");
    assert!(result.err().unwrap().contains("Unknown provider"));
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
    assert_eq!(probe.http_status, 200);
    assert!(
        probe.hit_count > 0,
        "Wikipedia search for Rust programming should yield hits"
    );
    assert!(!probe.sample_titles.is_empty());
}
