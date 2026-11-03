//! Discover → bench → promote. After a catalog refresh, [`catalog_delta`] names the models
//! that are new and the models that just became the target of a `~vendor/…-latest` alias
//! (a new version shipped under an existing alias). [`bench_queue`] orders them for
//! `vox model bench`; the recorded verdicts then move routing (`bench_evidence`), so a
//! newcomer that measures better takes over the modes it serves better without code changes.

use std::collections::HashSet;

use super::{ModelRegistry, ModelSpec};

/// What changed between two catalog snapshots.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct CatalogDelta {
    /// Ids present now that were not before.
    pub new_models: Vec<String>,
    /// Ids that are now a `-latest` alias target but were not before (including new ids).
    pub new_alias_targets: Vec<String>,
}

#[must_use]
pub fn catalog_delta(old: &[ModelSpec], new: &[ModelSpec]) -> CatalogDelta {
    let old_ids: HashSet<&str> = old.iter().map(|m| m.id.as_str()).collect();
    let old_targets: HashSet<&str> = old
        .iter()
        .filter(|m| m.capabilities.is_alias_target)
        .map(|m| m.id.as_str())
        .collect();
    let mut new_models: Vec<String> = new
        .iter()
        .filter(|m| !old_ids.contains(m.id.as_str()))
        .map(|m| m.id.clone())
        .collect();
    let mut new_alias_targets: Vec<String> = new
        .iter()
        .filter(|m| m.capabilities.is_alias_target && !old_targets.contains(m.id.as_str()))
        .map(|m| m.id.clone())
        .collect();
    new_models.sort();
    new_alias_targets.sort();
    CatalogDelta {
        new_models,
        new_alias_targets,
    }
}

/// Models to bench, best-first: new `-latest` targets (a vendor's current version), then other
/// new models. Skips ids routing cannot reach (no key / wire) and ids that already have graded
/// cases, unless they just became an alias target. At most `limit`.
#[must_use]
pub fn bench_queue(
    delta: &CatalogDelta,
    registry: &ModelRegistry,
    reachable: &dyn Fn(&ModelSpec) -> bool,
    limit: usize,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |id: &String, allow_benched: bool| {
        if out.len() >= limit || out.contains(id) {
            return;
        }
        let Some(spec) = registry.get(id) else {
            return;
        };
        if !reachable(&spec) {
            return;
        }
        if !allow_benched && registry.bench_evidence(id).is_some_and(|e| e.cases > 0) {
            return;
        }
        out.push(id.clone());
    };
    for id in &delta.new_alias_targets {
        push(id, true);
    }
    for id in &delta.new_models {
        push(id, false);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::bench_evidence::evidence;
    use crate::models::spec::{PricingSource, QualitySource};
    use crate::models::{
        ModelCapabilities, ModelSelectionRequest, ProviderType, SelectionIntent, StrengthTag,
        TaskCategory, decide,
    };
    use std::collections::HashMap;

    fn spec(id: &str, alias_target: bool) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 64_000,
            cost_per_1k: 0.001,
            cost_per_1k_input: 0.001,
            cost_per_1k_output: 0.001,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                is_alias_target: alias_target,
                max_context: 64_000,
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::UserConfig,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn delta_finds_new_models_and_moved_latest_aliases() {
        let old = vec![spec("acme/a-1", true), spec("acme/b", false)];
        let new = vec![
            spec("acme/a-1", false),
            spec("acme/a-2", true),
            spec("acme/b", true),
            spec("acme/c", false),
        ];
        let d = catalog_delta(&old, &new);
        assert_eq!(d.new_models, vec!["acme/a-2", "acme/c"]);
        assert_eq!(d.new_alias_targets, vec!["acme/a-2", "acme/b"]);
    }

    #[test]
    fn queue_puts_latest_targets_first_and_rebenches_them_but_skips_benched_newcomers() {
        let mut reg = ModelRegistry::default();
        for m in [
            spec("acme/a-2", true),
            spec("acme/b", true),
            spec("acme/c", false),
            spec("acme/d", false),
        ] {
            reg.register(m);
        }
        let mut ev = HashMap::new();
        ev.insert("acme/b".to_string(), evidence(10, 9, 0.9));
        ev.insert("acme/d".to_string(), evidence(10, 9, 0.9));
        reg.inject_benchmark_evidence(ev);
        let d = CatalogDelta {
            new_models: vec![
                "acme/a-2".into(),
                "acme/c".into(),
                "acme/d".into(),
                "acme/gone".into(),
            ],
            new_alias_targets: vec!["acme/a-2".into(), "acme/b".into()],
        };
        let q = bench_queue(&d, &reg, &|_| true, 10);
        assert_eq!(
            q,
            vec!["acme/a-2", "acme/b", "acme/c"],
            "b re-benched as a moved alias; d already benched; gone unknown"
        );
        assert_eq!(bench_queue(&d, &reg, &|_| true, 1), vec!["acme/a-2"]);
        assert!(
            bench_queue(&d, &reg, &|_| false, 10).is_empty(),
            "unreachable models are never queued"
        );
    }

    /// A model that appears after the fact, benches better, takes over chat routing; when a
    /// later re-bench fails it, routing falls back. No routing code knows its name.
    #[test]
    fn synthetic_newcomer_is_adopted_on_merit_and_dropped_on_failure() {
        let chat =
            ModelSelectionRequest::from_intent(SelectionIntent::for_task(TaskCategory::Chat));
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/incumbent-1", false));
        assert_eq!(
            decide(&chat, &reg)
                .expect("incumbent routes")
                .selected_model,
            "acme/incumbent-1"
        );

        let before: Vec<ModelSpec> = reg.list_models();
        reg.register(spec("zeta/newcomer-2", true));
        let delta = catalog_delta(&before, &reg.list_models());
        assert_eq!(
            bench_queue(&delta, &reg, &|_| true, 5),
            vec!["zeta/newcomer-2"]
        );

        let mut ev = HashMap::new();
        ev.insert("acme/incumbent-1".to_string(), evidence(20, 9, 0.45));
        ev.insert("zeta/newcomer-2".to_string(), evidence(20, 19, 0.95));
        reg.inject_benchmark_evidence(ev.clone());
        let picked = decide(&chat, &reg).expect("routes");
        assert_eq!(picked.selected_model, "zeta/newcomer-2");
        assert!(matches!(
            reg.get("zeta/newcomer-2")
                .and_then(|m| m.capabilities.quality_prior.clone())
                .map(|p| p.source),
            Some(QualitySource::Measured { cases: 20, .. })
        ));

        ev.insert("zeta/newcomer-2".to_string(), evidence(20, 1, 0.05));
        reg.inject_benchmark_evidence(ev);
        assert_eq!(
            decide(&chat, &reg).expect("routes").selected_model,
            "acme/incumbent-1"
        );
    }
}
