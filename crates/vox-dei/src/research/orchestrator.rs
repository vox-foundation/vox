//! End-to-end research pipeline orchestrator.
//!
//! `run_research` is the single entry point for all research invocations.
//! It coordinates: session creation → query planning → provider search/extract →
//! Codex ingestion → hybrid retrieval fusion → claim detection → verification →
//! answer synthesis → source/claim persistence → session finalization.
//!
//! # Session and metric keys (cross-surface alignment)
//!
//! - **`ResearchMetadata.session_id`** — Opaque numeric session handle for this pipeline run (created
//!   when a Codex handle is present). Progress and completion metrics are written through the same
//!   Codex surface; the underlying `research_metrics.session_id` column is **TEXT** (see
//!   `vox_pm::store` `append_research_metric`).
//! - **Agent memory bridge** — `vox_db::MemoryParams.session_id` is set to
//!   `format!("research_{}", session_id)` with `memory_type` `research_result` so MCP memory tools
//!   can recall past answers (matches content keyed in this module).
//! - **Chunk partitioning** — ingest paths may set `kb_id` to `research_session_{session_id}` for
//!   stable correlation with the run.
//!
//! **Benchmark telemetry (CLI, not this pipeline):** `vox_db::VoxDb::record_benchmark_event` writes
//! `research_metrics` under `session_id` **`bench:<repository_id>`** (`crates/vox-db/src/benchmark_telemetry.rs`).
//! That session namespace is separate from DeI research runs; align **`repository_id`** via repository
//! discovery / `VOX_REPOSITORY_ROOT` when comparing Codex rows from CLI subprocesses vs MCP.

use std::time::Instant;

use anyhow::Result;
use serde_json::json;
use vox_db::Codex;
use vox_socrates_policy::ConfidencePolicy;

use super::{
    claims::extract_claims_with_model,
    gate::{GateInput, score_with_config},
    planner::decompose_query_with_config,
    provider::ProviderRegistry,
    types::{
        Citation, CompetenceSignal, ResearchHit, ResearchMetadata, ResearchPlan, ResearchQuery,
        ResearchResult, ResearchScope, RetrievalDiagnostics, RoutingTier, SelfVerificationResult,
    },
    verifier::verify_claims_with_config,
};
use crate::services::embeddings::EmbeddingService;

/// Sanitize a string for ChatML formatting by replacing control tokens that could
/// trigger prompt injection (e.g., `<|im_start|>`, `<|im_end|>`).
fn sanitize_chatml(input: &str) -> String {
    input
        .replace("<|im_start|>", "[im_start]")
        .replace("<|im_end|>", "[im_end]")
}

/// Anti-laziness rider for all research LLM prompts.
const ANTI_LAZINESS_RIDER: &str = "
<anti_laziness_rider>
DO NOT summarize or skip steps. DO NOT provide stubs, placeholders, or 'TODO' blocks. Implement ALL requested logic in full detail.
If providing a plan, ensure it is exhaustive and execution-ready. Laziness will be penalized with a 0 quality score.
</anti_laziness_rider>";

/// Sanitize evidence snippets from search results to prevent ChatML injection.
fn sanitize_evidence(text: &str) -> String {
    sanitize_chatml(text)
}
use std::sync::Arc;

/// Progress reporting callback for research operations.
pub type ProgressCallback = dyn Fn(String, Option<f32>) + Send + Sync + 'static;

