//! Model selection: canonical task→strength mapping, pluggable scoring, and virtual models.
//!
//! Single source of truth for task-strength mapping and scoring weights.
//! Virtual models (e.g. openrouter/auto) are defined here and merged when applicable.

use crate::config::CostPreference;
use crate::mode::{ExecutionModeProfile, InferenceConfig};
use crate::models::{BestModelParams, ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType, RoutingStrategy};
use crate::types::{RoutingProfile, TaskCategory};

/// Map task category and capability flags to a routing profile for telemetry and specialist routing.
pub fn task_and_flags_to_profile(
    task: TaskCategory,
    requires_vision: bool,
    requires_web_search: bool,
    requires_structured_output: bool,
) -> RoutingProfile {
    if requires_vision {
        return RoutingProfile::Vision;
    }
    if requires_web_search || task == TaskCategory::Research {
        return RoutingProfile::Research;
    }
    if requires_structured_output {
        return RoutingProfile::StrictJson;
    }
    match task {
        TaskCategory::Review => RoutingProfile::VoxComposer,
        TaskCategory::Planning => RoutingProfile::Planning,
        TaskCategory::CodeGen | TaskCategory::Testing => RoutingProfile::General,
        TaskCategory::Debugging | TaskCategory::TypeChecking | TaskCategory::Parsing => {
            RoutingProfile::RustLangdev
        }
        TaskCategory::Research | TaskCategory::Ars => RoutingProfile::Research,
        TaskCategory::Merger | TaskCategory::Validator => RoutingProfile::General,
    }
}

/// Canonical mapping: TaskCategory → strength tags used for filtering and scoring.
/// Add new TaskCategory or strength only here.
pub fn task_strengths(task_type: TaskCategory) -> &'static [&'static str] {
    match task_type {
        TaskCategory::CodeGen => &["codegen"],
        TaskCategory::Testing => &["codegen"],
        TaskCategory::Debugging => &["debugging", "logic", "reasoning"],
        TaskCategory::TypeChecking => &["logic", "parsing"],
        TaskCategory::Research => &["research", "codegen"],
        TaskCategory::Parsing => &["parsing", "codegen"],
        TaskCategory::Review => &["review", "codegen"],
        TaskCategory::Planning => &["logic", "reasoning", "research"],

        TaskCategory::Ars => &["logic", "reasoning", "codegen"],
        TaskCategory::Merger | TaskCategory::Validator => &["logic", "reasoning", "codegen"],
    }
}

/// Primary strength for a task (used when a single tag is needed).
pub fn primary_strength(task_type: TaskCategory) -> &'static str {
    task_strengths(task_type)[0]
}

/// Returns true if the model has any strength matching the task.
pub fn model_matches_task(model: &ModelSpec, task_type: TaskCategory) -> bool {
    let strengths = task_strengths(task_type);
    model
        .strengths
        .iter()
        .any(|s| strengths.contains(&s.as_str()))
}

