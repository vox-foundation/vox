use std::collections::HashMap;

use crate::catalog::{ModelCatalog, OpenRouterCatalog};
use crate::config::CostPreference;
use crate::types::TaskCategory;

use super::spec::{
    ModelConfig, ModelSpec, built_in_premium_alias, task_category_premium_key,
    task_category_strength,
};

/// A registry managing available agent models and model routing.
#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    models: HashMap<String, ModelSpec>,
    agent_overrides: HashMap<u64, String>,
    premium_alias: HashMap<String, String>,
}

impl ModelRegistry {
    #[cfg_attr(test, allow(dead_code))] // Called from `new` only outside `cfg(test)` (avoids network in unit tests).
    fn maybe_refresh_openrouter_models(&mut self) {
        // Avoid `block_on` on a thread that already drives a Tokio runtime (e.g. `#[tokio::test]`,
        // `cargo nextest`): that panics with "Cannot start a runtime from within a runtime". Run the
        // ephemeral runtime on a fresh OS thread instead.
        enum RefreshFail {
            Runtime(String),
            Fetch(String),
        }

        let joined = std::thread::spawn(|| -> Result<Vec<ModelSpec>, RefreshFail> {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| RefreshFail::Runtime(e.to_string()))?;
            rt.block_on(async { OpenRouterCatalog::new().refresh().await })
                .map_err(|e| RefreshFail::Fetch(e.to_string()))
        })
        .join();

