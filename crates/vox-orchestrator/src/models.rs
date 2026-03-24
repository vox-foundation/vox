//! LLM model registry, routing metadata, and per-agent overrides.
//!
//! [`ModelRegistry`](crate::models::ModelRegistry) picks the best [`ModelSpec`](crate::models::ModelSpec) for a task category and records
//! sticky overrides used by the runtime scheduler.

use crate::config::CostPreference;
use crate::types::TaskCategory;
use crate::usage::LlmUsageKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Specification for an LLM model in the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Stable model slug used in APIs and config (e.g. `gemini-2.0-flash-lite`).
    pub id: String,
    /// Provider namespace for billing and routing (`google`, `openrouter`, …).
    pub provider: String,
    /// Which API endpoint to use: "google_direct", "openrouter", or "ollama".
    pub provider_type: ProviderType,
    /// Advertised context window / max output budget in tokens.
    pub max_tokens: u64,
    /// Simplified cost metric representing aggregate cost per 1000 tokens.
    pub cost_per_1k: f64,
    /// Whether this model is free (no per-token cost).
    pub is_free: bool,
    /// Tags describing fit (speed, reasoning, codegen) for heuristic routing.
    pub strengths: Vec<String>,
    /// Optional HTTP timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Provider routing type — determines which API endpoint to call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    /// Google AI Studio direct (generativelanguage.googleapis.com)
    GoogleDirect,
    /// OpenRouter API (openrouter.ai/api/v1)
    OpenRouter,
    /// Local Ollama instance (localhost:11434)
    Ollama,
}

impl ModelSpec {
    /// Keys for daily quota rows in `provider_usage` (aligned with `usage` module limits; OpenRouter `:free` aggregate, Ollama `*`).
    #[must_use]
    pub fn llm_usage_key(&self) -> LlmUsageKey {
        match self.provider_type {
            ProviderType::GoogleDirect => LlmUsageKey {
                provider: "google".to_string(),
                model: self.id.clone(),
            },
            ProviderType::OpenRouter => {
                let model = if self.is_free || self.id.contains(":free") {
                    ":free".to_string()
                } else {
                    self.id.clone()
                };
                LlmUsageKey {
                    provider: "openrouter".to_string(),
                    model,
                }
            }
            ProviderType::Ollama => LlmUsageKey {
                provider: "ollama".to_string(),
                model: "*".to_string(),
            },
        }
    }
}

/// Default [`ModelConfig::premium_alias`] entries (portable defaults; override in `models.toml`).
fn built_in_premium_alias() -> HashMap<String, String> {
    [
        (
            "codegen".to_string(),
            "anthropic/claude-sonnet-4.5".to_string(),
        ),
        ("testing".to_string(), "deepseek/deepseek-v3.2".to_string()),
        ("debugging".to_string(), "openai/o3".to_string()),
        ("logic".to_string(), "openai/o3".to_string()),
        ("research".to_string(), "openai/gpt-5".to_string()),
        ("parsing".to_string(), "openai/gpt-5".to_string()),
        ("review".to_string(), "openai/gpt-5".to_string()),
    ]
    .into_iter()
    .collect()
}

fn premium_alias_toml_default() -> HashMap<String, String> {
    HashMap::new()
}

