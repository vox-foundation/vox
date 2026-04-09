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
        ProviderType::Anthropic => {
            required_secret(vox_clavis::SecretId::AnthropicApiKey, "Anthropic")
        }
        ProviderType::Custom(_) => {
            required_secret(vox_clavis::SecretId::CustomOpenAiApiKey, "Custom OpenAI")
        }
        ProviderType::GoogleDirect | ProviderType::Ollama | ProviderType::PopuliMesh => {
            Err(HttpInferError {
                status: 0,
                message: format!(
                    "bearer_for is not applicable to provider {:?}",
                    model.provider_type
                ),
            })
        }
    }
}

/// Returns extra HTTP headers to send with a provider request.
///
/// For OpenRouter models, always injects the attribution headers when the env vars are set.
/// For `openrouter/auto` specifically, also injects the `X-OpenRouter-Provider-Preferences`
/// route hint header so OR's internal broker honours our cost-preference intent.
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
        // For the virtual auto-routing model, inject cost-preference route hint so
        // OpenRouter's broker picks the provider that matches our intent.
        if model.id == vox_config::OPENROUTER_AUTO {
            let hint = openrouter_route_hint_from_env();
            headers.insert(
                "X-OpenRouter-Provider-Preferences".to_string(),
                format!("{{\"route\":\"{}\"}}", hint.as_route_str()),
            );
        }
    }
    headers
}

/// Resolve the [`OpenRouterRouteHint`] from the `VOX_OPENROUTER_ROUTE_HINT` env var.
/// Falls back to `Fallback` (resilience-first) when unset or unknown.
fn openrouter_route_hint_from_env() -> vox_config::OpenRouterRouteHint {
    use vox_config::{OpenRouterRouteHint, RouteCostPreference, derive_openrouter_route_hint};
    let raw = std::env::var("VOX_OPENROUTER_ROUTE_HINT").unwrap_or_default();
    match raw.trim().to_ascii_lowercase().as_str() {
        "price" | "economy" | "cheap" => OpenRouterRouteHint::Price,
        "quality" | "performance" | "best" => OpenRouterRouteHint::Quality,
        "fallback" | "resilience" => OpenRouterRouteHint::Fallback,
        // Derive from orchestrator cost preference env when explicit hint absent.
        _ => {
            let pref_raw = std::env::var("VOX_COST_PREFERENCE").unwrap_or_default();
            let pref = match pref_raw.trim().to_ascii_lowercase().as_str() {
                "performance" | "quality" => RouteCostPreference::Performance,
                _ => RouteCostPreference::Economy,
            };
            derive_openrouter_route_hint(pref)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator::models::{ModelCapabilities, ModelSpec, ProviderType};

    fn mesh_model() -> ModelSpec {
        ModelSpec {
            id: "mesh-model".into(),
            canonical_slug: "mesh/model".into(),
            provider: "mesh".into(),
            provider_type: ProviderType::PopuliMesh,
            max_tokens: 8000,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            strengths: vec![],
            capabilities: ModelCapabilities::default(),
            supported_parameters: vec![],
        }
    }

    #[test]
    fn test_bearer_for_populi_mesh_rejected() {
        let model = mesh_model();
        let err = bearer_for(&model).expect_err("should reject mesh");
        assert!(
            err.message
                .contains("not applicable to provider PopuliMesh")
        );
    }

    #[test]
    fn extra_headers_openrouter_attribution_injected() {
        // SAFETY: test-only env mutation under single-threaded test.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_OPENROUTER_HTTP_REFERER", "https://vox.app");
            std::env::set_var("VOX_OPENROUTER_APP_TITLE", "Vox");
        }
        let model = ModelSpec {
            id: "openai/gpt-4o".into(),
            canonical_slug: "openai/gpt-4o".into(),
            provider: "openai".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 8192,
            cost_per_1k: 0.01,
            cost_per_1k_input: 0.01,
            cost_per_1k_output: 0.01,
            is_free: false,
            strengths: vec![],
            capabilities: ModelCapabilities::default(),
            supported_parameters: vec![],
        };
        let headers = extra_headers_for(&model);
        assert_eq!(
            headers.get("HTTP-Referer").map(String::as_str),
            Some("https://vox.app")
        );
        assert_eq!(headers.get("X-Title").map(String::as_str), Some("Vox"));
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_OPENROUTER_HTTP_REFERER");
            std::env::remove_var("VOX_OPENROUTER_APP_TITLE");
        }
    }
}
