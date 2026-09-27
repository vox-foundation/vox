use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_secrets::{FreeTierOffer, SecretId, list_free_tier_offers, resolve_secret, store_secret};

// Re-exported for `vox_gui` integration tests (`tests/search_probe_test.rs`); the bin target never calls them.
#[allow(unused_imports)]
pub use vox_search::probe::{
    ProviderProbeResult, probe_all_search_providers_with_policy, probe_search_provider_with_policy,
};

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
    let mut policy = SearchPolicy::from_env();

    // Check Vox.toml first for search overrides
    let mut configured_providers: Option<Vec<String>> = None;
    let current_dir = std::env::current_dir().unwrap_or_default();
    if let Ok((manifest, _)) = vox_package_types::VoxManifest::discover(&current_dir)
        && let Some(search_table) = manifest.search
    {
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
            && ft > 0
        {
            policy.fast_timeout_ms = ft as u64;
        }
        if let Some(dt) = search_table
            .get("deep_timeout_ms")
            .and_then(|v| v.as_integer())
            && dt > 0
        {
            policy.deep_timeout_ms = dt as u64;
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

    // If not in Vox.toml, check vox_db user preferences
    if let Some(db) =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::Runtime, true).await
    {
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
            && let Ok(ft) = ft_str.parse::<u64>()
            && ft > 0
        {
            policy.fast_timeout_ms = ft;
        }
        if let Ok(Some(dt_str)) = db
            .get_user_preference("local_user", "research.deep_timeout_ms")
            .await
            && let Ok(dt) = dt_str.parse::<u64>()
            && dt > 0
        {
            policy.deep_timeout_ms = dt;
        }
        if configured_providers.is_none()
            && let Ok(Some(prov_json)) = db
                .get_user_preference("local_user", "research.enabled_providers")
                .await
            && let Ok(list) = serde_json::from_str::<Vec<String>>(&prov_json)
            && !list.is_empty()
        {
            configured_providers = Some(list);
        }
    }

    let is_provider_enabled = |id: &str, default_val: bool| -> bool {
        if let Some(list) = &configured_providers {
            list.iter().any(|item| item.eq_ignore_ascii_case(id))
        } else {
            default_val
        }
    };

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
    if let Ok((_manifest, path)) = vox_package_types::VoxManifest::discover(&current_dir)
        && let Ok(content) = std::fs::read_to_string(&path)
        && let Ok(mut doc) = content.parse::<toml::Table>()
    {
        let mut search_table = doc
            .remove("search")
            .and_then(|v| match v {
                toml::Value::Table(t) => Some(t),
                _ => None,
            })
            .unwrap_or_default();

        if let Some(lane) = &config.active_lane {
            search_table.insert("active_lane".to_string(), toml::Value::String(lane.clone()));
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
        if let Some(providers) = &config.enabled_providers
            && let Ok(prov_json) = serde_json::to_string(providers)
        {
            let _ = db
                .set_user_preference("local_user", "research.enabled_providers", &prov_json)
                .await;
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn probe_search_provider(
    provider: String,
    query: String,
) -> Result<ProviderProbeResult, String> {
    vox_search::probe::probe_search_provider(provider, query).await
}

#[tauri::command]
pub async fn probe_all_search_providers(query: String) -> Result<Vec<ProviderProbeResult>, String> {
    vox_search::probe::probe_all_search_providers(query).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn probe_commands_delegate_input_validation_to_vox_search() {
        let one = probe_search_provider("wikipedia".into(), "  ".into()).await;
        assert_eq!(one.err().as_deref(), Some("Query cannot be empty"));
        let all = probe_all_search_providers("a".repeat(1001)).await;
        assert_eq!(
            all.err().as_deref(),
            Some("Query exceeds maximum allowed length of 1000 characters")
        );
    }

    #[test]
    fn engine_config_accepts_camel_case_from_the_ui() {
        let cfg: ResearchEngineConfigDto = serde_json::from_str(
            r#"{"activeLane":"deep","fastTimeoutMs":2200,"enabledProviders":["arxiv"]}"#,
        )
        .unwrap();
        assert_eq!(cfg.active_lane.as_deref(), Some("deep"));
        assert_eq!(cfg.fast_timeout_ms, Some(2200));
        assert_eq!(cfg.deep_timeout_ms, None);
        assert_eq!(cfg.enabled_providers, Some(vec!["arxiv".to_string()]));
    }
}