/// Configuration wrapper for models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfig {
    /// All models available to the orchestrator for this deployment.
    pub models: Vec<ModelSpec>,
    /// Optional premium model id per task bucket (`codegen`, `testing`, …). Empty = use ranked paid models.
    #[serde(default = "premium_alias_toml_default")]
    pub premium_alias: HashMap<String, String>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            models: vec![
                // ── Free Tier (Google AI Studio direct, no credit card) ──
                ModelSpec {
                    id: "gemini-2.0-flash-lite".to_string(),
                    provider: "google".to_string(),
                    provider_type: ProviderType::GoogleDirect,
                    max_tokens: 1_000_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec!["codegen".to_string(), "parsing".to_string()],
                timeout_ms: None,
                },
                ModelSpec {
                    id: "gemini-2.5-flash-preview".to_string(),
                    provider: "google".to_string(),
                    provider_type: ProviderType::GoogleDirect,
                    max_tokens: 1_000_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec![
                        "codegen".to_string(),
                        "review".to_string(),
                        "parsing".to_string(),
                    ],
                    timeout_ms: None,
                },
                ModelSpec {
                    id: "gemini-2.5-pro".to_string(),
                    provider: "google".to_string(),
                    provider_type: ProviderType::GoogleDirect,
                    max_tokens: 2_000_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec![
                        "codegen".to_string(),
                        "debugging".to_string(),
                        "review".to_string(),
                        "research".to_string(),
                    ],
                timeout_ms: None,
                },
                // ── Free Tier (OpenRouter :free, requires free API key) ──
                ModelSpec {
                    id: "mistral/devstral-2-2512:free".to_string(),
                    provider: "mistral".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 262_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec!["codegen".to_string(), "refactoring".to_string()],
                timeout_ms: None,
                },
                ModelSpec {
                    id: "qwen/qwen3-coder:free".to_string(),
                    provider: "qwen".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 262_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec!["codegen".to_string()],
                timeout_ms: None,
                },
                ModelSpec {
                    id: "meta-llama/llama-4-scout:free".to_string(),
                    provider: "meta".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 512_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec!["review".to_string(), "parsing".to_string()],
                timeout_ms: None,
                },
                ModelSpec {
                    id: "moonshotai/kimi-k2:free".to_string(),
                    provider: "moonshot".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 200_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec!["codegen".to_string(), "research".to_string()],
                timeout_ms: None,
                },
                // ── Paid Tier (OpenRouter, auto-selected when budget allows) ──
                ModelSpec {
                    id: "deepseek/deepseek-v3.2".to_string(),
                    provider: "deepseek".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 128_000,
                    cost_per_1k: 0.00027,
                    is_free: false,
                    strengths: vec![
                        "codegen".to_string(),
                        "debugging".to_string(),
                        "logic".to_string(),
                    ],
                timeout_ms: None,
                },
                ModelSpec {
                    id: "anthropic/claude-sonnet-4.5".to_string(),
                    provider: "anthropic".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 200_000,
                    cost_per_1k: 0.003,
                    is_free: false,
                    strengths: vec![
                        "codegen".to_string(),
                        "refactoring".to_string(),
                        "review".to_string(),
                    ],
                timeout_ms: None,
                },
                ModelSpec {
                    id: "openai/gpt-5".to_string(),
                    provider: "openai".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 256_000,
                    cost_per_1k: 0.005,
                    is_free: false,
                    strengths: vec![
                        "review".to_string(),
                        "parsing".to_string(),
                        "research".to_string(),
                    ],
                timeout_ms: None,
                },
                ModelSpec {
                    id: "openai/o3".to_string(),
                    provider: "openai".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 200_000,
                    cost_per_1k: 0.010,
                    is_free: false,
                    strengths: vec!["debugging".to_string(), "logic".to_string()],
                timeout_ms: None,
                },
                // ── Local Ollama / Populi (offline fallback; see `OLLAMA_URL` / `POPULI_URL`) ──
                ModelSpec {
                    id: "llama3.2".to_string(),
                    provider: "ollama".to_string(),
                    provider_type: ProviderType::Ollama,
                    max_tokens: 128_000,
                    cost_per_1k: 0.0,
                    is_free: true,
                    strengths: vec!["codegen".to_string(), "parsing".to_string()],
                timeout_ms: None,
                },
            ],
            premium_alias: built_in_premium_alias(),
        }
    }
}

/// Maps [`TaskCategory`] to a `premium_alias` / routing strength key.
#[must_use]
pub fn task_category_premium_key(task_type: TaskCategory) -> &'static str {
    match task_type {
        TaskCategory::CodeGen => "codegen",
        TaskCategory::Testing => "testing",
        TaskCategory::Debugging => "debugging",
        TaskCategory::TypeChecking => "logic",
        TaskCategory::Research => "research",
        TaskCategory::Parsing => "parsing",
        TaskCategory::Review => "review",
    }
}

fn task_category_strength(task_type: TaskCategory) -> &'static str {
    match task_type {
        TaskCategory::CodeGen => "codegen",
        TaskCategory::Testing => "codegen",
        TaskCategory::Debugging => "debugging",
        TaskCategory::TypeChecking => "logic",
        TaskCategory::Research => "research",
        TaskCategory::Parsing => "parsing",
        TaskCategory::Review => "review",
    }
}

/// A registry managing available agent models and model routing.
#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    models: HashMap<String, ModelSpec>,
    agent_overrides: HashMap<u64, String>,
    premium_alias: HashMap<String, String>,
}

impl ModelRegistry {
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
                if let Ok(contents) = std::fs::read_to_string(&config_path) {
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
                timeout_ms: spec.timeout_ms,
            })
    }
}

#[cfg(test)]
mod llm_usage_key_tests {
    use super::{ModelSpec, ProviderType};
    use crate::usage::LlmUsageKey;

    #[test]
    fn openrouter_free_maps_to_aggregate_free_bucket() {
        let m = ModelSpec {
            id: "qwen/qwen3-coder:free".into(),
            provider: "qwen".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1,
            cost_per_1k: 0.0,
            is_free: true,
            strengths: vec![],
        timeout_ms: None,
                };
        assert_eq!(
            m.llm_usage_key(),
            LlmUsageKey {
                provider: "openrouter".into(),
                model: ":free".into(),
            }
        );
    }

