use std::collections::HashSet;

use tracing::{info, warn};

use crate::policy::SearchPolicy;

pub struct WebSearchDispatcher;

impl WebSearchDispatcher {
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
        let mut results = Vec::new();

        // Tier 1: SearXNG
        if let Some(base_url) = &policy.searxng_url {
            if !registry.is_available(crate::search_circuit_breaker::SearchProviderId::Searxng) {
                warn!("SearXNG is in circuit breaker cooldown, skipping");
            } else {
                let client = crate::searxng::SearxngSearchClient::new(base_url.clone());
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
                        results = hits;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let is_rate_limit = err_str.contains("429")
                            || err_str.to_ascii_lowercase().contains("rate limit");
                        registry.record_failure(
                            crate::search_circuit_breaker::SearchProviderId::Searxng,
                            is_rate_limit,
                        );
                        warn!(error = %e, is_rate_limit, "SearXNG search failed, falling back");
                    }
                }
            }
        }

        // Tier 2: Tavily (when SearXNG produced nothing and policy allows it)
        #[cfg(feature = "tavily")]
        if results.is_empty() && policy.tavily_enabled {
            if !registry.is_available(crate::search_circuit_breaker::SearchProviderId::Tavily) {
                warn!("Tavily is in circuit breaker cooldown, skipping");
            } else if let Some(client) = crate::tavily::TavilySearchClient::from_env() {
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
                        results = hits
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
                    }
                }
            }
        }

        // Tier 3: DuckDuckGo Fallback (when SearXNG + Tavily produced nothing)
        if results.is_empty() && policy.duckduckgo_fallback_enabled {
            if !registry.is_available(crate::search_circuit_breaker::SearchProviderId::DuckDuckGo) {
                warn!("DuckDuckGo is in circuit breaker cooldown, skipping");
            } else {
                match crate::duckduckgo::DuckDuckGoClient::search(query, policy.searxng_max_results)
                    .await
                {
                    Ok(hits) => {
                        info!(count = hits.len(), "DuckDuckGo fallback succeeded");
                        registry.record_success(
                            crate::search_circuit_breaker::SearchProviderId::DuckDuckGo,
                        );
                        results = hits;
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let is_rate_limit = err_str.contains("429")
                            || err_str.to_ascii_lowercase().contains("rate limit");
                        registry.record_failure(
                            crate::search_circuit_breaker::SearchProviderId::DuckDuckGo,
                            is_rate_limit,
                        );
                        warn!(error = %e, is_rate_limit, "DuckDuckGo fallback failed");
                    }
                }
            }
        }

        // Tier 4: Wikipedia Encyclopedic Fallback (when SearXNG + Tavily + DDG returned nothing)
        if results.is_empty() && policy.wikipedia_fallback_enabled {
            match crate::wikipedia::WikipediaClient::search(query, policy.searxng_max_results).await
            {
                Ok(hits) if !hits.is_empty() => {
                    info!(
                        count = hits.len(),
                        "Wikipedia encyclopedic fallback succeeded"
                    );
                    results = hits;
                }
                Ok(_) => {}
                Err(e) => {
                    warn!(error = %e, "Wikipedia encyclopedic fallback failed");
                }
            }
        }

        if results.is_empty() {
            return Ok(Vec::new());
        }
        rank_and_dedupe_results(&mut results);

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
                .take(policy.searxng_max_urls_to_scrape)
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
            for res in results.iter().take(policy.searxng_max_urls_to_scrape) {
                let mut provenance = vec!["WebResearch".to_string()];
                if let Some(ref eng) = res.engine {
                    provenance.push(format!("engine:{eng}"));
                }
                provenance.push("scraped:disabled".to_string());
                final_hits.push(crate::memory_hybrid::HybridSearchHit {
                    path: res.url.clone(),
                    title: res.title.clone(),
                    content_snippet: res.content.clone(),
                    score: res.score.unwrap_or(0.5),
                    provenance,
                    potential_contradiction: false,
                });
            }
            Ok(final_hits)
        }
    }
}

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

fn source_authority_score(url: &str) -> f64 {
    let key = canonical_url_key(url);
    if key.contains(".gov/")
        || key.ends_with(".gov")
        || key.contains(".edu/")
        || key.ends_with(".edu")
        || key.contains("wikipedia.org")
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
    }
}
