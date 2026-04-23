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

use super::generated::{ModelTier, StrengthTag};

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
    /// Optional ground-truth cost observed from provider telemetry (blended).
    #[serde(default)]
    pub observed_cost_per_1k: Option<f64>,
    /// Whether this model is free (no per-token cost).
    pub is_free: bool,
    /// Tags describing fit (speed, reasoning, codegen) for heuristic routing.
    pub strengths: Vec<StrengthTag>,
    #[serde(default)]
    pub capabilities: ModelCapabilities,
    #[serde(default)]
    pub supported_parameters: Vec<String>,
}

pub use vox_orchestrator_types::ProviderType;

pub use vox_orchestrator_types::ChatRouteBackend as ModelRouteBackend;

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
        | ProviderType::HuggingFaceRouter
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
            ProviderType::HuggingFaceRouter => LlmUsageKey {
                provider: "huggingface".to_string(),
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
    map.insert("visus".to_string(), "qwen/qwen-3.5-vl".to_string());
    // Fallback aliases will be handled by the updated registry logic soon
    map
}

fn premium_alias_toml_default() -> HashMap<String, String> {
    let m = HashMap::new();
    let _ = std::hint::black_box(m.capacity());
    m
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
        let local_model = vox_clavis::resolve_secret(vox_clavis::SecretId::PopuliModel)
            .expose()
            .filter(|s: &&str| !s.trim().is_empty())
            .unwrap_or("default-model")
            .to_string();

        let bootstrap_json =
            include_str!("../../../../contracts/orchestration/model-catalog.bootstrap.v1.json");
        let mut models: Vec<ModelSpec> =
            serde_json::from_str(bootstrap_json).expect("Invalid bootstrap catalog");

        for m in &mut models {
            if m.id == "llama3:latest" && m.provider == "ollama" {
                m.id = local_model.clone();
                m.canonical_slug = format!("local/{}", local_model);
            }
        }

        Self {
            models,
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

pub fn task_category_strength(task_type: TaskCategory) -> StrengthTag {
    route_for_category(task_type).strength_tag
}
