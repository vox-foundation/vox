//! The selector's ranked, explained candidate list: the same filter chain and ordering as
//! `ModelRegistry::best_for_with_filter`, recording why each excluded model was excluded and
//! each candidate's `ScoreParts`. `best_for_with_filter` returns this ranking's first entry, so
//! an explanation cannot disagree with the choice.

use super::RoutingTask;
use super::scoring::{ScoreParts, auto_score_parts};
use super::spec::task_category_strength;
use super::{ModelRegistry, ModelSpec, ProviderType, StrengthTag, TaskCategory};
use crate::config::CostPreference;

/// Why a registered model was not a candidate.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Exclusion {
    /// Recently abstained on this task category.
    Penalized,
    /// Free models are skipped in performance mode.
    FreeInPerformanceMode,
    /// Estimated request cost is above the configured per-request safety cap.
    OverRequestCostCap,
    /// Estimated cost is above the task's own budget.
    OverTaskBudget,
    /// `VOX_ROUTE_*` policy excludes it.
    RoutePolicy,
    /// Local-only privacy mode excludes cloud models.
    PrivacyLocalOnly,
    /// Not suited to this task category.
    StrengthMismatch,
    /// The caller's own filter.
    Filtered,
    /// No resolvable key for its provider.
    NoProviderKey,
    /// The mode keeps flagships out while another model fits.
    FlagshipExcludedByMode,
    /// Free mode: only zero-cost models (`mode_select::DispatchGate`).
    NotFree,
    /// Today's exploration budget is spent, so unpriced models are skipped (`DispatchGate`).
    ExplorationBudgetSpent,
    /// Its provider has no usage budget left today (`DispatchGate`).
    ProviderBudgetExhausted,
    /// A newer member of its family is a candidate.
    Superseded { by: String },
}

/// A candidate and the parts of its routing score.
#[derive(Debug, Clone)]
pub struct RankedModel {
    pub spec: ModelSpec,
    pub parts: ScoreParts,
}

/// Candidates best-first, and every excluded model with its reason.
#[derive(Debug, Clone, Default)]
pub struct Ranking {
    pub ranked: Vec<RankedModel>,
    pub excluded: Vec<(String, Exclusion)>,
}

impl Ranking {
    /// The model routing picks: the first ranked candidate.
    #[must_use]
    pub fn chosen(&self) -> Option<&ModelSpec> {
        self.ranked.first().map(|r| &r.spec)
    }
}

impl ModelRegistry {
    /// The ranking behind [`Self::best_for_with_filter`], with the same two passes: penalties
    /// are respected unless that leaves nothing.
    pub fn rank_with_filter(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
        allow_free_in_performance_mode: bool,
        mut filter: impl FnMut(&ModelSpec) -> Option<Exclusion>,
        task: Option<&RoutingTask>,
    ) -> Ranking {
        let strength = task_category_strength(task_type);
        let first = self.rank_pass(
            task_type,
            strength,
            complexity,
            preference,
            allow_free_in_performance_mode,
            &mut filter,
            true,
            task,
        );
        if !first.ranked.is_empty() {
            return first;
        }
        self.rank_pass(
            task_type,
            strength,
            complexity,
            preference,
            allow_free_in_performance_mode,
            &mut filter,
            false,
            task,
        )
    }