/// Configurable scoring weights. Tune here or load from config.
#[derive(Debug, Clone)]
pub struct ScoringWeights {
    /// Bonus given if the model strengths match the task category.
    pub task_match: f64,
    /// Bonus for models that support native JSON mode.
    pub supports_json: f64,
    /// Bonus for models that support direct file input (multimodal).
    pub supports_file_input: f64,
    /// Bonus for models with vision/image capabilities.
    pub supports_vision: f64,
    /// Bonus for models with integrated web search.
    pub supports_web_search: f64,
    /// Context window bonus per 100,000 tokens of capacity.
    pub context_bonus_per_100k: f64,
    /// Hard cap on context window bonus score.
    pub max_context_cap: f64,
    /// Generation limit bonus per 16,000 tokens of output capacity.
    pub max_tokens_bonus_per_16k: f64,
    /// Hard cap on output generation bonus.
    pub max_tokens_cap: f64,
    /// Preference bonus for models routed through OpenRouter.
    pub openrouter_bonus: f64,
    /// Penalty for models using Google Direct API (encourages normalization).
    pub google_direct_penalty: f64,
    /// Slight penalty for local Ollama models (prefers cloud if budget allows).
    pub ollama_penalty: f64,
    /// Multiplier for per-token cost penalty in Economy mode.
    pub economy_cost_penalty_factor: f64,
    /// Maximum penalty deduction for high-cost models.
    pub economy_cost_cap: f64,
    /// Bonus for truly free models in Economy mode.
    pub economy_free_bonus: f64,
    /// Bonus for Pro-tier models in Performance mode.
    pub performance_pro_tier: f64,
    /// Bonus for Fast-tier models in Performance mode.
    pub performance_fast_tier: f64,
    /// Penalty for Free-tier models in Performance mode.
    pub performance_free_penalty: f64,
    /// Specific context bonus for the free-only selection pool.
    pub free_tier_context_bonus: f64,
    /// Specific JSON bonus for the free-only selection pool.
    pub free_tier_structured_output_bonus: f64,
    /// Specific vision bonus for the free-only selection pool.
    pub free_tier_vision_bonus: f64,
    /// Bonus for direct free-tier providers (Cerebras, Mistral, DeepSeek, SambaNova).
    /// Slightly below `openrouter_bonus` so OpenRouter (with its own fallback routing) is preferred
    /// when available, but above `ollama_penalty` or `google_direct_penalty`.
    pub direct_free_bonus: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            task_match: 10.0,
            supports_json: 1.0,
            supports_file_input: 1.0,
            supports_vision: 1.0,
            supports_web_search: 1.5,
            context_bonus_per_100k: 1.0,
            max_context_cap: 15.0,
            max_tokens_bonus_per_16k: 1.0,
            max_tokens_cap: 8.0,
            openrouter_bonus: 3.0,
            google_direct_penalty: -5.0,
            ollama_penalty: -1.0,
            economy_cost_penalty_factor: 10_000.0,
            economy_cost_cap: 20.0,
            economy_free_bonus: 3.0,
            performance_pro_tier: 3.0,
            performance_fast_tier: 1.0,
            performance_free_penalty: -1.0,
            free_tier_context_bonus: 0.1,
            free_tier_structured_output_bonus: 2.0,
            free_tier_vision_bonus: 1.0,
            direct_free_bonus: 2.0,
        }
    }
}

/// Select the best model from a list for a given task and preference.
pub fn select_best_model(
    models: &[ModelSpec],
    task: TaskCategory,
    pref: CostPreference,
) -> Option<ModelSpec> {
    let scorer = ModelScorer::default();
    models
        .iter()
        .map(|m| {
            (
                m,
                scorer.score(
                    m,
                    ScoreParams {
                        task_type: task,
                        effective_pref: pref,
                        ..Default::default()
                    },
                ),
            )
        })
        .filter(|(_, score)| score.is_finite())
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(m, _)| m.clone())
}

/// Pluggable model scorer with configurable weights.
#[derive(Debug, Clone, Default)]
pub struct ModelScorer {
    /// Weights used to calculate model scores.
    pub weights: ScoringWeights,
}

/// Parameters for model scoring.
#[derive(Debug, Clone, Default)]
pub struct ScoreParams {
    /// The model to score.
    pub model: Option<ModelSpec>,
    /// The category of task being performed.
    pub task_type: TaskCategory,
    /// Effective cost preference (Economy vs Performance).
    pub effective_pref: CostPreference,
    /// Whether only free models are allowed.
    pub free_only: bool,
    /// Whether vision support is required.
    pub requires_vision: bool,
    /// Whether web search support is required.
    pub requires_web_search: bool,
    /// Whether the model has an equivalent available via OpenRouter (to avoid duplicates).
    pub has_openrouter_equivalent: bool,
    /// Optional execution mode profile for specific bonuses.
    pub mode: Option<ExecutionModeProfile>,
}

impl ModelScorer {
    /// Score a model for selection. Returns f64::NEG_INFINITY if model does not satisfy constraints.
    /// When `mode` is provided, applies mode-specific bonuses/penalties on top of cost-preference scoring.
    pub fn score(
        &self,
        model: &ModelSpec,
        params: ScoreParams,
    ) -> f64 {
        let mut full_params = params;
        full_params.model = Some(model.clone());
        self.score_with_mode(full_params)
    }

