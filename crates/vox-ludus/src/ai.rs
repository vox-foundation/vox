//! Free AI client with multi-provider fallback.
//!
//! Supports a cascade of providers so Vox is fully redistributable:
//! 1. **Ollama** (local) — zero auth, best quality, no network
//! 2. **Pollinations.ai** — zero API key, zero signup, HTTP GET
//! 3. **Gemini Flash** — free tier, requires env var `GEMINI_API_KEY`
//! 4. **Deterministic** — always works, no AI, pattern-based responses

use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;

// ─── Constants ───────────────────────────────────────────

const POLLINATIONS_BASE: &str = "https://text.pollinations.ai/";
const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";
const OLLAMA_DEFAULT_MODEL: &str = "codellama";
const GEMINI_DEFAULT_MODEL: &str = "gemini-2.5-flash";
const GEMINI_ENDPOINT_TEMPLATE: &str =
    "https://generativelanguage.googleapis.com/v1beta/models/{MODEL}:generateContent?key={KEY}";
const HTTP_TIMEOUT_SECS: u64 = 15;
const OLLAMA_PROBE_TIMEOUT_SECS: u64 = 2;
const OPENROUTER_BASE: &str = "https://openrouter.ai/api/v1/chat/completions";

/// Free-tier OpenRouter models tried in order (most capable first).
/// All end with `:free` to guarantee zero cost.
const OPENROUTER_FREE_MODELS: &[&str] = &[
    "google/gemma-3-27b-it:free",
    "meta-llama/llama-3.3-70b-instruct:free",
    "qwen/qwen3-235b-a22b:free",
    "mistralai/mistral-7b-instruct:free",
    "microsoft/phi-3-mini-128k-instruct:free",
];

// ─── Errors ──────────────────────────────────────────────

/// Errors produced by the AI client.
#[derive(Debug, Error)]
pub enum AiError {
    /// All configured providers were exhausted without producing a response.
    #[error("All AI providers failed: {0}")]
    AllProvidersFailed(String),
    /// A network-level HTTP error occurred.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    /// The provider returned malformed JSON.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    /// The provider returned a 200 response with no content.
    #[error("Empty response from provider")]
    EmptyResponse,
    /// The provider returned a 429 Too Many Requests response.
    #[error("Rate limited by provider: {provider} (retry after {retry_after_secs:?}s)")]
    RateLimited {
        /// Name of the provider that returned the rate limit.
        provider: String,
        /// Seconds until the rate limit expires, if provided by the server.
        retry_after_secs: Option<u64>,
    },
}

// ─── Provider Enum ───────────────────────────────────────

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

// ─── Client ──────────────────────────────────────────────

/// Callback for reporting provider-specific events like rate limits.
pub type AiReportFn = Arc<dyn Fn(&str, Option<u64>) + Send + Sync>;

/// AI client that tries providers in order until one succeeds.
pub struct FreeAiClient {
    /// Ordered list of providers to try.
    providers: Vec<FreeAiProvider>,
    /// Shared HTTP client for all provider calls.
    http: reqwest::Client,
    /// Optional callback for rate limit and provider events.
    reporter: Option<AiReportFn>,
}

impl FreeAiClient {
    /// Create a client with an explicit provider list.
    pub fn new(providers: Vec<FreeAiProvider>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        Self {
            providers,
            http,
            reporter: None,
        }
    }

    /// Set a reporter to receive provider events.
    pub fn with_reporter(mut self, reporter: AiReportFn) -> Self {
        self.reporter = Some(reporter);
        self
    }

    /// Auto-discover available providers and build a fallback chain.
    ///
    /// Order: Ollama (if reachable) → Pollinations → Gemini (if key set) → Deterministic.
    pub async fn auto_discover() -> Self {
        let mut providers = Vec::new();

        // 1. Probe Ollama
        if Self::probe_ollama(OLLAMA_DEFAULT_URL).await {
            providers.push(FreeAiProvider::Ollama {
                url: OLLAMA_DEFAULT_URL.to_string(),
                model: OLLAMA_DEFAULT_MODEL.to_string(),
            });
        }

        // 2. Pollinations is always available (network permitting)
        providers.push(FreeAiProvider::Pollinations);

        // 3. Gemini if API key is set
        if let Ok(key) = std::env::var("GEMINI_API_KEY")
            && !key.is_empty()
        {
            providers.push(FreeAiProvider::Gemini {
                api_key: key,
                model: GEMINI_DEFAULT_MODEL.to_string(),
            });
        }

        // 4. OpenRouter free tier if key is set
        if let Ok(key) = std::env::var("OPENROUTER_API_KEY")
            && !key.is_empty()
        {
            providers.push(FreeAiProvider::OpenRouter {
                api_key: key,
                models: Vec::new(), // use default free model list
            });
        }

        // 5. Deterministic always last — never fails
        providers.push(FreeAiProvider::Deterministic);

        Self::new(providers)
    }

