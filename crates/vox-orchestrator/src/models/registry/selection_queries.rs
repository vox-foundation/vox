use super::*;

impl ModelRegistry {
    /// Register a new model specification.
    pub fn register(&mut self, mut spec: ModelSpec) {
        if spec.capabilities.tier == super::ModelTier::Unknown
            && !matches!(
                spec.provider_type,
                super::ProviderType::Ollama
                    | super::ProviderType::PopuliMesh
                    | super::ProviderType::VoxLocal
            )
        {
            spec.capabilities.tier = super::tiering::derive_tier_with(
                spec.is_free,
                spec.cost_per_1k_output,
                self.reference.elite_min_out,
                self.reference.pro_min_out,
            );
        }
        spec.capabilities.quality_prior = Some(self.reference.quality_prior(&spec, None));
        use super::spec::PricingSource;
        if let Some(existing) = self.models.get(&spec.id) {
            if existing.pricing_source == PricingSource::Telemetry {
                // Merge non-pricing fields only
                let mut merged = existing.clone();
                merged.capabilities = spec.capabilities;
                merged.strengths = spec.strengths;
                merged.max_tokens = spec.max_tokens;
                merged.supported_parameters = spec.supported_parameters;
                self.models.insert(spec.id.clone(), merged);
                return;
            }
        }
        self.models.insert(spec.id.clone(), spec);
    }

    /// Make a live Ollama listing authoritative for Ollama entries: drop every Ollama model
    /// not in `installed` (bootstrap suggestions the user never pulled would otherwise win
    /// free routing and fail at dispatch). `None` means Ollama was unreachable, so no Ollama
    /// entry can serve. Call [`Self::apply_routing_reference`] afterwards.
    pub fn reconcile_ollama(&mut self, installed: Option<&[String]>) {
        self.models.retain(|id, m| {
            m.provider_type != ProviderType::Ollama
                || installed.is_some_and(|names| names.iter().any(|n| n == id))
        });
    }

    /// Reconcile local model endpoints (Ollama and VoxLocal): drop unreachable local models
    /// so routing cascades to keyed cloud providers (e.g. OpenRouter) instead of failing at dispatch.
    pub fn reconcile_local_endpoints(
        &mut self,
        ollama_installed: Option<&[String]>,
        vox_local_reachable: bool,
    ) {
        self.reconcile_ollama(ollama_installed);
        if !vox_local_reachable {
            self.models.retain(|_, m| {
                !matches!(
                    m.provider_type,
                    ProviderType::VoxLocal | ProviderType::PopuliMesh
                )
            });
        }
        self.apply_routing_reference();
    }

    /// The routing reference this registry scores against.
    #[must_use]
    pub fn routing_reference(&self) -> super::reference::RoutingReference {
        self.reference
    }

    /// Re-derive the routing reference from the registered models, restamp every model's
    /// quality prior, and re-derive the tier of every model whose price came from a live
    /// catalog or from observed spend (an entry no catalog prices, and an unknown price, keep
    /// their tier). Call after every catalog merge and after injecting observed prices.
    pub fn apply_routing_reference(&mut self) -> super::reference::RoutingReference {
        use super::spec::PricingSource;
        let reference = super::reference::RoutingReference::derive(self.models.values());
        self.reference = reference;
        let family = super::family::family_benchmarks(self.models.values());
        for spec in self.models.values_mut() {
            let local = matches!(
                spec.provider_type,
                ProviderType::Ollama | ProviderType::PopuliMesh | ProviderType::VoxLocal
            );
            let inherited = if local || spec.capabilities.intelligence_index.is_some() {
                None
            } else {
                family
                    .get(&super::family::join_key(spec))
                    .filter(|(_, from)| *from != spec.id)
                    .map(|(index, from)| (*index, from.as_str()))
            };
            let mut prior = reference.quality_prior(spec, inherited);
            if let Some(ev) = self.bench.get(&spec.id).filter(|ev| ev.cases > 0) {
                prior = super::spec::QualityPrior {
                    value: ev.blended_quality(prior.value),
                    source: super::spec::QualitySource::Measured {
                        score: ev.mean_score as f32,
                        cases: ev.cases.min(i64::from(u32::MAX)) as u32,
                        prior: prior.value as f32,
                    },
                };
            }
            spec.capabilities.quality_prior = Some(prior);
            let priced = matches!(
                spec.pricing_source,
                PricingSource::OpenRouter
                    | PricingSource::LiteLLM
                    | PricingSource::AnthropicDirect
                    | PricingSource::Telemetry
            );
            if priced && !local {
                let tier = super::tiering::derive_tier_with(
                    spec.is_free,
                    spec.cost_per_1k_output,
                    reference.elite_min_out,
                    reference.pro_min_out,
                );
                if tier != super::ModelTier::Unknown {
                    spec.capabilities.tier = tier;
                }
            }
        }
        reference
    }