    /// [`Self::rank_with_filter`] for a task, with [`Self::best_for_task_with_filter`]'s
    /// adjustments (research hints route as Research; two or more tool hints raise complexity to 7).
    pub fn rank_task_with_filter(
        &self,
        task: &RoutingTask,
        preference: CostPreference,
        filter: impl FnMut(&ModelSpec) -> Option<Exclusion>,
    ) -> Ranking {
        let mut complexity = task.complexity;
        let mut task_type = task.category;
        if !task.research_hints.is_empty() && task_type != TaskCategory::Research {
            task_type = TaskCategory::Research;
        }
        if task.tool_hints.len() >= 2 && complexity < 7 {
            complexity = 7;
        }
        self.rank_with_filter(task_type, complexity, preference, false, filter, Some(task))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn rank_pass(
        &self,
        task_type: TaskCategory,
        strength: StrengthTag,
        complexity: u8,
        preference: CostPreference,
        allow_free_in_performance_mode: bool,
        filter: &mut dyn FnMut(&ModelSpec) -> Option<Exclusion>,
        respect_penalties: bool,
        task: Option<&RoutingTask>,
    ) -> Ranking {
        let safety_cap = vox_config::load_model_routing_config()
            .safety
            .max_cost_usd_per_request;
        let privacy_local_only = crate::route_policy::inference_privacy_local_only_from_env();
        let scoreboard = self.scoreboard_snapshot();
        let mut excluded: Vec<(String, Exclusion)> = Vec::new();
        let mut candidates: Vec<&ModelSpec> = Vec::new();
        for m in self.models_iter() {
            let why = if respect_penalties && self.is_penalized(&m.id, task_type) {
                Some(Exclusion::Penalized)
            } else if preference == CostPreference::Performance
                && m.is_free
                && !allow_free_in_performance_mode
            {
                Some(Exclusion::FreeInPerformanceMode)
            } else if request_cost(m, task) > safety_cap {
                Some(Exclusion::OverRequestCostCap)
            } else if over_task_budget(m, task, scoreboard) {
                Some(Exclusion::OverTaskBudget)
            } else if !crate::route_policy::route_policy_allows_model(m) {
                Some(Exclusion::RoutePolicy)
            } else if !crate::route_policy::privacy_allows_model_for_mode(m, privacy_local_only) {
                Some(Exclusion::PrivacyLocalOnly)
            } else if !Self::matches_strength(m, strength) {
                Some(Exclusion::StrengthMismatch)
            } else {
                filter(m)
            };
            match why {
                Some(e) => excluded.push((m.id.clone(), e)),
                None => candidates.push(m),
            }
        }
        let newest = super::family::newest_per_family(candidates.iter().copied());
        let mut ranked: Vec<RankedModel> = Vec::new();
        for m in &candidates {
            if super::family::is_superseded(m, &newest) {
                let key = super::family::family_key(&m.id);
                let by = candidates
                    .iter()
                    .find(|c| {
                        super::family::family_key(&c.id) == key
                            && c.capabilities
                                .released_at
                                .map(|at| (at, super::family::version_tuple(&c.id)))
                                == newest.get(&key).cloned()
                    })
                    .map_or_else(String::new, |c| c.id.clone());
                excluded.push((m.id.clone(), Exclusion::Superseded { by }));
            } else {
                ranked.push(RankedModel {
                    spec: (*m).clone(),
                    parts: auto_score_parts(
                        m,
                        complexity,
                        false,
                        None,
                        preference,
                        None,
                        scoreboard.get(&m.id),
                    ),
                });
            }
        }
        ranked.sort_by(|a, b| self.rank_order(b, a));
        Ranking { ranked, excluded }
    }

    /// `Greater` when `a` should be chosen over `b`: score, then cheaper, then higher observed
    /// success, then lower observed latency, then mesh when mesh is preferred, then smaller id.
    fn rank_order(&self, a: &RankedModel, b: &RankedModel) -> std::cmp::Ordering {
        let sb = self.scoreboard_snapshot();
        let success = |m: &ModelSpec| sb.get(&m.id).map(|s| s.success_rate).unwrap_or(0.5);
        let latency = |m: &ModelSpec| sb.get(&m.id).and_then(|s| s.p50_latency_ms).unwrap_or(2000);
        a.parts
            .total
            .total_cmp(&b.parts.total)
            .then_with(|| b.spec.cost_per_1k.total_cmp(&a.spec.cost_per_1k))
            .then_with(|| success(&a.spec).total_cmp(&success(&b.spec)))
            .then_with(|| latency(&b.spec).cmp(&latency(&a.spec)))
            .then_with(|| {
                let prefer_mesh =
                    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxRoutingPreferMesh)
                        .expose()
                        .map(|s: &str| s.trim() == "true")
                        .unwrap_or(false);
                if prefer_mesh {
                    (a.spec.provider_type == ProviderType::PopuliMesh)
                        .cmp(&(b.spec.provider_type == ProviderType::PopuliMesh))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then_with(|| b.spec.id.cmp(&a.spec.id))
    }
}

fn request_cost(m: &ModelSpec, task: Option<&RoutingTask>) -> f64 {
    let est_tokens = task.map(|t| t.estimated_token_count()).unwrap_or(1024) as f64;
    let per_1k = if m.cost_per_1k_input > 0.0 || m.cost_per_1k_output > 0.0 {
        (m.cost_per_1k_input + m.cost_per_1k_output) / 2.0
    } else {
        m.cost_per_1k
    };
    (est_tokens / 1000.0) * per_1k
}

fn over_task_budget(
    m: &ModelSpec,
    task: Option<&RoutingTask>,
    scoreboard: &std::collections::HashMap<String, super::ModelScore>,
) -> bool {
    let Some(t) = task else { return false };
    let Some(max) = t.max_cost_usd else {
        return false;
    };
    let basis = scoreboard
        .get(&m.id)
        .and_then(|s| s.cost_per_success_usd)
        .unwrap_or(m.cost_per_1k);
    (t.estimated_token_count() as f64 / 1000.0) * basis > max
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::spec::PricingSource;
    use crate::models::{ModelCapabilities, StrengthTag};

    fn spec(
        id: &str,
        provider_type: ProviderType,
        cost: f64,
        is_free: bool,
        strengths: Vec<StrengthTag>,
    ) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 64_000,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free,
            observed_cost_per_1k: None,
            strengths,
            capabilities: ModelCapabilities::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    fn reason(r: &Ranking, id: &str) -> Option<Exclusion> {
        r.excluded
            .iter()
            .find(|(x, _)| x == id)
            .map(|(_, e)| e.clone())
    }

    #[test]
    #[serial_test::file_serial]
    fn the_ranking_agrees_with_best_for_on_the_bootstrap_registry() {
        let reg = ModelRegistry::new();
        for task in [
            TaskCategory::CodeGen,
            TaskCategory::Research,
            TaskCategory::General,
        ] {
            for pref in [CostPreference::Economy, CostPreference::Performance] {
                for complexity in [2u8, 5, 9] {
                    let chosen =
                        reg.best_for_with_filter(task, complexity, pref, false, |_| true, None);
                    let ranking =
                        reg.rank_with_filter(task, complexity, pref, false, |_| None, None);
                    let top = ranking.ranked.first().map(|r| r.parts.total);
                    let chosen_score = chosen.as_ref().map(|m| {
                        crate::models::scoring::auto_score_model(
                            m,
                            complexity,
                            false,
                            None,
                            pref,
                            None,
                            reg.scoreboard_snapshot().get(&m.id),
                        )
                    });
                    assert_eq!(chosen_score, top, "{task:?} {pref:?} cx{complexity}");
                }
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn ranked_models_are_in_descending_score_order_ties_by_id() {
        let mut reg = ModelRegistry::default();
        reg.register(spec(
            "acme/b-twin",
            ProviderType::Ollama,
            0.01,
            false,
            vec![StrengthTag::Generalist],
        ));
        reg.register(spec(
            "acme/a-twin",
            ProviderType::Ollama,
            0.01,
            false,
            vec![StrengthTag::Generalist],
        ));
        reg.register(spec(
            "acme/pricey",
            ProviderType::Ollama,
            0.9,
            false,
            vec![StrengthTag::Generalist],
        ));
        let r = reg.rank_with_filter(
            TaskCategory::CodeGen,
            5,
            CostPreference::Economy,
            false,
            |_| None,
            None,
        );
        for w in r.ranked.windows(2) {
            assert!(w[0].parts.total >= w[1].parts.total);
        }
        let ids: Vec<&str> = r.ranked.iter().map(|m| m.spec.id.as_str()).collect();
        assert_eq!(
            &ids[..2],
            &["acme/a-twin", "acme/b-twin"],
            "exact ties resolve by id"
        );
    }

    #[test]
    fn every_model_is_ranked_or_excluded_exactly_once() {
        let reg = ModelRegistry::new();
        let r = reg.rank_with_filter(
            TaskCategory::CodeGen,
            5,
            CostPreference::Performance,
            false,
            |_| None,
            None,
        );
        let mut seen: Vec<String> = r
            .ranked
            .iter()
            .map(|m| m.spec.id.clone())
            .chain(r.excluded.iter().map(|(id, _)| id.clone()))
            .collect();
        seen.sort();
        let mut all: Vec<String> = reg.list_models().into_iter().map(|m| m.id).collect();
        all.sort();
        assert_eq!(seen, all);
    }

    #[test]
    fn exclusions_name_their_reason() {
        let mut reg = ModelRegistry::default();
        reg.register(spec(
            "acme/ok",
            ProviderType::Ollama,
            0.01,
            false,
            vec![StrengthTag::Generalist],
        ));
        reg.register(spec(
            "acme/free",
            ProviderType::Ollama,
            0.0,
            true,
            vec![StrengthTag::Generalist],
        ));
        reg.register(spec(
            "acme/vision-only",
            ProviderType::Ollama,
            0.01,
            false,
            vec![StrengthTag::Vision],
        ));
        reg.register(spec(
            "acme/huge",
            ProviderType::Ollama,
            1.0e6,
            false,
            vec![StrengthTag::Generalist],
        ));
        reg.register(spec(
            "acme/dropped",
            ProviderType::Ollama,
            0.01,
            false,
            vec![StrengthTag::Generalist],
        ));
        reg.record_penalty(
            "acme/ok-2".into(),
            TaskCategory::CodeGen,
            std::time::Duration::from_secs(600),
        );
        reg.register(spec(
            "acme/ok-2",
            ProviderType::Ollama,
            0.01,
            false,
            vec![StrengthTag::Generalist],
        ));
        let r = reg.rank_with_filter(
            TaskCategory::CodeGen,
            5,
            CostPreference::Performance,
            false,
            |m| (m.id == "acme/dropped").then_some(Exclusion::Filtered),
            None,
        );
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/ok"));
        assert_eq!(
            reason(&r, "acme/free"),
            Some(Exclusion::FreeInPerformanceMode)
        );
        assert_eq!(
            reason(&r, "acme/vision-only"),
            Some(Exclusion::StrengthMismatch)
        );
        assert_eq!(reason(&r, "acme/huge"), Some(Exclusion::OverRequestCostCap));
        assert_eq!(reason(&r, "acme/dropped"), Some(Exclusion::Filtered));
        assert_eq!(reason(&r, "acme/ok-2"), Some(Exclusion::Penalized));
    }

    #[test]
    fn a_superseded_member_names_its_successor() {
        let mut reg = ModelRegistry::default();
        let mut old = spec(
            "acme/widget-4.8",
            ProviderType::Ollama,
            0.01,
            false,
            vec![StrengthTag::Generalist],
        );
        old.capabilities.released_at = Some(1_700_000_000);
        let mut new = spec(
            "acme/widget-5.5",
            ProviderType::Ollama,
            0.02,
            false,
            vec![StrengthTag::Generalist],
        );
        new.capabilities.released_at = Some(1_760_000_000);
        reg.register(old);
        reg.register(new);
        let r = reg.rank_with_filter(
            TaskCategory::CodeGen,
            5,
            CostPreference::Economy,
            false,
            |_| None,
            None,
        );
        assert_eq!(
            reason(&r, "acme/widget-4.8"),
            Some(Exclusion::Superseded {
                by: "acme/widget-5.5".into()
            })
        );
    }

    #[test]
    fn exact_ties_resolve_by_id_ascending() {
        let reg = ModelRegistry::default();
        let parts = auto_score_parts(
            &spec(
                "acme/any",
                ProviderType::Ollama,
                0.01,
                false,
                vec![StrengthTag::Generalist],
            ),
            5,
            false,
            None,
            CostPreference::Economy,
            None,
            None,
        );
        let twin = |id: &str| RankedModel {
            spec: spec(
                id,
                ProviderType::Ollama,
                0.01,
                false,
                vec![StrengthTag::Generalist],
            ),
            parts,
        };
        let (a, b) = (twin("acme/a-twin"), twin("acme/b-twin"));
        assert_eq!(
            reg.rank_order(&a, &b),
            std::cmp::Ordering::Greater,
            "the smaller id is chosen"
        );
        assert_eq!(reg.rank_order(&b, &a), std::cmp::Ordering::Less);
    }

    #[test]
    fn the_ranking_agrees_with_best_for_on_every_gate() {
        use crate::types::{AgentTask, Budget, TaskId, TaskPriority};
        fn agree(
            reg: &ModelRegistry,
            pref: CostPreference,
            reject: &str,
            task: Option<&RoutingTask>,
        ) -> Ranking {
            let chosen = reg.best_for_with_filter(
                TaskCategory::CodeGen,
                5,
                pref,
                false,
                |m| m.id != reject,
                task,
            );
            let ranking = reg.rank_with_filter(
                TaskCategory::CodeGen,
                5,
                pref,
                false,
                |m| (m.id == reject).then_some(Exclusion::Filtered),
                task,
            );
            assert_eq!(chosen.map(|m| m.id), ranking.chosen().map(|m| m.id.clone()));
            ranking
        }
        let g = || vec![StrengthTag::Generalist];

        // 1. Supersession: the older member is cheaper, so it would win on cost.
        let mut reg = ModelRegistry::default();
        let mut old = spec("acme/widget-4.8", ProviderType::Ollama, 0.001, false, g());
        old.capabilities.released_at = Some(1_700_000_000);
        let mut new = spec("acme/widget-5.5", ProviderType::Ollama, 0.02, false, g());
        new.capabilities.released_at = Some(1_760_000_000);
        reg.register(old);
        reg.register(new);
        let r = agree(&reg, CostPreference::Economy, "", None);
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/widget-5.5"));

        // 2. The caller's filter rejects the model that would win.
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/cheap", ProviderType::Ollama, 0.001, false, g()));
        reg.register(spec("acme/dear", ProviderType::Ollama, 0.02, false, g()));
        let r = agree(&reg, CostPreference::Economy, "acme/cheap", None);
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/dear"));
        assert_eq!(reason(&r, "acme/cheap"), Some(Exclusion::Filtered));

        // 3. The task budget excludes the pricier model.
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/cheap", ProviderType::Ollama, 0.0001, false, g()));
        reg.register(spec("acme/big", ProviderType::Ollama, 0.5, false, g()));
        let mut task = AgentTask::new(TaskId(1), "budgeted", TaskPriority::Normal, vec![]);
        task.budget = Some(Budget {
            max_cost_usd: Some(0.001),
            max_latency_ms: None,
        });
        let rt = RoutingTask::from(&task);
        let r = agree(&reg, CostPreference::Performance, "", Some(&rt));
        assert_eq!(reason(&r, "acme/big"), Some(Exclusion::OverTaskBudget));

        // 4. Every model is penalised: the second pass must still choose.
        let mut reg = ModelRegistry::default();
        reg.register(spec("acme/only", ProviderType::Ollama, 0.01, false, g()));
        reg.record_penalty(
            "acme/only".into(),
            TaskCategory::CodeGen,
            std::time::Duration::from_secs(600),
        );
        let r = agree(&reg, CostPreference::Economy, "", None);
        assert_eq!(r.chosen().map(|m| m.id.as_str()), Some("acme/only"));
    }
}