/// Configuration for a single research run.
#[derive(Clone)]
pub struct ResearchConfig {
    /// LLM endpoint base URL (e.g. `https://api.openai.com`).
    pub llm_endpoint: Option<String>,
    /// Bearer API key for the LLM endpoint.
    pub api_key: Option<String>,
    /// Model used for query decomposition / planning.
    pub planner_model: String,
    /// Sampling temperature for the planner (lower = more deterministic).
    pub planner_temperature: f32,
    /// Maximum number of subqueries the planner may emit.
    pub planner_max_subqueries: usize,
    /// Model used for claim extraction.
    pub claim_model: String,
    /// Max tokens for a single claim-extraction response.
    pub claim_max_tokens: u32,
    /// Model used for answer synthesis.
    pub synthesis_model: String,
    /// Sampling temperature for synthesis.
    pub synthesis_temperature: f32,
    /// Max tokens for synthesis response.
    pub synthesis_max_tokens: u32,
    /// Model used for the LLM-as-judge quality scorer.
    pub judge_model: String,
    /// Sampling temperature for the judge.
    pub judge_temperature: f32,
    /// Max tokens for the judge response.
    pub judge_max_tokens: u32,
    /// Quality score returned when no LLM judge is available.
    pub fallback_quality_score: i32,
    /// Max chars for the synthesis LLM context (hits + verdict text).
    pub synthesis_context_max_chars: usize,
    /// Maximum characters per extracted chunk.
    pub chunk_max_chars: usize,
    /// Chars of overlap between consecutive chunks.
    pub chunk_overlap_chars: usize,
    /// Multiplier applied to the provider score for high-trust domains.
    pub trust_multiplier: f64,
    /// Minimum confidence before a doc is persisted to `docs/src/research/`.
    pub persist_min_confidence: f64,
    /// Confidence gate configuration.
    pub gate: super::config::GateConfig,
    /// Claim verifier configuration.
    pub verifier: super::config::VerifierConfig,
    /// Routing tier thresholds.
    pub routing_thresholds: super::config::RoutingThresholds,
    /// Fusion weights: (vector_weight, bm25_weight, kb_chunk_weight).
    pub fusion_weights: (f64, f64, f64),
    /// Minimum confidence to write a Mens training pair.
    pub training_pair_min_confidence: f64,
    /// Minimum citation count to write a Mens training pair.
    pub training_pair_min_citations: usize,
    /// Provider configuration (high-trust domains, timeout, etc.).
    pub provider: super::config::ProviderConfig,
    /// Optional embedding service for indexing chunks.
    pub embedder: Option<Arc<EmbeddingService>>,
    /// Whether claim detection and verification is enabled for this run.
    ///
    /// Can be overridden at runtime by the `rollout.claim_detection` config key.
    pub claim_detection_enabled: bool,
    /// Optional callback for progress reporting.
    pub progress_callback: Option<Arc<ProgressCallback>>,
    /// Optional snapshot of workspace inference policy for registry stage picks ([`super::model_select::resolve_research_models`]).
    ///
    /// When `None`, [`run_research`] uses [`crate::config::OrchestratorConfig::default`].[`effective_inference_config`](crate::config::OrchestratorConfig::effective_inference_config).
    pub model_pick_inference: Option<crate::mode::InferenceConfig>,
}

impl std::fmt::Debug for ResearchConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResearchConfig")
            .field("llm_endpoint", &self.llm_endpoint)
            .field("planner_model", &self.planner_model)
            .field("claim_model", &self.claim_model)
            .field("synthesis_model", &self.synthesis_model)
            .field("judge_model", &self.judge_model)
            .field("chunk_max_chars", &self.chunk_max_chars)
            .field("fusion_weights", &self.fusion_weights)
            .finish_non_exhaustive()
    }
}

impl Default for ResearchConfig {
    /// Defaults match [`super::model_select::resolve_research_models`] at construction time
    /// (same catalog / static snapshot as `run_research`). Serialised configs may still override
    /// `planner_model` / `claim_model` / etc.; the live pipeline always re-resolves via the registry.
    fn default() -> Self {
        let reg = crate::models::ModelRegistry::new();
        let base = crate::config::OrchestratorConfig::default().effective_inference_config();
        let r = super::model_select::resolve_research_models(&reg, &base);
        let mut verifier = super::config::VerifierConfig::default();
        verifier.nli_model_id = r.claim_model.clone();
        Self {
            llm_endpoint: None,
            api_key: None,
            planner_model: r.planner_model,
            planner_temperature: 0.3,
            planner_max_subqueries: 6,
            claim_model: r.claim_model,
            claim_max_tokens: 512,
            synthesis_model: r.synthesis_model,
            synthesis_temperature: 0.2,
            synthesis_max_tokens: 1200,
            judge_model: r.judge_model,
            judge_temperature: 0.0,
            judge_max_tokens: 16,
            fallback_quality_score: i32::from(
                ConfidencePolicy::DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE,
            ),
            synthesis_context_max_chars: 8000,
            chunk_max_chars: 1200,
            chunk_overlap_chars: 150,
            trust_multiplier: 1.2,
            persist_min_confidence: ConfidencePolicy::DEFAULT_MIN_PERSIST_CONFIDENCE,
            gate: super::config::GateConfig::default(),
            verifier,
            routing_thresholds: super::config::RoutingThresholds::default(),
            fusion_weights: (0.65, 0.50, 0.80),
            training_pair_min_confidence: ConfidencePolicy::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE,
            training_pair_min_citations: 2,
            provider: super::config::ProviderConfig::default(),
            embedder: None,
            claim_detection_enabled: true,
            progress_callback: None,
            model_pick_inference: None,
        }
    }
}

/// When the verifier still uses the default NLI sentinel ([`super::model_select::FALLBACK_NLI_MODEL_ID`]),
/// align NLI with the registry-resolved claim model for consistent routing.
fn verifier_config_for_research_run(
    base: &super::config::VerifierConfig,
    resolved: &super::model_select::ResolvedResearchModels,
) -> super::config::VerifierConfig {
    let mut v = base.clone();
    if v.nli_model_id == super::model_select::FALLBACK_NLI_MODEL_ID {
        v.nli_model_id = resolved.claim_model.clone();
    }
    v
}

