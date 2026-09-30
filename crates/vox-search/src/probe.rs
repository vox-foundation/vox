//! Live provider probes shared by the GUI engine drawer and `vox research probe`.

use serde::{Deserialize, Serialize};

use crate::arxiv::ArXivClient;
#[cfg(feature = "web-scrape")]
use crate::duckduckgo::DuckDuckGoClient;
use crate::openalex::OpenAlexClient;
use crate::policy::SearchPolicy;
use crate::searxng::SearxngSearchClient;
#[cfg(feature = "tavily")]
use crate::tavily::TavilyClient;
use crate::wikipedia::WikipediaClient;
use vox_secrets::{SecretId, resolve_secret};

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

/// `SearchPolicy::from_env()` plus the `VOX_SEARCH_*_URL` endpoint overrides (test / mirror hooks).
fn policy_with_env_url_overrides() -> SearchPolicy {
    let mut policy = SearchPolicy::from_env();
    let overrides: [(&str, &mut Option<String>); 4] = [
        ("VOX_SEARCH_WIKIPEDIA_URL", &mut policy.wikipedia_api_url),
        ("VOX_SEARCH_OPENALEX_URL", &mut policy.openalex_api_url),
        ("VOX_SEARCH_ARXIV_URL", &mut policy.arxiv_api_url),
        ("VOX_SEARCH_TAVILY_URL", &mut policy.tavily_api_url),
    ];
    for (var, slot) in overrides {
        if let Ok(u) = std::env::var(var)
            && !u.trim().is_empty()
        {
            *slot = Some(u);
        }
    }
    policy
}

/// Probe one provider (`searxng`, `tavily`, `openalex`, `arxiv`, `wikipedia`, `duckduckgo`) with the env-derived policy.
pub async fn probe_search_provider(
    provider: String,
    query: String,
) -> Result<ProviderProbeResult, String> {
    probe_search_provider_with_policy(provider, query, policy_with_env_url_overrides()).await
}

pub async fn probe_search_provider_with_policy(
    provider: String,
    query: String,
    policy: SearchPolicy,
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
            if let Some(api_url) = &policy.tavily_api_url {
                let client = vox_http_client::client();
                let key = resolve_secret(SecretId::TavilyApiKey)
                    .expose()
                    .unwrap_or("test-key")
                    .to_string();
                let payload = serde_json::json!({
                    "api_key": key,
                    "query": q,
                    "max_results": 3,
                    "search_depth": "basic",
                });
                match client.post(api_url).json(&payload).send().await {
                    Ok(resp) => {
                        let status_code = resp.status().as_u16();
                        if resp.status().is_success() {
                            let val: serde_json::Value = resp.json().await.unwrap_or_default();
                            let titles: Vec<String> = val
                                .get("results")
                                .and_then(|r| r.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|x| {
                                            x.get("title")
                                                .and_then(|t| t.as_str())
                                                .map(ToString::to_string)
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            Ok(ProviderProbeResult {
                                provider,
                                http_status: status_code,
                                latency_ms: start.elapsed().as_millis() as u64,
                                success: true,
                                hit_count: titles.len(),
                                sample_titles: titles,
                                error_message: None,
                                remediation_tip: None,
                            })
                        } else {
                            Ok(ProviderProbeResult {
                                provider,
                                http_status: status_code,
                                latency_ms: start.elapsed().as_millis() as u64,
                                success: false,
                                hit_count: 0,
                                sample_titles: vec![],
                                error_message: Some(format!(
                                    "Tavily API returned status {}",
                                    status_code
                                )),
                                remediation_tip: Some(
                                    "Verify your Tavily API quota and key validity".into(),
                                ),
                            })
                        }
                    }
                    Err(e) => Ok(ProviderProbeResult {
                        provider,
                        http_status: 500,
                        latency_ms: start.elapsed().as_millis() as u64,
                        success: false,
                        hit_count: 0,
                        sample_titles: vec![],
                        error_message: Some(e.to_string()),
                        remediation_tip: Some(
                            "Verify your Tavily API quota and key validity".into(),
                        ),
                    }),
                }
            } else {
                let client = match TavilyClient::from_env(policy.tavily_api_url.as_deref()) {
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
                        remediation_tip: Some(
                            "Verify your Tavily API quota and key validity".into(),
                        ),
                    }),
                }
            }
        }
        "openalex" => {
            match OpenAlexClient::search(q, 3, policy.openalex_api_url.as_deref(), None).await {
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
                    remediation_tip: Some("Verify OpenAlex API endpoint accessibility".into()),
                }),
            }
        }
        "arxiv" => match ArXivClient::search(q, 3, policy.arxiv_api_url.as_deref()).await {
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
                remediation_tip: Some("Verify arXiv API endpoint accessibility".into()),
            }),
        },
        "wikipedia" => {
            match WikipediaClient::search(q, 3, policy.wikipedia_api_url.as_deref()).await {
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
            }
        }
        #[cfg(feature = "web-scrape")]
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
        #[cfg(not(feature = "web-scrape"))]
        "duckduckgo" => {
            Err("duckduckgo probing requires the `web-scrape` feature of vox-search".into())
        }
        other => Err(format!("Unknown provider: {other}")),
    }
}

/// Probe every keyed/optional provider concurrently with the env-derived policy.
pub async fn probe_all_search_providers(query: String) -> Result<Vec<ProviderProbeResult>, String> {
    probe_all_search_providers_with_policy(query, policy_with_env_url_overrides()).await
}

pub async fn probe_all_search_providers_with_policy(
    query: String,
    policy: SearchPolicy,
) -> Result<Vec<ProviderProbeResult>, String> {
    let (searxng, tavily, openalex, arxiv, wikipedia) = tokio::join!(
        probe_search_provider_with_policy("searxng".to_string(), query.clone(), policy.clone()),
        probe_search_provider_with_policy("tavily".to_string(), query.clone(), policy.clone()),
        probe_search_provider_with_policy("openalex".to_string(), query.clone(), policy.clone()),
        probe_search_provider_with_policy("arxiv".to_string(), query.clone(), policy.clone()),
        probe_search_provider_with_policy("wikipedia".to_string(), query, policy),
    );
    Ok(vec![searxng?, tavily?, openalex?, arxiv?, wikipedia?])
}

#[cfg(test)]
mod tests {
    // Two tests mutate process env (`unsafe` since edition 2024) to exercise the unconfigured-provider paths.
    #![allow(unsafe_code)]
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

    #[cfg(feature = "web-scrape")]
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
        assert_eq!(probes.len(), 5);
        let providers: Vec<_> = probes.iter().map(|p| p.provider.as_str()).collect();
        assert_eq!(
            providers,
            vec!["searxng", "tavily", "openalex", "arxiv", "wikipedia"]
        );
    }
}
