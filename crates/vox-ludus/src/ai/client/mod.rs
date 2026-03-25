//! HTTP client and provider cascade.

use std::sync::Arc;

use crate::ai::provider::FreeAiProvider;

mod ctor;
mod transport;

/// Callback for reporting provider-specific events like rate limits.
pub type AiReportFn = Arc<dyn Fn(&str, Option<u64>) + Send + Sync>;

/// AI client that tries providers in order until one succeeds.
pub struct FreeAiClient {
    /// Ordered list of providers to try.
    pub(crate) providers: Vec<FreeAiProvider>,
    /// Shared HTTP client for all provider calls.
    pub(crate) http: reqwest::Client,
    /// Optional callback for rate limit and provider events.
    pub(crate) reporter: Option<AiReportFn>,
}