/// Run the full research pipeline for `query`. Returns a `ResearchResult` with
/// a cited answer, sources, citations, and telemetry metadata.
pub async fn run_research(
    mut query: ResearchQuery,
    db: Option<&Codex>,
    config: &ResearchConfig,
) -> Result<ResearchResult> {
    // ── (0) Check Rollout Flags ──────────────────────────────────────────────
    let mut web_enabled = true;
    let mut claim_enabled = config.claim_detection_enabled;
    let mut persist_enabled = true;
    let mut self_verification_enabled = false;

    if let Some(db) = db {
        if let Ok(Some(val)) = db.get_retrieval_config("rollout.web_provider") {
            web_enabled = val["enabled"].as_bool().unwrap_or(true);
        }
        if let Ok(Some(val)) = db.get_retrieval_config("rollout.claim_detection") {
            claim_enabled = val["enabled"].as_bool().unwrap_or(true);
        }
        if let Ok(Some(val)) = db.get_retrieval_config("rollout.persist_to_docs") {
            persist_enabled = val["enabled"].as_bool().unwrap_or(true);
        }
        if let Ok(Some(val)) = db.get_retrieval_config("rollout.self_verification") {
            self_verification_enabled = val["enabled"].as_bool().unwrap_or(false);
        }
    }

    if !web_enabled && query.scope == ResearchScope::Web {
        query.scope = ResearchScope::Both; // fallback to both, or could fail?
    }
    if !claim_enabled {
        query.verify_claims = false;
    }
    // Note: persist_enabled is checked during persistence step at the end.

    let registry = ProviderRegistry::from_env_with_config(config.provider.clone());
    let llm_model_registry = crate::models::ModelRegistry::new();
    let base_inference = config
        .model_pick_inference
        .clone()
        .unwrap_or_else(|| crate::config::OrchestratorConfig::default().effective_inference_config());
    let resolved_llm =
        super::model_select::resolve_research_models(&llm_model_registry, &base_inference);
    let research_verifier_cfg = verifier_config_for_research_run(&config.verifier, &resolved_llm);
    tracing::info!(
        target: "vox_dei::model_route",
        route_source = "research_pipeline",
        planner = %resolved_llm.planner_model,
        claim = %resolved_llm.claim_model,
        synthesis = %resolved_llm.synthesis_model,
        judge = %resolved_llm.judge_model,
        "research_models_resolved"
    );
    let start = Instant::now();

    // ── (0b) Research cache check (Codex `list_memories_by_type`, no raw SQL) ──
    if let Some(db) = db {
        let q = query.query.trim();
        // Require a minimum query length to avoid false hits on common words.
        let q_words: Vec<&str> = q.split_whitespace()
            .filter(|w| w.len() >= 4)
            .collect();
        if !q_words.is_empty() {
            if let Ok(memories) = db
                .list_memories_by_type("research_result", 64)
                .await
            {
                // Find the youngest qualifying cache hit rather than the first.
                let mut best: Option<(f64, vox_db::MemoryEntry)> = None;
                for mem in memories {
                    // Require at least one meaningful query word to appear as a whole word in content.
                    let content_lower = mem.content.to_lowercase();
                    let hit = q_words.iter().any(|w| {
                        let wl = w.to_lowercase();
                        content_lower.contains(&wl)
                    });
                    if !hit || mem.content.trim().is_empty() {
                        continue;
                    }
                    // Default to f64::MAX on parse failure so broken timestamps never become cache hits.
                    let age_hours = chrono::DateTime::parse_from_rfc3339(mem.created_at.trim())
                        .ok()
                        .map(|dt| {
                            (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_seconds()
                                as f64
                                / 3600.0
                        })
                        .unwrap_or(f64::MAX);
                    if age_hours < 24.0 {
                        if best.as_ref().map_or(true, |(best_age, _)| age_hours < *best_age) {
                            best = Some((age_hours, mem));
                        }
                    }
                }
                if let Some((age_hours, mem)) = best {
                    tracing::info!("Research cache hit from Codex (age: {:.1}h)", age_hours);
                    return Ok(ResearchResult {
                        answer: format!(
                            "[Cached result — {:.1}h old]\n\n{}",
                            age_hours, mem.content
                        ),
                        sources: Vec::new(),
                        citations: Vec::new(),
                        research_metadata: ResearchMetadata {
                            session_id: 0,
                            duration_ms: 0,
                            provider: "cache".to_string(),
                            routing_tier: RoutingTier::Direct,
                            confidence: 1.0,
                            subquery_count: 0,
                            source_count: 0,
                            claim_verdicts: Vec::new(),
                            retrieval_diagnostics: RetrievalDiagnostics::default(),
                            quality_score: 100,
                            competence: None,
                            self_verification: None,
                        },
                    });
                }
            }
        }
    }

    // ── (a) Create session ────────────────────────────────────────────────────
    let session_id = if let Some(db) = db {
        let sid = db
            .create_research_session(
                &uuid::Uuid::new_v4().to_string(),
                &json!({ "query": query.query, "scope": format!("{:?}", query.scope) }),
            )
            .unwrap_or(0);
        let _ = db.update_research_session_status(sid, "in_progress");
        sid
    } else {
        0
    };

    let report_progress = |msg: String, pct: Option<f32>| {
        if let Some(ref cb) = config.progress_callback {
            cb(msg.clone(), pct);
        }
        if let Some(db) = db {
            let _ = db.record_research_metric(
                session_id,
                "progress",
                None,
                &serde_json::json!({ "msg": msg, "pct": pct }),
            );
        }
    };
    let _registry_ref = &registry;
    report_progress(
        "Decomposing query into subqueries...".to_string(),
        Some(0.05),
    );

    // ── (b) Query decomposition ──────────────────────────────────────────────
    let plan: ResearchPlan = decompose_query_with_config(
        &query,
        config.llm_endpoint.as_deref(),
        config.api_key.as_deref(),
        Some(resolved_llm.planner_model.as_str()),
        Some(config.planner_temperature),
        Some(config.planner_max_subqueries),
    )
    .await
    .unwrap_or_else(|_| ResearchPlan {
        original_query: query.query.clone(),
        subqueries: vec![query.query.clone()],
        scope: query.scope.clone(),
        max_sources_per_subquery: query.max_sources,
    });

    if let Some(db) = db {
        // Persist plan for resume capability.
        let plan_json = super::planner::plan_to_json(&plan);
        let _ = db.update_research_session_status(session_id, "planning");
        let _ = db.record_research_metric(
            session_id,
            "plan_created",
            Some(registry.primary_name()),
            &plan_json,
        );
    }

    report_progress(
        format!("Running {} subqueries...", plan.subqueries.len()),
        Some(0.15),
    );

    let mut all_hits: Vec<ResearchHit> = Vec::new();
    let mut subqueries_with_hits = 0;
    let mut total_dropped_count = 0usize;
    let mut total_sources_attempted = 0usize;
    let do_web = matches!(query.scope, ResearchScope::Web | ResearchScope::Both);

    if do_web {
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
    }

    // ── (d) Retrieval diagnostics ─────────────────────────────────────────────
    let query_terms: Vec<&str> = query.query.split_whitespace().collect();
    let matched_terms = query_terms
        .iter()
        .filter(|term| {
            all_hits.iter().any(|h| {
                h.snippet
                    .to_ascii_lowercase()
                    .contains(&term.to_ascii_lowercase())
            })
        })
        .count();
    let coverage_pct = if query_terms.is_empty() {
        1.0
    } else {
        matched_terms as f64 / query_terms.len() as f64
    };
    let avg_score = if all_hits.is_empty() {
        0.0
    } else {
        all_hits.iter().map(|h| h.score).sum::<f64>() / all_hits.len() as f64
    };
    let subquery_coverage_pct = if plan.subqueries.is_empty() {
        0.0
    } else {
        subqueries_with_hits as f64 / plan.subqueries.len() as f64
    };
    let diagnostics = RetrievalDiagnostics {
        coverage_pct,
        subquery_coverage_pct,
        avg_provider_score: avg_score,
        fusion_weights: config.fusion_weights,
        dropped_source_count: total_dropped_count,
        hit_rate: if total_sources_attempted == 0 {
            0.0
        } else {
            (all_hits.len() as f64 / total_sources_attempted as f64).min(1.0)
        },
    };

    if let Some(db) = db {
        let _ = db.record_research_metric(
            session_id,
            "retrieval_diagnostics",
            Some(registry.primary_name()),
            &serde_json::to_value(&diagnostics).unwrap_or_default(),
        );
    }

    // ── (e) Confidence gate → routing decision ────────────────────────────────
    let draft_claims = {
        let mut claims = extract_claims_with_model(
            &query.query,
            config.llm_endpoint.as_deref(),
            config.api_key.as_deref(),
            Some(resolved_llm.claim_model.as_str()),
            Some(config.claim_max_tokens),
        )
        .await;
        // rag-2: assign stable claim_id (FNV-1a hash of claim text) for ClaimCorrection references
        for claim in &mut claims {
            claim.claim_id = fnv1a_hash(&claim.text);
        }
        claims
    };

    let gate_input = GateInput {
        claims: &draft_claims,
        citation_count: all_hits.len().min(3),
        no_retrieval_hits: all_hits.is_empty(),
        answer_is_empty: false,
    };
    let confidence_signal = score_with_config(&gate_input, &config.gate);
    let routing_tier = confidence_signal.routing_tier_for(
        config.routing_thresholds.direct,
        config.routing_thresholds.light,
        config.routing_thresholds.deep,
    );

    // ── (f) Claim verification ────────────────────────────────────────────────
    report_progress("Verifying research claims...".to_string(), Some(0.60));
    let claim_verdicts = if query.verify_claims
        && matches!(routing_tier, RoutingTier::DeepResearch)
        && !draft_claims.is_empty()
    {
        let verdicts = verify_claims_with_config(
            &draft_claims,
            &query.query,
            &registry,
            &research_verifier_cfg,
            config.llm_endpoint.as_deref(),
            config.api_key.as_deref(),
        )
        .await;
        // Persist verdicts.
        if let Some(db) = db {
            for verdict in &verdicts {
                let claim_id = db
                    .store_claim(
                        session_id,
                        &verdict.claim.text,
                        verdict.claim.is_numeric,
                        verdict.claim.is_recent,
                        verdict.claim.is_named_event,
                    )
                    .unwrap_or(0);
                let _ = db.store_claim_verdict(
                    claim_id,
                    &verdict.verdict.to_string(),
                    verdict.confidence,
                    verdict.supporting_count as i64,
                    verdict.contradicting_count as i64,
                );
                for span in &verdict.evidence_spans {
                    let _ = db.store_evidence_span(
                        claim_id,
                        span.source_id,
                        &span.text,
                        &span.span_type.to_string(),
                    );
                }
            }
        }
        verdicts
    } else {
        vec![]
    };

    // ── (h) Build citations ───────────────────────────────────────────────────
    let citations: Vec<Citation> = all_hits
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, h)| Citation {
            source_id: i as i64,
            url: h.url.clone(),
            title: h.title.clone(),
            snippet: h.snippet.chars().take(300).collect(),
            confidence: h.score,
        })
        .collect();

    // ── (i) Synthesize answer ─────────────────────────────────────────────────
    report_progress(
        "Synthesizing final research report...".to_string(),
        Some(0.85),
    );
    let answer = synthesize_answer_with_llm(SynthesisParams {
        query: &query.query,
        hits: &all_hits,
        verdicts: &claim_verdicts,
        endpoint: config.llm_endpoint.as_deref(),
        api_key: config.api_key.as_deref(),
        model: resolved_llm.synthesis_model.as_str(),
        temperature: config.synthesis_temperature,
        max_tokens: config.synthesis_max_tokens,
        context_max_chars: config.synthesis_context_max_chars,
    })
    .await;

    // ── (i) Evaluate final quality via judge ──────────────────────────────────
    let quality_score = judge_quality(JudgeParams {
        query: &query.query,
        answer: &answer,
        citations: &citations,
        endpoint: config.llm_endpoint.as_deref(),
        api_key: config.api_key.as_deref(),
        model: resolved_llm.judge_model.as_str(),
        temperature: config.judge_temperature,
        max_tokens: config.judge_max_tokens,
        fallback_score: config.fallback_quality_score,
    })
    .await;

    // ── (i) Store training pair when quality is high ──────────────────────────
    if let Some(db) = db {
        let overall_confidence = confidence_signal.score as f64;
        if overall_confidence >= config.training_pair_min_confidence
            && citations.len() >= config.training_pair_min_citations
        {
            let _ = db.store_training_pair(
                &query.query,
                &answer,
                &answer,
                Some(overall_confidence),
                &format!("research_session_{}", session_id),
            );
        }
    }

    // ── (j) Persistence ──────────────────────────────────────────────────────
    if persist_enabled
        && query.persist_to_docs
        && confidence_signal.score as f64 >= config.persist_min_confidence
        && answer.len() >= 200
    {
        use std::path::Path;
        if let Ok(root_str) = std::env::var("VOX_WORKSPACE_ROOT") {
            let root = Path::new(&root_str);
            let slug = super::persistence::slug_from_query(&query.query);
            let _ = super::persistence::write_research_doc(
                root,
                &slug,
                &query.query,
                &answer,
                resolved_llm.synthesis_model.as_str(),
            );
        }
    }

    // ── (k2) Write research summary to agent memory ─────────────────────────
    // This bridges the research subsystem and the memory subsystem so that
    // `vox_memory_search` and `vox_memory_recall_db` can surface past research.
    if let Some(db) = db {
        if answer.len() >= 100 {
            let summary = format!(
                "Research: {} | Confidence: {:.2} | Quality: {} | Sources: {} | Answer: {}",
                &query.query,
                confidence_signal.score,
                quality_score,
                all_hits.len(),
                &answer.chars().take(500).collect::<String>()
            );
            let _ = db
                .save_memory(vox_db::MemoryParams {
                    agent_id: "research_pipeline",
                    session_id: &format!("research_{}", session_id),
                    memory_type: "research_result",
                    content: &summary,
                    metadata: Some(&serde_json::json!({
                        "session_id": session_id,
                        "query": &query.query,
                        "confidence": confidence_signal.score,
                        "quality_score": quality_score,
                        "source_count": all_hits.len(),
                        "routing_tier": format!("{:?}", routing_tier),
                    }).to_string()),
                    importance: (confidence_signal.score as f64 * quality_score as f64 / 100.0)
                        .clamp(0.1, 1.0),
                    vcs_snapshot_id: None,
                })
                .await;
        }
    }

    // ── (k) Finalize session ──────────────────────────────────────────────────
    let duration_ms = start.elapsed().as_millis() as u64;
    if let Some(db) = db {
        let _ = db.record_research_metric(
            session_id,
            "end_to_end_latency_ms",
            Some(registry.primary_name()),
            &json!({ "ms": duration_ms, "source_count": all_hits.len() }),
        );
        let _ = db.record_research_metric(
            session_id,
            "quality_score",
            None,
            &json!({ "score": quality_score }),
        );
        let _ = db.update_research_session_status(session_id, "completed");
    }

    // ── (l) Optional CoVE-style self-verification ─────────────────────────────
    let self_verification = if self_verification_enabled && !answer.is_empty() && !all_hits.is_empty() {
        report_progress("Running self-verification step...".to_string(), Some(0.95));
        Some(run_self_verification(
            &query.query,
            &answer,
            &all_hits,
            config.llm_endpoint.as_deref(),
            config.api_key.as_deref(),
            resolved_llm.synthesis_model.as_str(),
        ).await)
    } else {
        None
    };

    // ── (m) Compute CompetenceSignal ──────────────────────────────────────────
    let competence = Some(CompetenceSignal::from_verdicts(
        confidence_signal.score,
        quality_score,
        &claim_verdicts,
        // "verified" = we ran NLI classification on at least one claim
        !claim_verdicts.is_empty(),
    ));

    let metadata = ResearchMetadata {
        session_id,
        duration_ms,
        provider: registry.primary_name().to_string(),
        routing_tier,
        confidence: confidence_signal.score as f64,
        subquery_count: plan.subqueries.len(),
        source_count: all_hits.len(),
        claim_verdicts: claim_verdicts.clone(),
        retrieval_diagnostics: diagnostics,
        quality_score,
        competence,
        self_verification,
    };

    Ok(ResearchResult {
        answer,
        sources: all_hits,
        citations,
        research_metadata: metadata,
    })
}

