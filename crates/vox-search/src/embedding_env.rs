//! Resolve a default embedding [`vox_actor_runtime::llm::LlmConfig`] from secrets / env (MCP parity).

use vox_actor_runtime::llm::LlmConfig;

fn embedding_model_or(default: &str) -> String {
    let resolved = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxEmbeddingModel);
    let raw = resolved.expose().unwrap_or_default();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        default.to_string()
    } else {
        trimmed.to_string()
    }
}

/// Build embedding configuration from well-known providers.
#[must_use]
pub fn embedding_config_from_env() -> Option<LlmConfig> {
    if let Some(token) = vox_config::inference::huggingface_hub_token() {
        return Some(LlmConfig {
            provider: "hf_router".to_string(),
            model: embedding_model_or("sentence-transformers/all-MiniLM-L6-v2"),
            cost_per_1k: Some(0.0),
            base_url: Some("https://router.huggingface.co/v1/embeddings".to_string()),
            api_key: Some(token),
            temperature: None,
            top_p: None,
            max_tokens: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            timeout_ms: None,
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: None,
            telemetry_strength_tag: None,
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: false,
            seed: None,
        });
    }
    let openai_key = vox_secrets::resolve_secret(vox_secrets::SecretId::OpenaiApiKey)
        .expose()
        .unwrap_or_default()
        .to_string();
    if !openai_key.trim().is_empty() {
        return Some(LlmConfig {
            provider: "openai".to_string(),
            model: embedding_model_or("text-embedding-3-small"),
            cost_per_1k: Some(0.00002),
            base_url: Some(vox_config::openai_embeddings_url()),
            api_key: Some(openai_key),
            temperature: None,
            top_p: None,
            max_tokens: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            timeout_ms: None,
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: None,
            telemetry_strength_tag: None,
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: false,
            seed: None,
        });
    }
    let openrouter_key = vox_secrets::resolve_secret(vox_secrets::SecretId::OpenRouterApiKey)
        .expose()
        .unwrap_or_default()
        .to_string();
    if !openrouter_key.trim().is_empty() {
        return Some(LlmConfig {
            provider: "openrouter".to_string(),
            model: embedding_model_or("text-embedding-3-small"),
            cost_per_1k: Some(0.00002),
            base_url: Some(vox_config::openrouter_embeddings_url()),
            api_key: Some(openrouter_key),
            temperature: None,
            top_p: None,
            max_tokens: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            timeout_ms: None,
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: None,
            telemetry_strength_tag: None,
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: false,
            seed: None,
        });
    }
    None
}

/// Default revision-pinned local embedding model identifier (`EMB-01`).
pub const DEFAULT_LOCAL_EMBED_MODEL: &str = "Alibaba-NLP/gte-modernbert-base";
/// Default pinned revision for the local embedder (`EMB-01`).
pub const DEFAULT_LOCAL_EMBED_REVISION: &str = "refs/pr/1";
/// Default vector dimension for the local embedder (`EMB-01`).
pub const DEFAULT_LOCAL_EMBED_DIM: usize = 256;

/// Build a zero-API-key `provider = "local"` embedding configuration (`EMB-01`).
#[must_use]
pub fn default_local_embedding_config() -> LlmConfig {
    let default_spec = format!("{DEFAULT_LOCAL_EMBED_MODEL}@{DEFAULT_LOCAL_EMBED_REVISION}");
    LlmConfig {
        provider: "local".to_string(),
        model: embedding_model_or(&default_spec),
        cost_per_1k: Some(0.0),
        base_url: None,
        api_key: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        response_format: None,
        tools: None,
        tool_choice: None,
        timeout_ms: None,
        telemetry_session_id: None,
        telemetry_user_id: None,
        telemetry_task_category: None,
        telemetry_strength_tag: None,
        telemetry_trace_id: None,
        telemetry_attempt_number: None,
        telemetry_skip_interaction: true,
        seed: None,
    }
}

/// Prefer explicit environment/Clavis API provider when configured, falling back to the
/// zero-key `provider = "local"` configuration (`EMB-01`, `EMB-05`).
#[must_use]
pub fn embedding_config_or_local() -> LlmConfig {
    embedding_config_from_env().unwrap_or_else(default_local_embedding_config)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Embedding requests never ask for a sampling seed, whichever provider is configured.
    #[test]
    fn embedding_config_never_carries_a_seed() {
        if let Some(cfg) = embedding_config_from_env() {
            assert!(cfg.seed.is_none());
        }
        let local = default_local_embedding_config();
        assert_eq!(local.provider, "local");
        assert!(local.seed.is_none());
    }
}
