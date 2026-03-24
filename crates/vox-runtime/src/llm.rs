use crate::inference_env::HF_ROUTER_CHAT_COMPLETIONS_URL;
use crate::{ActivityOptions, ActivityResult, execute_activity};
use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::future::Future;
use std::pin::Pin;
use tokio_stream::Stream;

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

type LlmChatActivityFuture =
    Pin<Box<dyn Future<Output = Result<Result<LlmResponse, String>, String>> + Send>>;
type LlmEmbedActivityFuture =
    Pin<Box<dyn Future<Output = Result<Result<Vec<f32>, String>, String>> + Send>>;

#[derive(Serialize)]
struct OpenRouterRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<&'a serde_json::Value>,
    stream: bool,
}

#[derive(Deserialize, Debug)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
    model: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenRouterChoice {
    message: Option<OpenRouterMessage>,
}

#[derive(Deserialize, Debug)]
struct OpenRouterMessage {
    content: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenRouterUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
}

fn resolve_chat_api_key(config: &LlmConfig) -> String {
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

fn chat_requires_nonempty_api_key(provider: &str) -> bool {
    matches!(provider, "openrouter" | "openai" | "anthropic")
}

/// Core durable wrapper for LLM chat (single complete response).
pub async fn llm_chat(
    options: &ActivityOptions,
    messages: Vec<ChatMessage>,
    config: LlmConfig,
) -> ActivityResult<Result<LlmResponse, String>> {
    let activity_name = format!("llm_chat_{}_{}", config.provider, config.model);

    execute_activity(&activity_name, options, || {
        let messages = messages.clone();
        let config = config.clone();

        let fut = async move {
            let api_key = resolve_chat_api_key(&config);

            if chat_requires_nonempty_api_key(&config.provider) && api_key.is_empty() {
                return Ok(Err("No API key available for LLM provider".to_string()));
            }

            let base_url = config
                .base_url
                .clone()
                .unwrap_or_else(|| match config.provider.as_str() {
                    "openrouter" => "https://openrouter.ai/api/v1/chat/completions".to_string(),
                    "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
                    "hf_router" | "huggingface" => HF_ROUTER_CHAT_COMPLETIONS_URL.to_string(),
                    _ => "https://openrouter.ai/api/v1/chat/completions".to_string(),
                });
            if matches!(config.provider.as_str(), "hf_endpoint")
                && (base_url.trim().is_empty()
                    || !base_url.contains("chat/completions"))
            {
                return Ok(Err(
                    "hf_endpoint requires a non-empty chat completions base_url (e.g. …/v1/chat/completions)"
                        .to_string(),
                ));
            }

            let client = Client::new();
            let req_body = OpenRouterRequest {
                model: &config.model,
                messages: &messages,
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                response_format: config.response_format.as_ref(),
                stream: false,
            };

            let mut req = client.post(&base_url).json(&req_body);
            if !api_key.is_empty() {
                req = req.bearer_auth(api_key);
            }
            let res = req
                .send()
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;

            if !res.status().is_success() {
                let err_text = res
                    .text()
                    .await
                    .unwrap_or_else(|_| String::from("<no body>"));
                return Ok(Err(format!("LLM API returned error: {}", err_text)));
            }

            let llm_res: OpenRouterResponse = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

            let content = llm_res
                .choices
                .into_iter()
                .next()
                .and_then(|c| c.message)
                .and_then(|m| m.content)
                .unwrap_or_default();

            let usage = llm_res.usage.unwrap_or(OpenRouterUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
            });

            Ok(Ok(LlmResponse {
                content,
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                model: llm_res.model.unwrap_or_else(|| config.model.clone()),
            }))
        };
        let fut_typed: LlmChatActivityFuture = Box::pin(fut);
        fut_typed
    })
    .await
}