    /// Return the `(provider, model)` for the highest-priority active provider.
    ///
    /// Used for cost/telemetry tagging. Falls back to `("deterministic", "none")`.
    pub fn active_provider_info(&self) -> (String, String) {
        self.providers
            .first()
            .map(|p| p.provider_and_model())
            .unwrap_or_else(|| ("deterministic".to_string(), "none".to_string()))
    }

    /// Check if Ollama is reachable at the given URL.
    async fn probe_ollama(url: &str) -> bool {
        let probe_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(OLLAMA_PROBE_TIMEOUT_SECS))
            .build()
            .unwrap_or_default();
        probe_client
            .get(format!("{}/api/version", url))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }

    /// Generate text using the fallback chain.
    ///
    /// Tries each provider in order. Returns the first successful response.
    /// If all providers fail, returns the deterministic fallback.
    pub async fn generate(&self, prompt: &str) -> Result<String, AiError> {
        let mut _last_error = String::new();

        for provider in &self.providers {
            match self.call_provider(provider, prompt).await {
                Ok(text) if !text.trim().is_empty() => return Ok(text),
                Err(AiError::RateLimited {
                    provider,
                    retry_after_secs,
                }) => {
                    if let Some(ref r) = self.reporter {
                        r(&provider, retry_after_secs);
                    }
                    _last_error = format!("{}: rate limited", provider);
                    continue;
                }
                Ok(_) => {
                    _last_error = format!("{}: empty response", provider.name());
                }
                Err(e) => {
                    _last_error = format!("{}: {}", provider.name(), e);
                }
            }
        }

        // Should not reach here if Deterministic is in the chain, but just in case:
        Ok(deterministic_response(prompt))
    }

    /// Generate a stream of tokens.
    ///
    /// Cascades through providers. If a provider doesn't support streaming,
    /// it will be called as a single block and yielded as a single chunk.
    pub async fn generate_stream(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<String, AiError>> + Send>> {
        let providers = self.providers.clone();
        let http = self.http.clone();
        let prompt = prompt.to_string();
        let reporter = self.reporter.clone();

        Box::pin(async_stream::try_stream! {
            let mut _last_error = String::new();

            for provider in providers {
                match provider {
                    FreeAiProvider::Ollama { ref url, ref model } => {
                        let mut stream = Self::stream_ollama(&http, url, model, &prompt).await;
                        let mut saw_rate_limit = false;
                        while let Some(chunk) = stream.next().await {
                            match chunk {
                                Ok(t) => yield t,
                                Err(AiError::RateLimited { provider, retry_after_secs }) => {
                                    if let Some(ref r) = reporter {
                                        r(&provider, retry_after_secs);
                                    }
                                    saw_rate_limit = true;
                                    break;
                                }
                                Err(e) => Err(e)?,
                            }
                        }
                        if !saw_rate_limit {
                            // Streaming succeeded — stop trying remaining providers
                            return;
                        }
                    }
                    FreeAiProvider::Gemini { ref api_key, ref model } => {
                        let mut stream = Self::stream_gemini(&http, api_key, model, &prompt).await;
                        let mut saw_rate_limit = false;
                        while let Some(chunk) = stream.next().await {
                            match chunk {
                                Ok(t) => yield t,
                                Err(AiError::RateLimited { provider, retry_after_secs }) => {
                                    if let Some(ref r) = reporter {
                                        r(&provider, retry_after_secs);
                                    }
                                    saw_rate_limit = true;
                                    break;
                                }
                                Err(e) => Err(e)?,
                            }
                        }
                        if !saw_rate_limit {
                            // Streaming succeeded — stop trying remaining providers
                            return;
                        }
                    }
                    _ => {
                        // Fallback to non-streaming for others
                        match Self::call_provider_static(&http, &provider, &prompt).await {
                            Ok(text) if !text.trim().is_empty() => {
                                yield text;
                                return;
                            }
                            Err(AiError::RateLimited { provider, retry_after_secs }) => {
                                if let Some(ref r) = reporter {
                                    r(&provider, retry_after_secs);
                                }
                                continue;
                            }
                            _ => continue,
                        }
                    }
                }
            }
            yield deterministic_response(&prompt);
        })
    }

    /// POST to Ollama `/api/generate` with stream=true.
    async fn stream_ollama(
        http: &reqwest::Client,
        url: &str,
        model: &str,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<String, AiError>> + Send>> {
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": true,
        });

        let http = http.clone();
        let url = format!("{}/api/generate", url);

        Box::pin(async_stream::try_stream! {
            let resp = http.post(&url).json(&body).send().await.map_err(AiError::Http)?;

            if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after = resp.headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok());
                Err(AiError::RateLimited {
                    provider: "ollama".to_string(),
                    retry_after_secs: retry_after
                })?;
            }

            let mut stream = resp.bytes_stream();

            while let Some(item) = stream.next().await {
                let chunk: Bytes = item.map_err(AiError::Http)?;
                let json: serde_json::Value = serde_json::from_slice(&chunk).map_err(AiError::Json)?;
                if let Some(token) = json["response"].as_str() {
                    yield token.to_string();
                }
                if json["done"].as_bool().unwrap_or(false) {
                    break;
                }
            }
        })
    }

    /// POST to Gemini `streamGenerateContent`.
    async fn stream_gemini(
        http: &reqwest::Client,
        api_key: &str,
        model: &str,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<String, AiError>> + Send>> {
        let resolved_key = if api_key.is_empty() {
            std::env::var("GEMINI_API_KEY").unwrap_or_default()
        } else {
            api_key.to_string()
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}",
            model, resolved_key
        );

        let body = serde_json::json!({
            "contents": [{ "parts": [{ "text": prompt }] }]
        });

        let http = http.clone();

        Box::pin(async_stream::try_stream! {
            let resp = http.post(&url).json(&body).send().await.map_err(AiError::Http)?;

            if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after = resp.headers()
                    .get(reqwest::header::RETRY_AFTER)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse().ok());
                Err(AiError::RateLimited {
                    provider: "google".to_string(),
                    retry_after_secs: retry_after
                })?;
            }

            let mut stream = resp.bytes_stream();

            while let Some(item) = stream.next().await {
                let chunk: Bytes = item.map_err(AiError::Http)?;
                // Gemini stream is an array of objects
                let json: serde_json::Value = serde_json::from_slice(&chunk).map_err(AiError::Json)?;
                if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                    yield text.to_string();
                }
            }
        })
    }

    async fn call_provider_static(
        http: &reqwest::Client,
        provider: &FreeAiProvider,
        prompt: &str,
    ) -> Result<String, AiError> {
        match provider {
            FreeAiProvider::Ollama { url, model } => {
                Self::call_ollama_static(http, url, model, prompt).await
            }
            FreeAiProvider::Pollinations => Self::call_pollinations_static(http, prompt).await,
            FreeAiProvider::Gemini { api_key, model } => {
                Self::call_gemini_static(http, api_key, model, prompt).await
            }
            FreeAiProvider::OpenRouter { api_key, models } => {
                Self::call_openrouter_static(http, api_key, models, prompt).await
            }
            FreeAiProvider::Deterministic => Ok(deterministic_response(prompt)),
        }
    }

    /// Call OpenRouter with model-level fallback through the free-tier list.
    ///
    /// Tries each model until one returns a non-empty response.
    /// On rate limit (429) or quota errors, advances to the next model.
    async fn call_openrouter_static(
        http: &reqwest::Client,
        api_key: &str,
        models: &[String],
        prompt: &str,
    ) -> Result<String, AiError> {
        let resolved_key = if api_key.is_empty() {
            std::env::var("OPENROUTER_API_KEY").unwrap_or_default()
        } else {
            api_key.to_string()
        };
        if resolved_key.is_empty() {
            return Err(AiError::AllProvidersFailed(
                "OPENROUTER_API_KEY not set".to_string(),
            ));
        }
        let model_list: Vec<&str> = if models.is_empty() {
            OPENROUTER_FREE_MODELS.to_vec()
        } else {
            models.iter().map(String::as_str).collect()
        };

        let mut last_err = String::new();
        let mut first_rate_limit: Option<(String, Option<u64>)> = None;

        for model in &model_list {
            let body = serde_json::json!({
                "model": model,
                "messages": [{ "role": "user", "content": prompt }],
                "max_tokens": 512,
            });
            match http
                .post(OPENROUTER_BASE)
                .header("Authorization", format!("Bearer {}", resolved_key))
                .header("HTTP-Referer", "https://github.com/vox-foundation/vox")
                .header("X-Title", "Vox Gamify")
                .json(&body)
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if status.as_u16() == 429 || status.as_u16() == 402 {
                        // Rate limited or quota exceeded — try next model
                        let retry_after = resp
                            .headers()
                            .get(reqwest::header::RETRY_AFTER)
                            .and_then(|h| h.to_str().ok())
                            .and_then(|s| s.parse().ok());

                        if first_rate_limit.is_none() {
                            first_rate_limit = Some((model.to_string(), retry_after));
                        }

                        last_err = format!("model '{}': HTTP {}", model, status);
                        tracing::debug!("OpenRouter {} for '{}', trying next", status, model);
                        continue;
                    }
                    match resp.json::<serde_json::Value>().await {
                        Ok(json) => {
                            if let Some(text) = json["choices"][0]["message"]["content"].as_str() {
                                let trimmed = text.trim().to_string();
                                if !trimmed.is_empty() {
                                    tracing::debug!("OpenRouter model '{}' succeeded", model);
                                    return Ok(trimmed);
                                }
                            }
                            last_err = format!("model '{}': empty content in response", model);
                        }
                        Err(e) => {
                            last_err = format!("model '{}': JSON parse: {}", model, e);
                        }
                    }
                }
                Err(e) => {
                    last_err = format!("model '{}': HTTP: {}", model, e);
                }
            }
            tracing::debug!(
                "OpenRouter model '{}' failed, trying next: {}",
                model,
                last_err
            );
        }

        if let Some((model, retry_after)) = first_rate_limit {
            return Err(AiError::RateLimited {
                provider: format!("openrouter:{}", model),
                retry_after_secs: retry_after,
            });
        }

        Err(AiError::AllProvidersFailed(format!(
            "OpenRouter exhausted all free models: {}",
            last_err
        )))
    }

    async fn call_ollama_static(
        http: &reqwest::Client,
        url: &str,
        model: &str,
        prompt: &str,
    ) -> Result<String, AiError> {
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        });
        let resp = http
            .post(format!("{}/api/generate", url))
            .json(&body)
            .send()
            .await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok());
            return Err(AiError::RateLimited {
                provider: "ollama".to_string(),
                retry_after_secs: retry_after,
            });
        }
        let json: serde_json::Value = resp.json().await?;
        json["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(AiError::EmptyResponse)
    }

    async fn call_pollinations_static(
        http: &reqwest::Client,
        prompt: &str,
    ) -> Result<String, AiError> {
        let encoded = urlencode(prompt);
        let url = format!("{}{}?model=openai&nologo=true", POLLINATIONS_BASE, encoded);
        let resp = http.get(&url).send().await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok());
            return Err(AiError::RateLimited {
                provider: "pollinations".to_string(),
                retry_after_secs: retry_after,
            });
        }

        let text = resp.text().await?;
        if text.trim().is_empty() {
            return Err(AiError::EmptyResponse);
        }
        Ok(text)
    }

    async fn call_gemini_static(
        http: &reqwest::Client,
        api_key: &str,
        model: &str,
        prompt: &str,
    ) -> Result<String, AiError> {
        let resolved_key = if api_key.is_empty() {
            std::env::var("GEMINI_API_KEY").unwrap_or_default()
        } else {
            api_key.to_string()
        };
        let url = GEMINI_ENDPOINT_TEMPLATE
            .replace("{MODEL}", model)
            .replace("{KEY}", &resolved_key);
        let body = serde_json::json!({ "contents": [{ "parts": [{ "text": prompt }] }] });
        let resp = http.post(&url).json(&body).send().await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok());
            return Err(AiError::RateLimited {
                provider: "google".to_string(),
                retry_after_secs: retry_after,
            });
        }

        let json: serde_json::Value = resp.json().await?;
        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(AiError::EmptyResponse)
    }

    /// Call a single provider.
    async fn call_provider(
        &self,
        provider: &FreeAiProvider,
        prompt: &str,
    ) -> Result<String, AiError> {
        Self::call_provider_static(&self.http, provider, prompt).await
    }

    /// Return the list of configured providers (for status display).
    pub fn providers(&self) -> &[FreeAiProvider] {
        &self.providers
    }
}

