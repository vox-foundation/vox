//! One streamed chat completion, measured end to end: time to the first content token, the
//! full text, and the usage, served model id and cost the provider reports in its final SSE
//! chunk (requested with `stream_options.include_usage`). Benchmarks use this instead of
//! `chat_once` because a unary call cannot observe time-to-first-token.

use std::time::Instant;

use futures_util::StreamExt;

use crate::{ChatMessage, ChatParams, EgressError, EgressRequest, throttle};

/// What one measured streaming call observed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeasuredCompletion {
    pub content: String,
    /// Milliseconds from sending the request to the first non-empty content delta.
    /// `None` when the model produced no content at all.
    pub ttft_ms: Option<u64>,
    /// Milliseconds to the first streamed token of any kind, reasoning included.
    pub first_token_ms: Option<u64>,
    /// Milliseconds from sending the request to the end of the stream.
    pub latency_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    /// Reasoning tokens the provider reports inside `completion_tokens`.
    pub reasoning_tokens: u32,
    /// The model id the provider says served the request. Differs from the requested id
    /// for aliases such as OpenRouter's `~vendor/…-latest`.
    pub served_model: Option<String>,
    /// Provider generation id (`id` on the chunks), for later cost reconciliation.
    pub generation_id: Option<String>,
    /// Provider-reported cost (`usage.cost` / `usage.total_cost`, else `x-response-cost`).
    pub cost_usd: Option<f64>,
}

impl MeasuredCompletion {
    /// Output tokens per second over the generation phase, from the first streamed token
    /// (reasoning included) to the end. Reasoning the provider counted but never streamed
    /// was generated before the first visible token, so then the whole call is the window.
    /// `None` when the window is too short to time (the answer arrived in one burst).
    #[must_use]
    pub fn tokens_per_sec(&self) -> Option<f64> {
        const MIN_WINDOW_MS: u64 = 250;
        let first = self.first_token_ms.or(self.ttft_ms)?;
        let hidden_reasoning = self.reasoning_tokens > 0 && Some(first) == self.ttft_ms;
        let window_ms = if hidden_reasoning {
            self.latency_ms
        } else {
            self.latency_ms.saturating_sub(first)
        };
        (self.completion_tokens > 0 && window_ms >= MIN_WINDOW_MS)
            .then(|| f64::from(self.completion_tokens) * 1000.0 / window_ms as f64)
    }

    /// Fold one SSE `data:` payload (already stripped of the `data: ` prefix) observed
    /// `elapsed_ms` after the request was sent.
    pub fn fold_payload(&mut self, payload: &str, elapsed_ms: u64) {
        let payload = payload.trim();
        if payload.is_empty() || payload == "[DONE]" {
            return;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) else {
            return;
        };
        let delta_obj = &v["choices"][0]["delta"];
        let streamed_reasoning = ["reasoning", "reasoning_content"]
            .iter()
            .any(|k| delta_obj[*k].as_str().is_some_and(|s| !s.is_empty()));
        if streamed_reasoning && self.first_token_ms.is_none() {
            self.first_token_ms = Some(elapsed_ms);
        }
        if let Some(delta) = delta_obj["content"].as_str().filter(|s| !s.is_empty()) {
            if self.first_token_ms.is_none() {
                self.first_token_ms = Some(elapsed_ms);
            }
            if self.ttft_ms.is_none() {
                self.ttft_ms = Some(elapsed_ms);
            }
            self.content.push_str(delta);
        }
        if let Some(m) = v["model"].as_str().filter(|s| !s.is_empty()) {
            self.served_model = Some(m.to_string());
        }
        if self.generation_id.is_none()
            && let Some(id) = v["id"].as_str().filter(|s| !s.is_empty())
        {
            self.generation_id = Some(id.to_string());
        }
        let usage = &v["usage"];
        if usage.is_object() {
            if let Some(p) = usage["prompt_tokens"].as_u64() {
                self.prompt_tokens = p as u32;
            }
            if let Some(c) = usage["completion_tokens"].as_u64() {
                self.completion_tokens = c as u32;
            }
            if let Some(r) = usage["completion_tokens_details"]["reasoning_tokens"].as_u64() {
                self.reasoning_tokens = r as u32;
            }
            if let Some(cost) = usage["cost"]
                .as_f64()
                .or_else(|| usage["total_cost"].as_f64())
            {
                self.cost_usd = Some(cost);
            }
        }
    }
}