    /// Score a model with optional mode-aware bonuses/penalties.
    pub fn score_with_mode(
        &self,
        params: ScoreParams,
    ) -> f64 {
        let model = params.model.as_ref().expect("model is required for scoring");
        let task_type = params.task_type;
        let effective_pref = params.effective_pref;
        let free_only = params.free_only;
        let requires_vision = params.requires_vision;
        let requires_web_search = params.requires_web_search;
        let has_openrouter_equivalent = params.has_openrouter_equivalent;
        let mode = params.mode;

        if free_only && !model.is_free {
            return f64::NEG_INFINITY;
        }
        if requires_vision && !model.capabilities.supports_vision {
            return f64::NEG_INFINITY;
        }
        if requires_web_search && !model.supports_web_search() {
            return f64::NEG_INFINITY;
        }
        if has_openrouter_equivalent {
            return f64::NEG_INFINITY;
        }

        let w = &self.weights;
        let mut score = 0.0;

        if model_matches_task(model, task_type) {
            score += w.task_match;
        }
        if model.capabilities.supports_json {
            score += w.supports_json;
        }
        if model.capabilities.supports_file_input {
            score += w.supports_file_input;
        }
        if model.capabilities.supports_vision {
            score += w.supports_vision;
        }
        if model.supports_web_search() {
            score += w.supports_web_search;
        }
        score += (model.capabilities.max_context as f64 / 100_000.0 * w.context_bonus_per_100k)
            .min(w.max_context_cap);
        score +=
            (model.max_tokens as f64 / 16_000.0 * w.max_tokens_bonus_per_16k).min(w.max_tokens_cap);

        match model.provider_type {
            ProviderType::OpenRouter | ProviderType::Groq => score += w.openrouter_bonus,
            ProviderType::Cerebras
            | ProviderType::Mistral
            | ProviderType::DeepSeek
            | ProviderType::SambaNova => score += w.direct_free_bonus,
            ProviderType::GoogleDirect => score += w.google_direct_penalty,
            ProviderType::Ollama => score += w.ollama_penalty,
        }

        match effective_pref {
            CostPreference::Economy => {
                score -=
                    (model.cost_per_1k * w.economy_cost_penalty_factor).min(w.economy_cost_cap);
                if model.is_free {
                    score += w.economy_free_bonus;
                }
            }
            CostPreference::Performance => match model.capabilities.tier {
                ModelTier::Pro => score += w.performance_pro_tier,
                ModelTier::Fast => score += w.performance_fast_tier,
                ModelTier::Free => score += w.performance_free_penalty,
            },
        }

        if free_only {
            score +=
                (model.capabilities.max_context as f64 / 100_000.0) * w.free_tier_context_bonus;
            if model.capabilities.supports_json || model.capabilities.supports_jsonl {
                score += w.free_tier_structured_output_bonus;
            }
            if model.capabilities.supports_vision {
                score += w.free_tier_vision_bonus;
            }
        }

        // Mode-specific bonuses/penalties
        if let Some(m) = mode {
            score += Self::mode_bonus(m, model);
        }

        score
    }

    /// Mode-specific score adjustment. Efficient/fast favor economy; verbose/precision favor quality.
    fn mode_bonus(mode: ExecutionModeProfile, model: &ModelSpec) -> f64 {
        match mode {
            ExecutionModeProfile::Efficient | ExecutionModeProfile::LegacyDefault => {
                if model.is_free { 1.0 } else { -0.5 }
            }
            ExecutionModeProfile::Fast => match model.capabilities.tier {
                ModelTier::Fast => 1.5,
                ModelTier::Free => 0.5,
                ModelTier::Pro => -0.5,
            },
            ExecutionModeProfile::Verbose => match model.capabilities.tier {
                ModelTier::Pro => 0.5,
                ModelTier::Fast => 0.0,
                ModelTier::Free => -0.3,
            },
            ExecutionModeProfile::Precision => match model.capabilities.tier {
                ModelTier::Pro => 1.0,
                ModelTier::Fast => -0.3,
                ModelTier::Free => -0.8,
            },
        }
    }

