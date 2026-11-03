//! Routing health: invariants of the live registry, checked after every catalog refresh, logged,
//! persisted for `vox doctor`, and shown in the GUI. A violation means "the routing logic or its
//! data drifted — retune", never "edit a model name".

use serde::{Deserialize, Serialize};

use super::reference::{MIN_DERIVE_SAMPLE, ReferenceSource};
use super::spec::QualitySource;
use super::{ModelRegistry, ModelTier};
use crate::mode::ClutchProfile;

pub const ROUTING_HEALTH_SCHEMA_VERSION: u32 = 1;
/// User-preference key (scope `global`) holding the last [`RoutingHealth`] as JSON.
pub const ROUTING_HEALTH_KEY: &str = "model_routing_health";
/// User-preference key (scope `global`) holding the last catalog refresh time (unix seconds).
pub const MODEL_CATALOG_LAST_REFRESH_KEY: &str = "model_catalog_last_refresh";
/// Largest share of cloud models allowed to have no tier.
pub const MAX_UNKNOWN_TIER_SHARE: f64 = 0.2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub invariant: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutingHealth {
    pub schema_version: u32,
    pub checked_at_unix: u64,
    pub models: usize,
    pub cloud_models: usize,
    pub benchmarked: usize,
    pub inherited: usize,
    pub unknown_tier_cloud: usize,
    pub quality_scale: ReferenceSource,
    pub price_bands: ReferenceSource,
    /// What Efficient picks at complexity 10 over the whole catalog (keys ignored).
    #[serde(default)]
    pub efficient_pick: Option<String>,
    pub violations: Vec<Violation>,
}

