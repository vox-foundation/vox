use std::collections::HashMap;

use super::metadata::HttpCallMetadata;
use crate::mcp_tools::llm_bridge::error::HttpInferError;
use vox_openai_wire::{
    ChatCompletionRequest as OpenAiChatRequest, ChatCompletionResponse as OpenAiChatResponse,
    ChatMessageTurn as OpenAiMsg,
};


pub(crate) async fn http_openai_compatible_with_headers(
    client: &reqwest::Client,
    url: &str,
    bearer: &str,
    model: &str,
    system: &str,
    user: vox_openai_wire::ChatMessageContent<'_>,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
    tools: Option<serde_json::Value>,
    tool_choice: Option<serde_json::Value>,
    extra_headers: &HashMap<String, String>,
) -> Result<(String, u32, u32, HttpCallMetadata), HttpInferError> {
    let mut messages = Vec::new();
    if !system.is_empty() {
        messages.push(OpenAiMsg {
            role: "system",
            content: vox_openai_wire::ChatMessageContent::Text(system),
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

    let requested_tools = tools.is_some();
    let body = OpenAiChatRequest {
        model,
        messages,
        temperature,
        max_tokens,
        stream: false,
        response_format,
        tools,
        tool_choice,
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

    let message = parsed.choices.into_iter().next().and_then(|c| c.message);

    fn coerce_json_fallback(mut s: &str) -> String {
        s = s.trim();
        if s.starts_with("```json") {
            s = s[7..].trim_start();
        } else if s.starts_with("```") {
            s = s[3..].trim_start();
        }
        if s.ends_with("```") {
            s = s[..s.len() - 3].trim_end();
        }
        s.to_string()
    }

    let text = if let Some(m) = message {
        if let Some(mut tc) = m.tool_calls {
            if let Some(first) = tc.pop() {
                first.function.arguments
            } else {
                let s = m.content.unwrap_or_default();
                if requested_tools || json_mode {
                    coerce_json_fallback(&s)
                } else {
                    s
                }
            }
        } else {
            let s = m.content.unwrap_or_default();
            if requested_tools || json_mode {
                coerce_json_fallback(&s)
            } else {
                s
            }
        }
    } else {
        String::new()
    };

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
