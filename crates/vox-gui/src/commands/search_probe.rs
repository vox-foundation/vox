use serde::{Deserialize, Serialize};
use vox_search::duckduckgo::DuckDuckGoClient;
use vox_search::policy::SearchPolicy;
use vox_search::searxng::SearxngSearchClient;
use vox_search::tavily::TavilySearchClient;
use vox_search::wikipedia::WikipediaClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProbeResult {
    pub provider: String,
    pub http_status: u16,
    pub latency_ms: u64,
    pub success: bool,
    pub hit_count: usize,
    pub sample_titles: Vec<String>,
    pub error_message: Option<String>,
    pub remediation_tip: Option<String>,
}

#[tauri::command]
pub async fn probe_search_provider(
    provider: String,
    query: String,
) -> Result<ProviderProbeResult, String> {
    let start = std::time::Instant::now();
    let q = query.trim();
    if q.is_empty() {
        return Err("Query cannot be empty".into());
    }
    if q.len() > 1000 {
        return Err("Query exceeds maximum allowed length of 1000 characters".into());
    }

    match provider.trim().to_lowercase().as_str() {
        "searxng" => {
            let policy = SearchPolicy::default();
            let base_url = match &policy.searxng_url {
                Some(url) if !url.trim().is_empty() => url.clone(),
                _ => {
                    return Ok(ProviderProbeResult {
                        provider,
                        http_status: 0,
                        latency_ms: 0,
                        success: false,
                        hit_count: 0,
                        sample_titles: vec![],
                        error_message: Some("SearXNG URL is not configured".into()),
                        remediation_tip: Some(
                            "Set VOX_SEARCH_SEARXNG_URL in environment or settings".into(),
                        ),
                    });
                }
            };
            let client = SearxngSearchClient::new(base_url);
            match client
                .search(
                    q,
                    3,
                    policy.searxng_engines_csv(),
                    policy.searxng_language_tag(),
                )
                .await
            {
                Ok(hits) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 200,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    hit_count: hits.len(),
                    sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                    error_message: None,
                    remediation_tip: None,
                }),
                Err(e) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 502,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some(e.to_string()),
                    remediation_tip: Some(
                        "Check if the SearXNG instance is running and reachable".into(),
                    ),
                }),
            }
        }
        "tavily" => {
            let client = match TavilySearchClient::from_env() {
                Some(c) => c,
                None => {
                    return Ok(ProviderProbeResult {
                        provider,
                        http_status: 0,
                        latency_ms: 0,
                        success: false,
                        hit_count: 0,
                        sample_titles: vec![],
                        error_message: Some("TAVILY_API_KEY is unset".into()),
                        remediation_tip: Some(
                            "Configure Tavily API key in settings or secrets".into(),
                        ),
                    });
                }
            };
            match client.search(q, 3, "basic").await {
                Ok(hits) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 200,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    hit_count: hits.len(),
                    sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                    error_message: None,
                    remediation_tip: None,
                }),
                Err(e) => Ok(ProviderProbeResult {
                    provider,
                    http_status: 500,
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some(e),
                    remediation_tip: Some("Verify your Tavily API quota and key validity".into()),
                }),
            }
        }
        "duckduckgo" => match DuckDuckGoClient::search(q, 3).await {
            Ok(hits) => Ok(ProviderProbeResult {
                provider,
                http_status: 200,
                latency_ms: start.elapsed().as_millis() as u64,
                success: true,
                hit_count: hits.len(),
                sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                error_message: if hits.is_empty() {
                    Some("Instant Answer returned 0 topics".into())
                } else {
                    None
                },
                remediation_tip: if hits.is_empty() {
                    Some("DDG Instant Answer only matches entity terms; general web requires SearXNG or Tavily".into())
                } else {
                    None
                },
            }),
            Err(e) => Ok(ProviderProbeResult {
                provider,
                http_status: 500,
                latency_ms: start.elapsed().as_millis() as u64,
                success: false,
                hit_count: 0,
                sample_titles: vec![],
                error_message: Some(e.to_string()),
                remediation_tip: None,
            }),
        },
        "wikipedia" => match WikipediaClient::search(q, 3).await {
            Ok(hits) => Ok(ProviderProbeResult {
                provider,
                http_status: 200,
                latency_ms: start.elapsed().as_millis() as u64,
                success: true,
                hit_count: hits.len(),
                sample_titles: hits.iter().map(|h| h.title.clone()).collect(),
                error_message: None,
                remediation_tip: None,
            }),
            Err(e) => Ok(ProviderProbeResult {
                provider,
                http_status: 500,
                latency_ms: start.elapsed().as_millis() as u64,
                success: false,
                hit_count: 0,
                sample_titles: vec![],
                error_message: Some(e.to_string()),
                remediation_tip: None,
            }),
        },
        other => Err(format!("Unknown provider: {other}")),
    }
}

