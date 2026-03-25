use crate::config::CostPreference;
use crate::mode::InferenceConfig;
use crate::models::{BestModelParams, ModelRegistry, ModelSpec, RoutingStrategy};
use crate::types::TaskCategory;

use super::free_tier::FreeTierRouteRequest;

/// Heuristic `(requires_vision, requires_web_search)` from prompt text for routing.
#[must_use]
pub fn infer_prompt_capability_hints(prompt: &str) -> (bool, bool) {
    let p = prompt.to_lowercase();
    let requires_vision = p.contains("image")
        || p.contains("screenshot")
        || p.contains("photo")
        || p.contains("diagram")
        || p.contains("multimodal")
        || p.contains("base64 image")
        || p.contains(".png")
        || p.contains(".jpg")
        || p.contains(".jpeg")
        || p.contains(".webp")
        || p.contains(".gif")
        || p.contains("<img")
        || p.contains("![](")
        || p.contains("ocr")
        || p.contains("cloud vision")
        || p.contains("figure ")
        || p.contains(" chart")
        || p.contains("webcam")
        || p.contains("video frame");
    let requires_web_search = p.contains("web search")
        || p.contains("search web")
        || p.contains("search the web")
        || p.contains("look up")
        || p.contains("lookup ")
        || p.contains(" google ")
        || p.contains("bing ")
        || p.contains(" browse ")
        || p.contains("browse the")
        || p.contains("duckduckgo")
        || p.contains("perplexity")
        || p.contains("on the internet")
        || p.contains("live data")
        || p.contains("real-time")
        || p.contains("real time")
        || p.contains("stock price")
        || p.contains("weather today")
        || p.contains("latest ")
        || p.contains("current event")
        || p.contains("today's ")
        || p.contains(" news");
    (requires_vision, requires_web_search)
}

/// Parameters for [`resolve_model_with_registry_fallbacks`] (MCP-style resolution).
#[derive(Debug, Clone, Copy)]
pub struct RegistryModelResolutionParams {
    /// Task category for scoring (`CodeGen`, `Review`, `Research`, …).
    pub task: TaskCategory,
    /// Complexity hint 1–10.
    pub complexity: u8,
    /// Prefer ultra-low-latency free-tier routes when falling back to [`FreeTierRouteRequest`].
    pub free_tier_latency_critical: bool,
    /// Prefer FIM-capable free-tier routes when falling back.
    pub free_tier_fill_in_middle: bool,
    /// Allow [`ModelRegistry::cheapest`] as last resort.
    pub allow_cheapest_fallback: bool,
    /// OR with `task == Research`: force `web_search` modality in [`InferenceConfig`].
    pub force_web_search_for_task: bool,
}

impl Default for RegistryModelResolutionParams {
    fn default() -> Self {
        Self {
            task: TaskCategory::CodeGen,
            complexity: 5,
            free_tier_latency_critical: false,
            free_tier_fill_in_middle: false,
            allow_cheapest_fallback: false,
            force_web_search_for_task: false,
        }
    }
}

/// Shared registry resolution: optional override id → `best_for_config` → `best_for_requirements` →
/// `best_free_tier` → optional `cheapest`. Used by MCP chat, `vox_suggest_model`, and research stage picks.
///
/// `cost_preference_override`: when `Some`, used for [`BestModelParams::preference`]; when `None`,
/// uses [`crate::mode::QualityLevel::to_cost_preference`] on the effective `cfg.quality` after merges.
#[must_use]
pub fn resolve_model_with_registry_fallbacks(
    models: &ModelRegistry,
    cost_preference_override: Option<CostPreference>,
    mut cfg: InferenceConfig,
    user_prompt: &str,
    preferred_id: Option<&str>,
    params: RegistryModelResolutionParams,
) -> Result<(ModelSpec, bool), String> {
    let (vis, web) = infer_prompt_capability_hints(user_prompt);
    cfg.modalities.vision |= vis;
    cfg.modalities.web_search |= web;
    if params.force_web_search_for_task || params.task == TaskCategory::Research {
        cfg.modalities.web_search = true;
    }
    let free_only = cfg.is_free_only();
    if let Some(id) = preferred_id.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(spec) = models.get(id) {
            return Ok((spec, free_only));
        }
        return Err(format!(
            "Model override '{id}' is not in the registry; clear the override or pick a valid id from the model list"
        ));
    }
    let complexity = params.complexity.min(10).max(1);
    let preference = cost_preference_override.unwrap_or_else(|| cfg.quality.to_cost_preference());
    let mut selected = models
        .best_for_config(params.task, complexity, &cfg)
        .or_else(|| {
            models.best_for_requirements(BestModelParams {
                task_type: params.task,
                complexity,
                preference,
                free_only,
                requires_vision: vis,
                requires_web_search: cfg.modalities.web_search,
                strategy: RoutingStrategy::AutoRouterPreferred,
                ..Default::default()
            })
        })
        .or_else(|| {
            models.best_free_tier(FreeTierRouteRequest {
                task: params.task,
                requires_vision: vis,
                latency_critical: params.free_tier_latency_critical,
                requires_fill_in_middle: params.free_tier_fill_in_middle,
                ..Default::default()
            })
        });
    if params.allow_cheapest_fallback {
        selected = selected.or_else(|| models.cheapest());
    }
    let model = selected.ok_or_else(|| "No models available in registry".to_string())?;
    Ok((model, free_only))
}
