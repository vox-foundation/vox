//! LLM model specifications, capabilities, and routing keys.
//!
//! [`ModelRegistry`](crate::models::ModelRegistry) (in `registry.rs`) uses these types for task-category routing.

use crate::types::TaskCategory;
use crate::usage::LlmUsageKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model tier for routing prioritization
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelTier {
    #[default]
    Unknown,
    Light,
    Pro,
    Elite,
}

/// Rich capabilities for a model, imported from DeI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    pub supports_json: bool,
    pub supports_vision: bool,
    pub max_context: u64,
    pub tier: ModelTier,
    pub rate_limit_rpm: Option<u32>,
    pub rate_limit_rpd: Option<u32>,
}

/// Specification for an LLM model in the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSpec {
    /// Stable model slug used in APIs and config (e.g. `gemini-2.0-flash-lite`).
    pub id: String,
    /// The unique system-wide slug
    #[serde(default)]
    pub canonical_slug: String,
    /// Provider namespace for billing and routing (`google`, `openrouter`, …).
    pub provider: String,
    /// Which API endpoint to use: "google_direct", "openrouter", or "ollama".
    pub provider_type: ProviderType,
    /// Advertised context window / max output budget in tokens.
    pub max_tokens: u64,
    /// Simplified cost metric representing aggregate cost per 1000 tokens.
    pub cost_per_1k: f64,
    #[serde(default)]
    pub cost_per_1k_input: f64,
    #[serde(default)]
    pub cost_per_1k_output: f64,
    /// Whether this model is free (no per-token cost).
    pub is_free: bool,
    /// Tags describing fit (speed, reasoning, codegen) for heuristic routing.
    pub strengths: Vec<String>,
    #[serde(default)]
    pub capabilities: ModelCapabilities,
    #[serde(default)]
    pub supported_parameters: Vec<String>,
}

/// Provider routing type — determines which API endpoint to call.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    /// Google AI Studio direct (generativelanguage.googleapis.com)
    GoogleDirect,
    /// OpenRouter API (openrouter.ai/api/v1)
    OpenRouter,
    /// Local Ollama instance (localhost:11434)
    Ollama,
    /// Groq LPU endpoint
    Groq,
    /// Cerebras endpoint
    Cerebras,
    /// Mistral direct
    Mistral,
    /// DeepSeek direct
    DeepSeek,
    /// SambaNova
    SambaNova,
    /// Custom third-party endpoint
    Custom(String),
}

/// Normalized provider-route decision shared by orchestrator runtime and MCP tooling.
///
/// Cross-surface telemetry uses the same `(provider_family, route_choice)` strings as
/// `vox_runtime::model_resolution::backend_telemetry_labels` (`ChatRouteBackend`); MCP delegates there. Keep the four
/// lanes aligned when changing routing semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelRouteBackend {
    GeminiDirect,
    OpenRouter,
    Ollama,
    CascadeFallback,
}

/// Resolve the transport/backend lane for a concrete model spec.
#[must_use]
pub fn route_backend_for_model(spec: &ModelSpec) -> ModelRouteBackend {
    match spec.provider_type {
        ProviderType::Ollama => ModelRouteBackend::Ollama,
        ProviderType::GoogleDirect => ModelRouteBackend::GeminiDirect,
        ProviderType::OpenRouter => ModelRouteBackend::OpenRouter,
        ProviderType::Groq
        | ProviderType::Mistral
        | ProviderType::DeepSeek
        | ProviderType::Cerebras
        | ProviderType::SambaNova
        | ProviderType::Custom(_) => {
            if spec.id.contains('/') {
                ModelRouteBackend::OpenRouter
            } else {
                ModelRouteBackend::CascadeFallback
            }
        }
    }
}

impl ModelSpec {
    /// Keys for daily quota rows in `provider_usage` (aligned with `usage` module limits; OpenRouter `:free` aggregate, Ollama `*`).
    #[must_use]
    pub fn llm_usage_key(&self) -> LlmUsageKey {
        match &self.provider_type {
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
            ProviderType::Groq => LlmUsageKey {
                provider: "groq".to_string(),
                model: self.id.clone(),
            },
            ProviderType::Cerebras => LlmUsageKey {
                provider: "cerebras".to_string(),
                model: self.id.clone(),
            },
            ProviderType::Mistral => LlmUsageKey {
                provider: "mistral".to_string(),
                model: self.id.clone(),
            },
            ProviderType::DeepSeek => LlmUsageKey {
                provider: "deepseek".to_string(),
                model: self.id.clone(),
            },
            ProviderType::SambaNova => LlmUsageKey {
                provider: "sambanova".to_string(),
                model: self.id.clone(),
            },
            ProviderType::Custom(_url) => LlmUsageKey {
                provider: "custom".to_string(),
                model: self.id.clone(),
            },
        }
    }
}

/// Default [`ModelConfig::premium_alias`] entries (portable defaults; override in `models.toml`).
pub(super) fn built_in_premium_alias() -> HashMap<String, String> {
    HashMap::new()
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
        let local_model = std::env::var("POPULI_MODEL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "default-model".to_string());
        Self {
            models: vec![
                // ── Local Ollama / Mens (offline fallback; see `OLLAMA_URL` / `POPULI_URL`) ──
                ModelSpec {
                    id: local_model.clone(),
                    canonical_slug: format!("local/{local_model}"),
                    provider: "ollama".to_string(),
                    provider_type: ProviderType::Ollama,
                    max_tokens: 128_000,
                    cost_per_1k: 0.0,
                    cost_per_1k_input: 0.0,
                    cost_per_1k_output: 0.0,
                    is_free: true,
                    strengths: vec!["codegen".to_string(), "parsing".to_string()],
                    capabilities: ModelCapabilities::default(),
                    supported_parameters: vec![],
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

pub(super) fn task_category_strength(task_type: TaskCategory) -> &'static str {
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
