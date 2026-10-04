//! Dispatch-path selection: Efficiency and Balanced never pick an Elite
//! (flagship) model while a non-Elite candidate fits, and a provider without
//! a resolvable key is never picked.

use super::RoutingTask;
use crate::config::CostPreference;
use crate::mode::ClutchProfile;
use crate::models::key_guard::selection_key_available;
use crate::models::{ModelRegistry, ModelSpec, ModelTier};

/// A dispatch-path pick. `only_candidate` is true when the mode's preferred set
/// was empty and the pick came from the unrestricted fallback pass.
#[derive(Debug, Clone)]
pub struct ModeSelection {
    pub spec: ModelSpec,
    pub only_candidate: bool,
    pub ranking: crate::models::ranking::Ranking,
}

fn excludes_elite(clutch: ClutchProfile) -> bool {
    matches!(clutch, ClutchProfile::Efficiency | ClutchProfile::Balanced)
}

impl ModelRegistry {
    /// Task dispatch entry point: [`Self::best_for_task_with_filter`] plus the clutch's tier
    /// guarantee and the provider-key gate.
    pub fn best_for_task_in_mode(
        &self,
        task: &RoutingTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModeSelection> {
        self.best_for_task_in_mode_keyed(task, preference, clutch, &selection_key_available, |m| {
            (!pred(m)).then_some(crate::models::ranking::Exclusion::Filtered)
        })
    }

    /// [`Self::best_for_task_in_mode`] with an explicit provider-key check and a reason-giving filter.
    /// Routing health passes `&|_| true` to judge the catalog rather than this process's keys; the
    /// routing explainer passes a [`DispatchGate`] so it applies dispatch's own rules.
    pub fn best_for_task_in_mode_keyed(
        &self,
        task: &RoutingTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        key_ok: &dyn Fn(&crate::models::ProviderType) -> bool,
        mut filter: impl FnMut(&ModelSpec) -> Option<crate::models::ranking::Exclusion>,
    ) -> Option<ModeSelection> {
        use crate::models::ranking::Exclusion;
        let exclude_elite = excludes_elite(clutch);
        let preferred = self.rank_task_with_filter(task, preference, |m| {
            if exclude_elite && m.capabilities.tier == ModelTier::Elite {
                Some(Exclusion::FlagshipExcludedByMode)
            } else if !key_ok(&m.provider_type) {
                Some(Exclusion::NoProviderKey)
            } else {
                filter(m)
            }
        });
        if let Some(spec) = preferred.chosen().cloned() {
            return Some(ModeSelection {
                spec,
                only_candidate: false,
                ranking: preferred,
            });
        }
        if !exclude_elite {
            return None;
        }
        let fallback = self.rank_task_with_filter(task, preference, |m| {
            if !key_ok(&m.provider_type) {
                Some(Exclusion::NoProviderKey)
            } else {
                filter(m)
            }
        });
        fallback.chosen().cloned().map(|spec| ModeSelection {
            spec,
            only_candidate: true,
            ranking: fallback,
        })
    }

    /// What task dispatch chooses under `gate`, with this process's provider keys. `runtime.rs` and the
    /// routing explainer both call this, so the explanation cannot drift from dispatch.
    pub fn best_for_task_under_gate(
        &self,
        task: &RoutingTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        gate: &DispatchGate,
    ) -> Option<ModeSelection> {
        self.best_for_task_in_mode_keyed(task, preference, clutch, &selection_key_available, |m| {
            gate.exclusion(m)
        })
    }
}

/// Caller-side rules task dispatch applies on top of the mode (`runtime.rs`), shared with the routing
/// explainer so the explanation and the dispatch cannot drift apart.
#[derive(Debug, Clone, Default)]
pub struct DispatchGate {
    /// Free mode: only zero-cost models.
    pub force_free_pool: bool,
    /// Today's exploration budget is spent: unpriced (`PricingSource::Unknown`) models are skipped.
    pub unknown_price_blocked: bool,
    /// Usage budgets exist: only these provider keys ([`provider_budget_key`]) have budget left.
    pub allowed_providers: Option<std::collections::HashSet<String>>,
}

impl DispatchGate {
    /// Why this gate keeps `m` out, if it does (same order as dispatch's old closure).
    #[must_use]
    pub fn exclusion(&self, m: &ModelSpec) -> Option<crate::models::ranking::Exclusion> {
        use crate::models::ranking::Exclusion;
        if self.unknown_price_blocked
            && m.pricing_source == crate::models::spec::PricingSource::Unknown
        {
            return Some(Exclusion::ExplorationBudgetSpent);
        }
        if self.force_free_pool && !m.is_free {
            return Some(Exclusion::NotFree);
        }
        match &self.allowed_providers {
            Some(allowed) if !allowed.contains(provider_budget_key(&m.provider_type)) => {
                Some(Exclusion::ProviderBudgetExhausted)
            }
            _ => None,
        }
    }
}

/// The provider key usage budgets are recorded under (moved verbatim from `runtime.rs`).
#[must_use]
pub fn provider_budget_key(p: &crate::models::ProviderType) -> &'static str {
    use crate::models::ProviderType;
    match p {
        ProviderType::OpenRouter => "openrouter",
        ProviderType::Ollama => "ollama",
        ProviderType::GoogleDirect => "google",
        ProviderType::Groq => "groq",
        ProviderType::Cerebras => "cerebras",
        ProviderType::Mistral => "mistral",
        ProviderType::DeepSeek => "deepseek",
        ProviderType::SambaNova => "sambanova",
        ProviderType::Anthropic => "anthropic",
        ProviderType::PopuliMesh => "populimesh",
        ProviderType::HuggingFaceRouter => "huggingface",
        ProviderType::Custom(_) => "custom",
        ProviderType::VoxLocal => "vox_local",
    }
}

