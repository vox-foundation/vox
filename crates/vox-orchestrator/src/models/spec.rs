//! LLM model specifications, capabilities, and routing keys.
//!
//! [`ModelRegistry`](crate::models::ModelRegistry) (in `registry.rs`) uses these types for task-category routing.

use crate::types::TaskCategory;
use crate::usage::LlmUsageKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

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
    /// Anthropic direct or proxy endpoint
    Anthropic,
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
        | ProviderType::Anthropic
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
            ProviderType::Anthropic => LlmUsageKey {
                provider: "anthropic".to_string(),
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
    let mut map = HashMap::new();
    let mythos_id = "claude-mythos-preview-20260407".to_string();
    let sonnet_id = "anthropic/claude-sonnet-4.6".to_string();
    let pro_planning_id = "google/gemini-2.5-pro-preview".to_string();
    let r1_id = "deepseek/deepseek-r1".to_string();
    
    map.insert("codegen".to_string(), mythos_id.clone());
    map.insert("debugging".to_string(), mythos_id.clone());
    map.insert("security".to_string(), mythos_id.clone());
    map.insert("research".to_string(), pro_planning_id.clone());
    map.insert("planning".to_string(), pro_planning_id.clone());
    map.insert("review".to_string(), sonnet_id.clone());
    map.insert("logic".to_string(), r1_id.clone());
    // Fallback aliases will be handled by the updated registry logic soon
    map
}

