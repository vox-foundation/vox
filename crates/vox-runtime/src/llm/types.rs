//! Public LLM config, messages, metrics, and response types.

use serde::{Deserialize, Serialize};

use crate::inference_env::HF_ROUTER_CHAT_COMPLETIONS_URL;

/// Message format for the LLM chat API wire protocol (OpenAI-compatible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChatMessage {
    /// Chat role string (`system`, `user`, `assistant`, …).
    pub role: String,
    /// Message body text.
    pub content: String,
}

/// Deprecated alias kept for callers within this crate during the rename.
#[allow(dead_code)]
pub(crate) type ChatMessage = LlmChatMessage;

/// A configuration block for an LLM provider integration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider key (e.g. `openrouter`, `openai`, `anthropic`, `hf_router`).
    pub provider: String,
    /// Provider-specific model id (e.g. `anthropic/claude-3.5-sonnet`).
    pub model: String,
    /// Override chat completions URL; defaults are chosen from `provider`.
    pub base_url: Option<String>,
    /// API key or bearer token when the provider requires one.
    pub api_key: Option<String>,
    /// Sampling temperature when supported by the endpoint.
    pub temperature: Option<f32>,
    /// Maximum tokens to generate when supported.
    pub max_tokens: Option<u64>,
    /// Optional JSON Schema / response-format object for structured output.
    pub response_format: Option<serde_json::Value>,
    /// Optional HTTP timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

impl LlmConfig {
    /// Convenience constructor for OpenRouter.
    pub fn openrouter(model: impl Into<String>) -> Self {
        Self {
            provider: "openrouter".into(),
            model: model.into(),
            base_url: Some("https://openrouter.ai/api/v1/chat/completions".into()),
            api_key: std::env::var("OPENROUTER_API_KEY").ok(),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
        }
    }

    /// Convenience constructor for OpenAI-compatible endpoints.
    pub fn openai(model: impl Into<String>) -> Self {
        Self {
            provider: "openai".into(),
            model: model.into(),
            base_url: Some("https://api.openai.com/v1/chat/completions".into()),
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
        }
    }

    /// Hugging Face Inference Providers router (OpenAI-compatible chat completions).
    pub fn huggingface_router(model: impl Into<String>) -> Self {
        Self {
            provider: "hf_router".into(),
            model: model.into(),
            base_url: Some(HF_ROUTER_CHAT_COMPLETIONS_URL.to_string()),
            api_key: vox_config::inference::huggingface_hub_token(),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
        }
    }

    /// Resolve from a model registry alias.
    ///
    /// `registry` maps alias names (e.g. `"fast"`, `"smart"`) to
    /// `(provider, model_id, temperature, api_key_env)` tuples.
    pub fn from_registry(
        alias: &str,
        registry: &std::collections::HashMap<String, ModelRegistryEntry>,
    ) -> Result<Self, String> {
        let entry = registry
            .get(alias)
            .ok_or_else(|| format!("Unknown model alias: {}", alias))?;
        let api_key = entry
            .api_key_env
            .as_deref()
            .and_then(|env_name| std::env::var(env_name).ok())
            .or_else(|| match entry.provider.as_str() {
                "openrouter" => std::env::var("OPENROUTER_API_KEY").ok(),
                "openai" => std::env::var("OPENAI_API_KEY").ok(),
                "anthropic" => std::env::var("ANTHROPIC_API_KEY").ok(),
                "hf_router" | "huggingface" | "hf_endpoint" => {
                    vox_config::inference::huggingface_hub_token()
                }
                _ => None,
            });
        let base_url = entry
            .base_url
            .clone()
            .or_else(|| match entry.provider.as_str() {
                "openrouter" => Some("https://openrouter.ai/api/v1/chat/completions".into()),
                "openai" => Some("https://api.openai.com/v1/chat/completions".into()),
                "hf_router" | "huggingface" => Some(HF_ROUTER_CHAT_COMPLETIONS_URL.to_string()),
                "hf_endpoint" => None,
                _ => None,
            });
        Ok(Self {
            provider: entry.provider.clone(),
            model: entry.model.clone(),
            base_url,
            api_key,
            temperature: entry.temperature,
            max_tokens: entry.max_tokens,
            response_format: None,
            timeout_ms: entry.timeout_ms,
        })
    }
}

/// An entry in a Vox `@config model_registry:` block, deserialized at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryEntry {
    /// Provider family for this alias.
    pub provider: String,
    /// Model id passed to the provider API.
    pub model: String,
    /// Default temperature for this alias.
    pub temperature: Option<f32>,
    /// Default max output tokens for this alias.
    pub max_tokens: Option<u64>,
    /// Name of an environment variable holding the API key, if any.
    pub api_key_env: Option<String>,
    /// Optional override for the chat completions URL.
    pub base_url: Option<String>,
    /// Optional HTTP timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Tracks token usage and cost per LLM call — stored in @table ModelMetric.
/// Serializable so it can be persisted to VoxDB directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetric {
    /// Millisecond-timestamp of the completion.
    pub ts: u64,
    /// Model id as reported by the provider response.
    pub model: String,
    /// Provider key used for the call.
    pub provider: String,
    /// Prompt (input) token count from usage metadata.
    pub prompt_tokens: u32,
    /// Completion (output) token count from usage metadata.
    pub completion_tokens: u32,
    /// Estimated cost in USD (computed from a model registry lookup if available).
    pub estimated_cost_usd: f64,
}

impl ModelMetric {
    /// Build from an LlmResponse, computing cost at `cost_per_1k` rate.
    pub fn from_response(res: &LlmResponse, provider: &str, cost_per_1k: f64) -> Self {
        let total_tokens = res.prompt_tokens + res.completion_tokens;
        Self {
            ts: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            model: res.model.clone(),
            provider: provider.to_string(),
            prompt_tokens: res.prompt_tokens,
            completion_tokens: res.completion_tokens,
            estimated_cost_usd: (total_tokens as f64 / 1000.0) * cost_per_1k,
        }
    }
}

/// The standard parsed response from an LLM chat operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Assistant message text from the first choice.
    pub content: String,
    /// Prompt token usage when the API returned it.
    pub prompt_tokens: u32,
    /// Completion token usage when the API returned it.
    pub completion_tokens: u32,
    /// Model id from the response body, or the configured model as fallback.
    pub model: String,
}
