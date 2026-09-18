use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vox_search::arxiv::ArXivClient;
use vox_search::duckduckgo::DuckDuckGoClient;
use vox_search::openalex::OpenAlexClient;
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::searxng::SearxngSearchClient;
use vox_search::tavily::TavilySearchClient;
use vox_search::wikipedia::WikipediaClient;
use vox_secrets::{FreeTierOffer, SecretId, list_free_tier_offers, resolve_secret, store_secret};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaUsageDto {
    #[serde(alias = "unitsSpent")]
    pub units_spent: i64,
    #[serde(alias = "unitsLimit")]
    pub units_limit: i64,
    #[serde(alias = "lastSyncedAt")]
    pub last_synced_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusDto {
    pub id: String,
    pub name: String,
    #[serde(alias = "isKeyless")]
    pub is_keyless: bool,
    #[serde(alias = "isEnabled")]
    pub is_enabled: bool,
    #[serde(alias = "hasKey")]
    pub has_key: bool,
    #[serde(alias = "quotaUsage")]
    pub quota_usage: Option<QuotaUsageDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchEngineStatusDto {
    #[serde(alias = "activeLane")]
    pub active_lane: String,
    #[serde(alias = "fastTimeoutMs")]
    pub fast_timeout_ms: u64,
    #[serde(alias = "deepTimeoutMs")]
    pub deep_timeout_ms: u64,
    pub providers: Vec<ProviderStatusDto>,
    #[serde(alias = "freeKeyOffers")]
    pub free_key_offers: Vec<FreeTierOffer>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResearchEngineConfigDto {
    #[serde(alias = "activeLane")]
    pub active_lane: Option<String>,
    #[serde(alias = "fastTimeoutMs")]
    pub fast_timeout_ms: Option<u64>,
    #[serde(alias = "deepTimeoutMs")]
    pub deep_timeout_ms: Option<u64>,
    #[serde(alias = "enabledProviders")]
    pub enabled_providers: Option<Vec<String>>,
    #[serde(alias = "providerApiKeys")]
    pub provider_api_keys: Option<HashMap<String, String>>,
}

#[tauri::command]
pub async fn get_research_engine_status() -> Result<ResearchEngineStatusDto, String> {
    let policy = SearchPolicy::from_env();

    // Check Tavily quota if db handle is available
    let tavily_quota = if let Some(db) = vox_search::tavily_budget::get_budget_db() {
        let period = vox_db::store::ops_quota::current_period_key();
        let conn = db.connection();
        if let Ok(Some(usage)) =
            vox_db::store::ops_quota::get_quota_usage(conn, "tavily", &period).await
        {
            Some(QuotaUsageDto {
                units_spent: usage.units_spent,
                units_limit: usage.units_limit,
                last_synced_at: usage.last_synced_at,
            })
        } else {
            None
        }
    } else {
        None
    };

    let tavily_has_key = resolve_secret(SecretId::TavilyApiKey).is_present();
    let openalex_has_key = resolve_secret(SecretId::VoxOpenAlexEmail).is_present();
    let arxiv_has_key = resolve_secret(SecretId::VoxArxivAccessToken).is_present();

    let providers = vec![
        ProviderStatusDto {
            id: "wikipedia".to_string(),
            name: "Wikipedia".to_string(),
            is_keyless: true,
            is_enabled: policy.enable_wikipedia,
            has_key: false,
            quota_usage: None,
        },
        ProviderStatusDto {
            id: "openalex".to_string(),
            name: "OpenAlex".to_string(),
            is_keyless: true,
            is_enabled: policy.enable_openalex,
            has_key: openalex_has_key,
            quota_usage: None,
        },
        ProviderStatusDto {
            id: "arxiv".to_string(),
            name: "arXiv".to_string(),
            is_keyless: true,
            is_enabled: policy.enable_arxiv,
            has_key: arxiv_has_key,
            quota_usage: None,
        },
        ProviderStatusDto {
            id: "tavily".to_string(),
            name: "Tavily Search".to_string(),
            is_keyless: false,
            is_enabled: policy.tavily_enabled,
            has_key: tavily_has_key,
            quota_usage: tavily_quota,
        },
        ProviderStatusDto {
            id: "searxng".to_string(),
            name: "SearXNG".to_string(),
            is_keyless: true,
            is_enabled: policy.searxng_url.is_some(),
            has_key: false,
            quota_usage: None,
        },
    ];

    let active_lane = match policy.default_lane {
        ResearchLane::Fast => "fast".to_string(),
        ResearchLane::Deep => "deep".to_string(),
    };

    Ok(ResearchEngineStatusDto {
        active_lane,
        fast_timeout_ms: policy.fast_timeout_ms,
        deep_timeout_ms: policy.deep_timeout_ms,
        providers,
        free_key_offers: list_free_tier_offers(),
    })
}

#[tauri::command]
pub async fn save_research_engine_config(config: ResearchEngineConfigDto) -> Result<(), String> {
    if let Some(keys) = config.provider_api_keys {
        for (provider, key_value) in keys {
            let trimmed = key_value.trim();
            if trimmed.is_empty() {
                continue;
            }
            let secret_id = match provider.to_ascii_lowercase().as_str() {
                "tavily" | "tavily_api_key" | "tavilyapikey" => Some(SecretId::TavilyApiKey),
                "openalex" => Some(SecretId::VoxOpenAlexEmail),
                "arxiv" => Some(SecretId::VoxArxivAccessToken),
                "gemini" => Some(SecretId::GeminiApiKey),
                "openrouter" => Some(SecretId::OpenRouterApiKey),
                "semantic_scholar" => Some(SecretId::VoxSemanticScholarApiKey),
                other => vox_secrets::secret_id_for_canonical_env(other),
            };
            if let Some(id) = secret_id {
                store_secret(id, trimmed, None)
                    .map_err(|e| format!("Failed to write secret for {provider}: {e}"))?;
            }
        }
    }
    Ok(())
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

    let mut policy = SearchPolicy::from_env();
    if let Ok(u) = std::env::var("VOX_SEARCH_WIKIPEDIA_URL") {
        if !u.trim().is_empty() {
            policy.wikipedia_api_url = Some(u);
        }
    }
    if let Ok(u) = std::env::var("VOX_SEARCH_OPENALEX_URL") {
        if !u.trim().is_empty() {
            policy.openalex_api_url = Some(u);
        }
    }
    if let Ok(u) = std::env::var("VOX_SEARCH_ARXIV_URL") {
        if !u.trim().is_empty() {
            policy.arxiv_api_url = Some(u);
        }
    }
    if let Ok(u) = std::env::var("VOX_SEARCH_TAVILY_URL") {
        if !u.trim().is_empty() {
            policy.tavily_api_url = Some(u);
        }
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
        other => Err(format!("Unknown provider: {other}")),
    }
}

#[tauri::command]
pub async fn probe_all_search_providers(query: String) -> Result<Vec<ProviderProbeResult>, String> {
    let (searxng, tavily, openalex, arxiv, wikipedia) = tokio::join!(
        probe_search_provider("searxng".to_string(), query.clone()),
        probe_search_provider("tavily".to_string(), query.clone()),
        probe_search_provider("openalex".to_string(), query.clone()),
        probe_search_provider("arxiv".to_string(), query.clone()),
        probe_search_provider("wikipedia".to_string(), query),
    );
    Ok(vec![searxng?, tavily?, openalex?, arxiv?, wikipedia?])
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
        assert_eq!(probes.len(), 5);
        let providers: Vec<_> = probes.iter().map(|p| p.provider.as_str()).collect();
        assert_eq!(
            providers,
            vec!["searxng", "tavily", "openalex", "arxiv", "wikipedia"]
        );
    }
}
