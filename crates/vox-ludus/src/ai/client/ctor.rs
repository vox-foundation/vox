use std::pin::Pin;

use futures_util::{Stream, StreamExt};

use crate::ai::constants::*;
use crate::ai::error::AiError;
use crate::ai::fallback::deterministic_response;
use crate::ai::provider::FreeAiProvider;

use super::{AiReportFn, FreeAiClient};

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
    pub(crate) async fn probe_ollama(url: &str) -> bool {
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
}
