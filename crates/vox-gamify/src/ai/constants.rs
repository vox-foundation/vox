// AI client HTTP/provider constants.

// ─── Constants ───────────────────────────────────────────

pub(crate) const POLLINATIONS_BASE: &str = "https://text.pollinations.ai/";
/// Ollama base URL, resolved through the config SSOT at call time. Was a
/// `const` aliasing `vox_config::LOCAL_OLLAMA_POPULI_BASE_URL_DEFAULT`, which
/// by construction could not observe `POPULI_URL` / `OLLAMA_URL`.
pub(crate) fn ollama_default_url() -> String {
    vox_config::inference::local_ollama_populi_base_url()
}
pub(crate) const OLLAMA_DEFAULT_MODEL: &str = "codellama";
pub(crate) const GEMINI_DEFAULT_MODEL: &str = "gemini-2.5-flash";
pub(crate) const GEMINI_ENDPOINT_TEMPLATE: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={KEY}";
pub(crate) const HTTP_TIMEOUT_SECS: u64 = 15;
pub(crate) const OLLAMA_PROBE_TIMEOUT_SECS: u64 = 2;

/// Free-tier OpenRouter models tried in order (most capable first), all `:free`.
///
/// SSOT: aliased to `vox_config::OPENROUTER_FREE_FALLBACK_MODELS` so the gamify
/// free tier and the research free-floor cannot drift apart. Edit the list in
/// `crates/vox-config/src/bootstrap_inference.rs`, not here.
pub(crate) const OPENROUTER_FREE_MODELS: &[&str] = vox_config::OPENROUTER_FREE_FALLBACK_MODELS;

#[cfg(test)]
mod tests {
    #[test]
    fn free_models_are_the_vox_config_ssot() {
        // The gamify list MUST be the single vox-config SSOT, not a private copy,
        // so the research free-floor and gamify free tier can never drift.
        assert_eq!(
            super::OPENROUTER_FREE_MODELS,
            vox_config::OPENROUTER_FREE_FALLBACK_MODELS
        );
    }
}
