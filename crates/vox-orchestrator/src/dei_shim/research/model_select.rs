//! Resolve research pipeline stage models from the canonical [`crate::models::ModelRegistry`].

use crate::mode::{InferenceConfig, QualityLevel};
use crate::models::ModelRegistry;
use crate::selection::{RegistryModelResolutionParams, resolve_model_with_registry_fallbacks};
use crate::types::TaskCategory;

// Re-export from the single source of truth in vox-config.
pub use vox_config::RESEARCH_FLASH_FALLBACK as FALLBACK_RESEARCH_FLASH_MODEL_ID;
pub use vox_config::REVIEW_PREMIUM_FALLBACK as FALLBACK_REVIEW_PREMIUM_MODEL_ID;
pub use vox_config::NLI_FALLBACK as FALLBACK_NLI_MODEL_ID;

/// Registry-selected model IDs for each research LLM stage.
#[derive(Debug, Clone)]
pub struct ResolvedResearchModels {
    /// Query decomposition / subquery planning.
    pub planner_model: String,
    /// Claim extraction from text.
    pub claim_model: String,
    /// Final cited answer synthesis.
    pub synthesis_model: String,
    /// LLM-as-judge quality score.
    pub judge_model: String,
}

fn fallback_for(task: TaskCategory) -> String {
    match task {
        TaskCategory::Review => FALLBACK_REVIEW_PREMIUM_MODEL_ID.to_string(),
        _ => FALLBACK_RESEARCH_FLASH_MODEL_ID.to_string(),
    }
}

fn pick(
    registry: &ModelRegistry,
    base_inference: &InferenceConfig,
    task: TaskCategory,
    complexity: u8,
    quality: QualityLevel,
) -> String {
    let mut cfg = base_inference.clone();
    cfg.quality = quality;
    let params = RegistryModelResolutionParams {
        task,
        complexity,
        free_tier_latency_critical: false,
        free_tier_fill_in_middle: false,
        allow_cheapest_fallback: true,
        force_web_search_for_task: task == TaskCategory::Research,
    };
    resolve_model_with_registry_fallbacks(
        registry,
        None,
        cfg,
        "",
        None,
        params,
        None,
    )
    .map(|(m, _)| m.id)
    .unwrap_or_else(|_| fallback_for(task))
}

/// Select models for planner, claim extraction, synthesis, and judge stages.
///
/// `base_inference` should reflect the caller's effective inference policy (tier / free-only).
/// When unavailable, use [`OrchestratorConfig::default`].[`effective_inference_config`](OrchestratorConfig::effective_inference_config).
#[must_use]
pub fn resolve_research_models(
    registry: &ModelRegistry,
    base_inference: &InferenceConfig,
) -> ResolvedResearchModels {
    ResolvedResearchModels {
        planner_model: pick(
            registry,
            base_inference,
            TaskCategory::Research,
            4,
            QualityLevel::Flash,
        ),
        claim_model: pick(
            registry,
            base_inference,
            TaskCategory::Research,
            3,
            QualityLevel::Flash,
        ),
        synthesis_model: pick(
            registry,
            base_inference,
            TaskCategory::Research,
            7,
            QualityLevel::Balanced,
        ),
        judge_model: pick(
            registry,
            base_inference,
            TaskCategory::Review,
            8,
            QualityLevel::Premium,
        ),
    }
}
