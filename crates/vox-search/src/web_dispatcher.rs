use std::collections::{HashMap, HashSet};

use tracing::{info, warn};

use crate::policy::{ResearchLane, SearchPolicy};

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

    pub async fn search_with_lane_and_registry(
        query: &str,
        lane: ResearchLane,
        policy: &SearchPolicy,
        registry: &crate::search_circuit_breaker::SearchProviderCircuitRegistry,
    ) -> anyhow::Result<Vec<crate::memory_hybrid::HybridSearchHit>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
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
                        hits
                    }
                    Err(e) => {
                        warn!(error = %e, "Wikipedia search failed");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
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
                        hits
                    }
                    Err(e) => {
                        warn!(error = %e, "OpenAlex search failed");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
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
                        hits
                    }
                    Err(e) => {
                        warn!(error = %e, "arXiv search failed");
                        Vec::new()
                    }
                }
            } else {
                Vec::new()
            }
        };

        // 4. SearXNG
        let searxng_task = async {
            if let Some(base_url) = &policy.searxng_url {
                if !registry.is_available(crate::search_circuit_breaker::SearchProviderId::Searxng)
                {
                    warn!("SearXNG is in circuit breaker cooldown, skipping");
                    Vec::new()
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
                            hits
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
                            Vec::new()
                        }
                    }
                }
            } else {
                Vec::new()
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
                        return hits
                            .into_iter()
                            .map(|h| crate::searxng::SearxngResult {
                                url: h.url,
                                title: h.title.clone(),
                                content: h.content,
                                engine: Some("tavily".to_string()),
                                score: Some(f64::from(h.score)),
                            })
                            .collect();
                    }
                    Err(e) => {
                        let is_rate_limit =
                            e.contains("429") || e.to_ascii_lowercase().contains("rate limit");
                        registry.record_failure(
                            crate::search_circuit_breaker::SearchProviderId::Tavily,
                            is_rate_limit,
                        );
                        warn!(error = %e, is_rate_limit, "Tavily web search failed");
                        return Vec::new();
                    }
                }
            }
            Vec::new()
        };

        // Bound each provider by lane timeout deadline
        let (wiki_res, openalex_res, arxiv_res, searxng_res, tavily_res) = tokio::join!(
            tokio::time::timeout(deadline, wiki_task),
            tokio::time::timeout(deadline, openalex_task),
            tokio::time::timeout(deadline, arxiv_task),
            tokio::time::timeout(deadline, searxng_task),
            tokio::time::timeout(deadline, tavily_task),
        );

        let unwrap_timed =
            |res: Result<Vec<crate::searxng::SearxngResult>, tokio::time::error::Elapsed>,
             provider: &'static str|
             -> Vec<crate::searxng::SearxngResult> {
                match res {
                    Ok(hits) => hits,
                    Err(_) => {
                        warn!(provider, "Search provider timed out");
                        match provider {
                            "searxng" => registry.record_failure(
                                crate::search_circuit_breaker::SearchProviderId::Searxng,
                                false,
                            ),
                            "tavily" => registry.record_failure(
                                crate::search_circuit_breaker::SearchProviderId::Tavily,
                                false,
                            ),
                            _ => {}
                        }
                        Vec::new()
                    }
                }
            };

        let provider_lists = vec![
            unwrap_timed(arxiv_res, "arxiv"),
            unwrap_timed(openalex_res, "openalex"),
            unwrap_timed(wiki_res, "wikipedia"),
            unwrap_timed(searxng_res, "searxng"),
            unwrap_timed(tavily_res, "tavily"),
        ];

        let mut results = true_rrf_fuse(provider_lists, policy.rrf_k);

        if results.is_empty() {
            return Ok(Vec::new());
        }

        Self::filter_and_penalize_results(&mut results, policy);
        if results.is_empty() {
            return Ok(Vec::new());
        }

        results.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

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

            Ok(final_hits)
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
            Ok(final_hits)
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
}
