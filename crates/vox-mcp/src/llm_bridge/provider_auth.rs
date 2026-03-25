use std::collections::HashMap;

use vox_orchestrator::models::{ModelSpec, ProviderType};

use super::error::HttpInferError;

fn required_secret(
    id: vox_clavis::SecretId,
    provider_label: &str,
) -> Result<String, HttpInferError> {
    let resolved = vox_clavis::resolve_secret(id);
    let value = resolved.expose().unwrap_or_default().to_string();
    if value.trim().is_empty() {
        return Err(HttpInferError {
            status: 0,
            message: format!(
                "{} is not set (required for {provider_label} models)",
                id.spec().canonical_env
            ),
        });
    }
    Ok(value)
}

pub(crate) fn bearer_for(model: &ModelSpec) -> Result<String, HttpInferError> {
    match model.provider_type {
        ProviderType::OpenRouter => {
            required_secret(vox_clavis::SecretId::OpenRouterApiKey, "OpenRouter")
        }
        ProviderType::Groq => required_secret(vox_clavis::SecretId::GroqApiKey, "Groq"),
        ProviderType::Cerebras => required_secret(vox_clavis::SecretId::CerebrasApiKey, "Cerebras"),
        ProviderType::Mistral => required_secret(vox_clavis::SecretId::MistralApiKey, "Mistral"),
        ProviderType::DeepSeek => required_secret(vox_clavis::SecretId::DeepSeekApiKey, "DeepSeek"),
        ProviderType::SambaNova => {
            required_secret(vox_clavis::SecretId::SambaNovaApiKey, "SambaNova")
        }
        ProviderType::Custom(_) => {
            required_secret(vox_clavis::SecretId::CustomOpenAiApiKey, "Custom OpenAI")
        }
        ProviderType::GoogleDirect | ProviderType::Ollama => Err(HttpInferError {
            status: 0,
            message: format!(
                "bearer_for is not applicable to provider {:?}",
                model.provider_type
            ),
        }),
    }
}

pub(crate) fn extra_headers_for(model: &ModelSpec) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    if matches!(model.provider_type, ProviderType::OpenRouter) {
        if let Ok(v) = std::env::var("VOX_OPENROUTER_HTTP_REFERER") {
            if !v.trim().is_empty() {
                headers.insert("HTTP-Referer".to_string(), v);
            }
        }
        if let Ok(v) = std::env::var("VOX_OPENROUTER_APP_TITLE") {
            if !v.trim().is_empty() {
                headers.insert("X-Title".to_string(), v);
            }
        }
    }
    headers
}
