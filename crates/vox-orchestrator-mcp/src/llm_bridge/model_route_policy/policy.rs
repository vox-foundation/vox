use vox_config::{
    GeminiRoutePolicy, gemini_route_targets_from_env, inference_profile_allows_local_ollama_http,
};
use vox_orchestrator::models::{ModelRegistry, ModelSpec, ProviderType};

use super::types::McpChatModelResolution;

pub(super) fn enforce_free_tier_if_needed(
    registry: &ModelRegistry,
    res: &McpChatModelResolution,
    spec: ModelSpec,
) -> Result<ModelSpec, String> {
    if !res.enforce_free_tier_only || spec.is_free {
        return Ok(spec);
    }
    let task = res.task_category;
    // Prefer the capability-/tier-aware FreeTierRouter for the forced-free swap
    // (so latency-critical/FIM hints are honored), surfacing its rationale; fall
    // back to the registry's simple free helpers when the router returns nothing.
    let free = registry.free_models();
    if let Some((m, rationale)) =
        super::free_tier_adapter::route_free_tier_latency(&free, res, &[], mcp_local_model_allowed)
    {
        tracing::info!(
            model_id = %m.id,
            provider = ?m.provider_type,
            rationale,
            route = "free-tier:enforced",
            "MCP forced free-tier swap via FreeTierRouter"
        );
        return Ok(m);
    }
    registry
        .best_free_for_with_filter(task, mcp_local_model_allowed)
        .or_else(|| registry.cheapest_free_with_filter(mcp_local_model_allowed))
        .ok_or_else(|| {
            "No free-tier model available (enforce_free_tier_only) after vox_populi::inference_PROFILE rules; clear sticky override, allow desktop_ollama/lan_gateway for Ollama, or add a non-Ollama free model in models.toml".to_string()
        })
}

#[must_use]
pub(super) fn mcp_local_model_allowed(m: &ModelSpec) -> bool {
    // VoxLocal (`vox mens serve`) is independent of the desktop-Ollama inference
    // profile. Gating it on `inference_profile_allows_local_ollama_http` made
    // sticky `mens/<run>` pins fall through to OpenRouter under
    // `cloud_openai_compatible`.
    if matches!(m.provider_type, ProviderType::VoxLocal) {
        return true;
    }
    let is_ollama_shaped = matches!(
        m.provider_type,
        ProviderType::Ollama | ProviderType::PopuliMesh
    );
    !is_ollama_shaped || inference_profile_allows_local_ollama_http()
}

/// Synthesize a sticky VoxLocal catalog row for `mens/<run>` when the orch
/// registry has not yet merged MensCatalog (OpenRouter TTL can skip refresh).
#[must_use]
pub(super) fn sticky_mens_vox_local_spec(id: &str) -> Option<ModelSpec> {
    let trimmed = id.trim();
    if !trimmed.starts_with("mens/") || trimmed.len() <= "mens/".len() {
        return None;
    }
    Some(ModelSpec {
        id: trimmed.to_string(),
        canonical_slug: trimmed.to_string(),
        provider: "populi_local".to_string(),
        provider_type: ProviderType::VoxLocal,
        max_tokens: 8192,
        cost_per_1k: 0.0,
        cost_per_1k_input: 0.0,
        cost_per_1k_output: 0.0,
        is_free: true,
        strengths: vec![
            vox_orchestrator::models::generated::StrengthTag::Generalist,
            vox_orchestrator::models::generated::StrengthTag::Codegen,
        ],
        capabilities: vox_orchestrator::models::ModelCapabilities {
            tier: vox_orchestrator::models::ModelTier::Local,
            writes_vox: true,
            ..Default::default()
        },
        supported_parameters: vec![],
        observed_cost_per_1k: None,
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source: vox_orchestrator::models::spec::PricingSource::Bootstrap,
    })
}

#[must_use]
pub(super) fn apply_gemini_policy(
    registry: &ModelRegistry,
    chosen: ModelSpec,
    sticky_override: bool,
) -> ModelSpec {
    if sticky_override {
        return chosen;
    }
    let targets = gemini_route_targets_from_env();
    let is_gemini = chosen.id.to_ascii_lowercase().contains("gemini");
    if !is_gemini {
        return chosen;
    }
    match GeminiRoutePolicy::from_env() {
        GeminiRoutePolicy::RegistryDefault => chosen,
        GeminiRoutePolicy::OpenRouterFirst => {
            if !matches!(chosen.provider_type, ProviderType::OpenRouter)
                && vox_config::openrouter_api_key().is_some()
            {
                registry.get(&targets.openrouter_model).unwrap_or(chosen)
            } else {
                chosen
            }
        }
        GeminiRoutePolicy::GoogleDirectOnly => {
            if !matches!(chosen.provider_type, ProviderType::GoogleDirect) {
                registry.get(&targets.google_direct_model).unwrap_or(chosen)
            } else {
                chosen
            }
        }
    }
}