// ─── Deterministic Fallback ──────────────────────────────

/// Always-available fallback that returns pattern-based responses.
///
/// This is NOT AI — it's a simple keyword matcher that ensures
/// Vox never fails when AI providers are unavailable.
pub fn deterministic_response(prompt: &str) -> String {
    let lower = prompt.to_lowercase();

    if lower.contains("sprite") || lower.contains("ascii") {
        return FALLBACK_SPRITE.to_string();
    }
    if lower.contains("name") || lower.contains("creative") {
        return "Code Companion".to_string();
    }
    if lower.contains("analyze") || lower.contains("quality") || lower.contains("review") {
        return "CLEAN".to_string();
    }
    if lower.contains("suggest") || lower.contains("fix") {
        return "Consider reviewing this code for potential improvements.".to_string();
    }

    "I'm running in offline mode. AI features will be available when a provider is reachable."
        .to_string()
}

const FALLBACK_SPRITE: &str = r#"  /\_/\
 ( o.o )
  > ^ <
 /|   |\
(_|   |_)"#;

// ─── Output Validators ───────────────────────────────────

/// Validate AI-generated SVG output.
///
/// Returns `Ok(svg)` if the SVG is plausibly valid, or `Err` with a reason.
/// Used to gate AI-generated sprites before persisting them.
pub fn validate_svg(svg: &str) -> Result<&str, &'static str> {
    let trimmed = svg.trim();
    if trimmed.is_empty() {
        return Err("SVG is empty");
    }
    if !trimmed.contains("<svg") {
        return Err("SVG missing <svg> element");
    }
    if !trimmed.contains("</svg>") {
        return Err("SVG missing closing </svg>");
    }
    // Basic safety: no script tags
    if trimmed.to_ascii_lowercase().contains("<script") {
        return Err("SVG contains <script> — rejected for safety");
    }
    Ok(trimmed)
}

