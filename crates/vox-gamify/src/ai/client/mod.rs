//! HTTP client and provider cascade.

use std::sync::Arc;

use crate::ai::provider::FreeAiProvider;

mod ctor;
mod transport;

/// Callback for reporting provider-specific events like rate limits.
pub type AiReportFn = Arc<dyn Fn(&str, Option<u64>) + Send + Sync>;

/// Callback for reporting reconciled costs (e.g. from OpenRouter x-response-cost).
pub type CostReportFn = Arc<dyn Fn(f64) + Send + Sync>;

/// Backend selection for a single-model streaming request (registry or explicit routing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LudusStreamBackend {
    Ollama,
    Gemini,
    OpenRouter,
    /// Local MENS HTTP (`VOX_LOCAL_ENDPOINT`, default `:11434`) — `mens/<run>` catalog ids.
    VoxLocal,
}

/// How [`FreeAiClient::generate_stream_routed`] should reach the LLM.
#[derive(Debug, Clone, Copy)]
pub enum StreamRoute<'a> {
    /// Use the configured provider cascade (same as [`FreeAiClient::generate_stream`]).
    Cascade,
    /// Call one backend with a specific model id (e.g. from an orchestrator routing table).
    Registry {
        backend: LudusStreamBackend,
        model: &'a str,
    },
    /// Honor a user-provided model slug.
    ///
    /// `mens/<run>` slugs go to VoxLocal (`VOX_LOCAL_ENDPOINT/generate`) only.
    /// Other slugs try Ollama, then OpenRouter, then Gemini, then cascade.
    UserModelOverride(&'a str),
}

/// True when a chat-picker / catalog id is a local MENS run (`mens/<run>`).
pub(crate) fn is_mens_local_model(model: &str) -> bool {
    model.trim().starts_with("mens/")
}

#[cfg(test)]
mod mens_override_tests {
    use super::is_mens_local_model;

    #[test]
    fn mens_run_slug_is_local() {
        assert!(is_mens_local_model("mens/e2e-smoke-metal"));
        assert!(is_mens_local_model("  mens/e2e-smoke  "));
    }

    #[test]
    fn ollama_and_cloud_slugs_are_not_mens_local() {
        assert!(!is_mens_local_model("llama3.2"));
        assert!(!is_mens_local_model("qwen/qwen3-8b"));
        assert!(!is_mens_local_model(""));
    }
}

/// AI client that tries providers in order until one succeeds.
#[derive(Clone)]
pub struct FreeAiClient {
    /// Ordered list of providers to try.
    pub(crate) providers: Vec<FreeAiProvider>,
    /// Shared HTTP client for all provider calls.
    pub(crate) http: reqwest::Client,
    /// Optional callback for rate limit and provider events.
    pub(crate) reporter: Option<AiReportFn>,
    /// Optional callback for cost reporting.
    pub(crate) cost_reporter: Option<CostReportFn>,
}
