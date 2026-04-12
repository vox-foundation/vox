//! Utilities to check whether provider API keys are present.

use super::ProviderType;
use vox_clavis::SecretId;

/// Checks if the primary required secret for a given provider type is currently available.
#[must_use]
pub fn provider_secret_is_available(ptype: &ProviderType) -> bool {
    let secret_id = match ptype {
        ProviderType::GoogleDirect => SecretId::GeminiApiKey,
        ProviderType::OpenRouter => SecretId::OpenRouterApiKey,
        ProviderType::Groq => SecretId::GroqApiKey,
        ProviderType::Cerebras => SecretId::CerebrasApiKey,
        ProviderType::Mistral => SecretId::MistralApiKey,
        ProviderType::DeepSeek => SecretId::DeepSeekApiKey,
        ProviderType::SambaNova => SecretId::SambaNovaApiKey,
        ProviderType::Anthropic => SecretId::AnthropicApiKey,
        ProviderType::Custom(_) => SecretId::CustomOpenaiApiKey,
        ProviderType::Ollama | ProviderType::PopuliMesh => {
            // Local endpoints don't strictly require a clavis secret in the same way,
            // or use environment variables instead.
            return true;
        }
    };

    vox_clavis::resolve_secret(secret_id).expose().is_some()
}
