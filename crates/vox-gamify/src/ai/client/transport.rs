use bytes::Bytes;
use futures_util::StreamExt;
use std::pin::Pin;

use futures_util::Stream;

use crate::ai::constants::*;
use crate::ai::error::AiError;
use crate::ai::fallback::deterministic_response;
use crate::ai::keys::{resolve_gemini_key, resolve_openrouter_key};
use crate::ai::provider::FreeAiProvider;
use crate::ai::validate::urlencode;

use super::FreeAiClient;

mod vox_local_probe {
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    use crate::ai::error::AiError;

    const VOX_LOCAL_PROBE_TIMEOUT_SECS: u64 = 2;
    const VOX_LOCAL_PROBE_CACHE_TTL_SECS: u64 = 30;

    #[derive(Clone, Debug)]
    struct VoxLocalProbeCache {
        cached_at: Instant,
        base: String,
        candidates_fingerprint: String,
    }

    fn vox_local_probe_cache() -> &'static Mutex<Option<VoxLocalProbeCache>> {
        static CACHE: OnceLock<Mutex<Option<VoxLocalProbeCache>>> = OnceLock::new();
        CACHE.get_or_init(|| Mutex::new(None))
    }

    fn candidates_fingerprint(candidates: &[String]) -> String {
        candidates.join("\n")
    }

    async fn health_ok_at(client: &reqwest::Client, base: &str) -> bool {
        let root = base.trim_end_matches('/');
        for path in ["/health", "/ready"] {
            let url = format!("{root}{path}");
            let Ok(res) = client
                .get(&url)
                .timeout(Duration::from_secs(VOX_LOCAL_PROBE_TIMEOUT_SECS))
                .send()
                .await
            else {
                continue;
            };
            if !res.status().is_success() {
                continue;
            }
            let Ok(body) = res.text().await else {
                continue;
            };
            if vox_config::inference::vox_local_health_identifies_serve(&body) {
                return true;
            }
        }
        false
    }

    fn store_probe_winner(base: String, fingerprint: String) {
        let mut cache = vox_local_probe_cache()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *cache = Some(VoxLocalProbeCache {
            cached_at: Instant::now(),
            base,
            candidates_fingerprint: fingerprint,
        });
    }

    /// Sync base URL: TTL-cached winner, else first probe candidate.
    #[allow(dead_code)] // sync cold-path; streaming callers use `resolve_vox_local_generate_url`
    pub(super) fn vox_local_generate_base_url() -> String {
        let candidates = vox_config::inference::vox_local_endpoint_probe_candidates();
        let fp = candidates_fingerprint(&candidates);
        if let Ok(guard) = vox_local_probe_cache().lock() {
            if let Some(entry) = guard.as_ref() {
                if entry.candidates_fingerprint == fp
                    && entry.cached_at.elapsed()
                        < Duration::from_secs(VOX_LOCAL_PROBE_CACHE_TTL_SECS)
                {
                    return entry.base.clone();
                }
            }
        }
        candidates
            .into_iter()
            .next()
            .unwrap_or_else(|| "http://127.0.0.1:11434".to_string())
    }

    #[allow(dead_code)] // exercised by unit tests; streaming uses `resolve_vox_local_generate_url`
    pub(super) fn vox_local_generate_url() -> String {
        format!(
            "{}/generate",
            vox_local_generate_base_url().trim_end_matches('/')
        )
    }

    /// Probe candidates, cache the first healthy `vox-ml-cli` base, return `/generate` URL.
    pub(super) async fn resolve_vox_local_generate_url(
        client: &reqwest::Client,
    ) -> Result<String, AiError> {
        let ttl = Duration::from_secs(VOX_LOCAL_PROBE_CACHE_TTL_SECS);
        let candidates = vox_config::inference::vox_local_endpoint_probe_candidates();
        let fp = candidates_fingerprint(&candidates);

        let cached_base = {
            let cache = vox_local_probe_cache()
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            cache.as_ref().and_then(|entry| {
                if entry.candidates_fingerprint == fp && entry.cached_at.elapsed() < ttl {
                    Some(entry.base.clone())
                } else {
                    None
                }
            })
        };

        if let Some(base) = cached_base {
            if health_ok_at(client, &base).await {
                store_probe_winner(base.clone(), fp);
                return Ok(format!("{}/generate", base.trim_end_matches('/')));
            }
        }

        if candidates.is_empty() {
            return Err(AiError::AllProvidersFailed(
                "VoxLocal probe candidates empty (VOX_LOCAL_ENDPOINT is blank)".into(),
            ));
        }

        let mut last_err = String::new();
        for base in &candidates {
            if health_ok_at(client, base).await {
                store_probe_winner(base.clone(), fp);
                return Ok(format!("{}/generate", base.trim_end_matches('/')));
            }
            last_err = format!("no healthy vox-ml-cli at {base}/{{health,ready}}");
        }

        Err(AiError::AllProvidersFailed(format!(
            "VoxLocal unreachable ({last_err}); run `vox mens serve` (often :11435 when Ollama owns :11434) or set VOX_LOCAL_ENDPOINT."
        )))
    }

    #[cfg(test)]
    pub(super) fn clear_probe_cache_for_test() {
        if let Ok(mut g) = vox_local_probe_cache().lock() {
            *g = None;
        }
    }

    #[cfg(test)]
    pub(super) fn store_probe_winner_for_test(base: String, fingerprint: String) {
        store_probe_winner(base, fingerprint);
    }

    #[cfg(test)]
    pub(super) fn candidates_fingerprint_for_test(candidates: &[String]) -> String {
        candidates_fingerprint(candidates)
    }
}

