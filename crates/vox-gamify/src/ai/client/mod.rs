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
    /// Honor a user-provided model slug: try Ollama, then OpenRouter, then Gemini, then cascade.
    UserModelOverride(&'a str),
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
