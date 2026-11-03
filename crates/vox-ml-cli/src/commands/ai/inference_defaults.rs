//! Shared defaults for Mens native HTTP inference (`vox mens serve`) and `super::serve::ServeConfig`.
//!
//! Keep in sync with CLI `#[arg(default_…)]` on serve-related subcommands.

/// Bind address for the inference HTTP server (loopback by default).
pub const DEFAULT_INFERENCE_HOST: &str = "127.0.0.1";
/// TCP port for the inference HTTP server. Matches `vox-config`'s SSOT
/// Ollama-conflict-safe alternate (`vox_config::inference::VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT`,
/// port 11435) rather than Ollama's own well-known port (11434,
/// `vox_config::inference::VOX_LOCAL_ENDPOINT_DEFAULT`), so `vox mens serve`
/// does not collide with a running Ollama instance by default.
pub const DEFAULT_INFERENCE_PORT: u16 = 11435;
/// Max new tokens per `/v1/completions`-style request unless overridden.
pub const DEFAULT_INFERENCE_MAX_TOKENS: usize = 256;
/// Sampling temperature (0.0 = greedy).
pub const DEFAULT_INFERENCE_TEMPERATURE: f32 = 0.7;

#[cfg(test)]
mod tests {
    use super::*;

    /// The serve default must equal the config SSOT's non-Ollama-conflicting
    /// alternate port, not Ollama's own well-known port (11434) — the whole
    /// point of the fix is that `vox mens serve` no longer collides with a
    /// running Ollama instance by default.
    #[test]
    fn default_inference_port_matches_config_ssot_ollama_conflict_alt() {
        assert!(
            vox_config::inference::VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT
                .ends_with(&format!(":{DEFAULT_INFERENCE_PORT}")),
            "DEFAULT_INFERENCE_PORT ({DEFAULT_INFERENCE_PORT}) must match the port embedded in \
             vox_config::inference::VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT \
             ({})",
            vox_config::inference::VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT
        );
        assert_ne!(
            DEFAULT_INFERENCE_PORT, 11434,
            "11434 is Ollama's well-known port; vox mens serve must not default to colliding with it"
        );
    }
}
