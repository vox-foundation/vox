#[derive(Debug, Clone, Default)]
pub(crate) struct HttpCallMetadata {
    pub provider_request_id: Option<String>,
    pub provider_reported_cost_usd: Option<f64>,
}

/// Base URL for Ollama (`OLLAMA_HOST` or Mens local default).
pub(crate) fn ollama_base_url() -> String {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOllamaHost)
        .expose()
        .map(|s| s.to_string())
        .unwrap_or_else(|| vox_config::inference::local_ollama_populi_base_url())
}
