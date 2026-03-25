//! SSE streaming chat completions.

use std::pin::Pin;

use futures_util::StreamExt;
use reqwest::Client;
use tokio_stream::Stream;

use crate::inference_env::HF_ROUTER_CHAT_COMPLETIONS_URL;

use super::types::{ChatMessage, LlmConfig};
use super::wire::{OpenRouterRequest, chat_requires_nonempty_api_key, resolve_chat_api_key};

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
