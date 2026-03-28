use std::collections::HashMap;

use super::metadata::HttpCallMetadata;
use crate::llm_bridge::error::HttpInferError;
use vox_openai_wire::{
    ChatCompletionRequest as OpenAiChatRequest, ChatCompletionResponse as OpenAiChatResponse,
    ChatCompletionUsage as OpenAiUsage, ChatMessageTurn as OpenAiMsg,
};

pub(crate) async fn http_openai_compatible(
    client: &reqwest::Client,
    url: &str,
    bearer: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<(String, u32, u32), HttpInferError> {
    let (text, in_tok, out_tok, _) = http_openai_compatible_with_headers(
        client,
        url,
        bearer,
        model,
        system,
        user,
        max_tokens,
        temperature,
        json_mode,
        &HashMap::new(),
    )
    .await?;
    Ok((text, in_tok, out_tok))
}

pub(crate) async fn http_openai_compatible_with_headers(
    client: &reqwest::Client,
    url: &str,
    bearer: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
    extra_headers: &HashMap<String, String>,
) -> Result<(String, u32, u32, HttpCallMetadata), HttpInferError> {
    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(OpenAiMsg {
            role: "system",
            content: system,
        });
    }
    messages.push(OpenAiMsg {
        role: "user",
        content: user,
    });

    let response_format = if json_mode {
        Some(serde_json::json!({ "type": "json_object" }))
    } else {
        None
    };

    let body = OpenAiChatRequest {
        model,
        messages,
        temperature,
        max_tokens,
        stream: false,
        response_format,
    };

    let mut req = client.post(url).json(&body);
    if !bearer.is_empty() {
        req = req.bearer_auth(bearer);
    }
    for (k, v) in extra_headers {
        req = req.header(k, v);
    }

    let res = req.send().await.map_err(|e| HttpInferError {
        status: 0,
        message: format!("LLM HTTP: {e}"),
    })?;
    let status = res.status();
    let code = status.as_u16();
    let provider_request_id = res
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    if !status.is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(HttpInferError {
            status: code,
            message: t,
        });
    }

    let parsed: OpenAiChatResponse = res.json().await.map_err(|e| HttpInferError {
        status: code,
        message: format!("LLM JSON: {e}"),
    })?;

    let text = parsed
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message)
        .and_then(|m| m.content)
        .unwrap_or_default();

    let u = parsed.usage.unwrap_or_default();
    let provider_reported_cost_usd = u.total_cost.or(u.cost);
    Ok((
        text,
        u.prompt_tokens,
        u.completion_tokens,
        HttpCallMetadata {
            provider_request_id: provider_request_id.or(parsed.id),
            provider_reported_cost_usd,
        },
    ))
}