/// Token-by-token streaming implementation.
pub async fn llm_stream(
    messages: Vec<ChatMessage>,
    config: LlmConfig,
) -> Result<Pin<Box<dyn Stream<Item = Result<String, String>> + Send>>, String> {
    let api_key = resolve_chat_api_key(&config);

    if chat_requires_nonempty_api_key(&config.provider) && api_key.is_empty() {
        return Err("No API key available for LLM provider".to_string());
    }

    let base_url = config
        .base_url
        .clone()
        .unwrap_or_else(|| match config.provider.as_str() {
            "openrouter" => "https://openrouter.ai/api/v1/chat/completions".to_string(),
            "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
            "hf_router" | "huggingface" => HF_ROUTER_CHAT_COMPLETIONS_URL.to_string(),
            _ => "https://openrouter.ai/api/v1/chat/completions".to_string(),
        });
    if matches!(config.provider.as_str(), "hf_endpoint")
        && (base_url.trim().is_empty() || !base_url.contains("chat/completions"))
    {
        return Err(
            "hf_endpoint requires a non-empty chat completions base_url (e.g. …/v1/chat/completions)"
                .to_string(),
        );
    }

    let client = Client::new();
    let req_body = OpenRouterRequest {
        model: &config.model,
        messages: &messages,
        temperature: config.temperature,
        max_tokens: config.max_tokens,
        response_format: config.response_format.as_ref(),
        stream: true,
    };

    let body = serde_json::to_string(&req_body).map_err(|e| e.to_string())?;

    let mut req = client
        .post(base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .body(body);
    if !api_key.is_empty() {
        req = req.bearer_auth(api_key);
    }
    let res = req
        .send()
        .await
        .map_err(|e| format!("HTTP request failed: {}", e))?;

    if !res.status().is_success() {
        let err_text = res
            .text()
            .await
            .unwrap_or_else(|_| String::from("<no body>"));
        return Err(format!("LLM API returned error: {}", err_text));
    }

    let byte_stream = res.bytes_stream();

    let string_stream = byte_stream.map(|chunk_res| {
        match chunk_res {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes);
                // Simple SSE parsing for just the token content
                // Very basic implementation targeting only OpenRouter / OpenAI SSE patterns
                let mut token_text = String::new();
                for line in text.lines() {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            continue;
                        }
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(choices) = parsed.get("choices") {
                                if let Some(choice) = choices.get(0) {
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(content) =
                                            delta.get("content").and_then(|c| c.as_str())
                                        {
                                            token_text.push_str(content);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(token_text)
            }
            Err(e) => Err(format!("Stream read error: {}", e)),
        }
    });

    Ok(Box::pin(string_stream))
}

#[derive(Serialize)]
struct OpenRouterEmbedRequest<'a> {
    model: &'a str,
    input: &'a str,
}

#[derive(Deserialize, Debug)]
struct OpenRouterEmbedResponse {
    data: Vec<OpenRouterEmbedData>,
    #[allow(dead_code)]
    usage: Option<OpenRouterUsage>,
    #[allow(dead_code)]
    model: Option<String>,
}

#[derive(Deserialize, Debug)]
struct OpenRouterEmbedData {
    embedding: Vec<f32>,
}

/// Core durable wrapper for LLM embedding generation.
pub async fn llm_embed(
    options: &ActivityOptions,
    text: &str,
    config: LlmConfig,
) -> ActivityResult<Result<Vec<f32>, String>> {
    let activity_name = format!("llm_embed_{}_{}", config.provider, config.model);

    execute_activity(&activity_name, options, || {
        let text = text.to_string();
        let config = config.clone();

        let fut = async move {
            let api_key = resolve_chat_api_key(&config);

            if chat_requires_nonempty_api_key(&config.provider) && api_key.is_empty() {
                return Ok(Err("No API key available for LLM provider".to_string()));
            }

            let base_url =
                config
                    .base_url
                    .clone()
                    .unwrap_or_else(|| match config.provider.as_str() {
                        "openrouter" => "https://openrouter.ai/api/v1/embeddings".to_string(),
                        "openai" => "https://api.openai.com/v1/embeddings".to_string(),
                        "hf_router" | "huggingface" => {
                            "https://router.huggingface.co/v1/embeddings".to_string()
                        }
                        _ => "https://openrouter.ai/api/v1/embeddings".to_string(),
                    });
            if matches!(config.provider.as_str(), "hf_endpoint")
                && (base_url.trim().is_empty() || !base_url.contains("embeddings"))
            {
                return Ok(Err(
                    "hf_endpoint embeddings require base_url pointing to …/v1/embeddings"
                        .to_string(),
                ));
            }

            let client = Client::new();
            let req_body = OpenRouterEmbedRequest {
                model: &config.model,
                input: &text,
            };

            let mut req = client.post(&base_url).json(&req_body);
            if !api_key.is_empty() {
                req = req.bearer_auth(api_key);
            }
            let res = req
                .send()
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;

            if !res.status().is_success() {
                let err_text = res
                    .text()
                    .await
                    .unwrap_or_else(|_| String::from("<no body>"));
                return Ok(Err(format!("LLM API returned error: {}", err_text)));
            }

            let embed_res: OpenRouterEmbedResponse = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

            let vector = embed_res
                .data
                .into_iter()
                .next()
                .map(|d| d.embedding)
                .unwrap_or_default();

            if vector.is_empty() {
                return Ok(Err("LLM API returned empty embedding vector".to_string()));
            }

            Ok(Ok(vector))
        };
        let fut_typed: LlmEmbedActivityFuture = Box::pin(fut);
        fut_typed
    })
    .await
}