/// Check the registry's routing invariants now.
#[must_use]
pub fn check_routing_health(registry: &ModelRegistry, now_unix: u64) -> RoutingHealth {
    let models = registry.list_models();
    let cloud: Vec<_> = models
        .iter()
        .filter(|m| !crate::route_policy::is_local_http_provider(&m.provider_type))
        .collect();
    let prior_kind = |m: &super::ModelSpec| {
        m.capabilities
            .quality_prior
            .as_ref()
            .map(|p| p.source.clone())
    };
    let benchmarked = cloud
        .iter()
        .filter(|m| matches!(prior_kind(m), Some(QualitySource::Benchmark { .. })))
        .count();
    let inherited = cloud
        .iter()
        .filter(|m| matches!(prior_kind(m), Some(QualitySource::Inherited { .. })))
        .count();
    let unknown_tier_cloud = cloud
        .iter()
        .filter(|m| m.capabilities.tier == ModelTier::Unknown)
        .count();
    let reference = registry.routing_reference();
    let mut violations = Vec::new();

    let mut task = crate::types::AgentTask::new(
        crate::types::TaskId(0),
        "routing health probe",
        crate::types::TaskPriority::Normal,
        vec![],
    );
    task.task_category = crate::types::TaskCategory::CodeGen;
    task.estimated_complexity = 10;
    let routing_task = super::RoutingTask::from(&task);
    // Probe the catalog, not this process's keys (`&|_| true`): health is about the routing logic and its
    // data, and two refreshers with different keys must write the same verdict.
    let mut efficient_pick = None;
    for clutch in [ClutchProfile::Efficiency, ClutchProfile::Balanced] {
        if let Some(sel) = registry.best_for_task_in_mode_keyed(
            &routing_task,
            clutch.resolve().cost_preference,
            clutch,
            &|_| true,
            |_| None,
        ) {
            if clutch == ClutchProfile::Efficiency {
                efficient_pick = Some(sel.spec.id.clone());
            }
            // Judged by price, not by the tier label: the mode already drops `Elite`-labelled models,
            // so a flagship that slipped through is one whose label is wrong (stale bands, a hand-set
            // tier).
            if !sel.only_candidate && sel.spec.cost_per_1k_output >= reference.elite_min_out {
                violations.push(Violation {
                    invariant: "efficient_never_flagship".into(),
                    detail: format!(
                        "{clutch:?} picked {} at a flagship price while others fit",
                        sel.spec.id
                    ),
                });
            }
        }
    }
    if !cloud.is_empty() && unknown_tier_cloud as f64 / cloud.len() as f64 > MAX_UNKNOWN_TIER_SHARE
    {
        violations.push(Violation {
            invariant: "tiers_known".into(),
            detail: format!(
                "{unknown_tier_cloud} of {} cloud models have no tier",
                cloud.len()
            ),
        });
    }
    if cloud.len() >= MIN_DERIVE_SAMPLE && reference.bands_source == ReferenceSource::Fallback {
        violations.push(Violation {
            invariant: "prices_known".into(),
            detail: format!(
                "{} cloud models but fewer than {MIN_DERIVE_SAMPLE} have a price",
                cloud.len()
            ),
        });
    }

    RoutingHealth {
        schema_version: ROUTING_HEALTH_SCHEMA_VERSION,
        checked_at_unix: now_unix,
        models: models.len(),
        cloud_models: cloud.len(),
        benchmarked,
        inherited,
        unknown_tier_cloud,
        quality_scale: reference.quality_source,
        price_bands: reference.bands_source,
        efficient_pick,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, ModelSpec, ModelTier, ProviderType, StrengthTag};

    fn spec(id: &str, out: f64, tier: ModelTier) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 64_000,
            cost_per_1k: out,
            cost_per_1k_input: out,
            cost_per_1k_output: out,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                tier,
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
    fn a_healthy_registry_has_no_violations() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/flagship", 0.0005, ModelTier::Elite));
        r.register(spec("acme/workhorse", 0.01, ModelTier::Pro)); // below the $20/M fallback Elite band (R5)
        let h = check_routing_health(&r, 1_790_000_000);
        assert_eq!(h.schema_version, ROUTING_HEALTH_SCHEMA_VERSION);
        assert_eq!(h.models, 2);
        assert!(h.violations.is_empty(), "{:?}", h.violations);
    }

    #[test]
    fn many_cloud_models_without_a_tier_are_a_violation() {
        let mut r = ModelRegistry::default();
        for i in 0..10 {
            let mut m = spec(&format!("acme/cloud-{i}"), 0.0, ModelTier::Unknown);
            m.provider_type = ProviderType::OpenRouter;
            r.register(m);
        }
        let h = check_routing_health(&r, 0);
        assert_eq!(h.unknown_tier_cloud, 10);
        assert!(
            h.violations.iter().any(|v| v.invariant == "tiers_known"),
            "{:?}",
            h.violations
        );
    }

    #[test]
    fn a_catalog_this_large_with_no_prices_is_a_violation() {
        let mut r = ModelRegistry::default();
        for i in 0..crate::models::reference::MIN_DERIVE_SAMPLE {
            let mut m = spec(&format!("acme/cloud-{i}"), 0.0, ModelTier::Pro);
            m.provider_type = ProviderType::OpenRouter;
            r.register(m);
        }
        r.apply_routing_reference();
        let h = check_routing_health(&r, 0);
        assert!(
            h.violations.iter().any(|v| v.invariant == "prices_known"),
            "{:?}",
            h.violations
        );
    }

    #[test]
    fn a_flagship_priced_pick_is_a_violation_whatever_its_label() {
        let mut r = ModelRegistry::default();
        r.register(spec("acme/mislabelled", 0.05, ModelTier::Pro));
        let h = check_routing_health(&r, 0);
        assert!(
            h.violations
                .iter()
                .any(|v| v.invariant == "efficient_never_flagship"),
            "{:?}",
            h.violations
        );
    }

    #[test]
    #[serial_test::file_serial]
    fn health_judges_the_catalog_not_this_processs_keys() {
        let mut r = ModelRegistry::default();
        let mut cloud = spec("acme/cloud", 0.01, ModelTier::Pro);
        cloud.provider_type = ProviderType::OpenRouter;
        r.register(cloud);
        crate::models::key_guard::set_test_key_availability(Some(vec![]));
        let h = check_routing_health(&r, 0);
        crate::models::key_guard::set_test_key_availability(None);
        assert_eq!(h.efficient_pick.as_deref(), Some("acme/cloud"));
    }

    #[test]
    fn health_round_trips_as_json() {
        let r = ModelRegistry::default();
        let h = check_routing_health(&r, 42);
        let back: RoutingHealth =
            serde_json::from_str(&serde_json::to_string(&h).unwrap()).unwrap();
        assert_eq!(back, h);
    }
}