    pub(crate) fn models_iter(&self) -> impl Iterator<Item = &ModelSpec> {
        self.models.values()
    }

    /// Remove all registered models. Useful in tests to isolate a clean registry from
    /// any on-disk cache that `new()` seeds automatically.
    pub fn clear(&mut self) {
        self.models.clear();
    }

    pub fn best_for_task(
        &self,
        task: &RoutingTask,
        preference: CostPreference,
    ) -> Option<ModelSpec> {
        self.best_for_task_with_filter(task, preference, |_| true)
    }

    /// Like [`Self::best_for_task`] but only considers models for which `pred` returns true.
    pub fn best_for_task_with_filter(
        &self,
        task: &RoutingTask,
        preference: CostPreference,
        pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModelSpec> {
        let mut pred = pred;
        self.rank_task_with_filter(task, preference, |m| {
            (!pred(m)).then_some(super::ranking::Exclusion::Filtered)
        })
        .chosen()
        .cloned()
    }

    /// Return the best model for a given task category and complexity.
    pub fn best_for(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
    ) -> Option<ModelSpec> {
        self.best_for_with_filter(task_type, complexity, preference, false, |_| true, None)
    }

    /// Like [`Self::best_for`] but only considers models for which `pred` returns true.
    #[must_use]
    pub fn best_for_with_filter(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
        allow_free_in_performance_mode: bool,
        mut pred: impl FnMut(&ModelSpec) -> bool,
        task: Option<&RoutingTask>,
    ) -> Option<ModelSpec> {
        self.rank_with_filter(
            task_type,
            complexity,
            preference,
            allow_free_in_performance_mode,
            |m| (!pred(m)).then_some(super::ranking::Exclusion::Filtered),
            task,
        )
        .ranked
        .into_iter()
        .next()
        .map(|r| r.spec)
    }

    /// Return all models matching the criteria, sorted by the effective score (priority order).
    ///
    /// `complexity` (1-10) must flow through to the same [`super::scoring::auto_score_model`]
    /// used by the canonical [`super::select::select`] path — this is the function
    /// `vox model explain` renders, and it must rank higher-complexity/precision-favoring
    /// tasks differently from trivial ones (F3a fix: previously this sorted purely by
    /// ascending `cost_per_1k`, completely ignoring `complexity` and the caller's task
    /// signal, which made `explain` byte-identical across trivial and hard tasks).
    pub fn explain_selection(
        &self,
        _task_type: TaskCategory,
        strength: crate::models::StrengthTag,
        complexity: u8,
        preference: crate::config::CostPreference,
    ) -> Vec<ModelSpec> {
        let mut candidates: Vec<ModelSpec> = self
            .models
            .values()
            .filter(|m| {
                // Task 2.1b: `Performance` preference historically excluded every
                // free model outright, at any complexity — including trivial
                // low-complexity tasks where a free local (e.g. Ollama) model is
                // plausibly the *right* choice and should at least be scored and
                // shown, not silently dropped pre-scoring. Gate the exclusion by
                // complexity: below `COMPLEXITY_HIGH_CUTOFF` free models remain
                // eligible for `explain_selection`'s ranking (still subject to
                // `auto_score_model`'s normal scoring, including the regression
                // guard against a free model winning purely on cost). At/above
                // the cutoff the original "Performance never means free" rule
                // is preserved unchanged.
                //
                // This mirrors `best_for_with_filter`'s existing low-complexity
                // preference-downgrade logic (see `effective_pref` above), but is
                // deliberately scoped to `explain_selection` only —
                // `rank_pass` (real production routing) doesn't thread
                // `complexity` through this filter today and changing that is out
                // of scope for this fix.
                if preference == crate::config::CostPreference::Performance
                    && m.is_free
                    && complexity >= super::scoring::COMPLEXITY_HIGH_CUTOFF
                {
                    return false;
                }
                if !crate::route_policy::route_policy_allows_model(m) {
                    return false;
                }
                // Code-review fix: this ranking backs `vox model explain`, which
                // must reflect the same eligibility as the real selection path
                // (rank_pass) — otherwise `explain` shows cloud models
                // as viable/ranked even under VOX_INFERENCE_PRIVACY=local_only.
                if !crate::route_policy::privacy_allows_model_for_mode(
                    m,
                    crate::route_policy::inference_privacy_local_only_from_env(),
                ) {
                    return false;
                }
                Self::matches_strength(m, strength)
            })
            .cloned()
            .collect();

        // Score each model once: rescoring inside the comparator reads shared scoring
        // weights that can change mid-sort, which breaks total order (and panics).
        let mut scored: Vec<(f64, ModelSpec)> = candidates
            .drain(..)
            .map(|m| {
                let score = super::scoring::auto_score_model(
                    &m,
                    complexity,
                    false,
                    None,
                    preference,
                    None,
                    self.scoreboard.get(&m.id),
                );
                (score, m)
            })
            .collect();
        // Descending: highest score first.
        scored.sort_by(|(sa, a), (sb, b)| sb.total_cmp(sa).then_with(|| a.id.cmp(&b.id)));

        scored.into_iter().map(|(_, m)| m).collect()
    }

    /// Models rejected by [crate::route_policy] under current VOX_ROUTE_* env (stable sort by id).
    #[must_use]
    pub fn explain_route_policy_exclusions(&self) -> Vec<(String, &'static str)> {
        let mut out: Vec<(String, &'static str)> = self
            .models
            .values()
            .filter_map(|m| {
                crate::route_policy::route_policy_exclusion_reason(m)
                    .map(|reason| (m.id.clone(), reason))
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Return the best free model for a given task category.
    pub fn best_free_for(&self, task_type: TaskCategory) -> Option<ModelSpec> {
        let strength = task_category_strength(task_type);

        self.models
            .values()
            .filter(|m| m.is_free && Self::matches_strength(m, strength))
            .max_by_key(|m| m.max_tokens)
            .cloned()
            .or_else(|| self.cheapest_free())
    }

    /// Like [`Self::best_free_for`] but only considers models for which `pred` returns true.
    #[must_use]
    pub fn best_free_for_with_filter(
        &self,
        task_type: TaskCategory,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModelSpec> {
        let strength = task_category_strength(task_type);

        self.models
            .values()
            .filter(|m| m.is_free && Self::matches_strength(m, strength) && pred(m))
            .max_by_key(|m| m.max_tokens)
            .cloned()
            .or_else(|| self.cheapest_free_with_filter(&mut pred))
    }

    /// Return all free models in the registry.
    pub fn free_models(&self) -> Vec<ModelSpec> {
        self.models
            .values()
            .filter(|m| m.is_free)
            .cloned()
            .collect()
    }

    /// Return the cheapest free model.
    pub fn cheapest_free(&self) -> Option<ModelSpec> {
        self.models
            .values()
            .filter(|m| m.is_free)
            .min_by(|a, b| {
                a.cost_per_1k
                    .total_cmp(&b.cost_per_1k)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .cloned()
    }

    /// Like [`Self::cheapest_free`] but only considers models for which `pred` returns true.
    #[must_use]
    pub fn cheapest_free_with_filter(
        &self,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModelSpec> {
        self.models
            .values()
            .filter(|m| m.is_free && pred(m))
            .min_by(|a, b| {
                a.cost_per_1k
                    .total_cmp(&b.cost_per_1k)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .cloned()
    }

    /// Return the absolute cheapest model in the registry.
    pub fn cheapest(&self) -> Option<ModelSpec> {
        self.models
            .values()
            .min_by(|a, b| a.cost_per_1k.total_cmp(&b.cost_per_1k))
            .cloned()
    }

    /// Like [`Self::cheapest`] but only considers models for which `pred` returns true.
    #[must_use]
    pub fn cheapest_with_filter(
        &self,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModelSpec> {
        self.models
            .values()
            .filter(|m| pred(m))
            .min_by(|a, b| {
                a.cost_per_1k
                    .total_cmp(&b.cost_per_1k)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .cloned()
    }

    /// Calculate the cost estimate for predicting use of a model for a certain amount of tokens.
    pub fn cost_estimate(&self, model_id: &str, estimated_tokens: u64) -> Option<f64> {
        self.models
            .get(model_id)
            .map(|spec| (estimated_tokens as f64 / 1000.0) * spec.cost_per_1k)
    }

    /// List all registered models.
    pub fn list_models(&self) -> Vec<ModelSpec> {
        self.models.values().cloned().collect()
    }

    /// Get a specific model definition by ID.
    pub fn get(&self, model_id: &str) -> Option<ModelSpec> {
        let key = model_id.trim();
        if let Some(m) = self.models.get(key) {
            return Some(m.clone());
        }
        if key.starts_with('~') {
            return self.resolve_live_alias(key);
        }
        let stripped_key = key.split_once('/').map_or(key, |(_, suffix)| suffix);
        let slug_suffix = format!("/{stripped_key}");
        if let Some(m) = self.models.values().find(|m| {
            m.id == key
                || m.canonical_slug == key
                || m.id == stripped_key
                || m.canonical_slug == stripped_key
                || m.id.ends_with(&slug_suffix)
                || m.canonical_slug.ends_with(&slug_suffix)
        }) {
            return Some(m.clone());
        }

        // Support hierarchical run keys e.g. "mens/runs/foo" -> "mens/foo" or "vox-local/mens/runs/foo"
        let norm_key = key
            .strip_prefix("vox-local/")
            .or_else(|| key.strip_prefix("voxlocal/"))
            .unwrap_or(key);
        let norm_key = if let Some(rest) = norm_key.strip_prefix("mens/runs/") {
            format!("mens/{rest}")
        } else if let Some(rest) = norm_key.strip_prefix("runs/") {
            format!("mens/{rest}")
        } else {
            norm_key.to_string()
        };

        if let Some(m) = self.models.get(&norm_key) {
            return Some(m.clone());
        }
        if let Some(m) = self
            .models
            .values()
            .find(|m| m.id == norm_key || m.canonical_slug == norm_key)
        {
            return Some(m.clone());
        }

        // If key points to a sub-checkpoint or subpath (e.g. "mens/foo/checkpoint-500"),
        // check if parent run "mens/foo" exists in registry and synthesize sub-checkpoint spec
        if let Some((parent, _sub)) = norm_key.rsplit_once('/') {
            if let Some(parent_spec) = self.get(parent) {
                let mut sub_spec = parent_spec;
                sub_spec.id = norm_key.clone();
                sub_spec.canonical_slug = norm_key;
                return Some(sub_spec);
            }
        }
        None
    }

    /// Lookup the premium-alias model id for a routing key
    /// (see `contracts/orchestration/model-routing.v1.yaml`).
    /// Used by `select()` to honor task-pinned premium aliases.
    pub fn premium_alias_for(&self, key: &str) -> Option<&str> {
        self.premium_alias.get(key).map(String::as_str)
    }

    /// Pin a routing key to a concrete model id (contract pins are live aliases).
    #[cfg(test)]
    pub(crate) fn set_premium_alias_for_test(&mut self, key: &str, model_id: &str) {
        self.premium_alias
            .insert(key.to_string(), model_id.to_string());
    }

    /// Set an explicit model override for a specific agent.
    pub fn set_override(&mut self, agent_id: u64, model_id: String) {
        self.agent_overrides.insert(agent_id, model_id);
    }

    /// Check if there's an active model override for an agent.
    pub fn get_override(&self, agent_id: u64) -> Option<String> {
        self.agent_overrides.get(&agent_id).cloned()
    }

    /// Apply verified `CommunityIntelligenceBundleV1` priors using bounded Empirical Bayes blending
    /// ($w_{\text{local}} = \frac{n_{\text{local}}}{n_{\text{local}} + 10}$, clamped to $\pm 15\%$ max delta) (`INTEL-03`, `INTEL-04`, `ROUTE-LEARN-01`).
    ///
    /// Returns the number of registered models updated. If the bundle fails `Ed25519` or $k \ge 20$
    /// verification, or if `VOX_COMMUNITY_INTELLIGENCE=off`, returns `0` without mutating any state (`INTEL-05`).
    pub fn apply_community_intelligence_bundle(
        &mut self,
        bundle: &super::intelligence_bundle::CommunityIntelligenceBundleV1,
    ) -> usize {
        if !super::intelligence_bundle::is_community_intelligence_enabled() {
            return 0;
        }
        if bundle.verify_bundle_signature().is_err() {
            return 0;
        }
        let mut updated = 0usize;
        for model_id in self.models.keys().cloned().collect::<Vec<_>>() {
            let model_hash = super::intelligence_bundle::model_id_blake3_hex(&model_id);
            let matching: Vec<&super::intelligence_bundle::ModelRoutingPriorRowV1> = bundle
                .model_routing_priors
                .iter()
                .filter(|r| r.model_id_hash == model_hash)
                .collect();
            let matching_bench: Vec<&super::intelligence_bundle::ModelBenchSpokePriorRowV1> =
                bundle
                    .model_bench_spoke_priors
                    .iter()
                    .filter(|r| r.model_id_hash == model_hash)
                    .collect();
            if matching.is_empty() && matching_bench.is_empty() {
                continue;
            }
            let entry = self.scoreboard.entry(model_id.clone()).or_default();
            let local_n = entry.n_calls.max(0) as u64;

            if !matching.is_empty() {
                let community_rate = matching
                    .iter()
                    .map(|r| r.empirical_success_rate)
                    .sum::<f64>()
                    / (matching.len() as f64);
                let base_local_rate = if entry.n_calls > 0 {
                    entry.success_rate
                } else {
                    0.50
                };
                entry.success_rate = super::intelligence_bundle::blend_empirical_bayes_prior(
                    base_local_rate,
                    local_n,
                    community_rate,
                );
            }

            if !matching_bench.is_empty() {
                let community_pass = matching_bench
                    .iter()
                    .map(|r| r.empirical_pass_rate)
                    .sum::<f64>()
                    / (matching_bench.len() as f64);
                let base_local_q = if entry.n_calls > 0
                    && entry.quality_score > 0.0
                    && (entry.quality_score - 1.0).abs() > 1e-6
                {
                    entry.quality_score
                } else {
                    0.50
                };
                entry.quality_score = super::intelligence_bundle::blend_empirical_bayes_prior(
                    base_local_q,
                    local_n,
                    community_pass,
                )
                .clamp(0.05, 0.95);
            }

            if entry.n_calls == 0 {
                entry.n_calls = 1;
                entry.success_count = i64::from(entry.success_rate >= 0.5);
                let arm = self.arm_stats.entry(model_id).or_insert((0, 0));
                if *arm == (0, 0) {
                    let s = (entry.success_rate * 4.0).round().clamp(1.0, 4.0) as u32;
                    *arm = (s, 4u32.saturating_sub(s));
                }
            }
            updated += 1;
        }
        updated
    }

    /// Record a pairwise Arena preference outcome (`winner_outcome`: `1.0` if `model_a` won,
    /// `0.0` if `model_b` won, `0.5` on tie) using an online Bradley-Terry step (`K = 0.12`)
    /// and updating contextual bandit `arm_stats` Beta posteriors (`ROUTE-LEARN-03`).
    ///
    /// Returns `(new_quality_a, new_quality_b)`.
    pub fn record_arena_preference_outcome(
        &mut self,
        model_a: &str,
        model_b: &str,
        outcome_a: f64,
    ) -> (f64, f64) {
        const BT_STEP_K: f64 = 0.12;
        let s_a = outcome_a.clamp(0.0, 1.0);
        let s_b = 1.0 - s_a;

        let cur_q = |entry: &super::ModelScore| -> f64 {
            if entry.n_calls > 0
                && entry.quality_score > 0.0
                && (entry.quality_score - 1.0).abs() > 1e-6
            {
                entry.quality_score.clamp(0.05, 0.95)
            } else {
                0.50
            }
        };

        let qa_old = self.scoreboard.get(model_a).map_or(0.50, cur_q);
        let qb_old = self.scoreboard.get(model_b).map_or(0.50, cur_q);
        let denom = (qa_old + qb_old).max(1e-6);
        let expected_a = qa_old / denom;
        let expected_b = qb_old / denom;

        let qa_new = (qa_old + BT_STEP_K * (s_a - expected_a)).clamp(0.05, 0.95);
        let qb_new = (qb_old + BT_STEP_K * (s_b - expected_b)).clamp(0.05, 0.95);

        {
            let entry_a = self.scoreboard.entry(model_a.to_string()).or_default();
            let prev_n = entry_a.n_calls.max(0);
            entry_a.n_calls = prev_n + 1;
            if s_a >= 0.5 {
                entry_a.success_count += 1;
            }
            entry_a.success_rate = if prev_n == 0 {
                s_a.max(0.5)
            } else {
                ((entry_a.success_rate * prev_n as f64) + s_a) / (entry_a.n_calls as f64)
            };
            entry_a.quality_score = qa_new;
        }
        {
            let entry_b = self.scoreboard.entry(model_b.to_string()).or_default();
            let prev_n = entry_b.n_calls.max(0);
            entry_b.n_calls = prev_n + 1;
            if s_b >= 0.5 {
                entry_b.success_count += 1;
            }
            entry_b.success_rate = if prev_n == 0 {
                s_b.max(0.5)
            } else {
                ((entry_b.success_rate * prev_n as f64) + s_b) / (entry_b.n_calls as f64)
            };
            entry_b.quality_score = qb_new;
        }

        let arm_a = self.arm_stats.entry(model_a.to_string()).or_insert((1, 1));
        if s_a > 0.5 {
            arm_a.0 = arm_a.0.saturating_add(2);
        } else if s_a < 0.5 {
            arm_a.1 = arm_a.1.saturating_add(1);
        } else {
            arm_a.0 = arm_a.0.saturating_add(1);
        }

        let arm_b = self.arm_stats.entry(model_b.to_string()).or_insert((1, 1));
        if s_b > 0.5 {
            arm_b.0 = arm_b.0.saturating_add(2);
        } else if s_b < 0.5 {
            arm_b.1 = arm_b.1.saturating_add(1);
        } else {
            arm_b.0 = arm_b.0.saturating_add(1);
        }

        (qa_new, qb_new)
    }

    /// Builds a [`vox_actor_runtime::llm::LlmConfig`] for the best matching model when the `runtime` feature is on.
    #[cfg(feature = "runtime")]
    pub fn get_llm_config(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
    ) -> Option<vox_actor_runtime::llm::LlmConfig> {
        self.best_for(task_type, complexity, preference)
            .map(|spec| llm_config_for_spec(&spec, task_type))
    }
}

/// Converts an already-selected [`ModelSpec`] into a dispatchable
/// [`vox_actor_runtime::llm::LlmConfig`]. Shared by [`ModelRegistry::get_llm_config`]
/// (which selects via [`ModelRegistry::best_for`]) and by callers that resolve a
/// spec through the key-gated [`super::select::decide`] path instead (e.g.
/// `vox-research-shim`, which depends on both this crate and
/// `vox-actor-runtime` and therefore can bridge the gap that
/// `vox_actor_runtime::llm::cascade` itself cannot, since that crate must not
/// depend on `vox-orchestrator`).
#[cfg(feature = "runtime")]
pub fn llm_config_for_spec(
    spec: &ModelSpec,
    task_type: TaskCategory,
) -> vox_actor_runtime::llm::LlmConfig {
    use super::key_guard::{InferenceWire, inference_wire_for};
    use vox_actor_runtime::llm::LlmConfig;

    let wire = inference_wire_for(&spec.provider_type);
    let mut cfg = match &spec.provider_type {
        // No vendor key, but OpenRouter serves this catalog slug.
        _ if wire == Some(InferenceWire::OpenRouterFallback) => {
            LlmConfig::openrouter(spec.id.clone())
        }
        ProviderType::OpenRouter => LlmConfig::openrouter(spec.id.clone()),
        ProviderType::Ollama => {
            let mut cfg = LlmConfig::direct("ollama", spec.id.clone());
            cfg.base_url = vox_secrets::resolve_secret(vox_secrets::SecretId::OllamaUrl)
                .expose()
                .filter(|s: &&str| !s.trim().is_empty())
                .map(|u: &str| format!("{}/v1/chat/completions", u.trim_end_matches('/')));
            cfg
        }
        ProviderType::HuggingFaceRouter => LlmConfig::huggingface_router(spec.id.clone()),
        // `vox mens serve` exposes an OpenAI-compatible `/v1/chat/completions`.
        ProviderType::VoxLocal => LlmConfig::direct("voxlocal", spec.id.clone()),
        ProviderType::Custom(base) => {
            let mut cfg = LlmConfig::direct("custom", spec.id.clone());
            let trimmed = base.trim().trim_end_matches('/');
            cfg.base_url = Some(if trimmed.ends_with("/chat/completions") {
                trimmed.to_string()
            } else {
                format!("{trimmed}/v1/chat/completions")
            });
            cfg
        }
        ProviderType::GoogleDirect => LlmConfig::direct("gemini", spec.id.clone()),
        ProviderType::Anthropic => LlmConfig::direct("anthropic", spec.id.clone()),
        ProviderType::Groq => LlmConfig::direct("groq", spec.id.clone()),
        ProviderType::Cerebras => LlmConfig::direct("cerebras", spec.id.clone()),
        ProviderType::Mistral => LlmConfig::direct("mistral", spec.id.clone()),
        ProviderType::DeepSeek => LlmConfig::direct("deepseek", spec.id.clone()),
        ProviderType::SambaNova => LlmConfig::direct("sambanova", spec.id.clone()),
        // Mesh dispatch happens upstream (StreamRoute); this facade cannot reach a peer.
        ProviderType::PopuliMesh => LlmConfig::openrouter(spec.id.clone()),
    };
    cfg.telemetry_task_category = Some(task_type.to_string());
    cfg.telemetry_strength_tag = Some(task_category_strength(task_type).to_string());
    cfg.max_tokens = Some(spec.max_tokens);
    // Propagate catalog cost so chat.rs can estimate spend without a DB lookup.
    if spec.cost_per_1k > 0.0 {
        cfg.cost_per_1k = Some(spec.cost_per_1k);
    }
    cfg
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(id: &str, provider_type: ProviderType, cost_per_1k: f64, is_free: bool) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type,
            max_tokens: 4096,
            cost_per_1k,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free,
            observed_cost_per_1k: None,
            strengths: vec![crate::models::generated::StrengthTag::Generalist],
            capabilities: Default::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    fn registry() -> ModelRegistry {
        let mut reg = ModelRegistry::new();
        reg.clear();
        reg.register(spec("paid/big", ProviderType::OpenRouter, 0.5, false));
        reg.register(spec("paid/small", ProviderType::OpenRouter, 0.05, false));
        reg.register(spec("free/a", ProviderType::VoxLocal, 0.0, true));
        reg
    }

    #[test]
    fn cost_queries_pick_cheapest_and_free_models() {
        let reg = registry();
        assert_eq!(reg.list_models().len(), 3);
        assert_eq!(reg.free_models().len(), 1);
        assert_eq!(reg.cheapest().map(|m| m.id), Some("free/a".to_string()));
        assert_eq!(
            reg.cheapest_free().map(|m| m.id),
            Some("free/a".to_string())
        );
        assert_eq!(
            reg.cheapest_with_filter(|m| !m.is_free).map(|m| m.id),
            Some("paid/small".to_string())
        );
        let est = reg.cost_estimate("paid/big", 2000).expect("known model");
        assert!((est - 1.0).abs() < 1e-9);
        assert_eq!(reg.cost_estimate("missing", 2000), None);
    }

    #[test]
    fn get_resolves_ids_and_vendor_stripped_slugs() {
        let reg = registry();
        assert_eq!(
            reg.get(" paid/big ").map(|m| m.id),
            Some("paid/big".to_string())
        );
        assert_eq!(
            reg.get("other/small").map(|m| m.id),
            Some("paid/small".to_string())
        );
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn overrides_and_ollama_reconcile_behave() {
        let mut reg = registry();
        assert_eq!(reg.get_override(7), None);
        reg.set_override(7, "paid/big".to_string());
        assert_eq!(reg.get_override(7).as_deref(), Some("paid/big"));

        reg.register(spec("llama3", ProviderType::Ollama, 0.0, true));
        reg.reconcile_ollama(None);
        assert!(reg.get("llama3").is_none());
        assert_eq!(reg.list_models().len(), 3);
        reg.register(spec("llama3", ProviderType::Ollama, 0.0, true));
        reg.reconcile_ollama(Some(&["llama3".to_string()]));
        assert!(reg.get("llama3").is_some());
    }
}
