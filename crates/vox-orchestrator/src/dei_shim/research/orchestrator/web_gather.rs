use std::time::Instant;

use serde_json::json;
use vox_db::Codex;

use super::config::ResearchConfig;
use super::super::provider::ProviderRegistry;
use super::super::types::{ResearchHit, ResearchPlan, ResearchQuery};

pub(super) async fn gather_web_hits_for_plan(
    db: Option<&Codex>,
    session_id: i64,
    query: &ResearchQuery,
    plan: &ResearchPlan,
    registry: &ProviderRegistry,
    config: &ResearchConfig,
) -> (
    Vec<ResearchHit>,
    usize,
    usize,
    usize,
) {
    let mut all_hits: Vec<ResearchHit> = Vec::new();
    let mut subqueries_with_hits = 0;
    let mut total_dropped_count = 0usize;
    let mut total_sources_attempted = 0usize;

    for subquery in &plan.subqueries {
        let mut sq = ResearchQuery {
            query: subquery.clone(),
            ..query.clone()
        };

        // map_site: discover child pages when site_scope is set.
        let bonus_urls: Vec<String> = if let Some(ref site) = query.site_scope {
            sq.site_scope = Some(site.clone());
            registry
                .map_site(&format!("https://{site}"))
                .await
                .into_iter()
                .flatten()
                .take(10)
                .collect()
        } else {
            vec![]
        };

        let run_start = Instant::now();
        let run_id = db.map(|d| {
            d.start_provider_run(session_id, registry.primary_name(), subquery)
                .unwrap_or(0)
        });

        let (hits, provider_used) = registry.search(&sq).await;
        if !hits.is_empty() {
            subqueries_with_hits += 1;
        }
        let elapsed_ms = run_start.elapsed().as_millis() as i64;
        let hit_count = hits.len() as i64;

        // Crawl top-N hit URLs plus any bonus URLs from map_site.
        let mut crawl_urls: Vec<String> = hits
            .iter()
            .take(query.max_sources)
            .map(|h| h.url.clone())
            .collect();
        crawl_urls.extend(bonus_urls);
        crawl_urls.sort();
        crawl_urls.dedup();

        total_sources_attempted += crawl_urls.len();
        let pages = registry.crawl(&crawl_urls).await;
        // Call provider extract() on each fetched page for chunk-quality content.
        let mut page_content: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for page in &pages {
            if page.http_status >= 200 && page.http_status < 300 {
                let chunks = registry
                    .extract(page, subquery, config.chunk_max_chars)
                    .await;
                if !chunks.is_empty() {
                    page_content.insert(
                        page.url.clone(),
                        chunks
                            .into_iter()
                            .map(|c| c.text)
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                } else {
                    // fallback: whole page html
                    page_content.insert(page.url.clone(), page.html.clone());
                }
            }
        }
        let mut dropped_count = 0usize;

        // Deduplicate and trust-filter by URL.
        let mut seen_urls = std::collections::HashSet::new();

        for hit in &hits {
            if !seen_urls.insert(hit.url.clone()) {
                dropped_count += 1;
                continue;
            }
            // Drop sources with failed HTTP status.
            if hit.http_status < 200 || hit.http_status >= 400 {
                dropped_count += 1;
                continue;
            }

            let mut h = hit.clone();
            // Apply trust multiplier for high-trust domains.
            if h.trust_score >= 1.0 {
                h.score = (h.score * config.trust_multiplier).min(1.0);
            }
            // Augment raw_content from extract() result (already retrieved above).
            if h.raw_content.is_empty()
                && let Some(content) = page_content.get(&h.url)
            {
                h.raw_content = content
                    .split_whitespace()
                    .take(1200)
                    .collect::<Vec<_>>()
                    .join(" ");
            }

            // Ingest into Codex when available.
            if let Some(db) = db {
                let source_id = db
                    .create_research_source(
                        session_id,
                        &h.url,
                        &h.title,
                        &h.snippet,
                        &h.raw_content,
                        h.score,
                        &provider_used,
                        h.http_status,
                        h.trust_score,
                        &json!({}),
                    )
                    .unwrap_or(0);

                if !h.raw_content.is_empty() {
                    let mut chunk_embeddings = Vec::new();
                    if let Some(ref embedder) = config.embedder {
                        let chunker_config = vox_db::chunker::ChunkerConfig {
                            max_chars: config.chunk_max_chars,
                            overlap_chars: config.chunk_overlap_chars,
                        };
                        let chunks = vox_db::chunker::chunk(&h.raw_content, &chunker_config);
                        for c in chunks {
                            if let Ok(v) = embedder.embed_query(&c.text).await {
                                chunk_embeddings.push(v);
                            } else {
                                chunk_embeddings.push(Vec::new());
                            }
                        }
                    }

                    let _ = db.ingest_research_document(&vox_db::ResearchIngestRequest {
                        packet: vox_db::ExternalResearchPacket {
                            topic: query.query.chars().take(80).collect(),
                            vendor: provider_used.clone(),
                            area: None,
                            source_url: h.url.clone(),
                            source_type: "web".to_string(),
                            title: h.title.clone(),
                            captured_at: chrono::Utc::now().to_rfc3339(),
                            summary: h.snippet.chars().take(500).collect(),
                            raw_excerpt: h.raw_content.chars().take(2000).collect(),
                            claims: vec![],
                            tags: vec![],
                            confidence: h.score,
                            content_hash: String::new(),
                            metadata: json!({ "session_id": session_id, "source_id": source_id }),
                        },
                        body: h.raw_content.clone(),
                        kb_id: Some(format!("research_session_{}", session_id)),
                        embeddings: chunk_embeddings,
                    });
                }
            }

            all_hits.push(h);
        }

        total_dropped_count += dropped_count;
        if let (Some(db), Some(run_id)) = (db, run_id) {
            let _ = db.finish_provider_run(run_id, "completed", hit_count, elapsed_ms, None);
        }
    }

    (
        all_hits,
        subqueries_with_hits,
        total_dropped_count,
        total_sources_attempted,
    )
}
