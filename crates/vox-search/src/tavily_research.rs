//! Tavily `/research` deep-research tier (optional; gated by `VOX_TAVILY_RESEARCH`).

use serde::{Deserialize, Serialize};
use tracing::{error, info};
use vox_secrets::{SecretId, resolve_secret};

const DEFAULT_RESEARCH_BASE: &str = "https://api.tavily.com";

/// Credits charged to the session budget before each `/research` call. Tavily prices
/// research dynamically (mini 4–110, pro 15–250 credits); this charges the pro minimum
/// so the budget can never be bypassed by the research tier.
pub const RESEARCH_CREDIT_COST: usize = 15;

#[derive(Debug, Clone, Serialize)]
struct ResearchRequest {
    api_key: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ResearchSource {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    score: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
struct ResearchResponse {
    /// `pending` / `in_progress` from the async API: results only come from polling.
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    answer: Option<String>,
    #[serde(default)]
    sources: Vec<ResearchSource>,
    #[serde(default)]
    response_time: Option<f64>,
}

/// Evaluates whether Tavily research is enabled given optional API key and override strings.
#[must_use]
pub fn tavily_research_enabled_with_values(
    api_key: Option<&str>,
    explicit_override: Option<&str>,
) -> bool {
    if let Some(v) = explicit_override {
        let v = v.trim();
        if matches!(v, "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON") {
            return true;
        }
        if matches!(v, "0" | "false" | "FALSE" | "no" | "NO" | "off" | "OFF") {
            return false;
        }
    }
    api_key.map(|k| !k.trim().is_empty()).unwrap_or(false)
}

/// Returns true only when `VOX_TAVILY_RESEARCH` is explicitly truthy.
///
/// Task 15 ruling: no longer auto-enabled by the mere presence of `TAVILY_API_KEY`.
/// Tavily's `/research` is asynchronous (POST returns `request_id` + `status: pending`,
/// results come from polling), but [`TavilyResearchClient::research`] parses the POST
/// response as if it held `sources` — so it spends research credits and always yields
/// nothing. Opt in only after polling is implemented.
#[must_use]
pub fn tavily_research_enabled() -> bool {
    let override_secret = resolve_secret(SecretId::VoxTavilyResearch);
    tavily_research_enabled_with_values(None, override_secret.expose())
}

pub struct TavilyResearchClient {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl TavilyResearchClient {
    /// `None` unless `VOX_TAVILY_RESEARCH` is truthy and a key exists. `base_url` is
    /// `SearchPolicy::tavily_api_url` (default: Tavily's public API).
    pub fn from_env(base_url: Option<&str>) -> Option<Self> {
        if !tavily_research_enabled() {
            return None;
        }
        let api_key = resolve_secret(SecretId::TavilyApiKey).expose()?.to_string();
        Some(Self {
            http: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_30S)
                .build()
                .ok()?,
            api_key,
            base_url: base_url.unwrap_or(DEFAULT_RESEARCH_BASE).to_string(),
        })
    }

    pub fn with_base_url(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            http: vox_http_client::client_builder()
                .timeout(vox_config::timeouts::D_30S)
                .build()
                .expect("reqwest client"),
            api_key: api_key.into(),
            base_url: base_url.into(),
        }
    }

    pub async fn research(
        &self,
        query: &str,
        instructions: Option<&str>,
    ) -> Result<Vec<crate::searxng::SearxngResult>, String> {
        let body = ResearchRequest {
            api_key: self.api_key.clone(),
            query: query.to_string(),
            instructions: instructions.map(str::to_string),
        };
        let url = format!("{}/research", self.base_url.trim_end_matches('/'));
        tracing::debug!(query, "tavily_research_request");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("tavily_research_http:{e}"))?;
        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| format!("tavily_research_body:{e}"))?;
        if !status.is_success() {
            return Err(format!(
                "tavily_research_status:{status}:{}",
                crate::tavily::truncate_chars(&text, crate::tavily::ERROR_DETAIL_MAX_CHARS)
            ));
        }
        let parsed: ResearchResponse =
            serde_json::from_str(&text).map_err(|e| format!("tavily_research_parse:{e}"))?;
        if parsed.sources.is_empty()
            && parsed.answer.is_none()
            && let Some(state) = parsed.status.as_deref()
        {
            // The credits are already spent; say so instead of returning an empty Ok.
            return Err(format!(
                "Tavily /research returned {state}; polling not implemented"
            ));
        }
        info!(
            source_count = parsed.sources.len(),
            response_time = ?parsed.response_time,
            "tavily research succeeded"
        );
        let mut out: Vec<crate::searxng::SearxngResult> = parsed
            .sources
            .into_iter()
            .map(|s| crate::searxng::SearxngResult {
                url: s.url,
                title: s.title,
                content: if s.content.is_empty() {
                    parsed.answer.clone().unwrap_or_default()
                } else {
                    s.content
                },
                engine: Some("tavily_research".to_string()),
                score: s.score.map(f64::from),
            })
            .collect();
        if out.is_empty()
            && let Some(answer) = parsed.answer.filter(|a| !a.trim().is_empty())
        {
            out.push(crate::searxng::SearxngResult {
                url: format!("tavily-research://{query}"),
                title: format!("Tavily research: {query}"),
                content: answer,
                engine: Some("tavily_research".to_string()),
                score: Some(0.85),
            });
        }
        Ok(out)
    }
}