struct JudgeParams<'a> {
    pub query: &'a str,
    pub answer: &'a str,
    pub citations: &'a [Citation],
    pub endpoint: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub temperature: f32,
    pub max_tokens: u32,
    pub fallback_score: i32,
}

async fn judge_quality(params: JudgeParams<'_>) -> i32 {
    let Some(ep) = params.endpoint else {
        return params.fallback_score;
    };
    let Some(key) = params.api_key else {
        return params.fallback_score;
    };

    let citation_snippets: String = params.citations
        .iter()
        .take(5)
        .map(|c| {
            format!(
                "- {} <{}>: {}",
                c.title,
                c.url,
                c.snippet.chars().take(200).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sys_prompt = "You are a research quality evaluator. Score the following answer strictly based on the rubric.
You MUST output your evaluation as a valid JSON object embedded in a ```json codeblock. Do not output anything else.

Schema required:
{
  \"factual_accuracy_reasoning\": \"string\",
  \"factual_accuracy_score\": integer (0-33),
  \"citation_density_reasoning\": \"string\",
  \"citation_density_score\": integer (0-33),
  \"coverage_reasoning\": \"string\",
  \"coverage_score\": integer (0-34),
  \"total_score\": integer (0-100)
}
{}";
    let sys_prompt = sys_prompt.replace("{}", ANTI_LAZINESS_RIDER);

    let user_prompt = format!(
        "Query: {}
Answer: {}

Citations used:
{}

Scoring rubric:
1. Factual accuracy: Does the answer align with the cited sources?
2. Citation density: Are key claims backed by at least one citation?
3. Coverage: Does the answer address all major aspects of the query?",
        sanitize_chatml(params.query),
        sanitize_chatml(params.answer),
        sanitize_chatml(&citation_snippets)
    );

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", ep.trim_end_matches('/'));
    let res = client
        .post(&url)
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": params.model,
            "messages": [
                {"role": "system", "content": sys_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "max_tokens": params.max_tokens,
            "temperature": params.temperature
        }))
        .send()
        .await;

    if let Ok(resp) = res
        && let Ok(json) = resp.json::<serde_json::Value>().await
        && let Some(content) = json
            .pointer("/choices/0/message/content")
            .and_then(|v| v.as_str())
    {
        let mut block = content;
        if let Some(start) = content.find("```json") {
            let rest = &content[start + 7..];
            if let Some(end) = rest.find("```") {
                block = &rest[..end];
            } else {
                block = rest;
            }
        } else if let Some(start) = content.find("```") {
            let rest = &content[start + 3..];
            if let Some(end) = rest.find("```") {
                block = &rest[..end];
            } else {
                block = rest;
            }
        }

        #[derive(serde::Deserialize)]
        struct JudgeResponse {
            #[serde(default)]
            total_score: i32,
        }

        if let Ok(parsed) = serde_json::from_str::<JudgeResponse>(block.trim()) {
            if parsed.total_score > 0 {
                return parsed.total_score.clamp(1, 100);
            }
        }
    }

    params.fallback_score
}

struct SynthesisParams<'a> {
    pub query: &'a str,
    pub hits: &'a [ResearchHit],
    pub verdicts: &'a [super::types::ClaimVerdict],
    pub endpoint: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub temperature: f32,
    pub max_tokens: u32,
    pub context_max_chars: usize,
}