    #[test]
    fn openrouter_paid_uses_full_model_id() {
        let m = ModelSpec {
            id: "anthropic/claude-sonnet-4.5".into(),
            provider: "anthropic".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1,
            cost_per_1k: 0.01,
            is_free: false,
            strengths: vec![],
        timeout_ms: None,
                };
        assert_eq!(
            m.llm_usage_key(),
            LlmUsageKey {
                provider: "openrouter".into(),
                model: "anthropic/claude-sonnet-4.5".into(),
            }
        );
    }

    #[test]
    fn ollama_maps_to_star_model() {
        let m = ModelSpec {
            id: "llama3.2".into(),
            provider: "ollama".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 1,
            cost_per_1k: 0.0,
            is_free: true,
            strengths: vec![],
        timeout_ms: None,
                };
        assert_eq!(
            m.llm_usage_key(),
            LlmUsageKey {
                provider: "ollama".into(),
                model: "*".into(),
            }
        );
    }

    #[test]
    fn google_direct_uses_google_provider_and_model_id() {
        let m = ModelSpec {
            id: "gemini-2.0-flash-lite".into(),
            provider: "google".into(),
            provider_type: ProviderType::GoogleDirect,
            max_tokens: 1,
            cost_per_1k: 0.0,
            is_free: true,
            strengths: vec![],
        timeout_ms: None,
                };
        assert_eq!(
            m.llm_usage_key(),
            LlmUsageKey {
                provider: "google".into(),
                model: "gemini-2.0-flash-lite".into(),
            }
        );
    }
}

#[cfg(test)]
mod premium_alias_tests {
    use super::ModelConfig;
    use std::collections::HashSet;

    #[test]
    fn default_premium_alias_targets_exist_in_models_list() {
        let cfg = ModelConfig::default();
        let ids: HashSet<_> = cfg.models.iter().map(|m| m.id.as_str()).collect();
        for (k, v) in &cfg.premium_alias {
            assert!(
                ids.contains(v.as_str()),
                "premium_alias {k} -> {v} not in default models list"
            );
        }
    }
}

#[cfg(test)]
mod registry_filter_tests {
    use super::{ModelRegistry, ModelSpec, ProviderType};
    use crate::config::CostPreference;
    use crate::types::TaskCategory;

    #[test]
    fn best_free_for_with_filter_skips_ollama() {
        let mut r = ModelRegistry::default();
        r.register(ModelSpec {
            id: "llama-local".into(),
            provider: "ollama".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 8192,
            cost_per_1k: 0.0,
            is_free: true,
            strengths: vec!["codegen".into()],
        timeout_ms: None,
                });
        r.register(ModelSpec {
            id: "gemini-2.0-flash-lite".into(),
            provider: "google".into(),
            provider_type: ProviderType::GoogleDirect,
            max_tokens: 1_000_000,
            cost_per_1k: 0.0,
            is_free: true,
            strengths: vec!["codegen".into()],
        timeout_ms: None,
                });
        let picked = r
            .best_for_with_filter(TaskCategory::CodeGen, 2, CostPreference::Performance, |m| {
                m.is_free && !matches!(m.provider_type, ProviderType::Ollama)
            })
            .expect("non-ollama free");
        assert_eq!(picked.id, "gemini-2.0-flash-lite");
    }

    #[test]
    fn cheapest_free_with_filter_stable_tiebreak_on_id() {
        let mut r = ModelRegistry::default();
        r.register(ModelSpec {
            id: "z-free".into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1000,
            cost_per_1k: 0.0,
            is_free: true,
            strengths: vec!["codegen".into()],
        timeout_ms: None,
                });
        r.register(ModelSpec {
            id: "a-free".into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1000,
            cost_per_1k: 0.0,
            is_free: true,
            strengths: vec!["codegen".into()],
        timeout_ms: None,
                });
        let picked = r
            .cheapest_free_with_filter(|_| true)
            .expect("free model");
        assert_eq!(picked.id, "a-free");
    }

    #[test]
    fn cheapest_with_filter_stable_tiebreak_on_id() {
        let mut r = ModelRegistry::default();
        r.register(ModelSpec {
            id: "z-paid".into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1000,
            cost_per_1k: 0.01,
            is_free: false,
            strengths: vec!["codegen".into()],
        timeout_ms: None,
                });
        r.register(ModelSpec {
            id: "a-paid".into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 1000,
            cost_per_1k: 0.01,
            is_free: false,
            strengths: vec!["codegen".into()],
        timeout_ms: None,
                });
        let picked = r.cheapest_with_filter(|_| true).expect("paid model");
        assert_eq!(picked.id, "a-paid");
    }
}
