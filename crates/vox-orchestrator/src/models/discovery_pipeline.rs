//! Closed-loop model auto-discovery → eligibility (Track C).
//!
//! Catalog ingestion (OpenRouter fetch → classify → admission → persist) already
//! exists. The missing piece is the loop that (a) indexes which discovered models
//! still owe an evaluation and (b) flips routing eligibility from a static
//! *pricing-source proxy* to **real `model_scoreboard` evidence** via the
//! autonomic [`super::autonomic::should_promote`] state machine.
//!
//! Both functions here are pure (no network, no DB, no config-cache I/O — the
//! caller supplies the registry snapshot, scoreboard rows, and retired set), so
//! the discovery loop is fully unit-testable headless against a captured catalog
//! fixture + an in-memory scoreboard. Model-agnostic: keys only on
//! `PricingSource` + scoreboard evidence, never a vendor hostname.

use std::collections::HashSet;

use vox_db::store::types::ModelScoreboardRow;

use super::autonomic::{ModelConfidence, should_promote};
use super::spec::{ModelSpec, PricingSource};

/// Base routing confidence implied purely by a model's pricing source — the
/// pre-existing heuristic, minus the retired/pins check (callers handle retired
/// separately). This is the `scoreboard: None` answer of [`resolve_eligibility`].
fn base_confidence_from_pricing(m: &ModelSpec) -> ModelConfidence {
    match m.pricing_source {
        PricingSource::Telemetry | PricingSource::UserConfig => ModelConfidence::Confirmed,
        PricingSource::Unknown => ModelConfidence::Provisional,
        PricingSource::LiteLLM
        | PricingSource::OpenRouter
        | PricingSource::AnthropicDirect
        | PricingSource::Bootstrap => ModelConfidence::Shadowed,
    }
}

/// Decide a model's routing confidence from **real scoreboard evidence** when a
/// row exists, falling back to the pricing-source heuristic otherwise.
///
/// With `scoreboard: None` this returns exactly the legacy pricing-source state,
/// so wiring it behind `confidence_state_for_model` is a zero-behavior-change
/// refactor. With a row, it walks the autonomic promotion chain
/// (Provisional→Shadowed→Confirmed) using the row's successful-call count,
/// p50 latency, and quality score — so a freshly-discovered OpenRouter model
/// (which starts `Shadowed`) becomes `Confirmed` once it has earned the evidence,
/// with no manual registry edit.
pub fn resolve_eligibility(
    m: &ModelSpec,
    scoreboard: Option<&ModelScoreboardRow>,
    catalog_median_p50_ms: f64,
) -> ModelConfidence {
    let base = base_confidence_from_pricing(m);
    let Some(row) = scoreboard else {
        return base;
    };
    let successful = row.success_count.max(0) as u32;
    let p50 = row.p50_latency_ms.unwrap_or(0).max(0) as f64;
    let classifier_confidence = row.quality_score as f32;

    let mut state = base;
    while let Some(next) = should_promote(
        state,
        successful,
        p50,
        catalog_median_p50_ms,
        classifier_confidence,
    ) {
        state = next;
    }
    state
}

/// Routing confidence from every piece of evidence the registry holds, strongest first:
///
/// 1. a graded benchmark verdict (`vox models bench`, Wilson interval on the pass rate) —
///    confirms *or* holds back a model regardless of where its price came from;
/// 2. an independent public benchmark (Artificial Analysis intelligence index) or a
///    user-installed local provider — both are assessed without Vox having to spend;
/// 3. scoreboard call evidence, walked through the autonomic promotion chain;
/// 4. the pricing-source heuristic.
///
/// The scoreboard's `quality_score` is deliberately not used: it is not graded.
pub fn resolve_eligibility_from_evidence(
    m: &ModelSpec,
    score: Option<&super::registry::ModelScore>,
    bench: Option<&super::bench_evidence::BenchEvidence>,
) -> ModelConfidence {
    if let Some(verdict) = bench.and_then(super::bench_evidence::BenchEvidence::verdict) {
        return verdict;
    }
    let mut state = base_confidence_from_pricing(m);
    if m.capabilities.intelligence_index.is_some_and(|i| i > 0.0)
        || crate::route_policy::is_local_http_provider(&m.provider_type)
    {
        state = ModelConfidence::Confirmed;
    }
    let Some(s) = score else {
        return state;
    };
    let successful = s.success_count.clamp(0, i64::from(u32::MAX)) as u32;
    let p50 = s.p50_latency_ms.unwrap_or(0).max(0) as f64;
    while let Some(next) = should_promote(state, successful, p50, 0.0, 0.0) {
        state = next;
    }
    state
}