/// Validate AI-generated hint text.
///
/// Returns `Ok(hint)` if the hint is plausibly useful, or `Err` with a reason.
pub fn validate_hint(hint: &str) -> Result<String, &'static str> {
    let trimmed = hint.trim().to_string();
    if trimmed.is_empty() {
        return Err("hint is empty");
    }
    if trimmed.len() < 10 {
        return Err("hint is too short");
    }
    if trimmed.len() > 2000 {
        return Err("hint is too long");
    }
    // Reject obvious garbage
    if trimmed.chars().all(|c| !c.is_alphabetic()) {
        return Err("hint contains no alphabetic content");
    }
    Ok(trimmed)
}

// ─── URL Encoding ────────────────────────────────────────

/// Minimal URL encoding for the Pollinations GET endpoint.
fn urlencode(s: &str) -> String {
    s.chars().map(urlencode_char).collect()
}

fn urlencode_char(c: char) -> String {
    match c {
        ' ' => "%20".to_string(),
        '\n' => "%0A".to_string(),
        '\r' => String::new(),
        '"' => "%22".to_string(),
        '#' => "%23".to_string(),
        '%' => "%25".to_string(),
        '&' => "%26".to_string(),
        '+' => "%2B".to_string(),
        '?' => "%3F".to_string(),
        _ if c.is_ascii_alphanumeric() || "-._~:/!$'()*,;=@".contains(c) => c.to_string(),
        _ => format!("%{:02X}", c as u32),
    }
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_sprite_response() {
        let resp = deterministic_response("Generate an ASCII sprite for a happy robot");
        assert!(resp.contains("/\\_/\\"));
    }

    #[test]
    fn deterministic_name_response() {
        let resp = deterministic_response("Generate a creative name");
        assert_eq!(resp, "Code Companion");
    }

    #[test]
    fn deterministic_clean_response() {
        let resp = deterministic_response("Analyze code quality");
        assert_eq!(resp, "CLEAN");
    }

    #[test]
    fn deterministic_generic_response() {
        let resp = deterministic_response("Hello world");
        assert!(resp.contains("offline mode"));
    }

    #[test]
    fn provider_names() {
        assert_eq!(
            FreeAiProvider::Pollinations.name(),
            "Pollinations.ai (free)"
        );
        assert_eq!(
            FreeAiProvider::Deterministic.name(),
            "Deterministic (offline)"
        );
    }

    #[test]
    fn url_encoding_basics() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("a&b"), "a%26b");
    }

    #[test]
    fn client_has_deterministic_last() {
        let client = FreeAiClient::new(vec![
            FreeAiProvider::Pollinations,
            FreeAiProvider::Deterministic,
        ]);
        assert_eq!(client.providers().len(), 2);
    }
}


