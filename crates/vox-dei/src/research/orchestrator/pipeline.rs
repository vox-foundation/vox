use std::time::Instant;

use anyhow::Result;
use serde_json::json;
use vox_db::Codex;

use super::config::ResearchConfig;
use super::helpers::{fnv1a_hash, verifier_config_for_research_run};
use super::stages::{
    judge_quality, run_self_verification, synthesize_answer_with_llm, JudgeParams, SynthesisParams,
};
use super::web_gather::gather_web_hits_for_plan;
use super::super::claims::extract_claims_with_model;
use super::super::gate::{GateInput, score_with_config};
use super::super::planner::decompose_query_with_config;
use super::super::provider::ProviderRegistry;
use super::super::types::{
    Citation, CompetenceSignal, ResearchHit, ResearchMetadata, ResearchPlan, ResearchQuery,
    ResearchResult, ResearchScope, RetrievalDiagnostics, RoutingTier,
};
use super::pipeline_cache::research_cache_short_circuit;
use super::super::verifier::verify_claims_with_config;

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
        super::super::model_select::resolve_research_models(&llm_model_registry, &base_inference);
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
        if let Some(cached) = research_cache_short_circuit(&query, db).await {
            return Ok(cached);
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
        let plan_json = super::super::planner::plan_to_json(&plan);
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
        let (h, s, d, t) = gather_web_hits_for_plan(
            db,
            session_id,
            &query,
            &plan,
            &registry,
            config,
        )
        .await;
        all_hits = h;
        subqueries_with_hits = s;
        total_dropped_count = d;
        total_sources_attempted = t;
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
            let slug = super::super::persistence::slug_from_query(&query.query);
            let _ = super::super::persistence::write_research_doc(
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
    let self_verification = if self_verification_enabled && !answer.is_empty() && !all_hits.is_empty()
    {
        report_progress("Running self-verification step...".to_string(), Some(0.95));
        Some(
            run_self_verification(
                &query.query,
                &answer,
                &all_hits,
                config.llm_endpoint.as_deref(),
                config.api_key.as_deref(),
                resolved_llm.synthesis_model.as_str(),
            )
            .await,
        )
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
