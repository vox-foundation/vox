use std::time::Instant;

use anyhow::Result;
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
// PHASE_0a_STUB: re-import in Phase 1 when cache is re-enabled.
use super::super::verifier::verify_claims_with_config;

/// Run the full research pipeline for `query`. Returns a `ResearchResult` with
/// a cited answer, sources, citations, and telemetry metadata.
pub async fn run_research(
    mut query: ResearchQuery,
    db: Option<&Codex>,
    config: &ResearchConfig,
) -> Result<ResearchResult> {
    // ── (0) Check Rollout Flags ──────────────────────────────────────────────
    let web_enabled = true;
    let claim_enabled = config.claim_detection_enabled;
    // PHASE_0a_STUB: rollout flag queries not yet wired to Codex; always use defaults.
    // Phase 1 re-enables after vox_db gains get_retrieval_config.
    let persist_enabled = true;
    let self_verification_enabled = false;

    let _ = db; // suppress unused warning during phase 0a stub period

    if !web_enabled && query.scope == ResearchScope::Web {
        query.scope = ResearchScope::Both;
    }
    if !claim_enabled {
        query.verify_claims = false;
    }
    let _ = web_enabled; // used above

    let registry = ProviderRegistry::from_env_with_config(config.provider.clone());
    let llm_model_registry = crate::models::ModelRegistry::new();
    let base_inference = config
        .model_pick_inference
        .clone()
        .unwrap_or_default();
    let resolved_llm =
        super::super::model_select::resolve_research_models(&llm_model_registry, &base_inference);
    let research_verifier_cfg = verifier_config_for_research_run(&config.verifier, &resolved_llm);
    tracing::info!(
        target: "vox_orchestrator::model_route",
        route_source = "research_pipeline",
        planner = %resolved_llm.planner_model,
        claim = %resolved_llm.claim_model,
        synthesis = %resolved_llm.synthesis_model,
        judge = %resolved_llm.judge_model,
        "research_models_resolved"
    );
    let start = Instant::now();

    // ── (0b) Research cache check ────────────────────────────────────────────
    // PHASE_0a_STUB: cache short-circuit disabled (list_memories_by_type not yet in vox_db).
    // Phase 1 re-enables after vox_db gains list_memories_by_type.
    let _ = &query; // keep query alive through cache bypass
    // (no cache check in Phase 0a)

    // ── (a) Session tracking ─────────────────────────────────────────────────
    // PHASE_0a_STUB: research session DB writes not yet wired.
    // Phase 1 re-enables after vox_db gains create_research_session.
    let session_id: i64 = 0;

    let report_progress = |msg: String, pct: Option<f32>| {
        if let Some(ref cb) = config.progress_callback {
            cb(msg, pct);
        }
    };
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
        verify_claims_with_config(
            &draft_claims,
            &query.query,
            &registry,
            &research_verifier_cfg,
            config.llm_endpoint.as_deref(),
            config.api_key.as_deref(),
        )
        .await
        // PHASE_0a_STUB: claim/verdict persistence not yet wired to vox_db.
        // Phase 1 re-enables after vox_db gains store_claim / store_claim_verdict / store_evidence_span.
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

    // PHASE_0a_STUB: training pair persistence not yet wired to vox_db.
    // Phase 1 re-enables after vox_db gains store_training_pair.

    // ── (j) Persistence ──────────────────────────────────────────────────────
    if persist_enabled
        && query.persist_to_docs
        && confidence_signal.score as f64 >= config.persist_min_confidence
        && answer.len() >= 200
    {
        use std::path::Path;
        if let Some(root_str) = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxWorkspaceRoot).expose() {
            let root = Path::new(root_str);
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

    // PHASE_0a_STUB: memory write not yet wired (save_memory API shape confirmed but
    // session management methods missing from vox_db).
    // Phase 1 re-enables after vox_db gains create_research_session etc.

    // ── (k) Finalize session ──────────────────────────────────────────────────
    let duration_ms = start.elapsed().as_millis() as u64;

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
