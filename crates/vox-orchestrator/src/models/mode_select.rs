//! Dispatch-path selection: Efficiency and Balanced never pick an Elite
//! (flagship) model while a non-Elite candidate fits, and a provider without
//! a resolvable key is never picked.

use crate::config::CostPreference;
use crate::mode::ClutchProfile;
use crate::models::key_guard::selection_key_available;
use crate::models::{ModelRegistry, ModelSpec, ModelTier};
use crate::types::AgentTask;

/// A dispatch-path pick. `only_candidate` is true when the mode's preferred set
/// was empty and the pick came from the unrestricted fallback pass.
#[derive(Debug, Clone)]
pub struct ModeSelection {
    pub spec: ModelSpec,
    pub only_candidate: bool,
}

fn excludes_elite(clutch: ClutchProfile) -> bool {
    matches!(clutch, ClutchProfile::Efficiency | ClutchProfile::Balanced)
}

impl ModelRegistry {
    /// Task dispatch entry point: [`Self::best_for_task_with_filter`] plus the clutch's tier
    /// guarantee and the provider-key gate.
    pub fn best_for_task_in_mode(
        &self,
        task: &AgentTask,
        preference: CostPreference,
        clutch: ClutchProfile,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModeSelection> {
        let mut eligible = |m: &ModelSpec| selection_key_available(&m.provider_type) && pred(m);
        if excludes_elite(clutch) {
            if let Some(spec) = self.best_for_task_with_filter(task, preference, |m| {
                m.capabilities.tier != ModelTier::Elite && eligible(m)
            }) {
                return Some(ModeSelection {
                    spec,
                    only_candidate: false,
                });
            }
            return self
                .best_for_task_with_filter(task, preference, eligible)
                .map(|spec| ModeSelection {
                    spec,
                    only_candidate: true,
                });
        }
        self.best_for_task_with_filter(task, preference, eligible)
            .map(|spec| ModeSelection {
                spec,
                only_candidate: false,
            })
    }
}

#[cfg(test)]
mod tests {
    use crate::config::CostPreference;
    use crate::mode::ClutchProfile;
    use crate::models::key_guard::set_test_key_availability;
    use crate::models::spec::PricingSource;
    use crate::models::{
        ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType, StrengthTag,
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

    fn hard_task() -> AgentTask {
        let mut t = AgentTask::new(TaskId(1), "hard codegen", TaskPriority::Normal, vec![]);
        t.task_category = TaskCategory::CodeGen;
        t.estimated_complexity = 10;
        t
    }

    /// <!-- AMENDED: R6 — the Elite model is the CHEAPEST with equal context, so without the
    /// guard it wins under Economy; the first fixture let the Fast model win regardless. -->
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
}
