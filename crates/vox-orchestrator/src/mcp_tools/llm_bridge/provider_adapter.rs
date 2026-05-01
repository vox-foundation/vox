use crate::models::{ModelSpec, ProviderType};
use std::future::Future;
use std::pin::Pin;

use super::error::HttpInferError;
use super::provider_auth::{bearer_for, extra_headers_for};
use super::provider_endpoints::endpoint_for;
use super::providers::{
    HttpCallMetadata, http_gemini_with_metadata, http_ollama_with_metadata,
    http_openai_compatible_with_headers, probe_ollama_tags,
};

#[derive(Debug, Clone)]
pub(crate) struct ProviderInferResult {
    pub text: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub provider_request_id: Option<String>,
    pub provider_reported_cost_usd: Option<f64>,
    /// Tokens that were served from the provider's prompt cache for this call, if reported.
    pub cached_input_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct InferRequest<'a> {
    pub system_prompt: &'a str,
    pub user_prompt: vox_openai_wire::ChatMessageContent<'a>,
    pub max_t: u64,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub json_mode: bool,
    pub tools: Option<serde_json::Value>,
    pub tool_choice: Option<serde_json::Value>,
}

trait ProviderAdapter: Send + Sync {
    fn supports(&self, provider_type: &ProviderType) -> bool;
    fn infer<'a>(
        &'a self,
        client: &'a reqwest::Client,
        model: &'a ModelSpec,
        req: InferRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderInferResult, HttpInferError>> + Send + 'a>>;
}

struct GoogleDirectAdapter;
struct OllamaAdapter;
struct AnthropicNativeAdapter;
struct OpenAiCompatAdapter;

fn adapt_result(
    text: String,
    prompt_tokens: u32,
    completion_tokens: u32,
    meta: HttpCallMetadata,
) -> ProviderInferResult {
    ProviderInferResult {
        text,
        prompt_tokens,
        completion_tokens,
        provider_request_id: meta.provider_request_id,
        provider_reported_cost_usd: meta.provider_reported_cost_usd,
        cached_input_tokens: meta.cached_input_tokens,
    }
}

impl ProviderAdapter for GoogleDirectAdapter {
    fn supports(&self, provider_type: &ProviderType) -> bool {
        matches!(provider_type, ProviderType::GoogleDirect)
    }

    fn infer<'a>(
        &'a self,
        client: &'a reqwest::Client,
        model: &'a ModelSpec,
        req: InferRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderInferResult, HttpInferError>> + Send + 'a>>
    {
        Box::pin(async move {
            let resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::GeminiApiKey);
            let key = resolved.expose().ok_or_else(|| HttpInferError {
                status: 0,
                message: "GEMINI_API_KEY is not set (required for Google-direct models)".into(),
            })?;
            let (text, prompt_tokens, completion_tokens, meta) = http_gemini_with_metadata(
                client,
                &model.id,
                key,
                model,
                req.system_prompt,
                req.user_prompt,
                req.max_t,
                req.temperature,
                req.top_p,
                req.json_mode,
            )
            .await?;
            Ok(adapt_result(text, prompt_tokens, completion_tokens, meta))
        })
    }
}

impl ProviderAdapter for OllamaAdapter {
    fn supports(&self, provider_type: &ProviderType) -> bool {
        matches!(provider_type, ProviderType::Ollama)
    }

    fn infer<'a>(
        &'a self,
        client: &'a reqwest::Client,
        model: &'a ModelSpec,
        req: InferRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderInferResult, HttpInferError>> + Send + 'a>>
    {
        Box::pin(async move {
            probe_ollama_tags(client).await?;
            let (text, prompt_tokens, completion_tokens, meta) = http_ollama_with_metadata(
                client,
                &model.id,
                req.system_prompt,
                req.user_prompt,
                req.max_t,
                req.temperature,
                req.top_p,
                req.json_mode,
            )
            .await?;
            Ok(adapt_result(text, prompt_tokens, completion_tokens, meta))
        })
    }
}
impl ProviderAdapter for AnthropicNativeAdapter {
    fn supports(&self, provider_type: &ProviderType) -> bool {
        matches!(provider_type, ProviderType::Anthropic)
            && vox_clavis::resolve_secret(vox_clavis::SecretId::VoxAnthropicDirect)
                .expose()
                .unwrap_or("")
                == "1"
    }

    fn infer<'a>(
        &'a self,
        client: &'a reqwest::Client,
        model: &'a ModelSpec,
        req: InferRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderInferResult, HttpInferError>> + Send + 'a>>
    {
        Box::pin(async move {
            use super::providers::http_anthropic_direct;
            let url = endpoint_for(model)?;
            let bearer = super::provider_auth::bearer_for(model)?;
            let api_key = if bearer.starts_with("Bearer ") {
                &bearer[7..]
            } else {
                &bearer
            };

            let (text, prompt_tokens, completion_tokens, meta) = http_anthropic_direct(
                client,
                &url,
                api_key,
                model,
                &model.id,
                req.system_prompt,
                req.user_prompt,
                req.max_t,
                req.temperature,
                req.top_p,
            )
            .await?;
            Ok(adapt_result(text, prompt_tokens, completion_tokens, meta))
        })
    }
}