impl FreeAiClient {
    /// POST to Ollama `/api/generate` with stream=true.
    ///
    /// T4.1 consolidation note (documented, narrow exception — not migrated onto
    /// `vox_llm_egress`/`vox_actor_runtime::llm_stream_activity`): Ollama's native
    /// `/api/generate` endpoint speaks NDJSON, not the OpenAI-compatible chat/completions
    /// wire `vox-llm-egress` implements. Ollama does also expose an OpenAI-compatible
    /// `/v1/chat/completions` endpoint in recent versions, so migrating this onto the
    /// egress core is plausible — but it requires adding an "ollama" branch to
    /// `vox_config::resolve_egress::resolve_egress` (base URL + auth-not-required handling)
    /// and verifying the local-Ollama SSE framing matches what `vox_openai::sse` expects.
    /// That is real, separately-scoped provider-coverage work; tracked as a T4-series
    /// follow-up rather than folded into this consolidation pass.
    pub(crate) async fn stream_ollama(
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
                Err(AiError::RateLimited {
                    provider: "ollama".to_string(),
                    retry_after_secs: vox_http_client::parse_retry_after(resp.headers()),
                })?;
            }

            use vox_openai::sse::Utf8LineBuffer;
            let mut stream = resp.bytes_stream();
            let mut line_buf = Utf8LineBuffer::new();

