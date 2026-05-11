use super::*;
use crate::config::CostPreference;
use crate::mode::{ExecutionModeProfile, InferenceConfig};
use crate::models::{ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType};
use crate::types::{RoutingProfile, TaskCategory};

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
        None,
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
fn infer_prompt_hints_web_search_only_no_vision_heuristic() {
    // Vision routing is driven by attachment manifests, not substring heuristics (SSOT).
    let (v, w) = infer_prompt_capability_hints("Please describe this screenshot.png");
    assert!(!v);
    assert!(!w);
    let (_v2, w2) = infer_prompt_capability_hints("look up the latest Vox release notes");
    assert!(w2);
    let (v3, w3) = infer_prompt_capability_hints("Run OCR on this scan and extract the table");
    assert!(!v3);
    assert!(!w3);
    let (_v4, w4) = infer_prompt_capability_hints("What is the weather today in Seattle?");
    assert!(w4);
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