    /// Score a model using the full `InferenceConfig` — the canonical scoring path.
    ///
    /// Respects tier constraints, capability flags, and quality-derived cost preference.
    /// Returns `f64::NEG_INFINITY` for models that don't satisfy hard constraints.
    pub fn score_with_config(
        &self,
        model: &ModelSpec,
        task_type: TaskCategory,
        cfg: &crate::mode::InferenceConfig,
        has_openrouter_equivalent: bool,
    ) -> f64 {
        use crate::mode::{QualityLevel, TierProfile};

        // Manual tier: only score the exact model ID.
        if let TierProfile::Manual(ref id) = cfg.tier {
            return if &model.id == id {
                1_000.0
            } else {
                f64::NEG_INFINITY
            };
        }

        // BYOK tier: filter to the specified provider.
        if let TierProfile::BringYourOwnKey { ref provider } = cfg.tier
            && !model
                .provider
                .to_ascii_lowercase()
                .contains(&provider.to_ascii_lowercase())
        {
            return f64::NEG_INFINITY;
        }

        let mode_hint = match cfg.quality {
            QualityLevel::Flash => Some(ExecutionModeProfile::Fast),
            QualityLevel::Balanced => Some(ExecutionModeProfile::Efficient),
            QualityLevel::Premium => Some(ExecutionModeProfile::Precision),
        };

        self.score_with_mode(ScoreParams {
            model: Some(model.clone()),
            task_type,
            effective_pref: cfg.quality.to_cost_preference(),
            free_only: cfg.is_free_only(),
            requires_vision: cfg.modalities.vision,
            requires_web_search: cfg.modalities.web_search,
            has_openrouter_equivalent,
            mode: mode_hint,
        })
    }
}

