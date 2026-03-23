//! Environment resolution for **inference providers** (local Populi/Ollama and cloud keys).
//!
//! This module is the **SSOT** for reading env vars used across CLI, MCP, and runtime. Callers that
//! need HTTP probes (health, model lists) use `vox_runtime::inference_env::probe_populi_capabilities`.

/// Where chat / completion traffic is expected to run (desktop daemon vs cloud vs on-device).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InferenceProfile {
    /// Default: local Ollama-compatible HTTP (`OLLAMA_HOST` / `POPULI_URL` / localhost).
    #[default]
    DesktopOllama,
    /// OpenRouter / HF / other OpenAI-compatible cloud endpoints from config.
    CloudOpenAiCompatible,
    /// On-device LiteRT-LM (app-owned runtime).
    MobileLitert,
    /// Apple Core ML (app-owned).
    MobileCoreml,
    /// Ollama or compatible gateway on LAN (explicit base URL).
    LanGateway,
}

impl InferenceProfile {
    /// Whether tooling may probe and call **local** Ollama-compatible HTTP (loopback or `OLLAMA_HOST`).
    #[must_use]
    pub const fn allows_local_ollama_http(self) -> bool {
        matches!(self, Self::DesktopOllama | Self::LanGateway)
    }
}

/// Read [`InferenceProfile`] from **`VOX_INFERENCE_PROFILE`** (case-insensitive).
#[must_use]
pub fn inference_profile_from_env() -> InferenceProfile {
    let raw = std::env::var("VOX_INFERENCE_PROFILE")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase());
    match raw.as_deref() {
        Some("cloud_openai_compatible") | Some("cloud") => InferenceProfile::CloudOpenAiCompatible,
        Some("mobile_litert") | Some("litert") => InferenceProfile::MobileLitert,
        Some("mobile_coreml") | Some("coreml") => InferenceProfile::MobileCoreml,
        Some("lan_gateway") | Some("lan") => InferenceProfile::LanGateway,
        Some("desktop_ollama") | Some("ollama") | None => InferenceProfile::DesktopOllama,
        _ => InferenceProfile::DesktopOllama,
    }
}

/// Whether MCP / other HTTP clients may use **local** Ollama (`VOX_INFERENCE_PROFILE`).
#[must_use]
pub fn inference_profile_allows_local_ollama_http() -> bool {
    inference_profile_from_env().allows_local_ollama_http()
}

/// OpenRouter chat completions endpoint (OpenAI-compatible).
pub const OPENROUTER_CHAT_COMPLETIONS_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Local Ollama-compatible API base URL.
///
/// Precedence: **`OLLAMA_URL`** → **`POPULI_URL`** → `http://localhost:11434`.
pub fn local_ollama_populi_base_url() -> String {
    std::env::var("OLLAMA_URL")
        .or_else(|_| std::env::var("POPULI_URL"))
        .unwrap_or_else(|_| "http://localhost:11434".to_string())
}

/// Hugging Face Hub / Inference token for router and Hub APIs.
///
/// Precedence: **`HF_TOKEN`** → **`HUGGING_FACE_HUB_TOKEN`**.
pub fn huggingface_hub_token() -> Option<String> {
    std::env::var("HF_TOKEN")
        .or_else(|_| std::env::var("HUGGING_FACE_HUB_TOKEN"))
        .ok()
}

/// OpenRouter API key (`OPENROUTER_API_KEY`).
pub fn openrouter_api_key() -> Option<String> {
    std::env::var("OPENROUTER_API_KEY").ok()
}

/// Preferred Hugging Face **router** model id for chat when policy selects HF (`HF_CHAT_MODEL`).
pub fn hf_chat_model_preference() -> Option<String> {
    std::env::var("HF_CHAT_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

/// Preferred OpenRouter model id when policy selects OpenRouter (`OPENROUTER_CHAT_MODEL`).
///
/// Falls back to [`crate::bootstrap_inference::OPENROUTER_AUTO`] when unset.
pub fn openrouter_chat_model_preference() -> String {
    std::env::var("OPENROUTER_CHAT_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| crate::bootstrap_inference::OPENROUTER_AUTO.to_string())
}

/// OpenAI-compatible chat completions URL for a **pinned** Hugging Face Inference Endpoint
/// (`HF_DEDICATED_CHAT_URL`), when policy should prefer dedicated over the shared router.
pub fn hf_dedicated_chat_completions_url() -> Option<String> {
    std::env::var("HF_DEDICATED_CHAT_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

/// Model id sent in the JSON body for [`hf_dedicated_chat_completions_url`] (`HF_DEDICATED_CHAT_MODEL`).
pub fn hf_dedicated_chat_model() -> Option<String> {
    std::env::var("HF_DEDICATED_CHAT_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
}

/// Sanitize a string for ChatML formatting by replacing control tokens that could
/// trigger prompt injection (e.g., `<|im_start|>`, `<|im_end|>`).
#[must_use]
pub fn sanitize_chatml(input: &str) -> String {
    input
        .replace("<|im_start|>", "[im_start]")
        .replace("<|im_end|>", "[im_end]")
}
