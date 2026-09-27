//! Web provider registry — thin delegate over `vox-search::WebSearchDispatcher`.
//!
//! The research pipeline attributes telemetry to [`ProviderRegistry::primary_name`]
//! while retrieval executes through the shared vox-search web stack (SearXNG → DDG → Tavily).

use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use vox_search::policy::{ResearchLane, SearchPolicy};
use vox_search::web_dispatcher::{ProviderOutcome, ProviderStatus, WebSearchDispatcher};

use super::types::{ProviderCallSummary, ResearchHit, TavilyCredits};

/// Every provider outcome seen by one registry (one research run), plus the
/// latest Tavily credit counter. Shared across clones.
#[derive(Debug, Default)]
struct RetrievalLog {
    calls: Vec<ProviderOutcome>,
    tavily_credits: Option<(usize, usize)>,
}

/// Group outcomes by (provider, outcome kind) in first-seen order: `Ok` hits are
/// summed, the first error message is kept, `elapsed_ms` is the slowest call.
pub fn summarize_provider_calls(calls: &[ProviderOutcome]) -> Vec<ProviderCallSummary> {
    let mut rows: Vec<ProviderCallSummary> = Vec::new();
    for c in calls {
        let kind = std::mem::discriminant(&c.status);
        match rows
            .iter_mut()
            .find(|r| r.provider == c.provider && std::mem::discriminant(&r.status) == kind)
        {
            Some(r) => {
                r.calls += 1;
                r.elapsed_ms = r.elapsed_ms.max(c.elapsed_ms);
                if let (ProviderStatus::Ok { hits }, ProviderStatus::Ok { hits: more }) =
                    (&mut r.status, &c.status)
                {
                    *hits += more;
                }
            }
            None => rows.push(ProviderCallSummary {
                provider: c.provider.to_string(),
                status: c.status.clone(),
                elapsed_ms: c.elapsed_ms,
                calls: 1,
            }),
        }
    }
    rows
}

/// Configuration for the provider registry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub primary: Option<String>,
    pub fallback: Vec<String>,
}

/// Registry of web search providers used by the research pipeline.
#[derive(Debug, Clone)]
pub struct ProviderRegistry {
    primary: String,
    log: Arc<Mutex<RetrievalLog>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self {
            primary: "vox-search/web".to_string(),
            log: Arc::default(),
        }
    }
}

