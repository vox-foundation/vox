use super::metadata::HttpCallMetadata;
use crate::mcp_tools::llm_bridge::error::HttpInferError;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: u64,
    temperature: f32,
    #[serde(skip_serializing_if = "str::is_empty")]
    system: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    id: String,
    content: Vec<AnthropicContentBlock>,
    usage: AnthropicUsage,
}

pub(crate) async fn http_anthropic_direct(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user: &str,
    max_tokens: u64,
    temperature: f32,
) -> Result<(String, u32, u32, HttpCallMetadata), HttpInferError> {
    let body = AnthropicRequest {
        model,
        max_tokens: max_tokens.max(1024), // Anthropic requires max_tokens > 0
        temperature,
        system,
        messages: vec![AnthropicMessage {
            role: "user",
            content: user,
        }],
    };

    let res = client
        .post(url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| HttpInferError {
            status: 0,
            message: format!("LLM HTTP: {e}"),
        })?;

    let status = res.status();
    let code = status.as_u16();

    let provider_request_id = res
        .headers()
        .get("request-id")
        .and_then(|v| v.to_str().ok())
        .map(ToString::to_string);

    if !status.is_success() {
        let t = res.text().await.unwrap_or_default();
        return Err(HttpInferError {
            status: code,
            message: t,
        });
    }

    let parsed: AnthropicResponse = res.json().await.map_err(|e| HttpInferError {
        status: code,
        message: format!("LLM JSON: {e}"),
    })?;

    let mut text = String::new();
    for block in parsed.content {
        if block.block_type == "text" {
            if let Some(t) = block.text {
                text.push_str(&t);
            }
        }
    }

    Ok((
        text,
        parsed.usage.input_tokens,
        parsed.usage.output_tokens,
        HttpCallMetadata {
            provider_request_id: provider_request_id.or(Some(parsed.id)),
            provider_reported_cost_usd: None,
        },
    ))
}