impl TavilyResearchClient {
    /// [`Self::research`] charged to `budget` ([`RESEARCH_CREDIT_COST`]); refuses with an
    /// error, without a request, once the budget cannot cover it.
    pub async fn research_within_budget(
        &self,
        query: &str,
        budget: &crate::tavily_budget::TavilySessionBudget,
    ) -> Result<Vec<crate::searxng::SearxngResult>, String> {
        if !budget.try_consume(RESEARCH_CREDIT_COST) {
            return Err("tavily_research_budget_exhausted".to_string());
        }
        self.research(query, None).await
    }
}

/// Optional research-tier fetch (opt-in via `VOX_TAVILY_RESEARCH=1`). Budgeted; every
/// failure — including the unimplemented async `pending` answer — is logged at error
/// level, and the caller gets no rows.
pub async fn try_tavily_research_hits(
    query: &str,
    policy: &crate::policy::SearchPolicy,
) -> Vec<crate::searxng::SearxngResult> {
    let Some(client) = TavilyResearchClient::from_env(policy.tavily_api_url.as_deref()) else {
        return Vec::new();
    };
    let budget =
        crate::tavily_budget::TavilySessionBudget::global(policy.tavily_credit_budget_per_session);
    match client.research_within_budget(query, budget).await {
        Ok(hits) => hits,
        Err(e) => {
            error!(error = %e, "tavily research tier failed");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pending_async_answer_is_an_error_and_the_budget_is_charged() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/research"))
            .and(header("authorization", "Bearer tvly-test"))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "request_id": "r-1", "created_at": "2026-09-23T00:00:00Z",
                "status": "pending", "input": "q", "model": "mini", "response_time": 0.1
            })))
            .expect(1)
            .mount(&mock)
            .await;
        let client = TavilyResearchClient::with_base_url("tvly-test", mock.uri());
        let budget = crate::tavily_budget::TavilySessionBudget::new(RESEARCH_CREDIT_COST + 1);

        let err = client
            .research_within_budget("q", &budget)
            .await
            .expect_err("pending must not be an empty Ok");
        assert!(err.contains("pending; polling not implemented"), "{err}");
        assert_eq!(budget.remaining(), 1);

        // Budget can no longer cover a call: refused before any request (`.expect(1)`).
        let err = client
            .research_within_budget("q", &budget)
            .await
            .unwrap_err();
        assert_eq!(err, "tavily_research_budget_exhausted");
    }

    #[test]
    fn research_gate_is_boolean() {
        let _ = tavily_research_enabled();
    }

    #[test]
    fn test_tavily_research_enabled_with_values() {
        // Truthy overrides
        assert!(tavily_research_enabled_with_values(None, Some("1")));
        assert!(tavily_research_enabled_with_values(None, Some("true")));
        assert!(tavily_research_enabled_with_values(None, Some("TRUE")));
        assert!(tavily_research_enabled_with_values(None, Some("yes")));
        assert!(tavily_research_enabled_with_values(None, Some("on")));

        // Falsy overrides override even a valid key
        assert!(!tavily_research_enabled_with_values(
            Some("tvly-xxx"),
            Some("0")
        ));
        assert!(!tavily_research_enabled_with_values(
            Some("tvly-xxx"),
            Some("false")
        ));
        assert!(!tavily_research_enabled_with_values(
            Some("tvly-xxx"),
            Some("FALSE")
        ));
        assert!(!tavily_research_enabled_with_values(
            Some("tvly-xxx"),
            Some("no")
        ));
        assert!(!tavily_research_enabled_with_values(
            Some("tvly-xxx"),
            Some("off")
        ));

        // Unset override relies on key presence
        assert!(tavily_research_enabled_with_values(Some("tvly-xxx"), None));
        assert!(!tavily_research_enabled_with_values(None, None));
        assert!(!tavily_research_enabled_with_values(Some(""), None));
        assert!(!tavily_research_enabled_with_values(Some("   "), None));
    }
}