impl ProviderAdapter for OpenAiCompatAdapter {
    fn supports(&self, provider_type: &ProviderType) -> bool {
        !matches!(
            provider_type,
            ProviderType::GoogleDirect | ProviderType::Ollama
        )
    }

    fn infer<'a>(
        &'a self,
        client: &'a reqwest::Client,
        model: &'a ModelSpec,
        req: InferRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderInferResult, HttpInferError>> + Send + 'a>>
    {
        Box::pin(async move {
            let url = endpoint_for(model)?;
            let bearer = bearer_for(model)?;
            let headers = extra_headers_for(model);
            let (text, prompt_tokens, completion_tokens, meta) =
                http_openai_compatible_with_headers(
                    client,
                    &url,
                    &bearer,
                    &model.id,
                    req.system_prompt,
                    req.user_prompt,
                    req.max_t,
                    req.temperature,
                    req.top_p,
                    req.json_mode,
                    req.tools.clone(),
                    req.tool_choice.clone(),
                    &headers,
                )
                .await?;
            Ok(adapt_result(text, prompt_tokens, completion_tokens, meta))
        })
    }
}

struct VoxLocalAdapter;

#[derive(serde::Serialize)]
struct VoxLocalGenerateRequest<'a> {
    prompt: &'a str,
    validate: bool,
    max_retries: u32,
}

#[derive(serde::Deserialize)]
struct VoxLocalGenerateResponse {
    code: String,
    valid: Option<bool>,
    #[serde(default)]
    errors: Vec<String>,
}

fn extract_prompt_text<'a>(content: &'a vox_openai_wire::ChatMessageContent<'a>) -> &'a str {
    match content {
        vox_openai_wire::ChatMessageContent::Text(t) => t,
        vox_openai_wire::ChatMessageContent::Parts(parts) => parts
            .iter()
            .find_map(|p| match p {
                vox_openai_wire::ChatMessagePart::Text { text } => Some(*text),
                _ => None,
            })
            .unwrap_or(""),
    }
}

impl ProviderAdapter for VoxLocalAdapter {
    fn supports(&self, provider_type: &ProviderType) -> bool {
        matches!(provider_type, ProviderType::VoxLocal)
    }

    fn infer<'a>(
        &'a self,
        client: &'a reqwest::Client,
        model: &'a ModelSpec,
        req: InferRequest<'a>,
    ) -> Pin<Box<dyn Future<Output = Result<ProviderInferResult, HttpInferError>> + Send + 'a>>
    {
        Box::pin(async move {
            let endpoint = endpoint_for(model)?;
            let prompt = extract_prompt_text(&req.user_prompt);
            let body = VoxLocalGenerateRequest {
                prompt,
                validate: true,
                max_retries: 3,
            };
            let resp = client
                .post(&endpoint)
                .json(&body)
                .send()
                .await
                .map_err(|e| HttpInferError {
                    status: 0,
                    message: format!("VoxLocal /generate request failed: {e}"),
                })?;

            let status = resp.status().as_u16();
            if !resp.status().is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(HttpInferError {
                    status,
                    message: format!("VoxLocal server error {status}: {body}"),
                });
            }

            let parsed: VoxLocalGenerateResponse =
                resp.json().await.map_err(|e| HttpInferError {
                    status: 0,
                    message: format!("VoxLocal response parse error: {e}"),
                })?;

            if parsed.valid == Some(false) && !parsed.errors.is_empty() {
                tracing::warn!(
                    target: "vox.mcp.llm.vox_local",
                    errors = ?parsed.errors,
                    "VoxLocal returned invalid code"
                );
            }

            // Token counts are not reported by the 7863 server; estimate from byte length.
            let approx_tokens = (parsed.code.len() / 4) as u32;
            Ok(ProviderInferResult {
                text: parsed.code,
                prompt_tokens: 0,
                completion_tokens: approx_tokens,
                provider_request_id: None,
                provider_reported_cost_usd: None,
                cached_input_tokens: None,
            })
        })
    }
}

fn adapters() -> Vec<Box<dyn ProviderAdapter>> {
    vec![
        Box::new(GoogleDirectAdapter),
        Box::new(VoxLocalAdapter),
        Box::new(OllamaAdapter),
        Box::new(AnthropicNativeAdapter),
        Box::new(OpenAiCompatAdapter),
    ]
}

pub(crate) async fn infer_via_provider_adapter(
    client: &reqwest::Client,
    model: &ModelSpec,
    system_prompt: &str,
    user_prompt: vox_openai_wire::ChatMessageContent<'_>,
    max_t: u64,
    temperature: Option<f32>,
    top_p: Option<f32>,
    json_mode: bool,
    tools: Option<serde_json::Value>,
    tool_choice: Option<serde_json::Value>,
) -> Result<ProviderInferResult, HttpInferError> {
    let req = InferRequest {
        system_prompt,
        user_prompt,
        max_t,
        temperature,
        top_p,
        json_mode,
        tools,
        tool_choice,
    };
    for adapter in adapters() {
        if adapter.supports(&model.provider_type) {
            return adapter.infer(client, model, req.clone()).await;
        }
    }
    Err(HttpInferError {
        status: 0,
        message: format!("No provider adapter found for {:?}", model.provider_type),
    })
}
