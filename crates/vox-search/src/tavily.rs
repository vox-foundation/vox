pub use crate::tavily_budget::TavilySessionBudget;

use vox_secrets::{SecretId, resolve_secret};

/// Tavily's public API root; `SearchPolicy::tavily_api_url` overrides it.
pub const DEFAULT_TAVILY_BASE_URL: &str = "https://api.tavily.com";

/// Direct HTTP client for Tavily `/search` and `/extract`.
///
/// Task 15: replaces the `tavily` crate wrapper, which (a) sent `api_key: ""` in every
/// search body and never an `Authorization` header — so every real call would 401 —,
/// (b) could not be pointed at `tavily_api_url`, and (c) retried 429s with multi-second
/// backoff inside the lane deadline, so rate limits surfaced as `Timeout`.
#[cfg(feature = "tavily")]
pub struct TavilyClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

#[cfg(feature = "tavily")]
impl TavilyClient {
    /// `None` when `api_key` is blank. `base_url` defaults to [`DEFAULT_TAVILY_BASE_URL`].
    pub fn new(api_key: &str, base_url: Option<&str>) -> Option<Self> {
        let api_key = api_key.trim();
        if api_key.is_empty() {
            return None;
        }
        let http = vox_http_client::client_builder()
            .timeout(vox_config::timeouts::D_30S)
            .build()
            .ok()?;
        Some(Self {
            http,
            api_key: api_key.to_string(),
            base_url: base_url
                .filter(|u| !u.trim().is_empty())
                .unwrap_or(DEFAULT_TAVILY_BASE_URL)
                .trim_end_matches('/')
                .to_string(),
        })
    }

    /// Key from Clavis (`TAVILY_API_KEY`); `None` when unset.
    pub fn from_env(base_url: Option<&str>) -> Option<Self> {
        let binding = resolve_secret(SecretId::TavilyApiKey);
        Self::new(binding.expose()?, base_url)
    }

    async fn post(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let resp = self
            .http
            .post(format!("{}/{endpoint}", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("tavily_{endpoint}_http:{e}"))?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            // Keep Tavily's own `detail.error` text; it is what tells the operator what to fix.
            // A non-JSON body (proxy HTML page, …) is capped so it cannot flood the trace.
            let detail = serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| {
                    v.pointer("/detail/error")
                        .and_then(|d| d.as_str())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| truncate_chars(&text, ERROR_DETAIL_MAX_CHARS));
            return Err(format!(
                "tavily_{endpoint}_status:{}:{detail}",
                status.as_u16()
            ));
        }
        serde_json::from_str(&text).map_err(|e| format!("tavily_{endpoint}_parse:{e}"))
    }

    pub async fn search(
        &self,
        query: &str,
        max_results: usize,
        depth: &str,
    ) -> Result<Vec<TavilyHit>, String> {
        let v = self
            .post(
                "search",
                serde_json::json!({
                    "query": query,
                    "search_depth": depth,
                    "max_results": max_results,
                }),
            )
            .await?;
        Ok(results(&v)
            .map(|r| TavilyHit {
                url: str_field(r, "url"),
                title: str_field(r, "title"),
                content: str_field(r, "content"),
                score: r.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0) as f32,
            })
            .filter(|h| !h.url.is_empty())
            .collect())
    }

    pub async fn extract(
        &self,
        urls: &[String],
    ) -> Result<Vec<crate::tavily_extract::ExtractHit>, String> {
        if urls.is_empty() {
            return Ok(Vec::new());
        }
        let v = self
            .post("extract", serde_json::json!({ "urls": urls }))
            .await?;
        Ok(results(&v)
            .map(|r| crate::tavily_extract::ExtractHit {
                url: str_field(r, "url"),
                content: str_field(r, "raw_content"),
            })
            .collect())
    }
}

#[cfg(feature = "tavily")]
fn results(v: &serde_json::Value) -> impl Iterator<Item = &serde_json::Value> {
    v.get("results")
        .and_then(|r| r.as_array())
        .into_iter()
        .flatten()
}

#[cfg(feature = "tavily")]
fn str_field(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|s| s.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Longest provider error detail kept in a status message.
pub(crate) const ERROR_DETAIL_MAX_CHARS: usize = 300;

pub(crate) fn truncate_chars(s: &str, max: usize) -> String {
    match s.char_indices().nth(max) {
        Some((i, _)) => format!("{}…", &s[..i]),
        None => s.to_string(),
    }
}

/// Credits one `/search` call costs (Tavily pricing: advanced = 2, every other depth = 1).
#[must_use]
pub fn search_credit_cost(depth: &str) -> usize {
    if depth.eq_ignore_ascii_case("advanced") {
        2
    } else {
        1
    }
}

/// True for Tavily statuses that mean "slow down / out of quota" (429, 432 plan
/// limit, 433 pay-as-you-go limit): these arm the long rate-limit cooldown.
#[must_use]
pub fn is_quota_error(message: &str) -> bool {
    [":429:", ":432:", ":433:"]
        .iter()
        .any(|code| message.contains(code))
}

#[derive(Debug, Clone)]
pub struct TavilyHit {
    pub url: String,
    pub title: String,
    pub content: String,
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credit_cost_follows_depth() {
        assert_eq!(search_credit_cost("basic"), 1);
        assert_eq!(search_credit_cost("advanced"), 2);
        assert_eq!(search_credit_cost("fast"), 1);
    }

    #[test]
    fn truncate_chars_caps_on_a_char_boundary() {
        assert_eq!(truncate_chars("abc", 5), "abc");
        assert_eq!(truncate_chars("ééééé", 2), "éé…");
        assert_eq!(truncate_chars(&"x".repeat(1000), 300).chars().count(), 301);
    }

    #[test]
    fn quota_errors_are_429_432_433_only() {
        assert!(is_quota_error("tavily_search_status:429:Too many requests"));
        assert!(is_quota_error("tavily_search_status:432:plan limit"));
        assert!(!is_quota_error("tavily_search_status:401:bad key"));
    }

    #[cfg(feature = "tavily")]
    #[test]
    fn blank_key_builds_no_client_and_base_url_is_normalised() {
        assert!(TavilyClient::new("  ", None).is_none());
        let c = TavilyClient::new("tvly-x", Some("http://h:1/")).unwrap();
        assert_eq!(c.base_url, "http://h:1");
        let d = TavilyClient::new("tvly-x", None).unwrap();
        assert_eq!(d.base_url, DEFAULT_TAVILY_BASE_URL);
    }
}
