use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vox_search::arxiv::ArXivClient;
use vox_search::openalex::OpenAlexClient;
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::searxng::SearxngSearchClient;
use vox_search::tavily::TavilyClient;
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

/// This month's persisted Tavily spend (`provider_quota_usage`, written by the
/// daemon). Read from the GUI's own DB handle: the daemon's budget-db OnceLock
/// (`vox_search::tavily_budget::get_budget_db`) is never set in this process.
async fn tavily_quota_from_db(db: Option<&vox_db::VoxDb>) -> Option<QuotaUsageDto> {
    let period = vox_db::store::ops_quota::current_period_key();
    let usage = match db?.get_quota_usage("tavily", &period).await {
        Ok(usage) => usage?,
        Err(e) => {
            tracing::warn!(error = %e, "tavily quota read from provider_quota_usage failed");
            return None;
        }
    };
    Some(QuotaUsageDto {
        units_spent: usage.units_spent,
        units_limit: usage.units_limit,
        last_synced_at: usage.last_synced_at,
    })
}

#[tauri::command]
pub async fn get_research_engine_status() -> Result<ResearchEngineStatusDto, String> {
    let mut policy = SearchPolicy::from_env();

    // Check Vox.toml first for search overrides
    let mut configured_providers: Option<Vec<String>> = None;
    let current_dir = std::env::current_dir().unwrap_or_default();
    if let Ok((manifest, _)) = vox_package_types::VoxManifest::discover(&current_dir) {
        if let Some(search_table) = manifest.search {
            if let Some(lane) = search_table
                .get("active_lane")
                .or_else(|| search_table.get("default_lane"))
                .and_then(|v| v.as_str())
            {
                if lane.eq_ignore_ascii_case("deep") {
                    policy.default_lane = ResearchLane::Deep;
                } else if lane.eq_ignore_ascii_case("fast") {
                    policy.default_lane = ResearchLane::Fast;
                }
            }
            if let Some(ft) = search_table
                .get("fast_timeout_ms")
                .and_then(|v| v.as_integer())
            {
                if ft > 0 {
                    policy.fast_timeout_ms = ft as u64;
                }
            }
            if let Some(dt) = search_table
                .get("deep_timeout_ms")
                .and_then(|v| v.as_integer())
            {
                if dt > 0 {
                    policy.deep_timeout_ms = dt as u64;
                }
            }
            if let Some(arr) = search_table
                .get("enabled_providers")
                .and_then(|v| v.as_array())
            {
                let list: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect();
                if !list.is_empty() {
                    configured_providers = Some(list);
                }
            }
        }
    }

    // If not in Vox.toml, check vox_db user preferences
    let db =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::Runtime, true).await;
    if let Some(db) = &db {
        if let Ok(Some(lane)) = db
            .get_user_preference("local_user", "research.active_lane")
            .await
        {
            if lane.eq_ignore_ascii_case("deep") {
                policy.default_lane = ResearchLane::Deep;
            } else if lane.eq_ignore_ascii_case("fast") {
                policy.default_lane = ResearchLane::Fast;
            }
        }
        if let Ok(Some(ft_str)) = db
            .get_user_preference("local_user", "research.fast_timeout_ms")
            .await
        {
            if let Ok(ft) = ft_str.parse::<u64>() {
                if ft > 0 {
                    policy.fast_timeout_ms = ft;
                }
            }
        }
        if let Ok(Some(dt_str)) = db
            .get_user_preference("local_user", "research.deep_timeout_ms")
            .await
        {
            if let Ok(dt) = dt_str.parse::<u64>() {
                if dt > 0 {
                    policy.deep_timeout_ms = dt;
                }
            }
        }
        if configured_providers.is_none() {
            if let Ok(Some(prov_json)) = db
                .get_user_preference("local_user", "research.enabled_providers")
                .await
            {
                if let Ok(list) = serde_json::from_str::<Vec<String>>(&prov_json) {
                    if !list.is_empty() {
                        configured_providers = Some(list);
                    }
                }
            }
        }
    }

    let is_provider_enabled = |id: &str, default_val: bool| -> bool {
        if let Some(list) = &configured_providers {
            list.iter().any(|item| item.eq_ignore_ascii_case(id))
        } else {
            default_val
        }
    };

    let tavily_quota = tavily_quota_from_db(db.as_ref()).await;

    let tavily_has_key = resolve_secret(SecretId::TavilyApiKey).is_present();
    let openalex_has_key = resolve_secret(SecretId::VoxOpenAlexEmail).is_present();
    let arxiv_has_key = resolve_secret(SecretId::VoxArxivAccessToken).is_present();

    let providers = vec![
        ProviderStatusDto {
            id: "wikipedia".to_string(),
            name: "Wikipedia".to_string(),
            is_keyless: true,
            is_enabled: is_provider_enabled("wikipedia", policy.enable_wikipedia),
            has_key: false,
            quota_usage: None,
        },
        ProviderStatusDto {
            id: "openalex".to_string(),
            name: "OpenAlex".to_string(),
            is_keyless: true,
            is_enabled: is_provider_enabled("openalex", policy.enable_openalex),
            has_key: openalex_has_key,
            quota_usage: None,
        },
        ProviderStatusDto {
            id: "arxiv".to_string(),
            name: "arXiv".to_string(),
            is_keyless: true,
            is_enabled: is_provider_enabled("arxiv", policy.enable_arxiv),
            has_key: arxiv_has_key,
            quota_usage: None,
        },
        ProviderStatusDto {
            id: "tavily".to_string(),
            name: "Tavily Search".to_string(),
            is_keyless: false,
            is_enabled: is_provider_enabled("tavily", policy.tavily_enabled),
            has_key: tavily_has_key,
            quota_usage: tavily_quota,
        },
        ProviderStatusDto {
            id: "searxng".to_string(),
            name: "SearXNG".to_string(),
            is_keyless: true,
            is_enabled: is_provider_enabled("searxng", policy.searxng_url.is_some()),
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
    if let Some(keys) = &config.provider_api_keys {
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

    // Persist active_lane, timeouts, and enabled_providers to Vox.toml
    let current_dir = std::env::current_dir().unwrap_or_default();
    if let Ok((_manifest, path)) = vox_package_types::VoxManifest::discover(&current_dir) {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(mut doc) = content.parse::<toml::Table>() {
                let mut search_table = doc
                    .remove("search")
                    .and_then(|v| match v {
                        toml::Value::Table(t) => Some(t),
                        _ => None,
                    })
                    .unwrap_or_default();

                if let Some(lane) = &config.active_lane {
                    search_table
                        .insert("active_lane".to_string(), toml::Value::String(lane.clone()));
                }
                if let Some(ft) = config.fast_timeout_ms {
                    search_table.insert(
                        "fast_timeout_ms".to_string(),
                        toml::Value::Integer(ft as i64),
                    );
                }
                if let Some(dt) = config.deep_timeout_ms {
                    search_table.insert(
                        "deep_timeout_ms".to_string(),
                        toml::Value::Integer(dt as i64),
                    );
                }
                if let Some(providers) = &config.enabled_providers {
                    let arr = providers
                        .iter()
                        .map(|p| toml::Value::String(p.clone()))
                        .collect();
                    search_table.insert("enabled_providers".to_string(), toml::Value::Array(arr));
                }
                doc.insert("search".to_string(), toml::Value::Table(search_table));
                if let Ok(toml_str) = toml::to_string_pretty(&doc) {
                    let _ = std::fs::write(&path, toml_str);
                }
            }
        }
    }

    // Also persist into user preferences in vox_db for runtime resilience
    if let Some(db) =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::Runtime, true).await
    {
        if let Some(lane) = &config.active_lane {
            let _ = db
                .set_user_preference("local_user", "research.active_lane", lane)
                .await;
        }
        if let Some(ft) = config.fast_timeout_ms {
            let _ = db
                .set_user_preference("local_user", "research.fast_timeout_ms", &ft.to_string())
                .await;
        }
        if let Some(dt) = config.deep_timeout_ms {
            let _ = db
                .set_user_preference("local_user", "research.deep_timeout_ms", &dt.to_string())
                .await;
        }
        if let Some(providers) = &config.enabled_providers {
            if let Ok(prov_json) = serde_json::to_string(providers) {
                let _ = db
                    .set_user_preference("local_user", "research.enabled_providers", &prov_json)
                    .await;
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
    let tavily = tavily_client_from_vault(&policy).await;
    probe_search_provider_with_policy(provider, query, policy, tavily).await
}

/// The Tavily client the Tauri commands probe with: Clavis key + `tavily_api_url`.
/// Only the commands call this; tests inject a client (or `None`) so a probe test can
/// never spend a real credit, whatever the vault holds.
async fn tavily_client_from_vault(policy: &SearchPolicy) -> Option<TavilyClient> {
    let base = policy.tavily_api_url.clone();
    tokio::task::spawn_blocking(move || TavilyClient::from_env(base.as_deref()))
        .await
        .ok()
        .flatten()
}

/// Probe one provider. Every endpoint comes from `policy`, and the Tavily client is
/// injected (`None` = unkeyed), so nothing here reads secrets.
pub async fn probe_search_provider_with_policy(
    provider: String,
    query: String,
    policy: SearchPolicy,
    tavily: Option<TavilyClient>,
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
            // Same client and endpoint override (`tavily_api_url`) the dispatcher uses, so a
            // green row here means the dispatcher's Tavily leg can really authenticate.
            let Some(client) = tavily else {
                return Ok(ProviderProbeResult {
                    provider,
                    http_status: 0,
                    latency_ms: 0,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some("TAVILY_API_KEY is unset".into()),
                    remediation_tip: Some("Configure Tavily API key in settings or secrets".into()),
                });
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
                    // `tavily_search_status:<code>:<detail>`; 0 when no HTTP status came back.
                    http_status: e
                        .split(':')
                        .nth(1)
                        .and_then(|c| c.parse().ok())
                        .unwrap_or(0),
                    latency_ms: start.elapsed().as_millis() as u64,
                    success: false,
                    hit_count: 0,
                    sample_titles: vec![],
                    error_message: Some(e),
                    remediation_tip: Some("Verify your Tavily API quota and key validity".into()),
                }),
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
        "duckduckgo" => Ok(ProviderProbeResult {
            provider,
            http_status: 0,
            latency_ms: 0,
            success: false,
            hit_count: 0,
            sample_titles: vec![],
            error_message: Some(
                "not_implemented: DuckDuckGo client is a stub (vox-search/src/duckduckgo.rs); not used by research"
                    .into(),
            ),
            remediation_tip: Some("Use SearXNG or Tavily for general web search".into()),
        }),
        other => Err(format!("Unknown provider: {other}")),
    }
}

#[tauri::command]
pub async fn probe_all_search_providers(query: String) -> Result<Vec<ProviderProbeResult>, String> {
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
    let tavily = tavily_client_from_vault(&policy).await;
    probe_all_search_providers_with_policy(query, policy, tavily).await
}

pub async fn probe_all_search_providers_with_policy(
    query: String,
    policy: SearchPolicy,
    tavily: Option<TavilyClient>,
) -> Result<Vec<ProviderProbeResult>, String> {
    let probe = |name: &str, t: Option<TavilyClient>| {
        probe_search_provider_with_policy(name.to_string(), query.clone(), policy.clone(), t)
    };
    let (searxng, tavily, openalex, arxiv, wikipedia) = tokio::join!(
        probe("searxng", None),
        probe("tavily", tavily),
        probe("openalex", None),
        probe("arxiv", None),
        probe("wikipedia", None),
    );
    Ok(vec![searxng?, tavily?, openalex?, arxiv?, wikipedia?])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every endpoint unset and no Tavily client: nothing in these tests reads the vault
    /// or reaches SearXNG / api.tavily.com, whatever the machine has configured.
    fn offline_policy() -> SearchPolicy {
        SearchPolicy {
            searxng_url: None,
            tavily_api_url: None,
            ..SearchPolicy::default()
        }
    }

    /// The daemon persists Tavily spend to `provider_quota_usage`; the GUI process
    /// never sets the daemon's budget-db OnceLock, so the status command must read
    /// the row from the DB connection it opens itself. In-memory DB only.
    #[tokio::test]
    async fn tavily_quota_reads_the_persisted_row_from_the_given_db() {
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .expect("in-memory db");
        assert!(tavily_quota_from_db(None).await.is_none());
        assert!(
            tavily_quota_from_db(Some(&db)).await.is_none(),
            "no row yet"
        );

        let period = vox_db::store::ops_quota::current_period_key();
        db.record_quota_spend("tavily", &period, 7)
            .await
            .expect("spend");
        let q = tavily_quota_from_db(Some(&db))
            .await
            .expect("persisted quota is surfaced");
        assert_eq!((q.units_spent, q.units_limit), (7, 1000));
    }

    #[tokio::test]
    async fn duckduckgo_probe_reports_not_implemented_without_a_network_call() {
        let p = probe("duckduckgo", "rust").await.expect("known provider");
        assert!(!p.success);
        assert_eq!((p.http_status, p.latency_ms, p.hit_count), (0, 0, 0));
        assert!(
            p.error_message
                .as_deref()
                .is_some_and(|m| m.starts_with("not_implemented")),
            "{p:?}"
        );
    }

    async fn probe(provider: &str, query: &str) -> Result<ProviderProbeResult, String> {
        probe_search_provider_with_policy(provider.into(), query.into(), offline_policy(), None)
            .await
    }

    #[tokio::test]
    async fn test_empty_query_rejected() {
        let res = probe("duckduckgo", " ").await;
        assert!(res.is_err());
        assert_eq!(res.err().unwrap(), "Query cannot be empty");
    }

    #[tokio::test]
    async fn test_overlong_query_rejected() {
        let res = probe("duckduckgo", &"a".repeat(1001)).await;
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap(),
            "Query exceeds maximum allowed length of 1000 characters"
        );
    }

    #[tokio::test]
    async fn test_unknown_provider_rejected() {
        let res = probe("unknown_provider", "test").await;
        assert!(res.is_err());
        assert!(res.err().unwrap().contains("Unknown provider"));
    }

    #[tokio::test]
    async fn test_whitespace_padded_provider_accepted() {
        // `searxng` with no URL answers locally, so the padding check needs no network.
        let res = probe("  searxng  ", "test").await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_unconfigured_searxng_returns_helpful_remediation() {
        let probe = probe("searxng", "rust").await.unwrap();
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
        let probe = probe("tavily", "rust").await.unwrap();
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
        // Discard port: the keyless providers fail fast locally instead of reaching the web.
        let dead = "http://127.0.0.1:9".to_string();
        let policy = SearchPolicy {
            wikipedia_api_url: Some(dead.clone()),
            openalex_api_url: Some(dead.clone()),
            arxiv_api_url: Some(dead),
            ..offline_policy()
        };
        let probes = probe_all_search_providers_with_policy("Rust".to_string(), policy, None)
            .await
            .unwrap();
        let providers: Vec<_> = probes.iter().map(|p| p.provider.as_str()).collect();
        assert_eq!(
            providers,
            vec!["searxng", "tavily", "openalex", "arxiv", "wikipedia"]
        );
        let tavily = &probes[1];
        assert!(!tavily.success && tavily.http_status == 0, "{tavily:?}");
    }
}
