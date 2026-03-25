use std::future::Future;
use std::pin::Pin;
use vox_orchestrator::models::{ModelSpec, ProviderType};

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
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct InferRequest<'a> {
    pub system_prompt: &'a str,
    pub user_prompt: &'a str,
    pub max_t: u64,
    pub temperature: f32,
    pub json_mode: bool,
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
                req.system_prompt,
                req.user_prompt,
                req.max_t,
                req.temperature,
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
                req.json_mode,
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
                    req.json_mode,
                    &headers,
                )
                .await?;
            Ok(adapt_result(text, prompt_tokens, completion_tokens, meta))
        })
    }
}

fn adapters() -> Vec<Box<dyn ProviderAdapter>> {
    vec![
        Box::new(GoogleDirectAdapter),
        Box::new(OllamaAdapter),
        Box::new(OpenAiCompatAdapter),
    ]
}

pub(crate) async fn infer_via_provider_adapter(
    client: &reqwest::Client,
    model: &ModelSpec,
    system_prompt: &str,
    user_prompt: &str,
    max_t: u64,
    temperature: f32,
    json_mode: bool,
) -> Result<ProviderInferResult, HttpInferError> {
    let req = InferRequest {
        system_prompt,
        user_prompt,
        max_t,
        temperature,
        json_mode,
    };
    for adapter in adapters() {
        if adapter.supports(&model.provider_type) {
            return adapter.infer(client, model, req).await;
        }
    }
    Err(HttpInferError {
        status: 0,
        message: format!("No provider adapter found for {:?}", model.provider_type),
    })
}