/// Stream one completion and measure it. Errors before the first byte are returned as
/// [`EgressError`]; a transport error mid-stream is returned too, since a truncated answer
/// is not a valid benchmark sample.
pub async fn stream_measured_once(
    req: &EgressRequest,
    messages: &[ChatMessage],
    params: &ChatParams<'_>,
) -> Result<MeasuredCompletion, EgressError> {
    let client = vox_http_client::client();
    let _permit = throttle::acquire_permit(&req.throttle_key, req.max_concurrent).await;

    let mut body = serde_json::to_value(crate::wire::build_request(req, params, messages, true))
        .map_err(|e| EgressError::Decode(e.to_string()))?;
    body["stream_options"] = serde_json::json!({ "include_usage": true });
    let mut http = client
        .post(&req.base_url)
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(&body);
    if !req.api_key.is_empty() {
        http = http.bearer_auth(&req.api_key);
    }
    for (name, value) in &req.headers {
        http = http.header(name, value);
    }

    let start = Instant::now();
    let res = http
        .send()
        .await
        .map_err(|e| EgressError::Http(e.to_string()))?;
    let status = res.status();
    if status.as_u16() == 429 {
        let retry_after = throttle::retry_after_from_headers(res.headers());
        throttle::on_rate_limited(&req.throttle_key, retry_after);
        return Err(EgressError::RateLimited { retry_after });
    }
    if !status.is_success() {
        let body = res.text().await.unwrap_or_default();
        return Err(EgressError::Status {
            code: status.as_u16(),
            body,
        });
    }
    throttle::on_success(&req.throttle_key);

    let mut out = MeasuredCompletion {
        cost_usd: res
            .headers()
            .get("x-response-cost")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<f64>().ok()),
        ..MeasuredCompletion::default()
    };
    let mut buf = vox_openai::sse::Utf8LineBuffer::new();
    let mut bytes = res.bytes_stream();
    while let Some(chunk) = bytes.next().await {
        let chunk = chunk.map_err(|e| EgressError::Http(e.to_string()))?;
        let elapsed = start.elapsed().as_millis() as u64;
        buf.push_lossy_bytes(&chunk, |line| {
            if let Some(payload) = line.trim().strip_prefix("data:") {
                out.fold_payload(payload, elapsed);
            }
        });
    }
    let elapsed = start.elapsed().as_millis() as u64;
    buf.flush_trailing(|line| {
        if let Some(payload) = line.trim().strip_prefix("data:") {
            out.fold_payload(payload, elapsed);
        }
    });
    out.latency_ms = elapsed;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_records_first_token_time_alias_target_usage_and_cost() {
        let mut m = MeasuredCompletion::default();
        m.fold_payload(r#"{"id":"gen-1","model":"acme/x-2","choices":[{"delta":{"role":"assistant","content":""}}]}"#, 40);
        assert_eq!(m.ttft_ms, None, "an empty role delta is not a token");
        m.fold_payload(
            r#"{"id":"gen-1","model":"acme/x-2","choices":[{"delta":{"content":"Hel"}}]}"#,
            120,
        );
        m.fold_payload(
            r#"{"id":"gen-1","choices":[{"delta":{"content":"lo"}}]}"#,
            180,
        );
        m.fold_payload(r#"{"id":"gen-1","choices":[],"usage":{"prompt_tokens":9,"completion_tokens":20,"cost":0.0004}}"#, 200);
        m.fold_payload("[DONE]", 201);
        m.latency_ms = 1120;
        assert_eq!(m.content, "Hello");
        assert_eq!(m.ttft_ms, Some(120));
        assert_eq!(m.served_model.as_deref(), Some("acme/x-2"));
        assert_eq!(m.generation_id.as_deref(), Some("gen-1"));
        assert_eq!((m.prompt_tokens, m.completion_tokens), (9, 20));
        assert_eq!(m.cost_usd, Some(0.0004));
        let tps = m.tokens_per_sec().expect("tokens/sec");
        assert!(
            (tps - 20.0).abs() < 1e-9,
            "20 tokens over 1000 ms of generation, got {tps}"
        );
    }

    #[test]
    fn speed_counts_streamed_reasoning_from_its_first_token() {
        let mut m = MeasuredCompletion::default();
        m.fold_payload(r#"{"choices":[{"delta":{"reasoning":"think"}}]}"#, 200);
        m.fold_payload(r#"{"choices":[{"delta":{"content":"ok"}}]}"#, 2150);
        m.fold_payload(r#"{"choices":[],"usage":{"completion_tokens":400,"completion_tokens_details":{"reasoning_tokens":398}}}"#, 2200);
        m.latency_ms = 2200;
        assert_eq!(
            m.ttft_ms,
            Some(2150),
            "TTFT is still the first visible token"
        );
        let tps = m.tokens_per_sec().expect("tokens/sec");
        assert!(
            (tps - 200.0).abs() < 1e-9,
            "400 tokens over 2000 ms, got {tps}"
        );
    }

    #[test]
    fn speed_spans_whole_call_when_reasoning_was_hidden() {
        let mut m = MeasuredCompletion::default();
        m.fold_payload(r#"{"choices":[{"delta":{"content":"ok"}}]}"#, 3990);
        m.fold_payload(r#"{"choices":[],"usage":{"completion_tokens":800,"completion_tokens_details":{"reasoning_tokens":798}}}"#, 4000);
        m.latency_ms = 4000;
        let tps = m.tokens_per_sec().expect("tokens/sec");
        assert!(
            (tps - 200.0).abs() < 1e-9,
            "800 tokens over the whole 4000 ms, got {tps}"
        );
    }

    #[test]
    fn speed_is_unmeasured_when_the_answer_arrives_in_one_burst() {
        let mut m = MeasuredCompletion::default();
        m.fold_payload(
            r#"{"choices":[{"delta":{"content":"a whole answer"}}]}"#,
            900,
        );
        m.fold_payload(r#"{"choices":[],"usage":{"completion_tokens":60}}"#, 910);
        m.latency_ms = 910;
        assert_eq!(m.tokens_per_sec(), None);
    }

    #[test]
    fn fold_ignores_garbage_and_reports_no_speed_without_tokens() {
        let mut m = MeasuredCompletion::default();
        m.fold_payload("not json", 5);
        m.fold_payload(": keepalive", 6);
        assert_eq!(m, MeasuredCompletion::default());
        assert_eq!(m.tokens_per_sec(), None);
    }
}