/// Virtual/synthetic models. Merged into registry when conditions are met.
/// Single definition for openrouter/auto, openrouter/free, and any future virtual models.
pub fn virtual_models() -> Vec<ModelSpec> {
    use crate::provider_constants::openrouter as or_c;
    vec![
        ModelSpec {
            id: or_c::VIRTUAL_AUTO.to_string(),
            canonical_slug: None,
            provider: "openrouter".to_string(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 65_536,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            cost_per_1k: 0.0,
            is_free: false,
            supported_parameters: vec![
                "max_tokens".to_string(),
                "temperature".to_string(),
                "web_search_options".to_string(),
            ],
            strengths: vec![
                "codegen".to_string(),
                "debugging".to_string(),
                "research".to_string(),
                "review".to_string(),
            ],
            capabilities: ModelCapabilities {
                supports_vision: true,
                supports_json: true,
                supports_jsonl: true,
                max_context: 1_000_000,
                rate_limit_rpm: None,
                rate_limit_rpd: None,
                is_nsfw_capable: false,
                supports_web_search: true,
                supports_file_input: true,
                tier: ModelTier::Pro,
            },
        },
        ModelSpec {
            id: or_c::VIRTUAL_FREE.to_string(),
            canonical_slug: None,
            provider: "openrouter".to_string(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 65_536,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            cost_per_1k: 0.0,
            is_free: true,
            supported_parameters: vec![
                "max_tokens".to_string(),
                "temperature".to_string(),
            ],
            strengths: vec![
                "codegen".to_string(),
                "debugging".to_string(),
                "research".to_string(),
                "review".to_string(),
            ],
            capabilities: ModelCapabilities {
                supports_vision: true,
                supports_json: true,
                supports_jsonl: true,
                max_context: 1_000_000,
                rate_limit_rpm: Some(or_c::FREE_RPM),
                rate_limit_rpd: Some(or_c::FREE_RPD_NO_CREDIT),
                is_nsfw_capable: false,
                supports_web_search: false,
                supports_file_input: false,
                tier: ModelTier::Free,
            },
        },
    ]
}

/// Returns the `openrouter/auto` virtual model if OpenRouter models exist in the registry.
pub fn openrouter_auto_model(has_openrouter_models: bool) -> Option<ModelSpec> {
    if has_openrouter_models {
        virtual_models().into_iter().next()
    } else {
        None
    }
}

/// Returns the `openrouter/free` virtual model if OpenRouter models exist in the registry.
pub fn openrouter_free_model(has_openrouter_models: bool) -> Option<ModelSpec> {
    if has_openrouter_models {
        virtual_models().into_iter().nth(1)
    } else {
        None
    }
}

/// Resolve a `RoutingProfile` from an `InferenceConfig` and task category.
///
/// This is the `InferenceConfig`-native replacement for constructing the profile
/// from individual `requires_vision`, `requires_web_search`, etc. booleans.
pub fn config_to_routing_profile(
    task: TaskCategory,
    cfg: &crate::mode::InferenceConfig,
) -> RoutingProfile {
    task_and_flags_to_profile(
        task,
        cfg.modalities.vision,
        cfg.modalities.web_search,
        cfg.modalities.structured_output,
    )
}

/// Request parameters for the `FreeTierRouter`.
#[derive(Debug, Clone, Default)]
pub struct FreeTierRouteRequest {
    /// The category of work (CodeGen, Debugging, etc.)
    pub task: TaskCategory,
    /// Minimum context window required (tokens).
    pub context_tokens: u64,
    /// Whether vision support is strictly required.
    pub requires_vision: bool,
    /// Whether structured output (JSON) is strictly required.
    pub requires_structured_output: bool,
    /// Whether fill-in-the-middle (FIM) support is requested (routes to Mistral/Codestral).
    pub requires_fill_in_middle: bool,
    /// Whether low-latency inference is prioritized (routes to Cerebras/Groq).
    pub latency_critical: bool,
    /// Maximum number of candidates to return for parallel dispatch.
    pub max_candidates: usize,
}

/// A prioritized model candidate for free-tier routing.
#[derive(Debug, Clone)]
pub struct RouteCandidate {
    /// The model specification.
    pub model: ModelSpec,
    /// The provider type for this candidate.
    pub provider_type: ProviderType,
    /// Human-readable rationale for this selection (shown in `vox provider status`).
    pub rationale: &'static str,
}

/// Intelligent router that selects the best free-tier models based on capability constraints.
///
/// It delegates scoring to `ModelScorer` but provides a high-level API for mult-candidate
/// routing and specialized constraint handling (e.g. FIM, Scout-speed).
#[derive(Debug, Clone, Default)]
pub struct FreeTierRouter {
    /// Configurable weights for scoring.
    pub weights: ScoringWeights,
}

impl FreeTierRouter {
    /// Construct a new router with default weights.
    pub fn new() -> Self {
        Self::default()
    }

    /// Select the best available free-tier models for the request.
    ///
    /// Returns a list of candidates ordered by suitability. Non-satisfying models are filtered.
    pub fn route(
        &self,
        req: &FreeTierRouteRequest,
        models: &[ModelSpec],
    ) -> Vec<RouteCandidate> {
        let scorer = ModelScorer {
            weights: self.weights.clone(),
        };

        let mut candidates: Vec<(f64, RouteCandidate)> = models
            .iter()
            .filter(|m| m.is_free)
            .filter_map(|m| {
                // Apply hard constraints
                if req.requires_vision && !m.capabilities.supports_vision {
                    return None;
                }
                if req.requires_structured_output && !m.capabilities.supports_json {
                    return None;
                }
                if req.context_tokens > 0 && m.capabilities.max_context < req.context_tokens {
                    return None;
                }

                // Specialized FIM routing: only route to providers with native FIM support.
                // Mistral (Codestral) is the gold standard; DeepSeek also supports FIM natively.
                if req.requires_fill_in_middle
                    && !matches!(
                        m.provider_type,
                        ProviderType::Mistral | ProviderType::DeepSeek
                    )
                {
                    return None;
                }

                // Initial score from standard model scorer
                let mut score = scorer.score(
                    m,
                    ScoreParams {
                        task_type: req.task,
                        effective_pref: CostPreference::Economy,
                        free_only: true,
                        requires_vision: req.requires_vision,
                        ..Default::default()
                    },
                );

                if !score.is_finite() {
                    return None;
                }

                let mut rationale = "General free-tier candidate";

                // Latency bonus for Fast-tier models (Cerebras, Groq)
                if req.latency_critical && m.capabilities.tier == ModelTier::Fast {
                    score += 5.0;
                    rationale = "Candidate selected for ultra-low latency";
                }

                // Preference for direct providers when keys are likely set
                if m.provider_type == ProviderType::Cerebras && req.latency_critical {
                     rationale = "Best-in-class performance (2600 tok/s)";
                } else if m.provider_type == ProviderType::Mistral && req.requires_fill_in_middle {
                     rationale = "Specialized code completion (FIM)";
                } else if m.provider_type == ProviderType::DeepSeek && req.task == TaskCategory::CodeGen {
                     rationale = "Top-tier coding reasoning (DeepSeek V3)";
                } else if m.provider_type == ProviderType::SambaNova && m.id.contains("405B") {
                     rationale = "Large-scale reasoning (405B model)";
                } else if m.capabilities.supports_vision && req.requires_vision {
                     rationale = "Vision-capable free model";
                } else if m.capabilities.max_context >= 1_000_000 {
                     rationale = "Extraordinary context window (1M tokens)";
                }

                Some((score, RouteCandidate {
                    model: m.clone(),
                    provider_type: m.provider_type,
                    rationale,
                }))
            })
            .collect();

        // Sort by score descending
        candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let max = if req.max_candidates > 0 { req.max_candidates } else { 1 };
        candidates.into_iter().take(max).map(|(_, c)| c).collect()
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::InferenceConfig;
    use crate::models::ModelRegistry;

    #[test]
    fn resolve_registry_fallback_errors_on_unknown_override() {
        let reg = ModelRegistry::new();
        let cfg = InferenceConfig::default();
        let params = RegistryModelResolutionParams::default();
        let err = resolve_model_with_registry_fallbacks(
            &reg,
            Some(CostPreference::Economy),
            cfg,
            "",
            Some("definitely_not_a_real_model_id_xyz"),
            params,
        );
        assert!(err.is_err());
    }

    #[test]
    fn task_strengths_cover_all_categories() {
        assert!(!task_strengths(TaskCategory::CodeGen).is_empty());
        assert!(!task_strengths(TaskCategory::Review).is_empty());
        assert!(primary_strength(TaskCategory::CodeGen) == "codegen");
    }

    #[test]
    fn infer_prompt_hints_detect_vision_and_web() {
        let (v, w) = infer_prompt_capability_hints("Please describe this screenshot.png");
        assert!(v);
        assert!(!w);
        let (v2, w2) = infer_prompt_capability_hints("look up the latest Vox release notes");
        assert!(w2);
        assert!(!v2);
        let (v3, w3) = infer_prompt_capability_hints("Run OCR on this scan and extract the table");
        assert!(v3);
        assert!(!w3);
        let (v4, w4) =
            infer_prompt_capability_hints("What is the weather today in Seattle?");
        assert!(w4);
        assert!(!v4);
    }

    #[test]
    fn virtual_models_contains_openrouter_auto() {
        let v = virtual_models();
        assert!(!v.is_empty());
        assert_eq!(v[0].id, "openrouter/auto");
    }

    #[test]
    fn task_and_flags_to_profile_vision_takes_precedence() {
        assert_eq!(
            task_and_flags_to_profile(TaskCategory::CodeGen, true, false, false),
            RoutingProfile::Vision
        );
    }

    #[test]
    fn task_and_flags_to_profile_research_from_web_search() {
        assert_eq!(
            task_and_flags_to_profile(TaskCategory::CodeGen, false, true, false),
            RoutingProfile::Research
        );
    }

    #[test]
    fn task_and_flags_to_profile_general_default() {
        assert_eq!(
            task_and_flags_to_profile(TaskCategory::CodeGen, false, false, false),
            RoutingProfile::General
        );
    }

    #[test]
    fn mode_bonus_efficient_prefers_free() {
        let scorer = ModelScorer::default();
        let free = mk_spec("free", true, ModelTier::Free);
        let paid = mk_spec("paid", false, ModelTier::Pro);
        let score_free = scorer.score_with_mode(ScoreParams {
            model: Some(free),
            task_type: TaskCategory::CodeGen,
            effective_pref: CostPreference::Economy,
            mode: Some(ExecutionModeProfile::Efficient),
            ..Default::default()
        });
        let score_paid = scorer.score_with_mode(ScoreParams {
            model: Some(paid),
            task_type: TaskCategory::CodeGen,
            effective_pref: CostPreference::Economy,
            mode: Some(ExecutionModeProfile::Efficient),
            ..Default::default()
        });
        assert!(
            score_free > score_paid,
            "efficient mode should favor free models"
        );
    }

    #[test]
    fn mode_bonus_precision_prefers_pro_tier() {
        let scorer = ModelScorer::default();
        let pro = mk_spec("pro", false, ModelTier::Pro);
        let free = mk_spec("free", true, ModelTier::Free);
        let score_pro = scorer.score_with_mode(ScoreParams {
            model: Some(pro),
            task_type: TaskCategory::CodeGen,
            effective_pref: CostPreference::Performance,
            mode: Some(ExecutionModeProfile::Precision),
            ..Default::default()
        });
        let score_free = scorer.score_with_mode(ScoreParams {
            model: Some(free),
            task_type: TaskCategory::CodeGen,
            effective_pref: CostPreference::Performance,
            mode: Some(ExecutionModeProfile::Precision),
            ..Default::default()
        });
        assert!(
            score_pro > score_free,
            "precision mode should favor pro tier"
        );
    }

    #[test]
    fn free_tier_router_satisfies_hard_constraints() {
        let models = vec![
            mk_spec("vision_free", true, ModelTier::Free),
            mk_spec("non_vision_free", true, ModelTier::Free),
        ];
        let mut vision_free = models[0].clone();
        vision_free.capabilities.supports_vision = true;
        let models = vec![vision_free, models[1].clone()];

        let router = FreeTierRouter::new();
        let mut req = FreeTierRouteRequest::default();
        req.requires_vision = true;

        let results = router.route(&req, &models);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].model.id, "vision_free");
    }

    #[test]
    fn free_tier_router_prioritizes_latency() {
        let models = vec![
            mk_spec("slow_free", true, ModelTier::Free),
            mk_spec("fast_free", true, ModelTier::Fast),
        ];

        let router = FreeTierRouter::new();
        let mut req = FreeTierRouteRequest::default();
        req.latency_critical = true;

        let results = router.route(&req, &models);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].model.id, "fast_free");
        assert!(results[0].rationale.contains("latency"));
    }

    #[test]
    fn free_tier_router_routes_to_fim() {
        let mut mistral = mk_spec("mistral-codestral", true, ModelTier::Free);
        mistral.provider_type = ProviderType::Mistral;
        let mut deepseek = mk_spec("deepseek-chat", true, ModelTier::Free);
        deepseek.provider_type = ProviderType::DeepSeek;

        let models = vec![mistral, deepseek];

        let router = FreeTierRouter::new();
        let mut req = FreeTierRouteRequest::default();
        req.requires_fill_in_middle = true;

        let results = router.route(&req, &models);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].model.id, "mistral-codestral");
        assert!(results[0].rationale.contains("FIM"));
    }

    fn mk_spec(id: &str, is_free: bool, tier: ModelTier) -> ModelSpec {
        ModelSpec {
            id: id.to_string(),
            canonical_slug: None,
            provider: "test".to_string(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 4096,
            cost_per_1k_input: if is_free { 0.0 } else { 0.001 },
            cost_per_1k_output: if is_free { 0.0 } else { 0.002 },
            cost_per_1k: if is_free { 0.0 } else { 0.0015 },
            is_free,
            supported_parameters: vec![],
            strengths: vec!["codegen".to_string()],
            capabilities: ModelCapabilities {
                tier,
                ..Default::default()
            },
        }
    }
}
