use serde::{Deserialize, Serialize};

use crate::ai::constants::*;


/// Which AI backend to attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum FreeAiProvider {
    /// Local Ollama — zero auth, recommended default.
    Ollama {
        /// URL of the local Ollama server (default: `http://localhost:11434`).
        #[serde(default = "default_ollama_url")]
        url: String,
        /// Model name to use (default: `codellama`).
        #[serde(default = "default_ollama_model")]
        model: String,
    },
    /// Pollinations.ai — zero API key, zero signup.
    Pollinations,
    /// Google Gemini Flash free tier — needs `GEMINI_API_KEY` env var.
    Gemini {
        /// Google API key, or empty to read from the `GEMINI_API_KEY` env var.
        #[serde(default)]
        api_key: String,
        /// Gemini model name (default: `gemini-2.5-flash`).
        #[serde(default = "default_gemini_model")]
        model: String,
    },
    /// Deterministic fallback — always succeeds, no AI.
    Deterministic,
    /// OpenRouter free-tier — tries models in `OPENROUTER_FREE_MODELS` order.
    /// Requires `OPENROUTER_API_KEY` env var (free account, no billing).
    OpenRouter {
        /// OpenRouter API key read from the `OPENROUTER_API_KEY` env var.
        api_key: String,
        /// Override model list; if empty, uses `OPENROUTER_FREE_MODELS`.
        #[serde(default)]
        models: Vec<String>,
    },
}

fn default_ollama_url() -> String {
    OLLAMA_DEFAULT_URL.to_string()
}
fn default_ollama_model() -> String {
    OLLAMA_DEFAULT_MODEL.to_string()
}
fn default_gemini_model() -> String {
    GEMINI_DEFAULT_MODEL.to_string()
}

impl FreeAiProvider {
    /// Human-readable name for display.
    pub fn name(&self) -> &str {
        match self {
            Self::Ollama { .. } => "Ollama (local)",
            Self::Pollinations => "Pollinations.ai (free)",
            Self::Gemini { .. } => "Gemini Flash (free tier)",
            Self::Deterministic => "Deterministic (offline)",
            Self::OpenRouter { .. } => "OpenRouter (free tier)",
        }
    }

    /// Return a `(provider, model)` pair for cost/telemetry tracking.
    pub fn provider_and_model(&self) -> (String, String) {
        match self {
            Self::Ollama { model, .. } => ("ollama".to_string(), model.clone()),
            Self::Pollinations => ("pollinations".to_string(), "openai-large".to_string()),
            Self::Gemini { model, .. } => ("google".to_string(), model.clone()),
            Self::Deterministic => ("deterministic".to_string(), "none".to_string()),
            Self::OpenRouter { models, .. } => (
                "openrouter".to_string(),
                models
                    .first()
                    .cloned()
                    .unwrap_or_else(|| OPENROUTER_FREE_MODELS[0].to_string()),
            ),
        }
    }
}