#[tauri::command]
pub async fn probe_all_search_providers(query: String) -> Result<Vec<ProviderProbeResult>, String> {
    let (searxng, tavily, duckduckgo, wikipedia) = tokio::join!(
        probe_search_provider("searxng".to_string(), query.clone()),
        probe_search_provider("tavily".to_string(), query.clone()),
        probe_search_provider("duckduckgo".to_string(), query.clone()),
        probe_search_provider("wikipedia".to_string(), query),
    );
    Ok(vec![searxng?, tavily?, duckduckgo?, wikipedia?])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_query_rejected() {
        let res = probe_search_provider("duckduckgo".to_string(), " ".to_string()).await;
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), "Query cannot be empty");
    }

    #[tokio::test]
    async fn test_overlong_query_rejected() {
        let long_query = "a".repeat(1001);
        let res = probe_search_provider("duckduckgo".to_string(), long_query).await;
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            "Query exceeds maximum allowed length of 1000 characters"
        );
    }

    #[tokio::test]
    async fn test_unknown_provider_rejected() {
        let res = probe_search_provider("unknown_provider".to_string(), "test".to_string()).await;
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Unknown provider"));
    }

    #[tokio::test]
    async fn test_whitespace_padded_provider_accepted() {
        let res = probe_search_provider("  duckduckgo  ".to_string(), "test".to_string()).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_unconfigured_searxng_returns_helpful_remediation() {
        let prev = std::env::var("VOX_SEARCH_SEARXNG_URL").ok();
        unsafe {
            std::env::remove_var("VOX_SEARCH_SEARXNG_URL");
        }
        let res = probe_search_provider("searxng".to_string(), "rust".to_string()).await;
        if let Some(val) = prev {
            unsafe {
                std::env::set_var("VOX_SEARCH_SEARXNG_URL", val);
            }
        }
        assert!(res.is_ok());
        let probe = res.unwrap();
        assert_eq!(probe.provider, "searxng");
        assert_eq!(probe.http_status, 0);
        assert!(!probe.success);
        assert!(
            probe
                .error_message
                .unwrap()
                .contains("SearXNG URL is not configured")
        );
        assert!(
            probe
                .remediation_tip
                .unwrap()
                .contains("VOX_SEARCH_SEARXNG_URL")
        );
    }

    #[tokio::test]
    async fn test_unconfigured_tavily_returns_helpful_remediation() {
        let prev = std::env::var("TAVILY_API_KEY").ok();
        unsafe {
            std::env::remove_var("TAVILY_API_KEY");
        }
        let res = probe_search_provider("tavily".to_string(), "rust".to_string()).await;
        if let Some(val) = prev {
            unsafe {
                std::env::set_var("TAVILY_API_KEY", val);
            }
        }
        assert!(res.is_ok());
        let probe = res.unwrap();
        assert_eq!(probe.provider, "tavily");
        assert_eq!(probe.http_status, 0);
        assert!(!probe.success);
        assert!(
            probe
                .error_message
                .unwrap()
                .contains("TAVILY_API_KEY is unset")
        );
        assert!(probe.remediation_tip.unwrap().contains("Tavily API key"));
    }

    #[tokio::test]
    async fn test_probe_all_search_providers_returns_all_results() {
        let res = probe_all_search_providers("Rust".to_string()).await;
        assert!(res.is_ok());
        let probes = res.unwrap();
        assert_eq!(probes.len(), 4);
        let providers: Vec<_> = probes.iter().map(|p| p.provider.as_str()).collect();
        assert_eq!(
            providers,
            vec!["searxng", "tavily", "duckduckgo", "wikipedia"]
        );
    }
}
