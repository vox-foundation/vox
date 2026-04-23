use std::cmp::Ord;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::catalog::{ModelCatalog, OpenRouterCatalog};
use crate::config::CostPreference;
use crate::types::{AgentTask, TaskCategory};

use super::key_guard::provider_secret_is_available;
use super::spec::{
    ModelConfig, ModelSpec, ProviderType, built_in_premium_alias, task_category_strength,
};

/// A performance score from the `model_scoreboard`.
#[derive(Debug, Clone, Default)]
pub struct ModelScore {
    pub success_rate: f64,
    pub quality_score: f64,
    pub cost_per_success_usd: Option<f64>,
    pub p50_latency_ms: Option<i64>,
    pub n_calls: i64,
}

impl From<vox_db::store::types::ModelScoreboardRow> for ModelScore {
    fn from(row: vox_db::store::types::ModelScoreboardRow) -> Self {
        Self {
            success_rate: row.success_rate,
            quality_score: row.quality_score,
            cost_per_success_usd: row.cost_per_success_usd,
            p50_latency_ms: row.p50_latency_ms,
            n_calls: row.n_calls,
        }
    }
}

/// A container of validated `ModelSpec`s that also tracks dynamic scoreboard scores.
#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    models: HashMap<String, ModelSpec>,
    agent_overrides: HashMap<u64, String>,
    premium_alias: HashMap<String, String>,
    /// Dynamic scores retrieved from `vox-db` `model_scoreboard` (keyed by model_id).
    scoreboard: HashMap<String, ModelScore>,
    /// In-memory penalty map for models that abstain (FIX-12).
    /// Key: (model_id, task_category). Value: Expiry time.
    penalty_map: HashMap<(String, TaskCategory), SystemTime>,
}

impl ModelRegistry {
    pub fn record_penalty(&mut self, model_id: String, category: TaskCategory, duration: Duration) {
        let expiry = SystemTime::now() + duration;
        self.penalty_map.insert((model_id, category), expiry);
    }

    pub fn is_penalized(&self, model_id: &str, category: TaskCategory) -> bool {
        if let Some(expiry) = self.penalty_map.get(&(model_id.to_string(), category)) {
            if *expiry > SystemTime::now() {
                return true;
            }
        }
        false
    }

    pub fn inject_scoreboard(&mut self, scores: HashMap<String, ModelScore>) {
        self.scoreboard = scores;
    }

    pub fn inject_pricing_catalog(
        &mut self,
        pricing: Vec<vox_db::store::types::ModelPricingCatalogRow>,
    ) {
        for row in pricing {
            if row.confidence == "medium" || row.confidence == "high" {
                if let Some(spec) = self.models.get_mut(&row.model_id) {
                    if let Some(blended) = row.observed_blended_per_1k {
                        tracing::info!(
                            model_id = %row.model_id,
                            catalog_price = %spec.cost_per_1k,
                            observed_price = %blended,
                            confidence = %row.confidence,
                            "Calibrating model price from telemetry loop"
                        );
                        spec.cost_per_1k = blended;
                        spec.observed_cost_per_1k = Some(blended);

                        // If provider separates inputs/outputs, override those too
                        if let Some(input) = row.observed_input_per_1k {
                            spec.cost_per_1k_input = input;
                        }
                        if let Some(output) = row.observed_output_per_1k {
                            spec.cost_per_1k_output = output;
                        }
                    }
                }
            }
        }
    }

    pub fn scoreboard_len(&self) -> usize {
        self.scoreboard.len()
    }

    pub fn get_score(&self, model_id: &str) -> Option<&ModelScore> {
        self.scoreboard.get(model_id)
    }

    fn matches_strength(m: &ModelSpec, strength: crate::models::StrengthTag) -> bool {
        m.strengths
            .iter()
            .any(|s| *s == strength || *s == crate::models::StrengthTag::Generalist)
    }

    #[allow(dead_code)]
    fn key_is_present_for(m: &ModelSpec) -> bool {
        provider_secret_is_available(&m.provider_type)
    }