/// Strength tags inferred from known provider families when name heuristics yield nothing.
///
/// Keyed on the provider prefix that appears before `/` in OpenRouter model ids (e.g. `anthropic`,
/// `openai`, `google`). Returns an empty slice for unknown prefixes so heuristics still apply.
#[must_use]
pub fn provider_family_strengths(provider_prefix: &str) -> &'static [&'static str] {
    match provider_prefix {
        "anthropic" => &[
            "codegen",
            "logic",
            "review",
            "research",
            "ui-codegen",
            "frontend",
        ],
        "openai" => &["codegen", "logic", "research"],
        "google" => &["research", "codegen", "logic"],
        "deepseek" => &["codegen", "logic", "debugging"],
        "qwen" | "qwen2" | "qwen2.5" | "qwen3" | "qwen3.5" | "qwen3_5" => &["codegen", "logic"],
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
                // ── Fast / Free Tier ──
                ModelSpec {
                    id: "qwen/qwen3-coder:free".to_string(),
                    canonical_slug: "qwen/qwen3-free".to_string(),
                    provider: "openrouter".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 32_000,
                    cost_per_1k: 0.0,
                    cost_per_1k_input: 0.0,
                    cost_per_1k_output: 0.0,
                    is_free: true,
                    strengths: vec!["codegen".to_string(), "parsing".to_string()],
                    capabilities: ModelCapabilities {
                        tier: ModelTier::Light,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                },
                ModelSpec {
                    id: "meta-llama/llama-4-scout:free".to_string(),
                    canonical_slug: "llama/llama-4-free".to_string(),
                    provider: "openrouter".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 128_000,
                    cost_per_1k: 0.0,
                    cost_per_1k_input: 0.0,
                    cost_per_1k_output: 0.0,
                    is_free: true,
                    strengths: vec!["inter_agent".to_string(), "logic".to_string()],
                    capabilities: ModelCapabilities {
                        tier: ModelTier::Light,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                },
                ModelSpec {
                    id: "google/gemini-2.0-flash-lite".to_string(),
                    canonical_slug: "google/gemini-flash-lite".to_string(),
                    provider: "google".to_string(),
                    provider_type: ProviderType::GoogleDirect,
                    max_tokens: 1_000_000,
                    cost_per_1k: 0.0,
                    cost_per_1k_input: 0.0,
                    cost_per_1k_output: 0.0,
                    is_free: true,
                    strengths: vec!["logic".to_string(), "inter_agent".to_string()],
                    capabilities: ModelCapabilities {
                        tier: ModelTier::Light,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                },
                // ── Pro Tier ──
                ModelSpec {
                    id: "meta-llama/llama-4-maverick".to_string(),
                    canonical_slug: "llama/llama-4-maverick".to_string(),
                    provider: "openrouter".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 128_000,
                    cost_per_1k: 0.06,
                    cost_per_1k_input: 0.02,
                    cost_per_1k_output: 0.1,
                    is_free: false,
                    strengths: vec!["inter_agent".to_string(), "logic".to_string()],
                    capabilities: ModelCapabilities {
                        tier: ModelTier::Pro,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                },
                ModelSpec {
                    id: "deepseek/deepseek-r1".to_string(),
                    canonical_slug: "deepseek/deepseek-r1".to_string(),
                    provider: "openrouter".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 128_000,
                    cost_per_1k: 0.2, // blended
                    cost_per_1k_input: 0.014,
                    cost_per_1k_output: 0.28,
                    is_free: false,
                    strengths: vec!["logic".to_string(), "review".to_string()],
                    capabilities: ModelCapabilities {
                        tier: ModelTier::Pro,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                },
                ModelSpec {
                    id: "google/gemini-2.5-pro-preview".to_string(),
                    canonical_slug: "google/gemini-pro".to_string(),
                    provider: "openrouter".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 2_000_000,
                    cost_per_1k: 5.0,
                    cost_per_1k_input: 1.25,
                    cost_per_1k_output: 5.0,
                    is_free: false,
                    strengths: vec!["planning".to_string(), "research".to_string()],
                    capabilities: ModelCapabilities {
                        supports_vision: true,
                        tier: ModelTier::Pro,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                },
                ModelSpec {
                    id: "anthropic/claude-sonnet-4.6".to_string(),
                    canonical_slug: "anthropic/sonnet".to_string(),
                    provider: "openrouter".to_string(),
                    provider_type: ProviderType::OpenRouter,
                    max_tokens: 200_000,
                    cost_per_1k: 15.0,
                    cost_per_1k_input: 3.0,
                    cost_per_1k_output: 15.0,
                    is_free: false,
                    strengths: vec!["codegen".to_string(), "review".to_string(), "debugging".to_string(), "security".to_string()],
                    capabilities: ModelCapabilities {
                        supports_vision: true,
                        tier: ModelTier::Pro,
                        ..Default::default()
                    },
                    supported_parameters: vec![],
                },
                // ── Elite Tier ──
                ModelSpec {
                    id: "claude-mythos-preview-20260407".to_string(),
                    canonical_slug: "anthropic/mythos-preview".to_string(),
                    provider: "anthropic".to_string(),
                    provider_type: ProviderType::Anthropic,
                    max_tokens: 200_000,
                    cost_per_1k: 125.0,
                    cost_per_1k_input: 25.0,
                    cost_per_1k_output: 125.0,
                    is_free: false,
                    strengths: vec![
                        "codegen".to_string(),
                        "debugging".to_string(),
                        "logic".to_string(),
                        "review".to_string(),
                        "research".to_string(),
                        "security".to_string(),
                    ],
                    capabilities: ModelCapabilities {
                        supports_json: true,
                        supports_vision: true,
                        supports_native_tools: true,
                        tier: ModelTier::Elite,
                        ..Default::default()
                    },
                    supported_parameters: vec![
                        "tools".to_string(),
                        "response_format".to_string(),
                        "reasoning".to_string(),
                    ],
                },
            ],
            premium_alias: built_in_premium_alias(),
        }
    }
}

use super::routing_table::route_for_category;

/// Maps [`TaskCategory`] to a `premium_alias` / routing strength key.
#[must_use]
pub fn task_category_premium_key(task_type: TaskCategory) -> &'static str {
    route_for_category(task_type).premium_alias_key
}

pub(super) fn task_category_strength(task_type: TaskCategory) -> &'static str {
    route_for_category(task_type).strength_tag
}
