//! Per-mode routing table: which model each clutch mode would dispatch for each benchmarked
//! task category, with where its quality rating came from. Shared by `vox model bench`
//! (adaptation report), the Axis leaderboard and the live-proof script, so all three read
//! the same routing rules as background dispatch (`mode_select::best_for_task_under_gate`).

use super::mode_select::DispatchGate;
use super::spec::QualitySource;
use super::{ModelRegistry, TaskCategory};
use crate::mode::ClutchProfile;

/// Categories the built-in benchmark suites measure (`vox model bench`).
pub const BENCH_CATEGORIES: [TaskCategory; 6] = [
    TaskCategory::Chat,
    TaskCategory::CodeGen,
    TaskCategory::Planning,
    TaskCategory::Research,
    TaskCategory::ToolOrchestration,
    TaskCategory::Parsing,
];

pub const MODES: [ClutchProfile; 4] = [
    ClutchProfile::Free,
    ClutchProfile::Efficiency,
    ClutchProfile::Balanced,
    ClutchProfile::Genius,
];

/// One cell of the table.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ModeRoute {
    pub mode: String,
    pub category: String,
    /// `None` when the mode would dispatch nothing (no key, or every model gated).
    pub model_id: Option<String>,
    /// The pick lies outside the mode's preferred tiers (`ModeSelection::only_candidate`).
    pub only_candidate: bool,
    /// Candidates that survived the filters.
    pub candidates: usize,
    /// Quality prior used for ranking, 0–1.
    pub quality: Option<f64>,
    /// `measured` (graded bench), `benchmark` (public index), `inherited`, `estimate`.
    pub quality_source: String,
    /// Graded bench cases behind a `measured` quality.
    pub bench_cases: u32,
}

/// The table for `modes` × `categories` at `complexity` (1–10), under dispatch's own gate
/// minus live budget state (exploration spend and provider usage need the running daemon).
#[must_use]
pub fn mode_routing_table(
    registry: &ModelRegistry,
    modes: &[ClutchProfile],
    categories: &[TaskCategory],
    complexity: u8,
) -> Vec<ModeRoute> {
    let mut out = Vec::with_capacity(modes.len() * categories.len());
    for &clutch in modes {
        let resolved = clutch.resolve();
        let gate = DispatchGate {
            force_free_pool: resolved.force_free_pool,
            ..Default::default()
        };
        for &category in categories {
            let mut probe = crate::types::AgentTask::new(
                crate::types::TaskId(0),
                "mode routing table",
                crate::types::TaskPriority::Normal,
                vec![],
            );
            probe.task_category = category;
            probe.estimated_complexity = complexity.clamp(1, 10);
            let sel =
                registry.best_for_task_under_gate(&probe, resolved.cost_preference, clutch, &gate);
            let (quality, quality_source, bench_cases) = sel
                .as_ref()
                .and_then(|s| s.spec.capabilities.quality_prior.as_ref())
                .map_or((None, "none".to_string(), 0), |p| {
                    let (src, cases) = match &p.source {
                        QualitySource::Measured { cases, .. } => ("measured", *cases),
                        QualitySource::Benchmark { .. } => ("benchmark", 0),
                        QualitySource::Inherited { .. } => ("inherited", 0),
                        _ => ("estimate", 0),
                    };
                    (Some(p.value), src.to_string(), cases)
                });
            out.push(ModeRoute {
                mode: format!("{clutch:?}").to_ascii_lowercase(),
                category: category.to_string(),
                model_id: sel.as_ref().map(|s| s.spec.id.clone()),
                only_candidate: sel.as_ref().is_some_and(|s| s.only_candidate),
                candidates: sel.as_ref().map_or(0, |s| s.ranking.ranked.len()),
                quality,
                quality_source,
                bench_cases,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::bench_evidence::evidence;
    use crate::models::{ModelCapabilities, ModelSpec, PricingSource, ProviderType, StrengthTag};
    use std::collections::HashMap;

    fn local(id: &str, tier: crate::models::ModelTier) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "ollama".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 4096,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                tier,
                max_context: 32_000,
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn a_benchmarked_newcomer_takes_over_routing_and_reports_measured_quality() {
        use crate::models::ModelTier;
        let mut reg = ModelRegistry::default();
        reg.register(local("acme/old-1", ModelTier::Fast));
        reg.register(local("acme/new-2", ModelTier::Fast));
        let mut ev = HashMap::new();
        ev.insert("acme/old-1".to_string(), evidence(20, 8, 0.4));
        ev.insert("acme/new-2".to_string(), evidence(20, 19, 0.95));
        reg.inject_benchmark_evidence(ev);

        let table = mode_routing_table(
            &reg,
            &[ClutchProfile::Balanced],
            &[TaskCategory::CodeGen],
            5,
        );
        assert_eq!(table.len(), 1);
        let cell = &table[0];
        assert_eq!(cell.model_id.as_deref(), Some("acme/new-2"), "{cell:?}");
        assert_eq!(cell.quality_source, "measured");
        assert_eq!(cell.bench_cases, 20);
        assert_eq!(cell.mode, "balanced");
        assert_eq!(cell.category, "CodeGen");
    }

    /// Live bench 2026-10-08: a hosted free model with a public-index quality took every
    /// non-Genius route over a cheap model measured at 0.97, and Efficiency and Balanced
    /// routed identically because neither mode's axes reached the scorer.
    #[test]
    fn balanced_weighs_measured_quality_over_a_free_hosted_model() {
        use crate::models::ModelTier;
        use crate::models::key_guard::set_test_key_availability;
        let hosted = |id: &str, out_per_1k: f64| {
            let mut m = local(id, ModelTier::Fast);
            m.provider_type = ProviderType::OpenRouter;
            m.provider = "openrouter".into();
            m.cost_per_1k_input = out_per_1k / 5.0;
            m.cost_per_1k_output = out_per_1k;
            m.cost_per_1k = out_per_1k;
            m.is_free = out_per_1k == 0.0;
            m.pricing_source = PricingSource::OpenRouter;
            m
        };
        let mut free = hosted("acme/free-flash", 0.0);
        free.capabilities.intelligence_index = Some(43.0);
        let mut paid = hosted("acme/cheap-pro", 0.0005);
        paid.capabilities.intelligence_index = Some(45.0);
        let mut reg = ModelRegistry::default();
        reg.register(free);
        reg.register(paid);
        let mut ev = HashMap::new();
        ev.insert("acme/cheap-pro".to_string(), evidence(20, 19, 0.97));
        reg.inject_benchmark_evidence(ev);

        set_test_key_availability(Some(vec![ProviderType::OpenRouter]));
        let table = mode_routing_table(
            &reg,
            &[ClutchProfile::Free, ClutchProfile::Balanced],
            &[TaskCategory::Chat],
            5,
        );
        set_test_key_availability(None);
        assert_eq!(
            table[0].model_id.as_deref(),
            Some("acme/free-flash"),
            "{:?}",
            table[0]
        );
        assert_eq!(
            table[1].model_id.as_deref(),
            Some("acme/cheap-pro"),
            "{:?}",
            table[1]
        );
        assert_eq!(table[1].quality_source, "measured");
    }

    #[test]
    fn table_has_one_cell_per_mode_and_category() {
        let reg = ModelRegistry::default();
        let table = mode_routing_table(&reg, &MODES, &BENCH_CATEGORIES, 5);
        assert_eq!(table.len(), MODES.len() * BENCH_CATEGORIES.len());
        assert!(
            table.iter().all(|c| c.model_id.is_none()),
            "empty registry routes nothing"
        );
    }
}