        let models = match joined {
            Ok(Ok(models)) => models,
            Ok(Err(RefreshFail::Runtime(msg))) => {
                tracing::warn!(
                    target: "vox.orchestrator.models",
                    error = %msg,
                    "openrouter catalog runtime init failed"
                );
                return;
            }
            Ok(Err(RefreshFail::Fetch(msg))) => {
                tracing::warn!(
                    target: "vox.orchestrator.models",
                    error = %msg,
                    "openrouter model catalog refresh failed; keeping static model registry"
                );
                return;
            }
            Err(_) => {
                tracing::warn!(
                    target: "vox.orchestrator.models",
                    "openrouter catalog refresh panicked; keeping static model registry"
                );
                return;
            }
        };
        let count = models.len();
        for m in models {
            self.register(m);
        }
        tracing::info!(target: "vox.orchestrator.models", count, "openrouter catalog refresh merged into model registry");
    }

    /// Create a new model registry, loading from the configuration file or falling back to defaults.
    pub fn new() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
            agent_overrides: HashMap::new(),
            premium_alias: HashMap::new(),
        };

        // Try to load from models.toml in the config directory
        let model_config = if let Some(mut config_path) = vox_db::paths::config_dir() {
            config_path.push("models.toml");
            if config_path.exists() {
                if let Ok(contents) = crate::bounded_fs::read_utf8_path_capped(&config_path) {
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
        // Live catalog merge hits the network and shifts `best_for` rankings; keep unit tests on the
        // static TOML/default model list unless integration coverage opts in elsewhere.
        #[cfg(not(test))]
        registry.maybe_refresh_openrouter_models();

        registry
    }

    /// Register a new model specification.
    pub fn register(&mut self, spec: ModelSpec) {
        self.models.insert(spec.id.clone(), spec);
    }

    /// Return the best model for a given task category and complexity.
    /// If preference is Economy, it will favor models with lower cost_per_1k.
    /// If complexity is low, it will favor cheaper models to save budget.
    pub fn best_for(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
    ) -> Option<ModelSpec> {
        // Automatic Dynamic Tiering: Low complexity tasks don't need premium models
        let effective_pref = if complexity <= 3 {
            CostPreference::Economy
        } else {
            preference
        };

        if effective_pref == CostPreference::Economy {
            // Find the cheapest model that has the relevant strength for the category
            let strength = match task_type {
                TaskCategory::CodeGen => "codegen",
                TaskCategory::Testing => "codegen",
                TaskCategory::Debugging => "debugging",
                TaskCategory::TypeChecking => "logic",
                TaskCategory::Research => "research",
                TaskCategory::Parsing => "parsing",
                TaskCategory::Review => "review",
            };

            return self
                .models
                .values()
                .filter(|m| m.strengths.iter().any(|s| s == strength))
                .min_by(|a, b| a.cost_per_1k.total_cmp(&b.cost_per_1k))
                .cloned()
                .or_else(|| self.cheapest());
        }

        // Premium routing: TOML `premium_alias` first, else cheapest paid model for the task strength.
        let key = task_category_premium_key(task_type);
        if let Some(id) = self.premium_alias.get(key) {
            if let Some(m) = self.models.get(id) {
                return Some(m.clone());
            }
        }
        let strength = task_category_strength(task_type);
        self.models
            .values()
            .filter(|m| !m.is_free && m.strengths.iter().any(|s| s == strength))
            .min_by(|a, b| a.cost_per_1k.total_cmp(&b.cost_per_1k))
            .cloned()
            .or_else(|| {
                self.models
                    .values()
                    .filter(|m| !m.is_free)
                    .min_by(|a, b| a.cost_per_1k.total_cmp(&b.cost_per_1k))
                    .cloned()
            })
            .or_else(|| self.cheapest())
    }

    /// Like [`Self::best_for`] but only considers models for which `pred` returns true.
    #[must_use]
    pub fn best_for_with_filter(
        &self,
        task_type: TaskCategory,
        complexity: u8,
        preference: CostPreference,
        mut pred: impl FnMut(&ModelSpec) -> bool,
    ) -> Option<ModelSpec> {
        let effective_pref = if complexity <= 3 {
            CostPreference::Economy
        } else {
            preference
        };

        if effective_pref == CostPreference::Economy {
            let strength = match task_type {
                TaskCategory::CodeGen => "codegen",
                TaskCategory::Testing => "codegen",
                TaskCategory::Debugging => "debugging",
                TaskCategory::TypeChecking => "logic",
                TaskCategory::Research => "research",
                TaskCategory::Parsing => "parsing",
                TaskCategory::Review => "review",
            };

            return self
                .models
                .values()
                .filter(|m| m.strengths.iter().any(|s| s == strength) && pred(m))
                .min_by(|a, b| a.cost_per_1k.total_cmp(&b.cost_per_1k))
                .cloned()
                .or_else(|| self.cheapest_with_filter(&mut pred));
        }

        let key = task_category_premium_key(task_type);
        if let Some(id) = self.premium_alias.get(key) {
            if let Some(m) = self.models.get(id) {
                if pred(m) {
                    return Some(m.clone());
                }
            }
        }
        let strength = task_category_strength(task_type);
        self.models
            .values()
            .filter(|m| !m.is_free && m.strengths.iter().any(|s| s == strength) && pred(m))
            .min_by(|a, b| a.cost_per_1k.total_cmp(&b.cost_per_1k))
            .cloned()
            .or_else(|| {
                self.models
                    .values()
                    .filter(|m| !m.is_free && pred(m))
                    .min_by(|a, b| a.cost_per_1k.total_cmp(&b.cost_per_1k))
                    .cloned()
            })
            .or_else(|| self.cheapest_with_filter(pred))
    }

    /// Return the best free model for a given task category.
    pub fn best_free_for(&self, task_type: TaskCategory) -> Option<ModelSpec> {
        let strength = match task_type {
            TaskCategory::CodeGen => "codegen",
            TaskCategory::Testing => "codegen",
            TaskCategory::Debugging => "debugging",
            TaskCategory::TypeChecking => "logic",
            TaskCategory::Research => "research",
            TaskCategory::Parsing => "parsing",
            TaskCategory::Review => "review",
        };

        self.models
            .values()
            .filter(|m| m.is_free && m.strengths.iter().any(|s| s == strength))
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
        let strength = match task_type {
            TaskCategory::CodeGen => "codegen",
            TaskCategory::Testing => "codegen",
            TaskCategory::Debugging => "debugging",
            TaskCategory::TypeChecking => "logic",
            TaskCategory::Research => "research",
            TaskCategory::Parsing => "parsing",
            TaskCategory::Review => "review",
        };

        self.models
            .values()
            .filter(|m| m.is_free && m.strengths.iter().any(|s| s == strength) && pred(m))
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
        self.models.values().find(|m| m.is_free).cloned()
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
            .map(|spec| vox_runtime::llm::LlmConfig {
                provider: spec.provider.clone(),
                model: spec.id.clone(),
                base_url: None,
                api_key: None,
                temperature: None,
                max_tokens: Some(spec.max_tokens),
                response_format: None,
                timeout_ms: None,
            })
    }
}