#[cfg(test)]
mod tests {
    use crate::config::CostPreference;
    use crate::mode::ClutchProfile;
    use crate::models::key_guard::set_test_key_availability;
    use crate::models::spec::PricingSource;
    use crate::models::{
        ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType, RoutingTask,
        StrengthTag,
    };
    use crate::types::{AgentTask, TaskCategory, TaskId, TaskPriority};

    fn spec(id: &str, provider_type: ProviderType, tier: ModelTier, cost: f64) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 200_000,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
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

    fn hard_task() -> RoutingTask {
        let mut t = AgentTask::new(TaskId(1), "hard codegen", TaskPriority::Normal, vec![]);
        t.task_category = TaskCategory::CodeGen;
        t.estimated_complexity = 10;
        RoutingTask::from(&t)
    }

    /// The Elite model is the CHEAPEST with equal context, so without the
    /// guard it wins under Economy; the first fixture let the Fast model win regardless.
    fn registry() -> ModelRegistry {
        let mut r = ModelRegistry::default();
        r.register(spec(
            "acme/flagship-9",
            ProviderType::Ollama,
            ModelTier::Elite,
            0.0005,
        ));
        r.register(spec(
            "acme/workhorse-9",
            ProviderType::Ollama,
            ModelTier::Pro,
            0.02,
        ));
        r.register(spec(
            "acme/quick-9",
            ProviderType::Ollama,
            ModelTier::Fast,
            0.02,
        ));
        r
    }

    #[test]
    fn efficiency_on_a_hard_task_never_picks_elite() {
        let pick = registry()
            .best_for_task_in_mode(
                &hard_task(),
                CostPreference::Economy,
                ClutchProfile::Efficiency,
                |_| true,
            )
            .expect("a candidate");
        assert_ne!(
            pick.spec.capabilities.tier,
            ModelTier::Elite,
            "picked {}",
            pick.spec.id
        );
        assert!(!pick.only_candidate);
    }

    #[test]
    fn balanced_never_picks_elite_when_alternatives_exist() {
        let pick = registry()
            .best_for_task_in_mode(
                &hard_task(),
                CostPreference::Economy,
                ClutchProfile::Balanced,
                |_| true,
            )
            .expect("a candidate");
        assert_ne!(pick.spec.capabilities.tier, ModelTier::Elite);
    }

