//! Serve command configuration.

use std::path::PathBuf;

use crate::commands::ai::inference_defaults;

/// Default host for inference server bind address.
pub const DEFAULT_SERVE_HOST: &str = inference_defaults::DEFAULT_INFERENCE_HOST;
/// Default port for inference server.
pub const DEFAULT_SERVE_PORT: u16 = inference_defaults::DEFAULT_INFERENCE_PORT;
/// Default max tokens per generation.
pub const DEFAULT_SERVE_MAX_TOKENS: usize = inference_defaults::DEFAULT_INFERENCE_MAX_TOKENS;
/// Default sampling temperature (0.0 = greedy).
pub const DEFAULT_SERVE_TEMPERATURE: f32 = inference_defaults::DEFAULT_INFERENCE_TEMPERATURE;

/// Serve command configuration.
///
/// Without `execution-api`, CLI paths that start the HTTP server are not compiled in, but we
/// still build this type for tests and for a future `execution-api` build.
#[cfg_attr(not(feature = "execution-api"), allow(dead_code))]
#[derive(Debug, Clone)]
pub struct ServeConfig {
    /// Path to the model checkpoint to load.
    pub model_path: PathBuf,
    /// Port to bind the HTTP server to.
    pub port: u16,
    /// Maximum tokens to generate per request.
    pub max_tokens: usize,
    /// Temperature for sampling (0.0 = greedy, 1.0 = full random).
    pub temperature: f32,
    /// Host to bind (default: 127.0.0.1)
    pub host: String,
    /// System prompt for ChatML format (must match training). When None, uses training default.
    pub system_prompt: Option<String>,
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            model_path: PathBuf::from("model_final.bin"),
            port: DEFAULT_SERVE_PORT,
            max_tokens: DEFAULT_SERVE_MAX_TOKENS,
            temperature: DEFAULT_SERVE_TEMPERATURE,
            host: DEFAULT_SERVE_HOST.to_string(),
            system_prompt: None,
        }
    }
}