// ── Answer synthesis ─────────────────────────────────────────────────────────

/// LLM-backed synthesis. Falls back to template when no endpoint is configured.
async fn synthesize_answer_with_llm(params: SynthesisParams<'_>) -> String {
    if params.hits.is_empty() {
        return format!(
            "No external sources were found for: **{}**. \
             Answering from internal knowledge only.",
            params.query
        );
    }

    // Try LLM synthesis first.
    if let (Some(_ep), Some(_key)) = (params.endpoint, params.api_key) {
        match call_synthesis_llm(&params).await {
            Ok(answer) => return answer,
            Err(e) => tracing::warn!("LLM synthesis failed: {e}, falling back to template"),
        }
    }

    // Template fallback.
    synthesize_answer_template(params.query, params.hits, params.verdicts)
}

async fn call_synthesis_llm(params: &SynthesisParams<'_>) -> anyhow::Result<String> {
    let mut context_budget = params.context_max_chars;

    // Build evidence context from hits.
    let evidence: String = params.hits
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let snippet = sanitize_evidence(&h.snippet.chars().take(600).collect::<String>());
            format!("[{}] {}\nURL: {}\n{}\n", i + 1, h.title, h.url, snippet)
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Truncate to budget.
    let evidence_text: String = evidence.chars().take(context_budget).collect();
    context_budget = context_budget.saturating_sub(evidence_text.len());

    // Append verdict summary if room remains.
    let verdict_text: String = if !params.verdicts.is_empty() && context_budget > 100 {
        params.verdicts
            .iter()
            .map(|v| {
                format!(
                    "{}: {} ({:.0}% confidence)",
                    v.claim.text,
                    v.verdict,
                    v.confidence * 100.0
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    } else {
        String::new()
    };

    let (endpoint, api_key) = (params.endpoint.unwrap(), params.api_key.unwrap());

    let system = format!(
        "You are a precise research synthesizer. Using ONLY the provided evidence \
         snippets, write a thorough, well-structured answer to the user's question. \
         Cite sources inline as [1], [2], etc. matching the evidence numbers. \
         If evidence is insufficient, say so clearly.\n{}",
        ANTI_LAZINESS_RIDER
    );

    let user = format!(
        "Question: {}\n\nEvidence:\n{}{verdict_section}",
        params.query,
        evidence_text,
        verdict_section = if verdict_text.is_empty() {
            String::new()
        } else {
            format!("\n\nClaim verdicts: {verdict_text}")
        }
    );

    let url = format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'));
    let resp = reqwest::Client::new()
        .post(&url)
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": params.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user}
            ],
            "max_tokens": params.max_tokens,
            "temperature": params.temperature
        }))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("synthesis request: {e}"))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| anyhow::anyhow!("synthesis parse: {e}"))?;

    let content = json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("no content in synthesis response"))?;

    Ok(content.to_string())
}