    #[test]
    fn genius_applies_no_tier_exclusion() {
        let pick = registry()
            .best_for_task_in_mode(
                &hard_task(),
                CostPreference::Economy,
                ClutchProfile::Genius,
                |_| true,
            )
            .expect("a candidate");
        assert_eq!(
            pick.spec.id, "acme/flagship-9",
            "same inputs, no exclusion: the cheaper Elite wins"
        );
    }

    #[test]
    fn efficiency_falls_back_to_elite_only_when_it_is_the_only_candidate() {
        let mut r = ModelRegistry::default();
        r.register(spec(
            "acme/flagship-9",
            ProviderType::Ollama,
            ModelTier::Elite,
            0.0005,
        ));
        let pick = r
            .best_for_task_in_mode(
                &hard_task(),
                CostPreference::Economy,
                ClutchProfile::Efficiency,
                |_| true,
            )
            .expect("must not return None when the only candidate is Elite");
        assert_eq!(pick.spec.id, "acme/flagship-9");
        assert!(pick.only_candidate);
    }

    #[test]
    fn caller_predicate_still_applies_in_both_passes() {
        let pick = registry()
            .best_for_task_in_mode(
                &hard_task(),
                CostPreference::Economy,
                ClutchProfile::Efficiency,
                |m| m.capabilities.tier == ModelTier::Elite,
            )
            .expect("falls back to the Elite model the predicate allows");
        assert_eq!(pick.spec.id, "acme/flagship-9");
        assert!(pick.only_candidate);
    }

    #[test]
    fn a_provider_without_a_key_is_never_picked() {
        let mut r = ModelRegistry::default();
        // The keyless provider is far cheaper, so it would win without the gate.
        r.register(spec(
            "acme/cheap-cloud",
            ProviderType::OpenRouter,
            ModelTier::Fast,
            0.0001,
        ));
        r.register(spec(
            "acme/keyed-model",
            ProviderType::DeepSeek,
            ModelTier::Fast,
            0.02,
        ));
        set_test_key_availability(Some(vec![ProviderType::DeepSeek]));
        let pick = r.best_for_task_in_mode(
            &hard_task(),
            CostPreference::Economy,
            ClutchProfile::Efficiency,
            |_| true,
        );
        set_test_key_availability(None);
        assert_eq!(
            pick.map(|s| s.spec.id),
            Some("acme/keyed-model".to_string())
        );
    }

    #[test]
    fn with_no_cloud_keys_only_local_remains() {
        let mut r = ModelRegistry::default();
        r.register(spec(
            "acme/cheap-cloud",
            ProviderType::OpenRouter,
            ModelTier::Fast,
            0.0001,
        ));
        r.register(spec(
            "acme/local",
            ProviderType::Ollama,
            ModelTier::Fast,
            0.05,
        ));
        set_test_key_availability(Some(vec![]));
        let pick = r.best_for_task_in_mode(
            &hard_task(),
            CostPreference::Economy,
            ClutchProfile::Efficiency,
            |_| true,
        );
        set_test_key_availability(None);
        assert_eq!(pick.map(|s| s.spec.id), Some("acme/local".to_string()));
    }

    #[test]
    #[serial_test::file_serial]
    fn the_mode_ranking_explains_its_exclusions() {
        let mut r = registry();
        r.register(spec(
            "acme/keyless-9",
            ProviderType::OpenRouter,
            ModelTier::Fast,
            0.0001,
        ));
        set_test_key_availability(Some(vec![]));
        let pick = r.best_for_task_in_mode(
            &hard_task(),
            CostPreference::Economy,
            ClutchProfile::Efficiency,
            |_| true,
        );
        set_test_key_availability(None);
        let pick = pick.expect("a candidate");
        assert_eq!(
            pick.ranking.chosen().map(|m| m.id.clone()),
            Some(pick.spec.id.clone())
        );
        let why = |id: &str| {
            pick.ranking
                .excluded
                .iter()
                .find(|(x, _)| x == id)
                .map(|(_, e)| e.clone())
        };
        assert_eq!(
            why("acme/flagship-9"),
            Some(crate::models::ranking::Exclusion::FlagshipExcludedByMode)
        );
        assert_eq!(
            why("acme/keyless-9"),
            Some(crate::models::ranking::Exclusion::NoProviderKey)
        );
    }