    fn min_refresh_interval() -> Duration {
        let secs = vox_clavis::resolve_secret(
            vox_clavis::SecretId::VoxOpenRouterCatalogMinRefreshIntervalSecs,
        )
        .expose()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600);
        Duration::from_secs(secs.max(30))
    }

    fn jitter_ms() -> u64 {
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOpenRouterCatalogRefreshJitterMs)
            .expose()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
            .min(60_000)
    }
    #[cfg_attr(test, allow(dead_code))]
    fn maybe_refresh_catalogs(&mut self) {
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let min_refresh = Self::min_refresh_interval().as_secs();
        let jitter = Self::jitter_ms();

        enum RefreshFail {
            Runtime(String),
            Fetch(String),
        }

        let joined = std::thread::spawn(move || -> Result<Option<Vec<ModelSpec>>, RefreshFail> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| RefreshFail::Runtime(e.to_string()))?;
            rt.block_on(async {
                let db = vox_db::VoxDb::open_default()
                    .await
                    .map_err(|_| anyhow::anyhow!("db error"))?;
                if let Ok(Some(last_str)) = db
                    .get_user_preference("global", "openrouter_catalog_refresh")
                    .await
                {
                    if let Ok(last_secs) = last_str.parse::<u64>() {
                        if now_secs.saturating_sub(last_secs) < min_refresh {
                            return Ok::<_, anyhow::Error>(None);
                        }
                    }
                }

                if jitter > 0 {
                    let offset = now_secs % (jitter + 1);
                    tokio::time::sleep(Duration::from_millis(offset)).await;
                }

                let mut models = OpenRouterCatalog::new().refresh().await.unwrap_or_default();
                let repo_root = vox_repository::find_project_manifest_root(&std::env::current_dir().unwrap_or_default()).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                if let Ok(mens_models) = crate::catalog::MensCatalog::new(&repo_root).refresh().await {
                    models.extend(mens_models);
                }
                crate::catalog_classifier::classify_models(&mut models).await;

                #[cfg(feature = "populi-transport")]
                {
                    let mut control_url_opt = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOrchestratorMeshControlUrl).expose().map(|s| s.to_string());
                    if control_url_opt.is_none() {
                        control_url_opt = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMeshControlAddr).expose().map(|s| s.to_string());
                    }
                    if let Some(control_url) = control_url_opt {
                        let client = vox_populi::http_client::PopuliHttpClient::new(control_url.trim()).with_env_token();
                        if let Ok(dir) = client.federation_directory().await {
                            for peer in dir.entries {
                                // Sybil/Reliability check: query peer reputation from db
                                if let Ok(Some((success, fail, timeout, invalid))) = db.get_peer_reputation(&peer.scope_id).await {
                                    let total_bad = fail + timeout + invalid;
                                    // Blacklist condition: more than 3 failures, and bad events exceed successful tasks.
                                    if total_bad > 3 && total_bad > success {
                                        tracing::warn!(target: "vox.orchestrator.models", peer=%peer.scope_id, success, total_bad, "mesh peer blacklisted due to poor reputation");
                                        continue;
                                    }
                                }

                                for kind in peer.task_kinds {
                                let kind_str = serde_json::to_value(&kind).unwrap().as_str().unwrap().to_string();
                                    models.push(ModelSpec {
                                        id: format!("mesh/{}/{}", peer.scope_id, kind_str),
                                        canonical_slug: format!("mesh/{}/{}", peer.scope_id, kind_str),
                                        provider: "mens".to_string(),
                                        provider_type: ProviderType::PopuliMesh,
                                        max_tokens: 128_000,
                                        cost_per_1k: 0.0,
                                        cost_per_1k_input: 0.0,
                                        cost_per_1k_output: 0.0,
                                        is_free: true,
                                        strengths: vec![
                                            crate::models::StrengthTag::from_str(&kind_str).unwrap_or(crate::models::StrengthTag::Unknown),
                                            crate::models::StrengthTag::Generalist,
                                        ],
                                        capabilities: crate::models::spec::ModelCapabilities {
                                            tier: crate::models::ModelTier::Local,
                                            ..Default::default()
                                        },
                                        supported_parameters: vec![],
                                        observed_cost_per_1k: None,
                                    });
                                }
                            }
                        }
                    }
                }

                let _ = db
                    .set_user_preference(
                        "global",
                        "catalog_refresh",
                        &now_secs.to_string(),
                    )
                    .await;

                if let Ok(json) = serde_json::to_string(&models) {
                    let cache_file = vox_config::paths::dot_vox_user_dir().join("cache").join("model-catalog.v1.json");
                    if let Some(parent) = cache_file.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&cache_file, json);
                }

                Ok(Some(models))
            })
            .map_err(|e| RefreshFail::Fetch(e.to_string()))
        })
        .join();

        let models_opt = match joined {
            Ok(Ok(m)) => m,
            Ok(Err(RefreshFail::Runtime(msg))) => {
                tracing::warn!(target: "vox.orchestrator.models", error = %msg, "openrouter catalog runtime init failed");
                return;
            }
            Ok(Err(RefreshFail::Fetch(msg))) => {
                tracing::warn!(target: "vox.orchestrator.models", error = %msg, "openrouter model catalog refresh failed");
                return;
            }
            Err(_) => {
                tracing::warn!(target: "vox.orchestrator.models", "openrouter catalog refresh panicked");
                return;
            }
        };

        if let Some(models) = models_opt {
            let count = models.len();
            for m in models {
                self.register(m);
            }
            tracing::info!(target: "vox.orchestrator.models", count, "catalog refresh merged into model registry");
        } else {
            tracing::debug!(target: "vox.orchestrator.models", "catalog refresh skipped (within min refresh interval)");
        }
    }

    /// Create a new model registry, loading from the configuration file or falling back to defaults.
    pub fn new() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
            agent_overrides: HashMap::new(),
            premium_alias: HashMap::new(),
            scoreboard: HashMap::new(),
            penalty_map: HashMap::new(),
        };

        // Try to load from models.toml in the config directory
        let model_config = if let Some(mut config_path) = vox_db::paths::config_dir() {
            config_path.push("models.toml");
            if config_path.exists() {
                if let Ok(contents) = vox_bounded_fs::read_utf8_path_capped(&config_path) {
                    toml::from_str(&contents).unwrap_or_else(|_| ModelConfig::default())
                } else {
                    ModelConfig::default()
                }
            } else {
                let default_config = ModelConfig::default();
                // Create config dir if needed and write default file
                if let Some(parent) = config_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                    if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
                        let _ = std::fs::write(&config_path, toml_str);
                    }
                }
                default_config
            }
        } else {
            ModelConfig::default()
        };

        registry.premium_alias = if model_config.premium_alias.is_empty() {
            built_in_premium_alias()
        } else {
            model_config.premium_alias.clone()
        };

        for model in model_config.models {
            registry.register(model);
        }

        let cache_file = vox_config::paths::dot_vox_user_dir()
            .join("cache")
            .join("model-catalog.v1.json");
        if let Ok(contents) = std::fs::read_to_string(&cache_file) {
            if let Ok(cached_models) = serde_json::from_str::<Vec<ModelSpec>>(&contents) {
                for m in cached_models {
                    registry.register(m);
                }
            }
        }

        // Live catalog merge hits the network and shifts `best_for` rankings; keep unit tests on the
        // static TOML/default model list unless integration coverage opts in elsewhere.
        #[cfg(not(test))]
        registry.maybe_refresh_catalogs();

        registry
    }

    /// Register a new model specification.
    pub fn register(&mut self, spec: ModelSpec) {
        self.models.insert(spec.id.clone(), spec);
    }

    pub fn best_for_task(&self, task: &AgentTask, preference: CostPreference) -> Option<ModelSpec> {
        self.best_for_task_with_filter(task, preference, |_| true)
    }

    /// Like [`Self::best_for_task`] but only considers models for which `pred` returns true.
    pub fn best_for_task_with_filter(
        &self,
        task: &AgentTask,
        preference: CostPreference,
        pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModelSpec> {
        let mut complexity = task.estimated_complexity;
        let mut task_type = task.task_category;

        if !task.research_hints.is_empty() && task_type != TaskCategory::Research {
            task_type = TaskCategory::Research;
        }

        if task.tool_hints.len() >= 2 && complexity < 7 {
            complexity = 7;
        }

        self.best_for_with_filter(task_type, complexity, preference, pred, Some(task))
    }

    /// Return the best model for a given task category and complexity.
    pub fn best_for(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
    ) -> Option<ModelSpec> {
        self.best_for_with_filter(task_type, complexity, preference, |_| true, None)
    }

    /// Like [`Self::best_for`] but only considers models for which `pred` returns true.
    #[must_use]
    pub fn best_for_with_filter(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
        mut pred: impl FnMut(&ModelSpec) -> bool,
        task: Option<&AgentTask>,
    ) -> Option<ModelSpec> {
        let effective_pref = if complexity <= 3 && preference == CostPreference::Economy {
            CostPreference::Economy
        } else {
            preference
        };

        let strength = task_category_strength(task_type);

        // First pass: Respect penalties
        let result =
            self.best_for_internal(task_type, strength, effective_pref, &mut pred, true, task);
        if result.is_some() {
            return result;
        }

        // Second pass: Ignore penalties if no other options
        self.best_for_internal(task_type, strength, effective_pref, &mut pred, false, task)
    }

    fn best_for_internal(
        &self,
        _task_type: TaskCategory,
        strength: crate::models::StrengthTag,
        preference: CostPreference,
        pred: &mut impl FnMut(&ModelSpec) -> bool,
        respect_penalties: bool,
        task: Option<&AgentTask>,
    ) -> Option<ModelSpec> {
        self.models
            .values()
            .filter(|m| {
                if respect_penalties && self.is_penalized(&m.id, _task_type) {
                    return false;
                }
                if preference == CostPreference::Performance && m.is_free {
                    return false; // Skip free models in performance mode unless they are explicitly mapped
                }

                // Budget Gating (FIX-18)
                if let (Some(t), Some(budget)) = (task, task.and_then(|t| t.budget.as_ref())) {
                    let est_tokens = t.estimated_token_count();
                    // Use scoreboard cost if available for more empirical gating
                    let cost_basis = self
                        .scoreboard
                        .get(&m.id)
                        .and_then(|s| s.cost_per_success_usd)
                        .unwrap_or(m.cost_per_1k);

                    let est_cost = (est_tokens as f64 / 1000.0) * cost_basis;
                    if let Some(max) = budget.max_cost_usd {
                        if est_cost > max {
                            return false;
                        }
                    }
                }

                Self::matches_strength(m, strength) && pred(m)
            })
            .min_by(|a, b| {
                let get_effective_cost = |m: &ModelSpec| {
                    if let Some(score) = self.scoreboard.get(&m.id) {
                        let base_cost = score.cost_per_success_usd.unwrap_or(m.cost_per_1k);
                        if score.n_calls >= 3 {
                            // Scoreboard-aware routing: penalize models with low quality scores.
                            // We use (2.0 - quality) as a multiplier to double cost if quality is 0.
                            return base_cost * (2.0 - score.quality_score.min(2.0));
                        }
                        return base_cost;
                    }
                    m.cost_per_1k
                };

                let cost_a = get_effective_cost(a);
                let cost_b = get_effective_cost(b);

                cost_a.total_cmp(&cost_b).then_with(|| {
                    // Secondary sort by success rate if costs (adjusted) are equal
                    let a_sr = self
                        .scoreboard
                        .get(&a.id)
                        .map(|s| s.success_rate)
                        .unwrap_or(0.5);
                    let b_sr = self
                        .scoreboard
                        .get(&b.id)
                        .map(|s| s.success_rate)
                        .unwrap_or(0.5);
                    b_sr.total_cmp(&a_sr).then_with(|| {
                        // Tertiary sort by latency if costs and success rates are similar
                        let a_lat = self
                            .scoreboard
                            .get(&a.id)
                            .and_then(|s| s.p50_latency_ms)
                            .unwrap_or(2000);
                        let b_lat = self
                            .scoreboard
                            .get(&b.id)
                            .and_then(|s| s.p50_latency_ms)
                            .unwrap_or(2000);

                        a_lat.cmp(&b_lat).then_with(|| {
                            let prefer_mesh = vox_clavis::resolve_secret(
                                vox_clavis::SecretId::VoxRoutingPreferMesh,
                            )
                            .expose()
                            .map(|s: &str| s.trim() == "true")
                            .unwrap_or(false);
                            if prefer_mesh {
                                let a_is_mesh = a.provider_type == ProviderType::PopuliMesh;
                                let b_is_mesh = b.provider_type == ProviderType::PopuliMesh;
                                b_is_mesh.cmp(&a_is_mesh)
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        })
                    })
                })
            })
            .cloned()
    }

    /// Return all models matching the criteria, sorted by the effective score (priority order).
    pub fn explain_selection(
        &self,
        _task_type: TaskCategory,
        strength: crate::models::StrengthTag,
        preference: crate::config::CostPreference,
    ) -> Vec<ModelSpec> {
        let mut candidates: Vec<ModelSpec> = self
            .models
            .values()
            .filter(|m| {
                if preference == crate::config::CostPreference::Performance && m.is_free {
                    return false;
                }
                Self::matches_strength(m, strength)
            })
            .cloned()
            .collect();

        candidates.sort_by(|a, b| {
            let get_effective_cost = |m: &ModelSpec| {
                if let Some(score) = self.scoreboard.get(&m.id) {
                    if score.n_calls >= 3 {
                        return m.cost_per_1k * (2.0 - score.quality_score.min(2.0));
                    }
                }
                m.cost_per_1k
            };

            let cost_a = get_effective_cost(a);
            let cost_b = get_effective_cost(b);

            cost_a.total_cmp(&cost_b).then_with(|| {
                let a_sr = self
                    .scoreboard
                    .get(&a.id)
                    .map(|s| s.success_rate)
                    .unwrap_or(0.5);
                let b_sr = self
                    .scoreboard
                    .get(&b.id)
                    .map(|s| s.success_rate)
                    .unwrap_or(0.5);
                b_sr.total_cmp(&a_sr).then_with(|| a.id.cmp(&b.id))
            })
        });

        candidates
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
        self.models.get(model_id).cloned()
    }

    /// Set an explicit model override for a specific agent.
    pub fn set_override(&mut self, agent_id: u64, model_id: String) {
        self.agent_overrides.insert(agent_id, model_id);
    }

    /// Check if there's an active model override for an agent.
    pub fn get_override(&self, agent_id: u64) -> Option<String> {
        self.agent_overrides.get(&agent_id).cloned()
    }

    /// Builds a [`vox_runtime::llm::LlmConfig`] for the best matching model when the `runtime` feature is on.
    #[cfg(feature = "runtime")]
    pub fn get_llm_config(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
    ) -> Option<vox_runtime::llm::LlmConfig> {
        self.best_for(task_type, complexity, preference)
            .map(|spec| {
                let mut cfg = match spec.provider_type {
                    ProviderType::OpenRouter => {
                        vox_runtime::llm::LlmConfig::openrouter(spec.id.clone())
                    }
                    ProviderType::Ollama => vox_runtime::llm::LlmConfig {
                        provider: "ollama".to_string(),
                        model: spec.id.clone(),
                        cost_per_1k: None,
                        base_url: vox_clavis::resolve_secret(vox_clavis::SecretId::OllamaUrl)
                            .expose()
                            .filter(|s: &&str| !s.trim().is_empty())
                            .map(|u: &str| {
                                format!("{}/v1/chat/completions", u.trim_end_matches('/'))
                            }),
                        api_key: None,
                        temperature: None,
                        top_p: None,
                        max_tokens: None,
                        response_format: None,
                        timeout_ms: None,
                        telemetry_session_id: None,
                        telemetry_user_id: None,
                        telemetry_task_category: Some(task_type.to_string()),
                        telemetry_strength_tag: Some(task_category_strength(task_type).to_string()),
                        telemetry_trace_id: None,
                        telemetry_attempt_number: None,
                        telemetry_skip_interaction: false,
                    },
                    ProviderType::GoogleDirect => vox_runtime::llm::LlmConfig {
                        provider: "openrouter".to_string(),
                        model: spec.id.clone(),
                        cost_per_1k: None,
                        base_url: Some(vox_config::OPENROUTER_CHAT_COMPLETIONS_URL.to_string()),
                        api_key: None,
                        temperature: None,
                        top_p: None,
                        max_tokens: None,
                        response_format: None,
                        timeout_ms: None,
                        telemetry_session_id: None,
                        telemetry_user_id: None,
                        telemetry_task_category: Some(task_type.to_string()),
                        telemetry_strength_tag: Some(task_category_strength(task_type).to_string()),
                        telemetry_trace_id: None,
                        telemetry_attempt_number: None,
                        telemetry_skip_interaction: false,
                    },
                    ProviderType::HuggingFaceRouter => {
                        let mut cfg =
                            vox_runtime::llm::LlmConfig::huggingface_router(spec.id.clone());
                        cfg.telemetry_task_category = Some(task_type.to_string());
                        cfg.telemetry_strength_tag =
                            Some(task_category_strength(task_type).to_string());
                        cfg
                    }
                    ProviderType::Custom(_)
                    | ProviderType::PopuliMesh
                    | ProviderType::Anthropic
                    | ProviderType::Mistral
                    | ProviderType::DeepSeek
                    | ProviderType::SambaNova
                    | ProviderType::Groq
                    | ProviderType::Cerebras => {
                        let mut cfg = vox_runtime::llm::LlmConfig::openrouter(spec.id.clone());
                        cfg.telemetry_task_category = Some(task_type.to_string());
                        cfg.telemetry_strength_tag =
                            Some(task_category_strength(task_type).to_string());
                        cfg
                    }
                };
                cfg.max_tokens = Some(spec.max_tokens);
                cfg
            })
    }
}
