//! Durable chat completion and multi-candidate retry.

use std::future::Future;
use std::pin::Pin;

use reqwest::Client;

use crate::inference_env::HF_ROUTER_CHAT_COMPLETIONS_URL;
use crate::{ActivityOptions, ActivityResult, execute_activity};

use super::types::{ChatMessage, LlmConfig, LlmResponse};
use super::wire::{
    OpenRouterRequest, OpenRouterResponse, OpenRouterUsage, chat_requires_nonempty_api_key,
    resolve_chat_api_key,
};

type LlmChatActivityFuture =
    Pin<Box<dyn Future<Output = Result<Result<LlmResponse, String>, String>> + Send>>;

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

/// Exhaustive retry loop over multiple candidate LLM configurations.
/// Used for robust agent fallback routing. Iterates models sequentially until
/// one succeeds, skipping specific candidates on 401s or continuing on 429/timeout.
pub async fn infer_with_retry(
    options: &ActivityOptions,
    messages: Vec<ChatMessage>,
    candidates: Vec<LlmConfig>,
) -> ActivityResult<Result<(LlmResponse, LlmConfig), String>> {
    let mut last_error = "No LLM candidates provided".to_string();

    for candidate in candidates {
        match llm_chat(options, messages.clone(), candidate.clone()).await {
            ActivityResult::Ok(Ok(response)) => {
                return ActivityResult::Ok(Ok((response, candidate)));
            }
            ActivityResult::Ok(Err(api_err)) => {
                last_error = format!("Candidate {} failed: {}", candidate.model, api_err);
            }
            ActivityResult::Failed(activity_err) => {
                last_error = format!(
                    "Candidate {} activity error: {:?}",
                    candidate.model, activity_err
                );
            }
            ActivityResult::Cancelled => {
                return ActivityResult::Cancelled;
            }
        }
    }

    ActivityResult::Ok(Err(last_error))
}