    #[test]
    fn the_dispatch_gate_names_each_rule() {
        use crate::models::ranking::Exclusion;
        use crate::models::spec::PricingSource;
        let mut paid = spec("acme/paid", ProviderType::OpenRouter, ModelTier::Fast, 0.01);
        let mut unpriced = spec(
            "acme/unpriced",
            ProviderType::OpenRouter,
            ModelTier::Fast,
            0.01,
        );
        unpriced.pricing_source = PricingSource::Unknown;
        let free_only = super::DispatchGate {
            force_free_pool: true,
            ..Default::default()
        };
        assert_eq!(free_only.exclusion(&paid), Some(Exclusion::NotFree));
        let spent = super::DispatchGate {
            unknown_price_blocked: true,
            ..Default::default()
        };
        assert_eq!(
            spent.exclusion(&unpriced),
            Some(Exclusion::ExplorationBudgetSpent)
        );
        let budgets = super::DispatchGate {
            allowed_providers: Some(["deepseek".to_string()].into_iter().collect()),
            ..Default::default()
        };
        assert_eq!(
            budgets.exclusion(&paid),
            Some(Exclusion::ProviderBudgetExhausted)
        );
        let deepseek = spec("acme/ds", ProviderType::DeepSeek, ModelTier::Fast, 0.01);
        assert_eq!(budgets.exclusion(&deepseek), None);
        paid.is_free = true;
        assert_eq!(free_only.exclusion(&paid), None);
        assert_eq!(super::DispatchGate::default().exclusion(&unpriced), None);
    }

    #[test]
    fn free_mode_through_the_gate_never_chooses_a_paid_model() {
        let mut r = registry();
        let mut free = spec("acme/free-9", ProviderType::Ollama, ModelTier::Fast, 0.0);
        free.is_free = true;
        r.register(free);
        let gate = super::DispatchGate {
            force_free_pool: true,
            ..Default::default()
        };
        let pick = r
            .best_for_task_in_mode_keyed(
                &hard_task(),
                CostPreference::Economy,
                ClutchProfile::Free,
                &|_| true,
                |m| gate.exclusion(m),
            )
            .expect("the free model");
        assert_eq!(pick.spec.id, "acme/free-9");
        assert!(pick.ranking.excluded.iter().any(|(id, e)| {
            id == "acme/workhorse-9" && *e == crate::models::ranking::Exclusion::NotFree
        }));
    }

    #[test]
    #[serial_test::file_serial]
    fn the_keyed_variant_can_ignore_this_processs_keys() {
        let mut r = ModelRegistry::default();
        r.register(spec(
            "acme/cloud-9",
            ProviderType::OpenRouter,
            ModelTier::Fast,
            0.01,
        ));
        set_test_key_availability(Some(vec![]));
        let catalog = r.best_for_task_in_mode_keyed(
            &hard_task(),
            CostPreference::Economy,
            ClutchProfile::Efficiency,
            &|_| true,
            |_| None,
        );
        let this_process = r.best_for_task_in_mode(
            &hard_task(),
            CostPreference::Economy,
            ClutchProfile::Efficiency,
            |_| true,
        );
        set_test_key_availability(None);
        let catalog = catalog.expect("the catalog view finds the model");
        assert_eq!(catalog.spec.id, "acme/cloud-9");
        assert!(
            !catalog.only_candidate,
            "found on the first pass, not the flagship-free fallback"
        );
        assert!(this_process.is_none());
    }

    #[test]
    fn the_fallback_pass_returns_its_own_ranking() {
        let mut r = ModelRegistry::default();
        r.register(spec(
            "acme/flagship-9",
            ProviderType::Ollama,
            ModelTier::Elite,
            0.0005,
        ));
        let pick = r
            .best_for_task_in_mode(
                &hard_task(),
                CostPreference::Economy,
                ClutchProfile::Efficiency,
                |_| true,
            )
            .expect("the only candidate");
        assert!(pick.only_candidate);
        assert_eq!(
            pick.ranking.chosen().map(|m| m.id.as_str()),
            Some("acme/flagship-9")
        );
    }
}
