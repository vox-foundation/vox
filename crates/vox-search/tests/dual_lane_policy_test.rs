//! Dual-lane policy and ResearchLane serde verification tests.

use vox_search::ResearchLane as ReExportedResearchLane;
use vox_search::policy::{ResearchLane, SearchPolicy};

#[test]
fn test_default_policy_lane_and_sources() {
    let policy = SearchPolicy::default();
    assert_eq!(policy.default_lane, ResearchLane::Fast);
    assert_eq!(policy.fast_timeout_ms, 1500);
    assert_eq!(policy.deep_timeout_ms, 4000);
    assert!(policy.enable_wikipedia);
    assert!(policy.enable_openalex);
    assert!(policy.enable_arxiv);
    assert!(policy.wikipedia_api_url.is_none());
    assert!(policy.openalex_api_url.is_none());
    assert!(policy.arxiv_api_url.is_none());
    assert!(policy.tavily_api_url.is_none());
}

#[test]
fn test_research_lane_serde_snake_case() {
    let lane: ResearchLane = serde_json::from_str("\"fast\"").expect("deserialize fast");
    assert_eq!(lane, ResearchLane::Fast);
    let deep: ResearchLane = serde_json::from_str("\"deep\"").expect("deserialize deep");
    assert_eq!(deep, ResearchLane::Deep);

    assert_eq!(
        serde_json::to_string(&ResearchLane::Fast).unwrap(),
        "\"fast\""
    );
    assert_eq!(
        serde_json::to_string(&ResearchLane::Deep).unwrap(),
        "\"deep\""
    );
}

#[test]
fn test_reexported_research_lane() {
    assert_eq!(ResearchLane::Fast, ReExportedResearchLane::Fast);
}
