use std::collections::{HashMap, HashSet};

use tracing::{info, warn};

use crate::policy::{ResearchLane, SearchPolicy};

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ProviderStatus {
    Ok { hits: usize },
    Timeout,
    Error { message: String },
    NotConfigured,
    Disabled,
    CircuitOpen,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderOutcome {
    pub provider: &'static str,
    pub status: ProviderStatus,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SearchReport {
    pub hits: Vec<crate::memory_hybrid::HybridSearchHit>,
    pub providers: Vec<ProviderOutcome>,
}

enum ProviderRun {
    Hits(Vec<crate::searxng::SearxngResult>),
    Failed(String),
    Skipped(ProviderStatus),
}

async fn timed(
    provider: &'static str,
    deadline: std::time::Duration,
    run: impl std::future::Future<Output = ProviderRun>,
) -> (Vec<crate::searxng::SearxngResult>, ProviderOutcome) {
    let started = std::time::Instant::now();
    let (hits, status) = match tokio::time::timeout(deadline, run).await {
        Ok(ProviderRun::Hits(h)) => {
            let n = h.len();
            (h, ProviderStatus::Ok { hits: n })
        }
        Ok(ProviderRun::Failed(message)) => (Vec::new(), ProviderStatus::Error { message }),
        Ok(ProviderRun::Skipped(s)) => (Vec::new(), s),
        Err(_) => (Vec::new(), ProviderStatus::Timeout),
    };
    (
        hits,
        ProviderOutcome {
            provider,
            status,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    )
}

pub struct WebSearchDispatcher;

impl Default for WebSearchDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn extract_registrable_domain(url_str: &str) -> Option<String> {
    let parsed = url::Url::parse(url_str)
        .or_else(|_| url::Url::parse(&format!("https://{url_str}")))
        .ok()?;
    let host = parsed.host_str()?;
    Some(host.trim_start_matches("www.").to_ascii_lowercase())
}

impl WebSearchDispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn filter_and_penalize_results(
        results: &mut Vec<crate::searxng::SearxngResult>,
        policy: &SearchPolicy,
    ) {
        results.retain(|r| {
            if let Some(domain) = extract_registrable_domain(&r.url) {
                !policy.blacklisted_domains.contains(&domain)
            } else {
                true
            }
        });

        for r in results.iter_mut() {
            let domain = extract_registrable_domain(&r.url);
            let penalty = domain
                .and_then(|d| policy.domain_penalties.get(&d))
                .copied()
                .unwrap_or(0.0);
            let base = r.score.unwrap_or(0.5);
            let multiplier = (1.0 - penalty).clamp(0.05, 1.0);
            r.score = Some(base * multiplier);
        }
    }

    /// Caps how many of the top `limit` kept hits any single provider (the
    /// `engine:<name>` provenance entry, e.g. "arxiv", "searxng") may occupy,
    /// while guaranteeing every provider that contributed at least one hit
    /// keeps at least one slot (as long as `limit >= contributing_providers`).
    ///
    /// D9 follow-up (Task 8 fix round 2/3): `true_rrf_fuse`'s per-source
    /// authority weights (arXiv 1.20 vs 1.00 for everything else) mean that at
    /// `rrf_k=60`, arXiv's *worst*-ranked hit still outscores an equal-size
    /// competing provider's *best*-ranked hit — so arXiv can structurally
    /// occupy every kept slot regardless of relevance, even when other
    /// providers returned just as many real hits.
    ///
    /// Task 8b fix: the original greedy single-pass (admit in score order
    /// until either a provider's cap or the overall `limit` is hit) could
    /// exhaust `limit` using only two of three contributing providers before
    /// the iteration ever reached the third, dropping it entirely even though
    /// its cap was never reached. This version first reserves one slot per
    /// provider (highest-scoring provider first), then fills the remaining
    /// slots by score across providers still under `cap`, and only relaxes
    /// the cap if providers run out of hits before `limit` is filled. Nothing
    /// is dropped — this is a pure reorder, so `results.len()` is unchanged
    /// and callers that later truncate to `limit` see a diversified head
    /// instead of a monoculture. A no-op when fewer than two providers
    /// contributed, or when `results.len() <= limit` (nothing would be
    /// truncated anyway).
    fn enforce_provider_diversity(results: &mut Vec<crate::searxng::SearxngResult>, limit: usize) {
        if limit == 0 || results.len() <= limit {
            return;
        }
        let contributing: HashSet<&str> = results
            .iter()
            .map(|r| r.engine.as_deref().unwrap_or("unknown"))
            .collect();
        if contributing.len() < 2 {
            return;
        }
        let cap = limit.div_ceil(contributing.len()).max(2);

        // Group items by provider, preserving each provider's relative
        // (already score-sorted) order.
        let old = std::mem::take(results);
        let mut group_order: Vec<String> = Vec::new();
        let mut groups: HashMap<String, Vec<crate::searxng::SearxngResult>> = HashMap::new();
        for item in old {
            let engine = item.engine.clone().unwrap_or_else(|| "unknown".to_string());
            if !groups.contains_key(&engine) {
                group_order.push(engine.clone());
            }
            groups.entry(engine).or_default().push(item);
        }
        // Providers ordered by their own best (first) hit's score, descending.
        group_order.sort_by(|a, b| {
            let sa = groups[a].first().and_then(|r| r.score).unwrap_or(0.0);
            let sb = groups[b].first().and_then(|r| r.score).unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut heads: HashMap<String, usize> = HashMap::new();
        let mut counts: HashMap<String, usize> = HashMap::new();
        let mut selected: Vec<crate::searxng::SearxngResult> = Vec::with_capacity(limit);

        // Pass 0: guarantee one slot per provider, strongest provider first.
        for engine in &group_order {
            if selected.len() >= limit {
                break;
            }
            if let Some(item) = groups[engine].first() {
                selected.push(item.clone());
                heads.insert(engine.clone(), 1);
                counts.insert(engine.clone(), 1);
            }
        }

        // Pass 1: fill remaining slots by best available score across
        // providers still under `cap`.
        loop {
            if selected.len() >= limit {
                break;
            }
            let mut best: Option<(&String, f64)> = None;
            for engine in &group_order {
                let head = *heads.get(engine).unwrap_or(&0);
                let count = *counts.get(engine).unwrap_or(&0);
                if count >= cap {
                    continue;
                }
                if let Some(item) = groups[engine].get(head) {
                    let score = item.score.unwrap_or(0.0);
                    if best.is_none_or(|(_, s)| score > s) {
                        best = Some((engine, score));
                    }
                }
            }
            match best {
                Some((engine, _)) => {
                    let engine = engine.clone();
                    let head = *heads.get(&engine).unwrap_or(&0);
                    selected.push(groups[&engine][head].clone());
                    *heads.entry(engine.clone()).or_insert(0) += 1;
                    *counts.entry(engine).or_insert(0) += 1;
                }
                None => break,
            }
        }

        // Pass 2: providers under cap ran out of hits before `limit` was
        // filled — relax the cap and take whatever is left, by score.
        loop {
            if selected.len() >= limit {
                break;
            }
            let mut best: Option<(&String, f64)> = None;
            for engine in &group_order {
                let head = *heads.get(engine).unwrap_or(&0);
                if let Some(item) = groups[engine].get(head) {
                    let score = item.score.unwrap_or(0.0);
                    if best.is_none_or(|(_, s)| score > s) {
                        best = Some((engine, score));
                    }
                }
            }
            match best {
                Some((engine, _)) => {
                    let engine = engine.clone();
                    let head = *heads.get(&engine).unwrap_or(&0);
                    selected.push(groups[&engine][head].clone());
                    *heads.entry(engine.clone()).or_insert(0) += 1;
                }
                None => break,
            }
        }

        // Anything left over (beyond `limit`) is appended, best score first,
        // so `results.len()` stays unchanged for callers that truncate later.
        let mut leftover: Vec<crate::searxng::SearxngResult> = Vec::new();
        for engine in &group_order {
            let head = *heads.get(engine).unwrap_or(&0);
            if let Some(items) = groups.get(engine) {
                leftover.extend(items[head..].iter().cloned());
            }
        }
        leftover.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        selected.extend(leftover);
        *results = selected;
    }

    /// Reranks the fused, penalty-adjusted candidate pool by query-term
    /// density over `title + content` (via `term_density_reranker`), rather
    /// than trusting per-source RRF authority weight as the final relevance
    /// signal. Task 8b: SearXNG returning the page that actually answers the
    /// query (e.g. an OpenRouter pricing page) was still losing the final cap
    /// to an irrelevant, high-authority arXiv hit because nothing scored
    /// query relevance before the cap ran. Reuses the existing reranker
    /// rather than duplicating a scorer; combines the fusion/penalty score
    /// (30%) with term-density (70%) as the reranker already does, and writes
    /// the combined score back onto each result so the provider cap (and any
    /// later truncation) operates on relevance order. `top_k` is the full
    /// pool size, so nothing is dropped here — only reordered. Ties keep
    /// fusion order, since the reranker's sort is stable over the incoming
    /// (fusion-ordered) vec.
    fn rerank_by_relevance(
        query: &str,
        results: Vec<crate::searxng::SearxngResult>,
    ) -> Vec<crate::searxng::SearxngResult> {
        let passages: Vec<crate::term_density_reranker::CandidatePassage> = results
            .iter()
            .enumerate()
            .map(|(i, r)| crate::term_density_reranker::CandidatePassage {
                id: i.to_string(),
                text: format!("{} {}", r.title, r.content),
                base_score: r.score.unwrap_or(0.5),
            })
            .collect();
        let top_k = passages.len();
        let reranked = crate::term_density_reranker::rerank_passages(query, &passages, top_k);

        reranked
            .into_iter()
            .filter_map(|p| {
                let idx: usize = p.id.parse().ok()?;
                let mut r = results.get(idx)?.clone();
                r.score = Some(p.base_score);
                Some(r)
            })
            .collect()
    }

    pub async fn search(
        query: &str,
        policy: &SearchPolicy,
    ) -> anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>> {
        Self::search_with_registry(
            query,
            policy,
            crate::search_circuit_breaker::SearchProviderCircuitRegistry::global(),
        )
        .await
    }

    pub async fn search_with_registry(
        query: &str,
        policy: &SearchPolicy,
        registry: &crate::search_circuit_breaker::SearchProviderCircuitRegistry,
    ) -> anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>> {
        Self::search_with_lane_and_registry(query, policy.default_lane, policy, registry).await
    }

    pub async fn search_with_lane(
        query: &str,
        lane: ResearchLane,
        policy: &SearchPolicy,
    ) -> anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>> {
        Self::search_with_lane_and_registry(
            query,
            lane,
            policy,
            crate::search_circuit_breaker::SearchProviderCircuitRegistry::global(),
        )
        .await
    }

    pub async fn search_with_report(
        query: &str,
        lane: ResearchLane,
        policy: &SearchPolicy,
    ) -> SearchReport {
        Self::search_with_report_and_registry(
            query,
            lane,
            policy,
            crate::search_circuit_breaker::SearchProviderCircuitRegistry::global(),
        )
        .await
    }

    pub async fn search_with_lane_and_registry(
        query: &str,
        lane: ResearchLane,
        policy: &SearchPolicy,
        registry: &crate::search_circuit_breaker::SearchProviderCircuitRegistry,
    ) -> anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>> {
        Ok(
            Self::search_with_report_and_registry(query, lane, policy, registry)
                .await
                .hits,
        )
    }

    pub async fn search_with_report_and_registry(
        query: &str,
        lane: ResearchLane,
        policy: &SearchPolicy,
        registry: &crate::search_circuit_breaker::SearchProviderCircuitRegistry,
    ) -> SearchReport {
        if query.trim().is_empty() {
            return SearchReport::default();
        }

        let timeout_ms = match lane {
            ResearchLane::Fast => policy.fast_timeout_ms,
            ResearchLane::Deep => policy.deep_timeout_ms,
        };
        let deadline = std::time::Duration::from_millis(timeout_ms.max(50));

        // 1. Wikipedia
        let wiki_task = async {
            if policy.enable_wikipedia && policy.wikipedia_fallback_enabled {
                match crate::wikipedia::WikipediaClient::search(
                    query,
                    policy.searxng_max_results,
                    policy.wikipedia_api_url.as_deref(),
                )
                .await
                {
                    Ok(hits) => {
                        info!(count = hits.len(), "Wikipedia search succeeded");
                        ProviderRun::Hits(hits)
                    }
                    Err(e) => {
                        warn!(error = %e, "Wikipedia search failed");
                        ProviderRun::Failed(e.to_string())
                    }
                }
            } else {
                ProviderRun::Skipped(ProviderStatus::Disabled)
            }
        };

        // 2. OpenAlex
        let openalex_task = async {
            if policy.enable_openalex {
                match crate::openalex::OpenAlexClient::search(
                    query,
                    policy.searxng_max_results,
                    policy.openalex_api_url.as_deref(),
                    None,
                )
                .await
                {
                    Ok(hits) => {
                        info!(count = hits.len(), "OpenAlex search succeeded");
                        ProviderRun::Hits(hits)
                    }
                    Err(e) => {
                        warn!(error = %e, "OpenAlex search failed");
                        ProviderRun::Failed(e.to_string())
                    }
                }
            } else {
                ProviderRun::Skipped(ProviderStatus::Disabled)
            }
        };

        // 3. arXiv
        let arxiv_task = async {
            if policy.enable_arxiv {
                let _permit = crate::safety_governor::ProviderSafetyGovernor::global()
                    .acquire_arxiv()
                    .await;
                match crate::arxiv::ArXivClient::search(
                    query,
                    policy.searxng_max_results,
                    policy.arxiv_api_url.as_deref(),
                )
                .await
                {
                    Ok(hits) => {
                        info!(count = hits.len(), "arXiv search succeeded");
                        ProviderRun::Hits(hits)
                    }
                    Err(e) => {
                        warn!(error = %e, "arXiv search failed");
                        ProviderRun::Failed(e.to_string())
                    }
                }
            } else {
                ProviderRun::Skipped(ProviderStatus::Disabled)
            }
        };

        // 4. SearXNG
        let searxng_task = async {
            if let Some(base_url) = &policy.searxng_url {
                if !registry.is_available(crate::search_circuit_breaker::SearchProviderId::Searxng)
                {
                    warn!("SearXNG is in circuit breaker cooldown, skipping");
                    ProviderRun::Skipped(ProviderStatus::CircuitOpen)
                } else {
                    let client = crate::searxng::SearxngSearchClient::new(base_url.clone());
                    let _permit = crate::safety_governor::ProviderSafetyGovernor::global()
                        .acquire_searxng()
                        .await;
                    match client
                        .search(
                            query,
                            policy.searxng_max_results,
                            policy.searxng_engines_csv(),
                            policy.searxng_language_tag(),
                        )
                        .await
                    {
                        Ok(hits) => {
                            info!(count = hits.len(), "SearXNG search succeeded");
                            registry.record_success(
                                crate::search_circuit_breaker::SearchProviderId::Searxng,
                            );
                            ProviderRun::Hits(hits)
                        }
                        Err(e) => {
                            let err_str = e.to_string();
                            let is_rate_limit = err_str.contains("429")
                                || err_str.to_ascii_lowercase().contains("rate limit");
                            registry.record_failure(
                                crate::search_circuit_breaker::SearchProviderId::Searxng,
                                is_rate_limit,
                            );
                            warn!(error = %e, is_rate_limit, "SearXNG search failed");
                            ProviderRun::Failed(err_str)
                        }
                    }
                }
            } else {
                ProviderRun::Skipped(ProviderStatus::NotConfigured)
            }
        };

        // 5. Tavily
        #[cfg(feature = "tavily")]
        let tavily_client = if policy.tavily_enabled
            && registry.is_available(crate::search_circuit_breaker::SearchProviderId::Tavily)
        {
            tokio::task::spawn_blocking(crate::tavily::TavilySearchClient::from_env)
                .await
                .ok()
                .flatten()
        } else {
            None
        };

        let tavily_task = async {
            #[cfg(feature = "tavily")]
            if let Some(client) = &tavily_client {
                let _permit = crate::safety_governor::ProviderSafetyGovernor::global()
                    .acquire_tavily()
                    .await;
                match client
                    .search(
                        query,
                        policy.tavily_max_results,
                        policy.tavily_search_depth.as_str(),
                    )
                    .await
                {
                    Ok(hits) => {
                        info!(count = hits.len(), "Tavily web search succeeded");
                        registry.record_success(
                            crate::search_circuit_breaker::SearchProviderId::Tavily,
                        );
                        return ProviderRun::Hits(
                            hits.into_iter()
                                .map(|h| crate::searxng::SearxngResult {
                                    url: h.url,
                                    title: h.title.clone(),
                                    content: h.content,
                                    engine: Some("tavily".to_string()),
                                    score: Some(f64::from(h.score)),
                                })
                                .collect(),
                        );
                    }
                    Err(e) => {
                        let is_rate_limit =
                            e.contains("429") || e.to_ascii_lowercase().contains("rate limit");
                        registry.record_failure(
                            crate::search_circuit_breaker::SearchProviderId::Tavily,
                            is_rate_limit,
                        );
                        warn!(error = %e, is_rate_limit, "Tavily web search failed");
                        return ProviderRun::Failed(e);
                    }
                }
            }
            #[cfg(not(feature = "tavily"))]
            {
                return ProviderRun::Skipped(ProviderStatus::NotConfigured);
            }
            #[cfg(feature = "tavily")]
            ProviderRun::Skipped(ProviderStatus::NotConfigured)
        };

        // Bound each provider by lane timeout deadline
        let (wiki, openalex, arxiv, searxng, tavily) = tokio::join!(
            timed("wikipedia", deadline, wiki_task),
            timed("openalex", deadline, openalex_task),
            timed("arxiv", deadline, arxiv_task),
            timed("searxng", deadline, searxng_task),
            timed("tavily", deadline, tavily_task),
        );
        for (outcome, id) in [
            (
                &searxng.1,
                crate::search_circuit_breaker::SearchProviderId::Searxng,
            ),
            (
                &tavily.1,
                crate::search_circuit_breaker::SearchProviderId::Tavily,
            ),
        ] {
            // A lane-deadline timeout is the caller's budget running out, not evidence the
            // provider is unhealthy — only a Deep-lane (generous budget) timeout indicates a
            // genuinely slow/unhealthy provider worth arming the breaker for.
            if outcome.status == ProviderStatus::Timeout && lane == ResearchLane::Deep {
                registry.record_failure(id, false);
            }
        }
        let providers = vec![
            arxiv.1.clone(),
            openalex.1.clone(),
            wiki.1.clone(),
            searxng.1.clone(),
            tavily.1.clone(),
        ];
        let provider_lists = vec![arxiv.0, openalex.0, wiki.0, searxng.0, tavily.0];

        let mut results = true_rrf_fuse(provider_lists, policy.rrf_k);

        if results.is_empty() {
            return SearchReport {
                hits: Vec::new(),
                providers,
            };
        }

        Self::filter_and_penalize_results(&mut results, policy);
        if results.is_empty() {
            return SearchReport {
                hits: Vec::new(),
                providers,
            };
        }

        results = Self::rerank_by_relevance(query, results);

        let kept_limit = policy
            .searxng_max_results
            .max(policy.searxng_max_urls_to_scrape);
        Self::enforce_provider_diversity(&mut results, kept_limit);

        #[cfg(feature = "tavily")]
        if policy.tavily_enabled
            && registry.is_available(crate::search_circuit_breaker::SearchProviderId::Tavily)
        {
            crate::tavily_extract::uplift_low_quality_snippets(
                &mut results,
                query,
                policy.searxng_max_urls_to_scrape,
            )
            .await;
        }

        #[cfg(feature = "web-scrape")]
        {
            // Integrated scraping for clean content (optional — pulls scraper/html2text).
            let mut final_hits = Vec::new();
            let urls_to_scrape = results
                .iter()
                .take(
                    policy
                        .searxng_max_results
                        .max(policy.searxng_max_urls_to_scrape),
                )
                .cloned()
                .collect::<Vec<_>>();

            for res in urls_to_scrape {
                match crate::scraper::fetch_and_extract(&res.url, policy.scraper_timeout_ms).await {
                    Ok(doc) => {
                        if doc.text_density >= policy.scraper_min_text_density {
                            let mut provenance = vec!["WebResearch".to_string()];
                            if let Some(ref eng) = res.engine {
                                provenance.push(format!("engine:{eng}"));
                            }
                            provenance.push("scraped:true".to_string());
                            final_hits.push(crate::memory_hybrid::HybridSearchHit {
                                path: doc.url,
                                title: doc.title.clone(),
                                content_snippet: doc.markdown.clone(),
                                score: res.score.unwrap_or(1.0),
                                provenance,
                                potential_contradiction: false,
                            });
                        } else {
                            let mut provenance = vec!["WebResearch".to_string()];
                            if let Some(ref eng) = res.engine {
                                provenance.push(format!("engine:{eng}"));
                            }
                            provenance.push("scraped:false".to_string());
                            final_hits.push(crate::memory_hybrid::HybridSearchHit {
                                path: res.url.clone(),
                                title: res.title.clone(),
                                content_snippet: res.content.clone(),
                                score: res.score.unwrap_or(0.5),
                                provenance,
                                potential_contradiction: false,
                            });
                        }
                    }
                    Err(e) => {
                        warn!(url = %res.url, error = %e, "Scraping failed for search result");
                        let mut provenance = vec!["WebResearch".to_string()];
                        if let Some(ref eng) = res.engine {
                            provenance.push(format!("engine:{eng}"));
                        }
                        provenance.push("scraped:false".to_string());
                        final_hits.push(crate::memory_hybrid::HybridSearchHit {
                            path: res.url.clone(),
                            title: res.title.clone(),
                            content_snippet: res.content.clone(),
                            score: res.score.unwrap_or(0.5),
                            provenance,
                            potential_contradiction: false,
                        });
                    }
                }
            }

            SearchReport {
                hits: final_hits,
                providers,
            }
        }

        #[cfg(not(feature = "web-scrape"))]
        {
            // Without `web-scrape`, return engine snippets only (no HTML fetch stack).
            let mut final_hits = Vec::new();
            let max_hits = policy
                .searxng_max_results
                .max(policy.searxng_max_urls_to_scrape);
            for res in results.into_iter().take(max_hits) {
                let mut provenance = vec!["WebResearch".to_string()];
                if let Some(ref eng) = res.engine {
                    provenance.push(format!("engine:{eng}"));
                }
                provenance.push("scraped:disabled".to_string());
                final_hits.push(crate::memory_hybrid::HybridSearchHit {
                    path: res.url,
                    title: res.title,
                    content_snippet: res.content,
                    score: res.score.unwrap_or(0.5),
                    provenance,
                    potential_contradiction: false,
                });
            }
            SearchReport {
                hits: final_hits,
                providers,
            }
        }
    }
}

pub trait WebSearchDispatcherExt {
    fn search_with_lane<'a>(
        &'a self,
        query: &'a str,
        lane: ResearchLane,
        policy: &'a SearchPolicy,
    ) -> impl std::future::Future<
        Output = anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>>,
    > + Send
    + 'a;
}

impl WebSearchDispatcherExt for WebSearchDispatcher {
    fn search_with_lane<'a>(
        &'a self,
        query: &'a str,
        lane: ResearchLane,
        policy: &'a SearchPolicy,
    ) -> impl std::future::Future<
        Output = anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>>,
    > + Send
    + 'a {
        WebSearchDispatcher::search_with_lane(query, lane, policy)
    }
}

/// Reciprocal Rank Fusion with authority weights across provider result lists:
/// RRF(d) = \sum_{m \in M} \frac{1}{k + r_m(d)} \cdot \omega_m
///
/// arXiv: 1.20, OpenAlex: 1.10, Wikipedia: 1.00, others: 1.00.
/// $k$ is clamped to at least 1.0.
pub fn true_rrf_fuse(
    provider_lists: Vec<Vec<crate::searxng::SearxngResult>>,
    rrf_k: f64,
) -> Vec<crate::searxng::SearxngResult> {
    let k = rrf_k.max(1.0);
    let mut dedup_map: HashMap<String, (crate::searxng::SearxngResult, f64)> = HashMap::new();

    for list in provider_lists {
        for (i, item) in list.into_iter().enumerate() {
            let rank = (i + 1) as f64;
            let weight = match item.engine.as_deref() {
                Some("arxiv") => 1.20,
                Some("openalex") => 1.10,
                Some("wikipedia") => 1.00,
                _ => 1.00,
            };
            let contrib = (1.0 / (k + rank)) * weight;
            let key = canonical_url_key(&item.url);

            if let Some((existing, score)) = dedup_map.get_mut(&key) {
                *score += contrib;
                if existing.content.trim().is_empty() && !item.content.trim().is_empty() {
                    existing.content = item.content;
                }
                if existing.title.trim().is_empty() && !item.title.trim().is_empty() {
                    existing.title = item.title;
                }
            } else {
                dedup_map.insert(key, (item, contrib));
            }
        }
    }

    let mut fused: Vec<crate::searxng::SearxngResult> = dedup_map
        .into_values()
        .map(|(mut item, score)| {
            item.score = Some(score);
            item
        })
        .collect();

    fused.sort_by(|a, b| {
        b.score
            .unwrap_or(0.0)
            .partial_cmp(&a.score.unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    fused
}

#[allow(dead_code)]
fn rank_and_dedupe_results(results: &mut Vec<crate::searxng::SearxngResult>) {
    let mut seen = HashSet::new();
    results.retain(|result| seen.insert(canonical_url_key(&result.url)));
    results.sort_by(|a, b| {
        let a_score = a.score.unwrap_or(0.5) * source_authority_score(&a.url);
        let b_score = b.score.unwrap_or(0.5) * source_authority_score(&b.url);
        b_score
            .partial_cmp(&a_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

pub(crate) fn canonical_url_key(url: &str) -> String {
    let mut key = url.trim().to_ascii_lowercase();
    if let Some(stripped) = key.strip_prefix("https://") {
        key = stripped.to_string();
    } else if let Some(stripped) = key.strip_prefix("http://") {
        key = stripped.to_string();
    }
    if let Some((base, _)) = key.split_once('#') {
        key = base.to_string();
    }
    if let Some((base, query)) = key.split_once('?') {
        let curid = query.split('&').find(|param| param.starts_with("curid="));
        if let Some(c) = curid {
            key = format!("{}?{}", base.trim_end_matches('/'), c);
        } else {
            key = base.to_string();
        }
    }
    key.trim_end_matches('/').to_string()
}

#[allow(dead_code)]
fn is_wikipedia_host(key: &str) -> bool {
    key == "wikipedia.org"
        || key.starts_with("wikipedia.org/")
        || key.starts_with("wikipedia.org?")
        || key.contains(".wikipedia.org/")
        || key.contains(".wikipedia.org?")
        || key.ends_with(".wikipedia.org")
}

#[allow(dead_code)]
fn source_authority_score(url: &str) -> f64 {
    let key = canonical_url_key(url);
    if key.contains(".gov/")
        || key.ends_with(".gov")
        || key.contains(".edu/")
        || key.ends_with(".edu")
        || is_wikipedia_host(&key)
        || key.contains("reuters.com/")
        || key.contains("apnews.com/")
        || key.contains("bbc.co")
    {
        1.25
    } else if crate::trust::CORE_ACADEMIC_DOMAINS
        .iter()
        .any(|d| key.contains(d))
        || key.contains("docs.rs/")
        || key.contains("github.com/")
    {
        1.15
    } else {
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranking_boost_and_academic_gate_agree_on_the_shared_core_domains() {
        // `source_authority_score` and `crate::trust::is_plausibly_academic`
        // are separate checks for related-but-different concerns, but both
        // should treat crate::trust::CORE_ACADEMIC_DOMAINS consistently —
        // this pins that agreement so a future edit to one list can't
        // silently diverge from the other on the domains they share.
        for domain in crate::trust::CORE_ACADEMIC_DOMAINS {
            let url = format!("https://{domain}some-id");
            assert!(
                source_authority_score(&url) >= 1.15,
                "{domain} is in CORE_ACADEMIC_DOMAINS, so source_authority_score must boost it \
                 (pubmed.ncbi.nlm.nih.gov/ also matches the separate .gov/ tier at 1.25, which is fine)"
            );
            assert!(
                crate::trust::is_plausibly_academic(&url),
                "{domain} is in CORE_ACADEMIC_DOMAINS, so is_plausibly_academic must accept it"
            );
        }
    }

    fn result(url: &str, score: f64) -> crate::searxng::SearxngResult {
        crate::searxng::SearxngResult {
            url: url.to_string(),
            title: url.to_string(),
            content: String::new(),
            engine: Some("test".to_string()),
            score: Some(score),
        }
    }

    #[test]
    fn rank_and_dedupe_prefers_authoritative_free_sources() {
        let mut results = vec![
            result("https://blog.example/post?utm=1", 0.9),
            result("https://blog.example/post", 0.8),
            result("https://docs.rs/vox-search/latest/vox_search/", 0.82),
        ];

        rank_and_dedupe_results(&mut results);

        assert_eq!(results.len(), 2);
        assert!(results[0].url.contains("docs.rs"));
    }

    #[test]
    fn rank_and_dedupe_boosts_general_authority_sources() {
        let mut results = vec![
            result("https://blog.example/post", 0.8),
            result("https://en.wikipedia.org/wiki/Research", 0.8),
            result("https://www.reuters.com/world/some-article", 0.8),
        ];

        rank_and_dedupe_results(&mut results);

        let wiki_pos = results
            .iter()
            .position(|r| r.url.contains("wikipedia.org"))
            .expect("wikipedia result present");
        let reuters_pos = results
            .iter()
            .position(|r| r.url.contains("reuters.com"))
            .expect("reuters result present");
        let blog_pos = results
            .iter()
            .position(|r| r.url.contains("blog.example"))
            .expect("blog result present");

        assert!(
            wiki_pos < blog_pos,
            "wikipedia.org should outrank an unboosted blog"
        );
        assert!(
            reuters_pos < blog_pos,
            "reuters.com should outrank an unboosted blog"
        );
    }

    #[test]
    fn rank_and_dedupe_preserves_distinct_wikipedia_articles_and_boosts() {
        let mut results = vec![
            result("https://en.wikipedia.org/wiki/Accessibility", 0.8),
            result(
                "https://en.wikipedia.org/wiki/Rust_(programming_language)",
                0.8,
            ),
            result("https://en.wikipedia.org/?curid=1475", 0.8),
            result("https://en.wikipedia.org/?curid=42", 0.8),
            result("https://blog.example/post", 0.95),
        ];

        rank_and_dedupe_results(&mut results);

        // All 4 distinct Wikipedia articles should survive deduplication alongside the blog post
        assert_eq!(results.len(), 5);

        // With 1.25 authority boost, 0.8 * 1.25 = 1.00, outranking 0.95 * 1.0 = 0.95 blog
        let blog_pos = results
            .iter()
            .position(|r| r.url.contains("blog.example"))
            .expect("blog result present");
        assert_eq!(
            blog_pos, 4,
            "all boosted Wikipedia results should outrank blog"
        );

        // Verify source_authority_score returns 1.25 for all forms
        assert_eq!(
            source_authority_score("https://en.wikipedia.org/wiki/Accessibility"),
            1.25
        );
        assert_eq!(
            source_authority_score("https://en.wikipedia.org/?curid=1475"),
            1.25
        );
        assert_eq!(source_authority_score("https://en.wikipedia.org/"), 1.25);
        assert_eq!(source_authority_score("https://en.wikipedia.org"), 1.25);

        // Verify spoofed/subdomain attack URLs do not receive the Wikipedia authority boost
        assert_eq!(
            source_authority_score("https://evil-wikipedia.org/wiki/Phishing"),
            1.0
        );
        assert_eq!(
            source_authority_score("https://wikipedia.org.attacker.com/malware"),
            1.0
        );
    }

    /// D9 fix (Task 8 fix round 3): `enforce_provider_diversity` must stop a
    /// single heavily-RRF-weighted provider (arXiv) from crowding out every
    /// other provider that also returned real hits. Three providers (arXiv,
    /// Wikipedia, SearXNG) each mocked to return 5 hits via wiremock — no
    /// real network. Asserts the kept report has at least one hit from each
    /// provider, no provider exceeds the diversity cap, and the total kept
    /// count is unchanged from what the pre-existing truncation limit would
    /// have produced anyway.
    #[tokio::test]
    async fn enforce_provider_diversity_keeps_every_contributing_provider() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let arxiv_server = MockServer::start().await;
        let arxiv_entries: String = (1..=5)
            .map(|i| {
                format!(
                    "<entry><id>http://arxiv.org/abs/2000.0000{i}v1</id><title>ArXiv Paper {i}</title><summary>Summary {i}</summary></entry>"
                )
            })
            .collect();
        let arxiv_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><feed xmlns=\"http://www.w3.org/2005/Atom\">{arxiv_entries}</feed>"
        );
        Mock::given(method("GET"))
            .and(path("/api/query"))
            .respond_with(ResponseTemplate::new(200).set_body_string(arxiv_xml))
            .mount(&arxiv_server)
            .await;

        let wiki_server = MockServer::start().await;
        let wiki_results: Vec<serde_json::Value> = (1..=5)
            .map(|i| {
                serde_json::json!({
                    "title": format!("Wiki Article {i}"),
                    "pageid": i,
                    "snippet": format!("Wiki snippet {i}")
                })
            })
            .collect();
        Mock::given(method("GET"))
            .and(path("/wiki"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "query": { "search": wiki_results }
            })))
            .mount(&wiki_server)
            .await;

        let searxng_server = MockServer::start().await;
        let searxng_results: Vec<serde_json::Value> = (1..=5)
            .map(|i| {
                serde_json::json!({
                    "url": format!("https://example.com/searxng-{i}"),
                    "title": format!("SearXNG Result {i}"),
                    "content": format!("SearXNG content {i}"),
                    "engine": "duckduckgo",
                    "score": 1.0
                })
            })
            .collect();
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "results": searxng_results })),
            )
            .mount(&searxng_server)
            .await;

        let policy = SearchPolicy {
            enable_wikipedia: true,
            wikipedia_fallback_enabled: true,
            wikipedia_api_url: Some(format!("{}/wiki", wiki_server.uri())),
            enable_openalex: false,
            enable_arxiv: true,
            arxiv_api_url: Some(format!("{}/api/query", arxiv_server.uri())),
            searxng_url: Some(searxng_server.uri()),
            searxng_max_results: 5,
            searxng_max_urls_to_scrape: 3,
            tavily_enabled: false,
            ..SearchPolicy::default()
        };
        let registry = crate::search_circuit_breaker::SearchProviderCircuitRegistry::new();

        let report = WebSearchDispatcher::search_with_report_and_registry(
            "diversity test query",
            ResearchLane::Deep,
            &policy,
            &registry,
        )
        .await;

        let kept_limit = policy
            .searxng_max_results
            .max(policy.searxng_max_urls_to_scrape);
        assert_eq!(
            report.hits.len(),
            kept_limit,
            "kept-hit count must be unchanged by diversification: {:?}",
            report.hits
        );

        fn engine_of(h: &crate::memory_hybrid::HybridSearchHit) -> &str {
            h.provenance
                .iter()
                .find_map(|p| p.strip_prefix("engine:"))
                .unwrap_or("unknown")
        }
        let mut counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for h in &report.hits {
            *counts.entry(engine_of(h)).or_insert(0) += 1;
        }

        for provider in ["arxiv", "wikipedia", "duckduckgo"] {
            assert!(
                counts.get(provider).copied().unwrap_or(0) >= 1,
                "provider {provider} must have at least one kept hit: {counts:?} ({:?})",
                report.hits
            );
        }
        // cap = max(2, ceil(kept_limit / contributing_providers)) = max(2, ceil(5/3)) = 2
        let cap = 2;
        for (provider, count) in &counts {
            assert!(
                *count <= cap,
                "provider {provider} exceeded the diversity cap of {cap}: {count} ({counts:?})"
            );
        }
    }

    /// Task 8b: SearXNG returns the page that actually answers the query
    /// (an OpenRouter Gemini 3.8 Flash pricing page, ranked 4th/5th by
    /// SearXNG itself) while arXiv's mocked hits share no query terms at
    /// all. Before the fix, `true_rrf_fuse`'s arXiv authority weight (1.20)
    /// alone decided the order, so the irrelevant arXiv paper displaced the
    /// relevant OpenRouter page. Asserts the kept report includes an
    /// `openrouter.ai` hit, and that it precedes the zero-term-overlap arXiv
    /// hits.
    #[tokio::test]
    async fn rerank_by_relevance_keeps_relevant_searxng_hit_over_irrelevant_arxiv() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let query = "latest Gemini Flash model OpenRouter released";

        let arxiv_server = MockServer::start().await;
        let arxiv_entries: String = (1..=5)
            .map(|i| {
                format!(
                    "<entry><id>http://arxiv.org/abs/2503.2002{i}v1</id><title>Attention Mechanisms in Transformer Architectures {i}</title><summary>A survey of scaling laws for sequence models {i}</summary></entry>"
                )
            })
            .collect();
        let arxiv_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><feed xmlns=\"http://www.w3.org/2005/Atom\">{arxiv_entries}</feed>"
        );
        Mock::given(method("GET"))
            .and(path("/api/query"))
            .respond_with(ResponseTemplate::new(200).set_body_string(arxiv_xml))
            .mount(&arxiv_server)
            .await;

        let searxng_server = MockServer::start().await;
        let searxng_results = serde_json::json!([
            {"url": "https://example.com/unrelated-1", "title": "Unrelated Page 1", "content": "nothing to do with the query", "engine": "duckduckgo", "score": 1.0},
            {"url": "https://example.com/unrelated-2", "title": "Unrelated Page 2", "content": "still nothing relevant here", "engine": "duckduckgo", "score": 0.9},
            {"url": "https://example.com/unrelated-3", "title": "Unrelated Page 3", "content": "more filler content", "engine": "duckduckgo", "score": 0.8},
            {"url": "https://openrouter.ai/google/gemini-3.8-flash", "title": "Gemini 3.8 Flash - API Pricing, Provider Status & Uptime | OpenRouter", "content": "OpenRouter released the latest Gemini Flash model, Gemini 3.8 Flash, with pricing and uptime details.", "engine": "duckduckgo", "score": 0.7},
            {"url": "https://openrouter.ai/google/gemini-3.8-flash-lite", "title": "Gemini 3.8 Flash Lite - OpenRouter", "content": "OpenRouter's listing for the latest Gemini Flash model variant.", "engine": "duckduckgo", "score": 0.6},
        ]);
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "results": searxng_results })),
            )
            .mount(&searxng_server)
            .await;

        let policy = SearchPolicy {
            enable_wikipedia: false,
            enable_openalex: false,
            enable_arxiv: true,
            arxiv_api_url: Some(format!("{}/api/query", arxiv_server.uri())),
            searxng_url: Some(searxng_server.uri()),
            searxng_max_results: 5,
            searxng_max_urls_to_scrape: 3,
            tavily_enabled: false,
            ..SearchPolicy::default()
        };
        let registry = crate::search_circuit_breaker::SearchProviderCircuitRegistry::new();

        let report = WebSearchDispatcher::search_with_report_and_registry(
            query,
            ResearchLane::Deep,
            &policy,
            &registry,
        )
        .await;

        let openrouter_pos = report
            .hits
            .iter()
            .position(|h| h.path.contains("openrouter.ai"))
            .unwrap_or_else(|| {
                panic!(
                    "expected an openrouter.ai hit in kept results: {:?}",
                    report.hits
                )
            });
        let first_arxiv_pos = report
            .hits
            .iter()
            .position(|h| h.path.contains("arxiv.org"));
        if let Some(arxiv_pos) = first_arxiv_pos {
            assert!(
                openrouter_pos < arxiv_pos,
                "relevant OpenRouter hit (pos {openrouter_pos}) should precede the \
                 zero-term-overlap arXiv hit (pos {arxiv_pos}): {:?}",
                report.hits
            );
        }
    }

    /// Task 8b: relevance reranking is not an arXiv ban — when arXiv's hits
    /// genuinely match the query terms (an academic-shaped query), arXiv
    /// should still lead the kept results.
    #[tokio::test]
    async fn rerank_by_relevance_lets_genuinely_relevant_arxiv_lead() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let query = "transformer attention scaling laws";

        let arxiv_server = MockServer::start().await;
        let arxiv_xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><feed xmlns=\"http://www.w3.org/2005/Atom\">\
             <entry><id>http://arxiv.org/abs/2503.20020v1</id>\
             <title>Scaling Laws for Transformer Attention</title>\
             <summary>We study transformer attention scaling laws across model sizes.</summary></entry>\
             </feed>";
        Mock::given(method("GET"))
            .and(path("/api/query"))
            .respond_with(ResponseTemplate::new(200).set_body_string(arxiv_xml))
            .mount(&arxiv_server)
            .await;

        let searxng_server = MockServer::start().await;
        let searxng_results = serde_json::json!([
            {"url": "https://example.com/unrelated-1", "title": "Unrelated Page", "content": "nothing to do with the query", "engine": "duckduckgo", "score": 1.0},
        ]);
        Mock::given(method("GET"))
            .and(path("/search"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "results": searxng_results })),
            )
            .mount(&searxng_server)
            .await;

        let policy = SearchPolicy {
            enable_wikipedia: false,
            enable_openalex: false,
            enable_arxiv: true,
            arxiv_api_url: Some(format!("{}/api/query", arxiv_server.uri())),
            searxng_url: Some(searxng_server.uri()),
            searxng_max_results: 5,
            searxng_max_urls_to_scrape: 3,
            tavily_enabled: false,
            ..SearchPolicy::default()
        };
        let registry = crate::search_circuit_breaker::SearchProviderCircuitRegistry::new();

        let report = WebSearchDispatcher::search_with_report_and_registry(
            query,
            ResearchLane::Deep,
            &policy,
            &registry,
        )
        .await;

        assert!(
            report
                .hits
                .first()
                .is_some_and(|h| h.path.contains("arxiv.org")),
            "genuinely relevant arXiv hit should lead when it matches the query: {:?}",
            report.hits
        );
    }

    /// D9 live diagnostic (Task 8 fix round 2, finding (c)): the live daemon's
    /// deep-research run on "compare SearXNG and Tavily for agent web search"
    /// retrieved 25 sources spanning only 1 distinct domain — all arXiv — even
    /// though quick research's `search_with_report` on the same process/lane
    /// showed SearXNG reachable (`ok hits=5`) and the planner's own subqueries
    /// were clean and explicitly named SearXNG/Tavily. Runs one of those real
    /// planner subqueries through `search_with_report` on the Deep lane and
    /// prints each provider's raw outcome plus the fused top-10 with URLs, to
    /// distinguish "SearXNG errored/rate-limited under deep query volume" from
    /// "arXiv's RRF authority weight (1.20) just wins fusion for this query".
    ///
    /// Run manually (needs network): `cargo test -p vox-search -- --ignored --nocapture deep_lane_probe`
    #[tokio::test]
    #[ignore = "live network probe — run manually with --ignored"]
    async fn deep_lane_probe_searxng_vs_tavily_subquery() {
        let policy = crate::SearchPolicy::from_env();
        let report = WebSearchDispatcher::search_with_report(
            "SearXNG vs Tavily comparison for AI agents",
            crate::policy::ResearchLane::Deep,
            &policy,
        )
        .await;
        eprintln!("--- provider outcomes ---");
        for p in &report.providers {
            eprintln!("{}: {:?}", p.provider, p.status);
        }
        eprintln!("--- fused top-10 ---");
        for (i, h) in report.hits.iter().take(10).enumerate() {
            let engine = h
                .provenance
                .iter()
                .find_map(|p| p.strip_prefix("engine:"))
                .unwrap_or("?");
            eprintln!("{}. [{engine}] {} — {}", i + 1, h.title, h.path);
        }
        eprintln!(
            "total hits: {}, distinct engines: {:?}",
            report.hits.len(),
            report
                .hits
                .iter()
                .filter_map(|h| h.provenance.iter().find_map(|p| p.strip_prefix("engine:")))
                .collect::<std::collections::HashSet<_>>()
        );
    }
}