            while let Some(item) = stream.next().await {
                let chunk: Bytes = item.map_err(AiError::Http)?;
                let mut lines: Vec<String> = Vec::new();
                line_buf.push_lossy_bytes(&chunk, |line| {
                    if !line.is_empty() {
                        lines.push(line.to_string());
                    }
                });
                for line in lines {
                    let json: serde_json::Value =
                        serde_json::from_str(&line).map_err(AiError::Json)?;
                    if let Some(token) = json["response"].as_str() {
                        yield token.to_string();
                    }
                    if json["done"].as_bool().unwrap_or(false) {
                        return;
                    }
                }
            }
            let mut tail: Vec<String> = Vec::new();
            line_buf.flush_trailing(|line| tail.push(line.to_string()));
            for line in tail {
                let json: serde_json::Value =
                    serde_json::from_str(&line).map_err(AiError::Json)?;
                if let Some(token) = json["response"].as_str() {
                    yield token.to_string();
                }
            }
        })
    }

    /// One-shot POST to MENS / VoxLocal `{base}/generate`.
    ///
    /// The Axis chat picker stores `mens/<run>` as `model_override`, which used
    /// to hit Ollama `/api/generate` (wrong wire + wrong port when Ollama
    /// already owns `:11434`). Walks [`vox_config::inference::vox_local_endpoint_probe_candidates`]
    /// with TTL-cached winner after a successful `/health` or `/ready` probe.
    /// Metal first-token can exceed the cascade client's 15s timeout, so the
    /// POST overrides it.
    #[allow(dead_code)] // exercised by unit tests; streaming uses `resolve_vox_local_generate_url`
    pub(crate) fn vox_local_generate_url() -> String {
        vox_local_probe::vox_local_generate_url()
    }

    pub(crate) async fn stream_vox_local(
        http: &reqwest::Client,
        prompt: &str,
        model: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, AiError>> + Send>> {
        let url = match vox_local_probe::resolve_vox_local_generate_url(http).await {
            Ok(url) => url,
            Err(e) => {
                return Box::pin(futures_util::stream::once(async move { Err(e) }));
            }
        };
        let mut body = serde_json::json!({
            "prompt": prompt,
            "validate": false,
            "max_retries": 0,
            "max_tokens": 512,
        });
        if let Some(model) = model {
            body["model"] = serde_json::Value::String(model.to_string());
        }
        let http = http.clone();

        Box::pin(async_stream::try_stream! {
            let resp = http
                .post(&url)
                .timeout(std::time::Duration::from_secs(300))
                .json(&body)
                .send()
                .await
                .map_err(AiError::Http)?;

            if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                Err(AiError::RateLimited {
                    provider: "vox_local".to_string(),
                    retry_after_secs: vox_http_client::parse_retry_after(resp.headers()),
                })?;
            }
            if !resp.status().is_success() {
                Err(AiError::ProviderStatus {
                    provider: "vox_local".to_string(),
                    status: resp.status().as_u16(),
                })?;
            }

            let json: serde_json::Value = resp.json().await.map_err(AiError::Http)?;
            let text = json
                .get("code")
                .and_then(|v| v.as_str())
                .or_else(|| json.get("text").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                Err(AiError::EmptyResponse)?;
            }
            yield text;
        })
    }

    /// POST to Gemini `streamGenerateContent`.
    ///
    /// T4.1 consolidation note (documented, narrow exception — not migrated onto
    /// `vox_llm_egress`): direct Gemini (`generativelanguage.googleapis.com`) is NOT
    /// OpenAI-compatible on the wire (different request/response JSON shape, `?key=`
    /// query-param auth instead of a bearer header, no SSE `data:` framing for
    /// `streamGenerateContent`). `vox-llm-egress` is deliberately scoped to the
    /// OpenAI-compatible wire only (see its crate doc). Adding a second wire protocol
    /// to the egress core is a real, separately-scoped provider-coverage project, not a
    /// small addition — left as an explicit follow-up rather than attempted here.
    pub(crate) async fn stream_gemini(
        http: &reqwest::Client,
        api_key: &str,
        model: &str,
        prompt: &str,
    ) -> Pin<Box<dyn Stream<Item = Result<String, AiError>> + Send>> {
        let resolved_key = resolve_gemini_key(api_key);

        // Direct Gemini (generativelanguage) is NOT OpenAI-compatible — the egress core can't
        // carry it; documented local egress per the egress design spec.
        // vox-arch-check: allow llm-egress
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
                Err(AiError::RateLimited {
                    provider: "google".to_string(),
                    retry_after_secs: vox_http_client::parse_retry_after(resp.headers()),
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

    /// OpenRouter streaming chat completions, routed through `vox_actor_runtime`'s durable
    /// activity envelope (`execute_activity` — same retry/backoff primitive `llm_chat` and
    /// `llm_stream_activity` use), which in turn calls the sanctioned egress core
    /// (`vox_llm_egress::stream_once`). This is the SAME consolidated path
    /// `vox_actor_runtime::llm::llm_stream_activity` uses internally; it is inlined here
    /// (rather than calling `llm_stream_activity` directly) only so the OpenRouter
    /// response-cost header — which `llm_stream`'s facade intentionally discards, since
    /// unary cost telemetry already lives in the facade — can still reach gamify's
    /// `cost_reporter`. Gamify keeps its own key + attribution headers.
    pub(crate) fn stream_openrouter(
        // The wire goes through the egress core (which owns its client); kept for signature compat.
        _http: &reqwest::Client,
        api_key: &str,
        model: &str,
        prompt: &str,
        cost_reporter: Option<super::CostReportFn>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, AiError>> + Send>> {
        let model = model.to_string();
        let prompt = prompt.to_string();
        let api_key = api_key.to_string();
        Box::pin(async_stream::try_stream! {
            let resolved_key = resolve_openrouter_key(&api_key);
            if resolved_key.is_empty() {
                Err(AiError::AllProvidersFailed(
                    "OpenRouter API key not set (configure Clavis or OPENROUTER_API_KEY)".to_string(),
                ))?;
            }
            let ereq = vox_llm_egress::EgressRequest {
                base_url: vox_config::openrouter_chat_completions_url(),
                api_key: resolved_key,
                model: model.clone(),
                headers: vec![
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/vox-foundation/vox".to_string(),
                    ),
                    ("X-Title".to_string(), "Vox Gamify".to_string()),
                ],
                throttle_key: "openrouter".to_string(),
                max_concurrent: 8,
                timeout_ms: None,
            };
            let msgs = [vox_llm_egress::ChatMessage {
                role: "user".to_string(),
                content: prompt,
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            }];
            let params = vox_llm_egress::ChatParams { max_tokens: Some(512), ..Default::default() };

            // Retry only the connection attempt (headers + stream handle), exactly like
            // `llm_stream_activity`'s envelope — a fresh attempt after the first byte has
            // already been yielded would duplicate output, so mid-stream errors are not retried.
            let activity_options = vox_actor_runtime::ActivityOptions::new().with_retries(1);
            let connect = vox_actor_runtime::execute_activity(
                "gamify_stream_openrouter",
                &activity_options,
                || {
                    let ereq = ereq.clone();
                    let msgs = msgs.clone();
                    let params = params.clone();
                    async move { vox_llm_egress::stream_once(&ereq, &msgs, &params).await }
                },
            )
            .await;

            let (mut inner, cost) = match connect {
                vox_actor_runtime::ActivityResult::Ok(v) => v,
                vox_actor_runtime::ActivityResult::Failed(e) => {
                    Err(AiError::AllProvidersFailed(e.to_string()))?;
                    return;
                }
                vox_actor_runtime::ActivityResult::Cancelled => {
                    Err(AiError::AllProvidersFailed("cancelled".to_string()))?;
                    return;
                }
            };
            if let (Some(reporter), Some(c)) = (cost_reporter.as_ref(), cost) {
                reporter(c);
            }
            while let Some(item) = inner.next().await {
                let chunk = item.map_err(|e| AiError::AllProvidersFailed(e.to_string()))?;
                yield chunk;
            }
        })
    }

    pub(crate) async fn call_provider_static(
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
    pub(crate) async fn call_openrouter_static(
        // The wire now goes through vox_llm_egress::chat_once (which owns its HTTP client),
        // so the passed client is no longer used; kept for signature compatibility.
        _http: &reqwest::Client,
        api_key: &str,
        models: &[String],
        prompt: &str,
    ) -> Result<String, AiError> {
        let resolved_key = resolve_openrouter_key(api_key);
        if resolved_key.is_empty() {
            return Err(AiError::AllProvidersFailed(
                "OpenRouter API key not set (configure Clavis or OPENROUTER_API_KEY)".to_string(),
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
            // Route the wire through the sanctioned egress core, but keep gamify's own
            // key + attribution headers (X-Title differs from the orchestrator's).
            let ereq = vox_llm_egress::EgressRequest {
                base_url: vox_config::openrouter_chat_completions_url(),
                api_key: resolved_key.clone(),
                model: (*model).to_string(),
                headers: vec![
                    (
                        "HTTP-Referer".to_string(),
                        "https://github.com/vox-foundation/vox".to_string(),
                    ),
                    ("X-Title".to_string(), "Vox Gamify".to_string()),
                ],
                throttle_key: "openrouter".to_string(),
                max_concurrent: 8,
                timeout_ms: Some(30_000),
            };
            let msgs = [vox_llm_egress::ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
                name: None,
                content_parts: None,
            }];
            let params = vox_llm_egress::ChatParams {
                max_tokens: Some(512),
                ..Default::default()
            };
            match vox_llm_egress::chat_once(&ereq, &msgs, &params).await {
                Ok(resp) => {
                    let trimmed = resp.content.trim().to_string();
                    if !trimmed.is_empty() {
                        tracing::debug!("OpenRouter model '{}' succeeded", model);
                        return Ok(trimmed);
                    }
                    last_err = format!("model '{}': empty content in response", model);
                }
                Err(vox_llm_egress::EgressError::RateLimited { retry_after }) => {
                    if first_rate_limit.is_none() {
                        first_rate_limit =
                            Some((model.to_string(), retry_after.map(|d| d.as_secs())));
                    }
                    last_err = format!("model '{}': rate limited", model);
                    tracing::debug!("OpenRouter rate-limited for '{}', trying next", model);
                    continue;
                }
                // Quota exceeded (402) — treat like a rate-limit and try the next model.
                Err(vox_llm_egress::EgressError::Status { code: 402, .. }) => {
                    if first_rate_limit.is_none() {
                        first_rate_limit = Some((model.to_string(), None));
                    }
                    last_err = format!("model '{}': HTTP 402", model);
                    tracing::debug!("OpenRouter 402 for '{}', trying next", model);
                    continue;
                }
                Err(e) => {
                    last_err = format!("model '{}': {}", model, e);
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

    pub(crate) async fn call_ollama_static(
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
            return Err(AiError::RateLimited {
                provider: "ollama".to_string(),
                retry_after_secs: vox_http_client::parse_retry_after(resp.headers()),
            });
        }
        let json: serde_json::Value = resp.json().await?;
        json["response"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(AiError::EmptyResponse)
    }

    pub(crate) async fn call_pollinations_static(
        http: &reqwest::Client,
        prompt: &str,
    ) -> Result<String, AiError> {
        let encoded = urlencode(prompt);
        let url = format!("{}{}?model=openai&nologo=true", POLLINATIONS_BASE, encoded);
        let resp = http.get(&url).send().await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AiError::RateLimited {
                provider: "pollinations".to_string(),
                retry_after_secs: vox_http_client::parse_retry_after(resp.headers()),
            });
        }
        if !resp.status().is_success() {
            return Err(AiError::ProviderStatus {
                provider: "pollinations".to_string(),
                status: resp.status().as_u16(),
            });
        }

        let text = resp.text().await?;
        if text.trim().is_empty() {
            return Err(AiError::EmptyResponse);
        }
        Ok(text)
    }

    pub(crate) async fn call_gemini_static(
        http: &reqwest::Client,
        api_key: &str,
        model: &str,
        prompt: &str,
    ) -> Result<String, AiError> {
        let resolved_key = resolve_gemini_key(api_key);
        let url = GEMINI_ENDPOINT_TEMPLATE
            .replace("{MODEL}", model)
            .replace("{KEY}", &resolved_key);
        let body = serde_json::json!({ "contents": [{ "parts": [{ "text": prompt }] }] });
        let resp = http.post(&url).json(&body).send().await?;

        if resp.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AiError::RateLimited {
                provider: "google".to_string(),
                retry_after_secs: vox_http_client::parse_retry_after(resp.headers()),
            });
        }
        if !resp.status().is_success() {
            return Err(AiError::ProviderStatus {
                provider: "google".to_string(),
                status: resp.status().as_u16(),
            });
        }

        let json: serde_json::Value = resp.json().await?;
        json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(AiError::EmptyResponse)
    }

    /// Call a single provider.
    pub(crate) async fn call_provider(
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

#[cfg(test)]
mod ollama_ndjson_line_tests {
    use vox_openai::sse::Utf8LineBuffer;

    /// Mirrors the line handling in [`FreeAiClient::stream_ollama`]
    /// (framing itself is covered in vox-openai's sse tests).
    fn collect_responses_from_chunks(chunks: &[&[u8]]) -> Vec<String> {
        let mut line_buf = Utf8LineBuffer::new();
        let mut out: Vec<String> = Vec::new();
        let mut on_line = |line: &str| {
            if line.is_empty() {
                return;
            }
            let json: serde_json::Value = serde_json::from_str(line).unwrap();
            if let Some(s) = json["response"].as_str() {
                out.push(s.to_string());
            }
        };
        for chunk in chunks {
            line_buf.push_lossy_bytes(chunk, &mut on_line);
        }
        line_buf.flush_trailing(&mut on_line);
        out
    }

    #[test]
    fn ndjson_split_across_tcp_like_chunks() {
        let out = collect_responses_from_chunks(&[
            b"{\"response\":\"hel",
            b"lo\",\"done\":false}\n{\"response\":\"\",\"done\":true}\n",
        ]);
        assert_eq!(out, vec!["hello", ""]);
    }
}

#[cfg(test)]
mod openrouter_stream_consolidation_tests {
    // Rust 2024 made std::env::{set_var,remove_var} unsafe; mutated single-threaded
    // under a process-wide lock (this module owns the only writer of OPENROUTER_BASE_URL
    // among vox-gamify's tests).
    #![allow(unsafe_code)]

    use futures_util::StreamExt;
    use std::sync::Mutex;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::FreeAiClient;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// RED test 2: `FreeAiClient::stream_openrouter` genuinely routes through
    /// `vox_llm_egress::stream_once` (the sanctioned egress core), not a bespoke
    /// gamify-owned HTTP client. Proven by pointing `OPENROUTER_BASE_URL` at a
    /// wiremock server and asserting the request lands on it with the exact
    /// OpenAI-compatible request shape `vox-llm-egress::wire::build_request` emits
    /// (a `stream: true` field, `messages` array) — a bespoke NDJSON-style client
    /// (like `stream_ollama`'s) would not produce this shape, and a stale bespoke
    /// HTTP path would never hit this mock server at all since it wouldn't honor
    /// `OPENROUTER_BASE_URL` via `vox_config::resolve_egress`.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // intentional: serializes this module's sole
    // OPENROUTER_BASE_URL mutation across the whole test body, mirroring
    // vox-config's env-mutation test-lock pattern (see resolve_egress.rs tests).
    async fn stream_openrouter_routes_through_llm_egress_stream_once() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let server = MockServer::start().await;
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"con\"}}]}\n\
                   data: {\"choices\":[{\"delta\":{\"content\":\"solidated\"}}]}\n\
                   data: [DONE]\n\n";
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(sse),
            )
            .mount(&server)
            .await;

        let prev = std::env::var("OPENROUTER_BASE_URL").ok();
        unsafe {
            std::env::set_var("OPENROUTER_BASE_URL", server.uri());
        }
        vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

        let http = vox_http_client::client();
        let mut stream =
            FreeAiClient::stream_openrouter(&http, "test-api-key", "test/model", "hello", None);
        let mut got = String::new();
        while let Some(item) = stream.next().await {
            got.push_str(&item.expect("chunk"));
        }

        unsafe {
            match prev {
                Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
                None => std::env::remove_var("OPENROUTER_BASE_URL"),
            }
        }
        vox_config::snapshot::bump(&["OPENROUTER_BASE_URL"]);

        assert_eq!(
            got, "consolidated",
            "stream_openrouter must yield SSE deltas assembled by vox_llm_egress::stream_once \
             against the mock server reachable only via OPENROUTER_BASE_URL resolution"
        );

        // Confirm the request actually landed on the mock (not e.g. silently short-circuited).
        let received = server
            .received_requests()
            .await
            .expect("mock tracks requests");
        assert_eq!(
            received.len(),
            1,
            "exactly one request must reach the egress core's target"
        );
        let body: serde_json::Value = serde_json::from_slice(&received[0].body).expect("json body");
        assert_eq!(
            body["stream"], true,
            "request body must be the OpenAI-compatible shape vox-llm-egress builds, \
             not a bespoke NDJSON-style payload"
        );
        assert!(
            body["messages"].is_array(),
            "request body must carry a `messages` array (egress core's ChatMessage wire shape)"
        );
    }
}

#[cfg(test)]
mod vox_local_stream_tests {
    #![allow(unsafe_code)]

    use futures_util::StreamExt;
    use std::sync::Mutex;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::{FreeAiClient, StreamRoute};
    use super::vox_local_probe::{
        candidates_fingerprint_for_test, clear_probe_cache_for_test, store_probe_winner_for_test,
        vox_local_generate_base_url,
    };
    use crate::ai::provider::FreeAiProvider;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    async fn mount_vox_local_health(server: &MockServer) {
        for health_path in ["/health", "/ready"] {
            Mock::given(method("GET"))
                .and(path(health_path))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "status": "ok",
                    "service": "vox-ml-cli"
                })))
                .mount(server)
                .await;
        }
    }

    fn restore_vox_local_endpoint(prev: Option<String>) {
        unsafe {
            match prev {
                Some(v) => std::env::set_var("VOX_LOCAL_ENDPOINT", v),
                None => std::env::remove_var("VOX_LOCAL_ENDPOINT"),
            }
        }
    }

    async fn drain_stream(
        mut stream: impl futures_util::Stream<Item = Result<String, crate::ai::error::AiError>> + Unpin,
    ) -> Result<String, crate::ai::error::AiError> {
        let mut got = String::new();
        while let Some(item) = stream.next().await {
            got.push_str(&item?);
        }
        Ok(got)
    }

    fn post_generate_requests(
        received: &[wiremock::Request],
    ) -> impl Iterator<Item = &wiremock::Request> {
        received
            .iter()
            .filter(|r| r.method.as_str() == "POST" && r.url.path() == "/generate")
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn stream_vox_local_posts_generate_to_vox_local_endpoint() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_probe_cache_for_test();
        let server = MockServer::start().await;
        mount_vox_local_health(&server).await;
        Mock::given(method("POST"))
            .and(path("/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "The capital of France is Paris.",
                "valid": true,
            })))
            .mount(&server)
            .await;

        let prev = std::env::var("VOX_LOCAL_ENDPOINT").ok();
        unsafe {
            std::env::set_var("VOX_LOCAL_ENDPOINT", server.uri());
        }

        let http = vox_http_client::client();
        let stream = FreeAiClient::stream_vox_local(
            &http,
            "What is the capital of France?",
            Some("mens/e2e-smoke-metal"),
        )
        .await;
        let got = drain_stream(stream).await.expect("chunk");
        restore_vox_local_endpoint(prev);

        assert_eq!(got, "The capital of France is Paris.");
        let received = server
            .received_requests()
            .await
            .expect("mock tracks requests");
        let generate_posts: Vec<_> = post_generate_requests(&received).collect();
        assert_eq!(generate_posts.len(), 1, "expected one POST /generate");
        let body: serde_json::Value =
            serde_json::from_slice(&generate_posts[0].body).expect("json body");
        assert_eq!(body["validate"], false);
        assert_eq!(body["prompt"], "What is the capital of France?");
        assert_eq!(body["max_tokens"], 512);
        assert_eq!(body["model"], "mens/e2e-smoke-metal");
    }

    #[test]
    fn vox_local_generate_url_honors_env_when_cache_cold() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_probe_cache_for_test();
        let prev = std::env::var("VOX_LOCAL_ENDPOINT").ok();
        unsafe {
            std::env::set_var("VOX_LOCAL_ENDPOINT", "http://127.0.0.1:17863");
        }
        assert_eq!(
            FreeAiClient::vox_local_generate_url(),
            "http://127.0.0.1:17863/generate"
        );
        restore_vox_local_endpoint(prev);
    }

    #[test]
    fn vox_local_generate_base_url_falls_back_to_first_candidate_when_cache_cold() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_probe_cache_for_test();
        let prev = std::env::var("VOX_LOCAL_ENDPOINT").ok();
        unsafe {
            std::env::remove_var("VOX_LOCAL_ENDPOINT");
        }
        let base = vox_local_generate_base_url();
        restore_vox_local_endpoint(prev);
        assert!(
            base.contains("11434") || base.contains("11435") || !base.is_empty(),
            "unexpected base {base}"
        );
    }

    #[test]
    fn vox_local_generate_base_url_prefers_cached_winner() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let prev = std::env::var("VOX_LOCAL_ENDPOINT").ok();
        unsafe {
            std::env::remove_var("VOX_LOCAL_ENDPOINT");
        }
        let fp = candidates_fingerprint_for_test(
            &vox_config::inference::vox_local_endpoint_probe_candidates(),
        );
        store_probe_winner_for_test("http://127.0.0.1:11435".into(), fp);
        let base = vox_local_generate_base_url();
        restore_vox_local_endpoint(prev);
        assert!(
            base.contains("11435"),
            "warm cache should win over first candidate; got {base}"
        );
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn user_model_override_mens_slug_posts_generate_to_vox_local_endpoint() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        clear_probe_cache_for_test();
        let server = MockServer::start().await;
        mount_vox_local_health(&server).await;
        Mock::given(method("POST"))
            .and(path("/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "mens via vox-local",
                "valid": true,
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{\"response\":\"ollama should not run\",\"done\":true}\n"),
            )
            .mount(&server)
            .await;

        let prev = std::env::var("VOX_LOCAL_ENDPOINT").ok();
        unsafe {
            std::env::set_var("VOX_LOCAL_ENDPOINT", server.uri());
        }

        let client = FreeAiClient::new(vec![FreeAiProvider::Ollama {
            url: server.uri(),
            model: "llama3.2".to_string(),
        }]);
        let stream = client
            .generate_stream_routed(
                "smoke",
                StreamRoute::UserModelOverride("mens/e2e-smoke-metal"),
            )
            .await;
        let got = drain_stream(stream).await.expect("mens chunk");
        restore_vox_local_endpoint(prev);

        assert_eq!(got, "mens via vox-local");
        let received = server
            .received_requests()
            .await
            .expect("mock tracks requests");
        let generate_posts: Vec<_> = post_generate_requests(&received).collect();
        assert_eq!(
            generate_posts.len(),
            1,
            "mens override must POST /generate exactly once"
        );
        let body: serde_json::Value =
            serde_json::from_slice(&generate_posts[0].body).expect("json body");
        assert_eq!(body["prompt"], "smoke");
        assert_eq!(body["max_tokens"], 512);
        assert_eq!(body["model"], "mens/e2e-smoke-metal");
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn user_model_override_non_mens_slug_does_not_post_vox_local_generate() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/generate"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "code": "wrong wire",
                "valid": true,
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/generate"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string("{\"response\":\"ollama ok\",\"done\":true}\n"),
            )
            .mount(&server)
            .await;

        let prev = std::env::var("VOX_LOCAL_ENDPOINT").ok();
        unsafe {
            std::env::set_var("VOX_LOCAL_ENDPOINT", server.uri());
        }

        let client = FreeAiClient::new(vec![FreeAiProvider::Ollama {
            url: server.uri(),
            model: "llama3.2".to_string(),
        }]);
        let stream = client
            .generate_stream_routed("smoke", StreamRoute::UserModelOverride("llama3.2"))
            .await;
        let got = drain_stream(stream).await.expect("ollama chunk");
        restore_vox_local_endpoint(prev);

        assert_eq!(got, "ollama ok");
        let received = server
            .received_requests()
            .await
            .expect("mock tracks requests");
        assert_eq!(received.len(), 1);
        assert_eq!(received[0].url.path(), "/api/generate");
        assert_ne!(
            received[0].url.path(),
            "/generate",
            "non-mens slugs must not use VOX_LOCAL_ENDPOINT /generate"
        );
    }
}
