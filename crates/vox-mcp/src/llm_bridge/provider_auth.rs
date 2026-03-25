use std::collections::HashMap;

use vox_orchestrator::models::{ModelSpec, ProviderType};

use super::error::HttpInferError;

fn required_env(key: &str, provider_label: &str) -> Result<String, HttpInferError> {
    let val = std::env::var(key).unwrap_or_default();
    if val.trim().is_empty() {
        return Err(HttpInferError {
            status: 0,
            message: format!("{key} is not set (required for {provider_label} models)"),
        });
    }
    Ok(val)
}

pub(crate) fn bearer_for(model: &ModelSpec) -> Result<String, HttpInferError> {
    match model.provider_type {
        ProviderType::OpenRouter => {
            let key = vox_config::inference::openrouter_api_key().unwrap_or_default();
            if key.trim().is_empty() {
                return Err(HttpInferError {
                    status: 0,
                    message: "OPENROUTER_API_KEY is not set (required for OpenRouter models)"
                        .into(),
                });
            }
            Ok(key)
        }
        ProviderType::Groq => required_env("GROQ_API_KEY", "Groq"),
        ProviderType::Cerebras => required_env("CEREBRAS_API_KEY", "Cerebras"),
        ProviderType::Mistral => required_env("MISTRAL_API_KEY", "Mistral"),
        ProviderType::DeepSeek => required_env("DEEPSEEK_API_KEY", "DeepSeek"),
        ProviderType::SambaNova => required_env("SAMBANOVA_API_KEY", "SambaNova"),
        ProviderType::Custom(_) => Ok(std::env::var("CUSTOM_OPENAI_API_KEY").unwrap_or_default()),
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
