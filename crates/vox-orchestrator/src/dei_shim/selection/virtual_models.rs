use crate::models::{ModelCapabilities, ModelSpec, ModelTier, ProviderType};

/// Virtual/synthetic models. Merged into registry when conditions are met.
/// Single definition for openrouter/auto, openrouter/free, and any future virtual models.
pub fn virtual_models() -> Vec<ModelSpec> {
    use crate::provider_constants::openrouter as or_c;
    vec![
        ModelSpec {
            id: or_c::VIRTUAL_AUTO.to_string(),
            canonical_slug: None,
            provider: "openrouter".to_string(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 65_536,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            cost_per_1k: 0.0,
            is_free: false,
            supported_parameters: vec![
                "max_tokens".to_string(),
                "temperature".to_string(),
                "web_search_options".to_string(),
            ],
            strengths: vec![
                "codegen".to_string(),
                "debugging".to_string(),
                "research".to_string(),
                "review".to_string(),
            ],
            capabilities: ModelCapabilities {
                supports_vision: true,
                supports_json: true,
                supports_jsonl: true,
                max_context: 1_000_000,
                rate_limit_rpm: None,
                rate_limit_rpd: None,
                is_nsfw_capable: false,
                supports_web_search: true,
                supports_file_input: true,
                tier: ModelTier::Pro,
            },
        },
        ModelSpec {
            id: or_c::VIRTUAL_FREE.to_string(),
            canonical_slug: None,
            provider: "openrouter".to_string(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 65_536,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            cost_per_1k: 0.0,
            is_free: true,
            supported_parameters: vec![
                "max_tokens".to_string(),
                "temperature".to_string(),
            ],
            strengths: vec![
                "codegen".to_string(),
                "debugging".to_string(),
                "research".to_string(),
                "review".to_string(),
            ],
            capabilities: ModelCapabilities {
                supports_vision: true,
                supports_json: true,
                supports_jsonl: true,
                max_context: 1_000_000,
                rate_limit_rpm: Some(or_c::FREE_RPM),
                rate_limit_rpd: Some(or_c::FREE_RPD_NO_CREDIT),
                is_nsfw_capable: false,
                supports_web_search: false,
                supports_file_input: false,
                tier: ModelTier::Free,
            },
        },
    ]
}

/// Returns the `openrouter/auto` virtual model if OpenRouter models exist in the registry.
pub fn openrouter_auto_model(has_openrouter_models: bool) -> Option<ModelSpec> {
    if has_openrouter_models {
        virtual_models().into_iter().next()
    } else {
        None
    }
}

/// Returns the `openrouter/free` virtual model if OpenRouter models exist in the registry.
pub fn openrouter_free_model(has_openrouter_models: bool) -> Option<ModelSpec> {
    if has_openrouter_models {
        virtual_models().into_iter().nth(1)
    } else {
        None
    }
}
