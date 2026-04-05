//! LLM model specifications, capabilities, and routing keys.
//!
//! [`ModelRegistry`](crate::models::ModelRegistry) (in `registry.rs`) uses these types for task-category routing.

use crate::types::TaskCategory;
use crate::usage::LlmUsageKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_true() -> bool { true }

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

/// Rich capabilities for a model, imported from DeI and the OpenRouter /models catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModelCapabilities {
    pub supports_json: bool,
    pub supports_vision: bool,
    #[serde(default = "default_true")]
    pub supports_native_tools: bool,
    pub max_context: u64,
    pub tier: ModelTier,
    /// Provider-reported RPM limit (e.g. from OpenRouter `per_request_limits`).
    pub rate_limit_rpm: Option<u32>,
    /// Provider-reported RPD limit (e.g. from OpenRouter `per_request_limits`).
    pub rate_limit_rpd: Option<u32>,
    /// Median response latency in milliseconds from catalog metadata (p50).
    #[serde(default)]
    pub latency_p50_ms: Option<u32>,
    /// Whether the provider applies content moderation to outputs.
    #[serde(default)]
    pub is_moderated: bool,
    /// Provider-reported uptime score 0.0–1.0 (1.0 = fully available).
    #[serde(default)]
    pub uptime_score: Option<f32>,
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
    /// Populi Remote mesh endpoint
    PopuliMesh,
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
    PopuliMesh,
    CascadeFallback,
}

/// Resolve the transport/backend lane for a concrete model spec.
#[must_use]
pub fn route_backend_for_model(spec: &ModelSpec) -> ModelRouteBackend {
    match spec.provider_type {
        ProviderType::Ollama => ModelRouteBackend::Ollama,
        ProviderType::PopuliMesh => ModelRouteBackend::PopuliMesh,
        ProviderType::GoogleDirect => ModelRouteBackend::GeminiDirect,
        ProviderType::OpenRouter => ModelRouteBackend::OpenRouter,
        ProviderType::Groq
        | ProviderType::Mistral
        | ProviderType::DeepSeek
        | ProviderType::Cerebras
        | ProviderType::SambaNova
        | ProviderType::Custom(_) => {
            // P0 Fix: Map arbitrarily typed third-party providers (even those lacking '/') to
            // OpenRouter or a non-cascading endpoint. CascadeFallback on unknown IDs loops infinitely.
            ModelRouteBackend::OpenRouter
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
            ProviderType::PopuliMesh => LlmUsageKey {
                provider: "mens".to_string(),
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

/// Strength tags inferred from known provider families when name heuristics yield nothing.
///
/// Keyed on the provider prefix that appears before `/` in OpenRouter model ids (e.g. `anthropic`,
/// `openai`, `google`). Returns an empty slice for unknown prefixes so heuristics still apply.
#[must_use]
pub fn provider_family_strengths(provider_prefix: &str) -> &'static [&'static str] {
    match provider_prefix {
        "anthropic" => &["codegen", "logic", "review", "research", "ui-codegen", "frontend"],
        "openai" => &["codegen", "logic", "research"],
        "google" => &["research", "codegen", "logic"],
        "deepseek" => &["codegen", "logic", "debugging"],
        "qwen" | "qwen2" | "qwen2.5" => &["codegen", "logic"],
        "mistral" | "mistralai" => &["codegen", "logic"],
        "meta-llama" | "meta" => &["codegen", "logic", "research"],
        "cohere" => &["research", "review"],
        "perplexity" => &["research"],
        "x-ai" => &["research", "logic"],
        "nvidia" => &["codegen", "logic"],
        "01-ai" => &["logic", "codegen"],
        "amazon" => &["codegen", "research"],
        "microsoft" => &["codegen", "logic"],
        _ => &[],
    }
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
        TaskCategory::General | TaskCategory::Ars | TaskCategory::Planning => "logic",
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
        TaskCategory::General | TaskCategory::Ars | TaskCategory::Planning => "logic",
    }
}
