use super::metadata::HttpCallMetadata;
use super::types::{
    GeminiCandidate, GeminiContent, GeminiGenCfg, GeminiGenerateBody, GeminiPart, GeminiPartOut,
    GeminiResponse, GeminiSys, GeminiTurn,
};
use crate::mcp_tools::llm_bridge::error::HttpInferError;


pub(crate) async fn http_gemini_with_metadata(
    client: &reqwest::Client,
    model_id: &str,
    api_key: &str,
    system: &str,
    user: &str,
    max_tokens: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<(String, u32, u32, HttpCallMetadata), HttpInferError> {
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model_id}:generateContent?key={api_key}"
    );

    let system_instruction = if system.is_empty() {
        None
    } else {
        Some(GeminiSys {
            parts: vec![GeminiPartOut { text: system }],
        })
    };

    let response_mime_type = if json_mode {
        Some("application/json")
    } else {
        None
    };

    let body = GeminiGenerateBody {
        system_instruction,
        contents: vec![GeminiTurn {
            role: "user",
            parts: vec![GeminiPartOut { text: user }],
        }],
        generation_config: GeminiGenCfg {
            temperature,
            max_output_tokens: max_tokens.min(u32::MAX as u64) as u32,
            response_mime_type,
        },
    };

    let res = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| HttpInferError {
            status: 0,
            message: format!("Gemini HTTP: {e}"),
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

    let parsed: GeminiResponse = res.json().await.map_err(|e| HttpInferError {
        status: code,
        message: format!("Gemini JSON: {e}"),
    })?;

    let text = parsed
        .candidates
        .unwrap_or_default()
        .into_iter()
        .next()
        .and_then(|c: GeminiCandidate| c.content)
        .and_then(|c: GeminiContent| c.parts)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p: GeminiPart| p.text)
        .collect::<Vec<_>>()
        .join("");

    let prompt_t = parsed
        .usage_metadata
        .as_ref()
        .and_then(|u| u.prompt_token_count)
        .unwrap_or(0);
    let out_t = parsed
        .usage_metadata
        .as_ref()
        .and_then(|u| u.candidates_token_count)
        .unwrap_or(0);

    Ok((
        text,
        prompt_t,
        out_t,
        HttpCallMetadata {
            provider_request_id,
            provider_reported_cost_usd: None,
        },
    ))
}