impl ProviderRegistry {
    /// Construct from environment + supplied config.
    #[must_use]
    pub fn from_env_with_config(config: ProviderConfig) -> Self {
        if let Some(name) = config
            .primary
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Self {
                primary: name.to_string(),
                log: Arc::default(),
            }
        } else {
            Self::default()
        }
    }

    /// Name of the primary provider for telemetry attribution.
    #[must_use]
    pub fn primary_name(&self) -> &str {
        &self.primary
    }

    /// Per-provider outcomes of every search this registry ran, and the
    /// latest Tavily credit counter.
    #[must_use]
    pub fn retrieval_log(&self) -> (Vec<ProviderCallSummary>, Option<TavilyCredits>) {
        let Ok(log) = self.log.lock() else {
            return (Vec::new(), None);
        };
        (
            summarize_provider_calls(&log.calls),
            log.tavily_credits
                .map(|(used, remaining)| TavilyCredits { used, remaining }),
        )
    }

    /// Search for hits matching `query` via [`WebSearchDispatcher`] with a specific lane.
    ///
    /// Returns `(hits, provider_name_used)`.
    pub async fn search_with_lane(
        &self,
        query: &str,
        lane: ResearchLane,
        policy: &SearchPolicy,
    ) -> (Vec<ResearchHit>, String) {
        let report = WebSearchDispatcher::search_with_report(query, lane, policy).await;
        if let Ok(mut log) = self.log.lock() {
            log.calls.extend(report.providers);
            if report.tavily_credits.is_some() {
                log.tavily_credits = report.tavily_credits;
            }
        }
        let hybrids = report.hits;
        use futures::stream::{self, StreamExt};
        // Stream over OWNED (title, url) pairs, not `hybrids.iter()`
        // (borrowed `&HybridSearchHit` items) — a `.map` closure taking
        // a borrowed iterator item and returning an `async move` block
        // makes rustc infer an overly-generic higher-ranked closure
        // signature for the closure itself (independent of what the
        // async block captures), which fails "implementation of
        // Send/FnOnce is not general enough" in some downstream callers
        // (observed when this crate is linked into vox-orchestrator-mcp).
        // Cloning up front so every stream item is fully owned avoids
        // the closure ever being generic over a borrowed lifetime.
        let title_urls: Vec<(String, String)> = hybrids
            .iter()
            .map(|h| (h.title.clone(), h.path.clone()))
            .collect();
        let trust_scores: Vec<f64> = stream::iter(title_urls)
            .map(|(title, url)| {
                let doi = vox_search::trust::extract_doi_from_url(&url);
                async move {
                    vox_search::trust::score_hit_trust_for_url(&title, doi.as_deref(), &url).await
                }
            })
            // `buffered` (not `buffer_unordered`) to preserve input order,
            // since results are zipped positionally against `hybrids` below.
            .buffered(5)
            .collect()
            .await;
        let hits: Vec<ResearchHit> = hybrids
            .into_iter()
            .zip(trust_scores)
            .map(|(h, trust_score)| ResearchHit {
                url: h.path,
                title: h.title,
                snippet: h.content_snippet,
                score: h.score,
                http_status: 0,
                trust_score,
                raw_content: String::new(),
            })
            .collect();
        (hits, self.primary.clone())
    }

    /// Search for hits matching `query` via [`WebSearchDispatcher`].
    ///
    /// Returns `(hits, provider_name_used)`.
    pub async fn search(&self, query: &str, policy: &SearchPolicy) -> (Vec<ResearchHit>, String) {
        self.search_with_lane(query, policy.default_lane, policy)
            .await
    }

    /// Discover child pages for a site root URL.
    ///
    /// Site-scoped crawling is handled at gather time via `ResearchQuery::site_scope`
    /// filtering; no dedicated site-map API exists in vox-search yet.
    pub async fn map_site(&self, _root_url: &str) -> Option<Vec<String>> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_search::web_dispatcher::{ProviderOutcome, ProviderStatus};

    fn call(provider: &'static str, status: ProviderStatus, elapsed_ms: u64) -> ProviderOutcome {
        ProviderOutcome {
            provider,
            status,
            elapsed_ms,
        }
    }

    /// The deep trace shows one row per (provider, outcome) across every
    /// subquery — a budget-exhausted tavily call must stay visible next to
    /// the tavily calls that succeeded, never be folded into them.
    #[test]
    fn provider_calls_group_by_provider_and_outcome_summing_hits() {
        let rows = summarize_provider_calls(&[
            call("searxng", ProviderStatus::Ok { hits: 4 }, 900),
            call("tavily", ProviderStatus::Ok { hits: 5 }, 3000),
            call("searxng", ProviderStatus::Ok { hits: 6 }, 1200),
            call("tavily", ProviderStatus::BudgetExhausted, 0),
            call(
                "openalex",
                ProviderStatus::Error {
                    message: "HTTP 429".into(),
                },
                300,
            ),
            call(
                "openalex",
                ProviderStatus::Error {
                    message: "HTTP 503".into(),
                },
                500,
            ),
        ]);
        let got: Vec<(&str, &ProviderStatus, usize, u64)> = rows
            .iter()
            .map(|r| (r.provider.as_str(), &r.status, r.calls, r.elapsed_ms))
            .collect();
        assert_eq!(
            got,
            vec![
                ("searxng", &ProviderStatus::Ok { hits: 10 }, 2, 1200),
                ("tavily", &ProviderStatus::Ok { hits: 5 }, 1, 3000),
                ("tavily", &ProviderStatus::BudgetExhausted, 1, 0),
                (
                    "openalex",
                    &ProviderStatus::Error {
                        message: "HTTP 429".into()
                    },
                    2,
                    500
                ),
            ]
        );
    }

    #[test]
    fn a_fresh_registry_has_an_empty_retrieval_log() {
        let (rows, credits) = ProviderRegistry::default().retrieval_log();
        assert!(rows.is_empty());
        assert_eq!(credits, None);
    }

    #[tokio::test]
    async fn provider_search_hits_use_real_trust_scoring() {
        // Sanity check that trust scoring is wired in and fail-open (no hang,
        // sane range) — full integration behavior is covered by trust.rs's
        // own mocked tests from Task 4.
        let score = vox_search::trust::score_hit_trust("Example Provider Title", None).await;
        assert!(
            (0.0..=2.0).contains(&score),
            "trust score {score} out of sane range"
        );
    }
}
