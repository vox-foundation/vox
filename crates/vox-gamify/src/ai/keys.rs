//! Gemini / OpenRouter API keys: explicit provider config → Clavis → empty.

#[must_use]
pub(crate) fn resolve_gemini_key(explicit: &str) -> String {
    if !explicit.trim().is_empty() {
        return explicit.to_string();
    }
    vox_secrets::resolve_secret(vox_secrets::SecretId::GeminiApiKey)
        .expose()
        .map(std::string::ToString::to_string)
        .unwrap_or_default()
}

#[must_use]
pub(crate) fn resolve_openrouter_key(explicit: &str) -> String {
    if !explicit.trim().is_empty() {
        return explicit.to_string();
    }
    vox_secrets::resolve_secret(vox_secrets::SecretId::OpenRouterApiKey)
        .expose()
        .map(std::string::ToString::to_string)
        .unwrap_or_default()
}