/// Template synthesis fallback (always succeeds, no network call).
fn synthesize_answer_template(
    query: &str,
    hits: &[ResearchHit],
    verdicts: &[super::types::ClaimVerdict],
) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(format!("# Research Findings: {query}\n"));

    if !verdicts.is_empty() {
        parts.push("## Verification Status\n".to_string());
        for verdict in verdicts {
            let icon = match verdict.verdict {
                super::types::Verdict::Supported => "✅",
                super::types::Verdict::Contradicted => "❌",
                super::types::Verdict::Contested => "⚠️",
                super::types::Verdict::Unverified => "❓",
            };
            parts.push(format!(
                "- {icon} **{}**: {} (confidence: {:.0}%)",
                verdict.claim.text,
                verdict.verdict,
                verdict.confidence * 100.0
            ));
        }
        parts.push(String::new());
    }

    parts.push("## Evidence Summary\n".to_string());
    for (i, hit) in hits.iter().take(5).enumerate() {
        let snippet = hit.snippet.chars().take(500).collect::<String>();
        parts.push(format!(
            "### [{}] {}\n\nSource: <{}>\n\n{}\n",
            i + 1,
            hit.title,
            hit.url,
            snippet
        ));
    }
    if hits.len() > 5 {
        parts.push(format!(
            "*And {} other sources examined.*\n",
            hits.len() - 5
        ));
    }

    parts.push("## Citations\n".to_string());
    for (i, hit) in hits.iter().take(10).enumerate() {
        parts.push(format!(
            "{}. [^source{}]: {} - <{}>",
            i + 1,
            i + 1,
            hit.title,
            hit.url
        ));
    }

    parts.join("\n")
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// FNV-1a 64-bit hash used to generate stable `claim_id` values from claim text.
///
/// No external dependency — uses the FNV-1a algorithm (public domain).
fn fnv1a_hash(text: &str) -> u64 {
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;
    let mut hash = FNV_OFFSET;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// CoVE-style self-verification step.
///
/// Generates simple verification questions from the draft answer, answers them
/// using the retrieved context only, and counts inconsistencies.
///
/// This is intentionally lightweight: the goal is a "consistency signal" that
/// can downgrade confidence when the model's own draft contradicts the sources,
/// not a full independent fact-check.
async fn run_self_verification(
    _query: &str,
    answer: &str,
    hits: &[ResearchHit],
    endpoint: Option<&str>,
    api_key: Option<&str>,
    model: &str,
) -> SelfVerificationResult {
    let Some(ep) = endpoint else {
        return SelfVerificationResult {
            checked: false,
            questions_generated: 0,
            inconsistency_count: 0,
            critical_inconsistency: false,
        };
    };
    let Some(key) = api_key else {
        return SelfVerificationResult {
            checked: false,
            questions_generated: 0,
            inconsistency_count: 0,
            critical_inconsistency: false,
        };
    };

    // Build a compact context from top-5 hits.
    let context: String = hits
        .iter()
        .take(5)
        .map(|h| format!("- {} — {}", h.title, h.snippet.chars().take(300).collect::<String>()))
        .collect::<Vec<_>>()
        .join("\n");

    // Step 1: Ask the model to generate verification questions from the draft.
    let question_prompt = format!(
        "Given the following research answer, generate up to 5 yes/no verification questions \
that target specific factual claims in the answer. Return one question per line, no numbering.\n\n\
Answer: {answer}\n\nQuestions:"
    );

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", ep.trim_end_matches('/'));

    let question_res = client
        .post(&url)
        .bearer_auth(key)
        .json(&serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": question_prompt}],
            "max_tokens": 300,
            "temperature": 0.3
        }))
        .send()
        .await;

    let questions: Vec<String> = if let Ok(resp) = question_res
        && let Ok(json) = resp.json::<serde_json::Value>().await
        && let Some(content) = json.pointer("/choices/0/message/content").and_then(|v| v.as_str())
    {
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(5)
            .map(|l| l.trim().to_string())
            .collect()
    } else {
        return SelfVerificationResult {
            checked: true,
            questions_generated: 0,
            inconsistency_count: 0,
            critical_inconsistency: false,
        };
    };

    let questions_generated = questions.len();
    if questions_generated == 0 {
        return SelfVerificationResult {
            checked: true,
            questions_generated: 0,
            inconsistency_count: 0,
            critical_inconsistency: false,
        };
    }

    // Step 2: Answer each question from the retrieved context only and check consistency.
    let mut inconsistency_count = 0usize;
    for q in &questions {
        let verify_prompt = format!(
            "Based ONLY on the following sources, answer this yes/no question.\n\
Sources:\n{context}\n\nQuestion: {q}\n\nAnswer with only 'yes', 'no', or 'unknown'."
        );
        let ans_res = client
            .post(&url)
            .bearer_auth(key)
            .json(&serde_json::json!({
                "model": model,
                "messages": [{"role": "user", "content": verify_prompt}],
                "max_tokens": 10,
                "temperature": 0.0
            }))
            .send()
            .await;

        if let Ok(resp) = ans_res
            && let Ok(json) = resp.json::<serde_json::Value>().await
            && let Some(ans) = json.pointer("/choices/0/message/content").and_then(|v| v.as_str())
        {
            let cleaned = ans.trim().to_lowercase();
            // "unknown" counts as a soft inconsistency (answer claimed something the context can't confirm)
            if cleaned.contains("no") || cleaned.contains("unknown") {
                inconsistency_count += 1;
            }
        }
    }

    let critical_inconsistency = inconsistency_count > questions_generated / 2;
    SelfVerificationResult {
        checked: true,
        questions_generated,
        inconsistency_count,
        critical_inconsistency,
    }
}