/// The discovery backlog: ids of models that are discovered-but-unconfirmed and
/// have no scoreboard row yet — i.e. they owe an evaluation. A pure derivation
/// over the already-persisted catalog snapshot + the set of model ids that
/// already have scoreboard rows + the retired set (no new DB table needed).
pub fn pending_eval_candidates(
    models: &[ModelSpec],
    models_with_scoreboard: &HashSet<String>,
    retired: &HashSet<String>,
) -> Vec<String> {
    models
        .iter()
        .filter(|m| !retired.contains(&m.id))
        .filter(|m| !models_with_scoreboard.contains(&m.id))
        .filter(|m| base_confidence_from_pricing(m) != ModelConfidence::Confirmed)
        .map(|m| m.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ModelCapabilities, ModelTier, ProviderType, StrengthTag};

    fn spec(id: &str, pricing_source: PricingSource) -> ModelSpec {
        ModelSpec {
            id: id.to_string(),
            canonical_slug: String::new(),
            provider: "test".to_string(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 32_000,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            observed_cost_per_1k: None,
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source,
            is_free: false,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                max_context: 32_000,
                tier: ModelTier::Pro,
                ..Default::default()
            },
            supported_parameters: vec![],
        }
    }

    fn scoreboard_row(
        model_id: &str,
        success_count: i64,
        p50_ms: i64,
        quality: f64,
    ) -> ModelScoreboardRow {
        ModelScoreboardRow {
            model_id: model_id.to_string(),
            task_category: "codegen".to_string(),
            strength_tag: "generalist".to_string(),
            window_days: 7,
            n_calls: success_count,
            success_rate: 1.0,
            p50_latency_ms: Some(p50_ms),
            p99_latency_ms: Some(p50_ms * 2),
            cost_per_success_usd: Some(0.0),
            quality_score: quality,
            updated_at_ms: 0,
            success_count,
            cumulative_cost_usd: 0.0,
            p95_ttft_ms: None,
            p95_tpot_ms: None,
            goodput_tokens_per_sec: None,
        }
    }

    #[test]
    fn resolve_without_scoreboard_matches_pricing_heuristic() {
        assert_eq!(
            resolve_eligibility(&spec("a", PricingSource::OpenRouter), None, 0.0),
            ModelConfidence::Shadowed
        );
        assert_eq!(
            resolve_eligibility(&spec("b", PricingSource::Telemetry), None, 0.0),
            ModelConfidence::Confirmed
        );
        assert_eq!(
            resolve_eligibility(&spec("c", PricingSource::Unknown), None, 0.0),
            ModelConfidence::Provisional
        );
    }

    #[test]
    fn resolve_promotes_shadowed_openrouter_on_strong_scoreboard_evidence() {
        let m = spec("acme/frontier", PricingSource::OpenRouter);
        let row = scoreboard_row("acme/frontier", 1000, 10, 1.0);
        // Strong evidence: 1000 successful calls, 10ms p50 vs a 10s catalog median.
        assert_eq!(
            resolve_eligibility(&m, Some(&row), 10_000.0),
            ModelConfidence::Confirmed,
            "a discovered OpenRouter model with strong evidence must become Confirmed"
        );
    }

    #[test]
    fn resolve_keeps_shadowed_on_weak_scoreboard_evidence() {
        let m = spec("acme/frontier", PricingSource::OpenRouter);
        let row = scoreboard_row("acme/frontier", 0, 10, 1.0); // no successful calls yet
        assert_eq!(
            resolve_eligibility(&m, Some(&row), 10_000.0),
            ModelConfidence::Shadowed,
            "without successful calls the model must stay Shadowed (not routing-eligible)"
        );
    }

    #[test]
    fn benchmark_verdict_outranks_price_provenance_both_ways() {
        use crate::models::bench_evidence::evidence;
        let discovered = spec("acme/new", PricingSource::OpenRouter);
        assert_eq!(
            resolve_eligibility_from_evidence(&discovered, None, Some(&evidence(20, 18, 0.9))),
            ModelConfidence::Confirmed,
            "a discovered model that passes the bench becomes routable"
        );
        let mut indexed = spec("acme/indexed", PricingSource::Telemetry);
        indexed.capabilities.intelligence_index = Some(50.0);
        assert_eq!(
            resolve_eligibility_from_evidence(&indexed, None, Some(&evidence(30, 0, 0.0))),
            ModelConfidence::Shadowed,
            "a model that fails the bench is held back even with telemetry pricing and an index"
        );
    }

    #[test]
    fn external_index_and_local_providers_are_routable_without_spend() {
        let mut indexed = spec("acme/indexed", PricingSource::OpenRouter);
        assert_eq!(
            resolve_eligibility_from_evidence(&indexed, None, None),
            ModelConfidence::Shadowed
        );
        indexed.capabilities.intelligence_index = Some(41.0);
        assert_eq!(
            resolve_eligibility_from_evidence(&indexed, None, None),
            ModelConfidence::Confirmed
        );
        let mut local = spec("qwen3:8b", PricingSource::Bootstrap);
        local.provider_type = ProviderType::Ollama;
        assert_eq!(
            resolve_eligibility_from_evidence(&local, None, None),
            ModelConfidence::Confirmed
        );
    }

    #[test]
    fn scoreboard_calls_promote_without_reading_ungraded_quality() {
        let m = spec("acme/used", PricingSource::OpenRouter);
        let used = crate::models::ModelScore {
            n_calls: 40,
            success_count: 40,
            success_rate: 1.0,
            quality_score: 0.0,
            ..Default::default()
        };
        assert_eq!(
            resolve_eligibility_from_evidence(&m, Some(&used), None),
            ModelConfidence::Confirmed
        );
        let barely = crate::models::ModelScore {
            n_calls: 3,
            success_count: 3,
            quality_score: 1.0,
            ..Default::default()
        };
        assert_eq!(
            resolve_eligibility_from_evidence(&m, Some(&barely), None),
            ModelConfidence::Shadowed
        );
    }

    #[test]
    fn pending_eval_candidates_excludes_confirmed_retired_and_scored() {
        let models = vec![
            spec("confirmed/telemetry", PricingSource::Telemetry),
            spec("shadowed/no-scoreboard", PricingSource::OpenRouter),
            spec("shadowed/has-scoreboard", PricingSource::OpenRouter),
            spec("retired/openrouter", PricingSource::OpenRouter),
        ];
        let with_scoreboard: HashSet<String> = ["shadowed/has-scoreboard".to_string()].into();
        let retired: HashSet<String> = ["retired/openrouter".to_string()].into();

        let got = pending_eval_candidates(&models, &with_scoreboard, &retired);
        assert_eq!(got, vec!["shadowed/no-scoreboard".to_string()]);
    }
}
