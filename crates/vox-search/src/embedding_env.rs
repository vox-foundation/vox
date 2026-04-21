//! Resolve a default embedding [`vox_runtime::llm::LlmConfig`] from secrets / env (MCP parity).

use vox_runtime::llm::LlmConfig;

/// Build embedding configuration from well-known providers.
#[must_use]
pub fn embedding_config_from_env() -> Option<LlmConfig> {
    if let Some(token) = vox_config::inference::huggingface_hub_token() {
        return Some(LlmConfig {
            provider: "hf_router".to_string(),
            model: std::env::var("VOX_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "sentence-transformers/all-MiniLM-L6-v2".to_string()),
            cost_per_1k: Some(0.0),
            base_url: Some("https://router.huggingface.co/v1/embeddings".to_string()),
            api_key: Some(token),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: None,
            telemetry_strength_tag: None,
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: false,
        });
    }
    let openai_key = vox_clavis::resolve_secret(vox_clavis::SecretId::OpenaiApiKey)
        .expose()
        .unwrap_or_default()
        .to_string();
    if !openai_key.trim().is_empty() {
        return Some(LlmConfig {
            provider: "openai".to_string(),
            model: std::env::var("VOX_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
            cost_per_1k: Some(0.00002),
            base_url: Some(vox_config::OPENAI_EMBEDDINGS_URL.to_string()),
            api_key: Some(openai_key),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: None,
            telemetry_strength_tag: None,
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: false,
        });
    }
    let openrouter_key = vox_clavis::resolve_secret(vox_clavis::SecretId::OpenRouterApiKey)
        .expose()
        .unwrap_or_default()
        .to_string();
    if !openrouter_key.trim().is_empty() {
        return Some(LlmConfig {
            provider: "openrouter".to_string(),
            model: std::env::var("VOX_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-3-small".to_string()),
            cost_per_1k: Some(0.00002),
            base_url: Some(vox_config::OPENROUTER_EMBEDDINGS_URL.to_string()),
            api_key: Some(openrouter_key),
            temperature: None,
            max_tokens: None,
            response_format: None,
            timeout_ms: None,
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: None,
            telemetry_strength_tag: None,
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: false,
        });
    }
    None
}
