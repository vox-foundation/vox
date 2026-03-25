//! Wire JSON shapes and API key resolution for chat / stream.

use serde::{Deserialize, Serialize};
use std::env;

use super::types::{ChatMessage, LlmConfig};

#[derive(Serialize)]
pub(super) struct OpenRouterRequest<'a> {
    pub(super) model: &'a str,
    pub(super) messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) response_format: Option<&'a serde_json::Value>,
    pub(super) stream: bool,
}

#[derive(Deserialize, Debug)]
pub(super) struct OpenRouterResponse {
    pub(super) choices: Vec<OpenRouterChoice>,
    pub(super) usage: Option<OpenRouterUsage>,
    pub(super) model: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(super) struct OpenRouterChoice {
    pub(super) message: Option<OpenRouterMessage>,
}

#[derive(Deserialize, Debug)]
pub(super) struct OpenRouterMessage {
    pub(super) content: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct OpenRouterUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

pub(super) fn resolve_chat_api_key(config: &LlmConfig) -> String {
    config
        .api_key
        .clone()
        .unwrap_or_else(|| match config.provider.as_str() {
            "openrouter" => env::var("OPENROUTER_API_KEY").unwrap_or_default(),
            "openai" => env::var("OPENAI_API_KEY").unwrap_or_default(),
            "anthropic" => env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            "hf_router" | "huggingface" | "hf_endpoint" => {
                vox_config::inference::huggingface_hub_token().unwrap_or_default()
            }
            _ => String::new(),
        })
}

pub(super) fn chat_requires_nonempty_api_key(provider: &str) -> bool {
    matches!(provider, "openrouter" | "openai" | "anthropic")
}
