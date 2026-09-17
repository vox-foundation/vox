//! Live keyed-provider catalog fetch + in-memory search (picker autocomplete).
//!
//! OpenRouter exposes only a full `/v1/models` list (no server-side search). We
//! fetch that list live (short TTL cache), merge other keyed catalogs, then
//! filter/rank in process so Axis autocomplete does not depend on the disk
//! `model-catalog.v1.json` cache.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::catalog::{
    AnthropicDirectCatalog, GoogleDirectCatalog, MensCatalog, ModelCatalog, OpenRouterCatalog,
};
use crate::models::key_guard::provider_secret_is_available;
use crate::models::spec::{ModelSpec, ProviderType};

const OPENROUTER_LIVE_TTL: Duration = Duration::from_secs(300);

static OPENROUTER_LIVE_CACHE: Mutex<Option<(Instant, Vec<ModelSpec>)>> = Mutex::new(None);

/// Drop the in-process OpenRouter live cache (tests / forced refresh).
pub fn clear_openrouter_live_cache() {
    if let Ok(mut guard) = OPENROUTER_LIVE_CACHE.lock() {
        *guard = None;
    }
}

/// Live OpenRouter catalog with a short process-local TTL.
pub async fn openrouter_models_cached_live() -> anyhow::Result<Vec<ModelSpec>> {
    if let Ok(guard) = OPENROUTER_LIVE_CACHE.lock() {
        if let Some((fetched_at, models)) = guard.as_ref() {
            if fetched_at.elapsed() < OPENROUTER_LIVE_TTL {
                return Ok(models.clone());
            }
        }
    }

    let models = OpenRouterCatalog::new().refresh().await?;
    if let Ok(mut guard) = OPENROUTER_LIVE_CACHE.lock() {
        *guard = Some((Instant::now(), models.clone()));
    }
    Ok(models)
}

/// Warm the OpenRouter live cache when the OpenRouter key is present (no-op otherwise).
pub async fn warm_openrouter_live_cache_if_keyed() {
    if !provider_secret_is_available(&ProviderType::OpenRouter) {
        return;
    }
    let _ = openrouter_models_cached_live().await;
}

/// Substring match on id / canonical slug / provider (case-insensitive).
#[must_use]
pub fn model_matches_live_query(spec: &ModelSpec, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    spec.id.to_lowercase().contains(&q)
        || spec.canonical_slug.to_lowercase().contains(&q)
        || spec.provider.to_lowercase().contains(&q)
}

fn match_rank(spec: &ModelSpec, query: &str) -> u8 {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 2;
    }
    let id = spec.id.to_lowercase();
    let provider = spec.provider.to_lowercase();
    if id == q {
        return 0;
    }
    // Prefer org-prefix hits (`qwen/…`) over incidental substrings (`meta/qwen-lookalike`).
    if provider == q || id.starts_with(&format!("{q}/")) {
        return 1;
    }
    if id.starts_with(&q) {
        return 2;
    }
    3
}

/// Filter + rank models for autocomplete (`limit` caps the result set).
#[must_use]
pub fn filter_rank_models(models: Vec<ModelSpec>, query: &str, limit: usize) -> Vec<ModelSpec> {
    let mut matched: Vec<ModelSpec> = models
        .into_iter()
        .filter(|m| model_matches_live_query(m, query))
        .collect();
    matched.sort_by(|a, b| {
        match_rank(a, query)
            .cmp(&match_rank(b, query))
            .then_with(|| a.id.cmp(&b.id))
    });
    matched.truncate(limit);
    matched
}

fn insert_preferring_existing(map: &mut HashMap<String, ModelSpec>, spec: ModelSpec) {
    map.entry(spec.id.clone()).or_insert(spec);
}

/// Collect models from every provider the user currently has credentials for,
/// plus always-on local Mens FS discovery. OpenRouter is fetched live (TTL cache).
pub async fn collect_live_keyed_models(repo_root: &Path) -> Vec<ModelSpec> {
    let mut by_id: HashMap<String, ModelSpec> = HashMap::new();

    if let Ok(mens) = MensCatalog::new(repo_root).refresh().await {
        for spec in mens {
            insert_preferring_existing(&mut by_id, spec);
        }
    }

    if provider_secret_is_available(&ProviderType::OpenRouter) {
        if let Ok(models) = openrouter_models_cached_live().await {
            for spec in models {
                insert_preferring_existing(&mut by_id, spec);
            }
        }
    }

    if provider_secret_is_available(&ProviderType::Anthropic) {
        if let Ok(models) = AnthropicDirectCatalog::new().refresh().await {
            for spec in models {
                insert_preferring_existing(&mut by_id, spec);
            }
        }
    }

    if provider_secret_is_available(&ProviderType::GoogleDirect) {
        if let Ok(models) = GoogleDirectCatalog::new().refresh().await {
            for spec in models {
                insert_preferring_existing(&mut by_id, spec);
            }
        }
    }

    by_id.into_values().collect()
}

/// Live keyed catalog filtered for picker autocomplete.
pub async fn search_live_keyed_models(
    repo_root: &Path,
    query: &str,
    limit: usize,
) -> Vec<ModelSpec> {
    let live = collect_live_keyed_models(repo_root).await;
    filter_rank_models(live, query, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::spec::{ModelCapabilities, PricingSource, ProviderType};
    use crate::models::{ModelTier, StrengthTag};

    fn sample(id: &str, provider: &str) -> ModelSpec {
        ModelSpec {
            id: id.to_string(),
            canonical_slug: id.to_string(),
            provider: provider.to_string(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 8192,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                tier: ModelTier::Pro,
                ..Default::default()
            },
            supported_parameters: vec![],
            observed_cost_per_1k: None,
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::OpenRouter,
        }
    }

    #[test]
    fn model_matches_live_query_is_case_insensitive_on_id_and_provider() {
        let m = sample("qwen/qwen3-8b", "qwen");
        assert!(model_matches_live_query(&m, "QWEN"));
        assert!(model_matches_live_query(&m, "qwen3"));
        assert!(!model_matches_live_query(&m, "claude"));
    }

    #[test]
    fn filter_rank_models_prefers_prefix_and_respects_limit() {
        let models = vec![
            sample("acme/other", "acme"),
            sample("qwen/qwen3-8b", "qwen"),
            sample("qwen/qwen-plus", "qwen"),
            sample("meta/qwen-lookalike", "meta"),
        ];
        let ranked = filter_rank_models(models, "qwen", 2);
        assert_eq!(ranked.len(), 2);
        assert!(ranked.iter().all(|m| m.id.to_lowercase().contains("qwen")));
        assert!(ranked[0].id.starts_with("qwen/"));
        assert!(ranked[1].id.starts_with("qwen/"));
    }
}
