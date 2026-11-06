pub(super) use std::cmp::Ord;
use std::collections::HashMap;
pub(super) use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) use crate::catalog::{ModelCatalog, OpenRouterCatalog};
pub(super) use crate::config::CostPreference;
use crate::types::TaskCategory;

pub(super) use super::RoutingTask;

pub(super) use super::health::MODEL_CATALOG_LAST_REFRESH_KEY;
pub(super) use super::spec::{
    ModelCapabilities, ModelConfig, ModelSpec, PricingSource, ProviderType, task_category_strength,
};
/// Receipt for one step of a multi-spoke [`AgentHarness`] + Axis development workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiSpokeHarnessStepReceipt {
    pub step_index: usize,
    pub classified_spoke: vox_corpus::training::contract::OrganicSpokeKind,
    pub task_category: TaskCategory,
    pub complexity: u8,
    pub routed_model_id: String,
    pub provider_type: ProviderType,
    pub oracle_passed: bool,
    pub deep_compilation_passed: bool,
    pub constructs_covered: Vec<String>,
}

/// Summary report of an 8-step multi-spoke [`AgentHarness`] + Axis local LLM workflow.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiSpokeHarnessWorkflowReport {
    pub total_steps: usize,
    pub spokes_wielded: usize,
    pub admitted_sft_records: usize,
    pub all_steps_passed: bool,
    pub steps: Vec<MultiSpokeHarnessStepReceipt>,
}

/// How a multi-turn human utterance relates to prior conversation context in [`ConversationalHarnessSession`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConversationalTurnMode {
    /// Standalone natural-language request introducing a new task or spoke.
    FreshIntent,
    /// Follow-up turn referencing earlier turns/artifacts ("in that file", "what we just built").
    ContextualContinuation,
    /// Follow-up turn reporting an error or requesting a fix ("wait, that gave a borrow error—fix it"),
    /// which escalates Axis routing to `SelectionIntent::repair_loop()`.
    SelfRepairLoop,
}

/// Detailed routing telemetry for a single turn in an extended human conversation.
#[derive(Debug, Clone, PartialEq)]
pub struct ConversationalRoutingDecision {
    pub chosen_spoke: vox_corpus::training::contract::OrganicSpokeKind,
    pub turn_mode: ConversationalTurnMode,
    pub task_category: TaskCategory,
    pub complexity: u8,
    pub primary_score: i32,
    pub runner_up_spoke: Option<vox_corpus::training::contract::OrganicSpokeKind>,
    pub runner_up_score: i32,
    pub matched_signals: Vec<String>,
    pub used_prior_turn_context: bool,
    pub routed_model_id: String,
    pub routed_provider: ProviderType,
    pub selection_rationale: String,
}

/// Receipt for one turn of an extended multi-turn human conversation through Axis + [`AgentHarness`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConversationalTurnReceipt {
    pub turn_index: usize,
    pub user_prompt: String,
    pub routing: ConversationalRoutingDecision,
    pub assistant_reply: String,
    pub assistant_artifact: String,
    pub oracle_passed: bool,
    pub deep_compilation_passed: bool,
    pub constructs_covered: Vec<String>,
    pub repaired_from_rejected: Option<String>,
    pub admitted_sft: bool,
    pub emitted_dpo: bool,
}

/// End-to-end report for an extended multi-turn human conversation across all 8 spokes.
#[derive(Debug, Clone, PartialEq)]
pub struct ConversationalSessionReport {
    pub session_id: String,
    pub total_turns: usize,
    pub unique_spokes_used: usize,
    pub context_carryover_turns: usize,
    pub self_repair_turns: usize,
    pub sft_records_admitted: usize,
    pub dpo_records_emitted: usize,
    pub all_turns_verified: bool,
    pub turns: Vec<ConversationalTurnReceipt>,
    pub dpo_contrast_records: Vec<vox_corpus::training::contract::TrainingRecordV2>,
}

/// Before/After SOTA Agent Harness scorecard (`SOTA-FLYWHEEL-02..04`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SotaHarnessScorecardV1 {
    pub schema_version: String,
    pub legacy_tool_schema_cap: usize,
    pub sota_ranked_tool_schema_cap: usize,
    pub tool_schema_token_reduction_pct: f64,
    pub legacy_unpinned_late_alphabet_tool_recall: f64,
    pub sota_ranked_late_alphabet_tool_recall: f64,
    pub legacy_inline_verified_file_classes: usize,
    pub sota_inline_verified_file_classes: usize,
    pub observation_masking_tokens_saved: usize,
    pub kv_cache_prefix_overlap_ratio: f64,
    pub kv_cache_hit_predicted: bool,
    pub working_memory_artifacts_preserved: bool,
    pub extended_session_turns_verified: usize,
    pub extended_session_spokes_covered: usize,
    pub extended_session_context_carryover_turns: usize,
    pub extended_session_self_repairs: usize,
    pub live_sft_records_admitted: usize,
    pub live_dpo_contrast_pairs_harvested: usize,
    pub single_pass_sse_tool_calls_enabled: bool,
    pub tool_turn_http_requests_per_iteration_before: usize,
    pub tool_turn_http_requests_per_iteration_after: usize,
    pub provider_tool_loop_coverage_after: f64,
    pub symbol_skeleton_compression_ratio: f64,
    pub system_prompt_kv_prefix_stable: bool,
    pub streaming_cache_read_accounting_enabled: bool,
    pub intent_gated_preamble_enabled: bool,
    pub intraturn_tool_masking_enabled: bool,
    pub progressive_tool_schema_compression_pct: f64,
    pub wire_cache_breakpoints_enabled: bool,
    pub streaming_goodput_feedback_closed_loop: bool,
    pub cache_aware_efficiency_routing_enabled: bool,
    pub intraturn_read_tool_dedup_enabled: bool,
    pub rust_structural_ast_verification_enabled: bool,
    pub rolling_intraturn_cache_breakpoints_enabled: bool,
    pub live_context_fill_and_spoke_router_wired: bool,
    pub path_scoped_read_memo_invalidation_enabled: bool,
    pub edit_window_recovery_enabled: bool,
    pub adversarial_48_case_spoke_pass_rate: f64,
    pub write_tool_priority_and_target_content_recovery_enabled: bool,
    pub prefix_immutable_kv_cache_tool_loops_enabled: bool,
    pub universal_multi_format_tool_salvage_enabled: bool,
    pub syn_ast_rust_structural_verification_enabled: bool,
    pub speculative_repair_deescalation_enabled: bool,
    pub adversarial_64_case_spoke_pass_rate: f64,
}

/// A performance score from the `model_scoreboard`.
#[derive(Debug, Clone, Default)]
pub struct ModelScore {
    pub success_rate: f64,
    pub quality_score: f64,
    pub cost_per_success_usd: Option<f64>,
    pub p50_latency_ms: Option<i64>,
    pub n_calls: i64,
    pub success_count: i64,
    /// Task M3: p95 time-to-first-token, ms. See [`vox_db::store::types::ModelScoreboardRow::p95_ttft_ms`].
    pub p95_ttft_ms: Option<i64>,
    /// Task M3: p95 time-per-output-token, ms.
    pub p95_tpot_ms: Option<f64>,
    /// Task M3: mean successful-call throughput, tokens/sec ("goodput").
    pub goodput_tokens_per_sec: Option<f64>,
    /// Exponential moving average of deterministic `ResponseQualityAnalyzer` scores (`0.0..=1.0`, normalized from `0..=100`, `T3-ROUTER-DPO-FLY-02`).
    pub response_quality_ema: Option<f64>,
    /// Multi-benchmark composite prior (`SWE-Bench`, `LiveBench`, `ArtificialAnalysis`, `SkateBench`, `SnitchBench`, `T3-ROUTER-DPO-FLY-01`).
    pub benchmark_prior: Option<super::bench_evidence::ExternalBenchmarkPrior>,
}

impl From<vox_db::store::types::ModelScoreboardRow> for ModelScore {
    fn from(row: vox_db::store::types::ModelScoreboardRow) -> Self {
        Self {
            success_rate: row.success_rate,
            quality_score: row.quality_score,
            cost_per_success_usd: row.cost_per_success_usd,
            p50_latency_ms: row.p50_latency_ms,
            n_calls: row.n_calls,
            success_count: row.success_count,
            p95_ttft_ms: row.p95_ttft_ms,
            p95_tpot_ms: row.p95_tpot_ms,
            goodput_tokens_per_sec: row.goodput_tokens_per_sec,
            response_quality_ema: None,
            benchmark_prior: None,
        }
    }
}

/// Task M3: 95% Wilson score interval for a success rate — unlike the naive normal
/// approximation, stays well-behaved at small `n` and near 0%/100%, which is exactly the
/// regime a freshly-onboarded or rarely-used model sits in. Returns `None` when `n == 0`
/// (nothing to estimate).
///
/// Reporting-only: does not feed model *selection* (`ModelSelectionEngine`) — see
/// `crates/vox-orchestrator/src/routing/engine.rs`'s `pick_with_auto_score_thompson`, which
/// already runs its own Beta-Thompson exploration and is out of scope here.
#[must_use]
pub fn wilson_score_interval(successes: i64, n: i64) -> Option<(f64, f64)> {
    if n <= 0 {
        return None;
    }
    const Z: f64 = 1.96; // 95% confidence
    let n = n as f64;
    let p_hat = successes as f64 / n;
    let z2 = Z * Z;
    let denom = 1.0 + z2 / n;
    let center = (p_hat + z2 / (2.0 * n)) / denom;
    let margin = (Z * ((p_hat * (1.0 - p_hat) / n) + z2 / (4.0 * n * n)).sqrt()) / denom;
    Some(((center - margin).max(0.0), (center + margin).min(1.0)))
}

/// Task M3: below this many observed *calls*, a model holds no rank position — see
/// `vox model explain`'s `partition_by_rank_confidence`.
///
/// Deliberately distinct from `vox model scoreboard`'s `COST_PER_SUCCESS_MIN_SUCCESSES` (10):
/// these gate different statistics on different denominators. Ranking needs enough *calls* to
/// place a model relative to others; a cost-per-success *ratio* needs enough *successes* in
/// its denominator before one expensive fallback swings it by an order of magnitude.
/// Equalizing them would make one of the two wrong.
///
/// The two surfaces also read it differently, deliberately: `explain` **suppresses**
/// sub-threshold models from its ranked list (a rank position is a claim), while `scoreboard`
/// only **marks** them — a scoreboard is a full inventory keyed (model, category, strength),
/// and hiding thin rows would hide what a reader queries it for. Neither surface will mark a
/// sub-threshold row Pareto-optimal.
pub const MIN_CALLS_FOR_CONFIDENT_RANK: i64 = 5;

#[cfg(test)]
mod wilson_interval_tests {
    use super::wilson_score_interval;

    #[test]
    fn zero_calls_is_none() {
        assert_eq!(wilson_score_interval(0, 0), None);
    }

    #[test]
    fn perfect_record_interval_still_excludes_zero_and_stays_below_one() {
        let (lo, hi) = wilson_score_interval(3, 3).expect("n=3");
        assert!(lo > 0.0, "lo={lo}");
        assert!(hi <= 1.0, "hi={hi}");
        assert!(
            hi - lo > 0.1,
            "small n must leave real uncertainty, got {lo}..{hi}"
        );
    }

    #[test]
    fn interval_narrows_as_n_grows_for_the_same_rate() {
        let (lo_small, hi_small) = wilson_score_interval(5, 10).expect("n=10");
        let (lo_large, hi_large) = wilson_score_interval(500, 1000).expect("n=1000");
        assert!(
            (hi_large - lo_large) < (hi_small - lo_small),
            "more data at the same rate must produce a tighter interval"
        );
    }

    #[test]
    fn interval_is_centered_near_the_observed_rate_for_large_n() {
        let (lo, hi) = wilson_score_interval(800, 1000).expect("n=1000");
        assert!(
            lo < 0.8 && hi > 0.8,
            "0.8 observed rate should sit inside its own interval: {lo}..{hi}"
        );
        assert!(
            hi - lo < 0.05,
            "n=1000 should give a tight interval, got {lo}..{hi}"
        );
    }
}

/// A container of validated `ModelSpec`s that also tracks dynamic scoreboard scores.
#[derive(Debug, Clone, Default)]
pub struct ModelRegistry {
    pub(super) models: HashMap<String, ModelSpec>,
    pub(super) agent_overrides: HashMap<u64, String>,
    pub(super) premium_alias: HashMap<String, String>,
    /// Dynamic scores retrieved from `vox-db` `model_scoreboard` (keyed by model_id).
    pub(super) scoreboard: HashMap<String, ModelScore>,
    /// Thompson/Beta arm stats `(successes, failures)` from DB scoreboard aggregates.
    pub(super) arm_stats: HashMap<String, (u32, u32)>,
    /// In-memory penalty map for models that abstain (FIX-12).
    /// Key: (model_id, task_category). Value: Expiry time.
    pub(super) penalty_map: HashMap<(String, TaskCategory), SystemTime>,
    /// Catalog-derived scales (`models::reference`); re-derived by [`Self::apply_routing_reference`].
    pub(super) reference: super::reference::RoutingReference,
    /// Graded `vox models bench` evidence (keyed by model_id); stamped into quality priors.
    pub(super) bench: HashMap<String, super::bench_evidence::BenchEvidence>,
    /// Live operational status map (`VOX-ROUTER-T3-01`), keyed by `model_id`.
    pub(super) operational_status: HashMap<String, super::spec::ModelOperationalStatus>,
}

impl ModelRegistry {
    /// Set the live operational health status for `model_id` (`VOX-ROUTER-T3-01`).
    pub fn set_operational_status(
        &mut self,
        model_id: impl Into<String>,
        status: super::spec::ModelOperationalStatus,
    ) {
        self.operational_status.insert(model_id.into(), status);
    }

    /// Query the live operational health status for `model_id` (defaults to `Operational`).
    #[must_use]
    pub fn operational_status(&self, model_id: &str) -> super::spec::ModelOperationalStatus {
        self.operational_status
            .get(model_id)
            .copied()
            .unwrap_or(super::spec::ModelOperationalStatus::Operational)
    }

    /// Ingest a T3 Chat / provider status indicator string (`"operational"`, `"degraded"`, `"outage"`)
    /// and record the mapped [`super::spec::ModelOperationalStatus`].
    pub fn ingest_t3_status_indicator(
        &mut self,
        model_id: &str,
        indicator: &str,
    ) -> super::spec::ModelOperationalStatus {
        let status = super::spec::ModelOperationalStatus::parse_t3_indicator(indicator);
        self.operational_status.insert(model_id.to_string(), status);
        status
    }

    /// Reconcile a turn's exact credit/cost delta and billing tier (`VOX-ROUTER-T3-03`).
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn reconcile_turn_cost_delta(
        &self,
        model_id: &str,
        has_user_byok: bool,
        pre_balance: Option<f64>,
        post_balance: Option<f64>,
        prompt_tokens: u32,
        completion_tokens: u32,
        cache_read_tokens: u32,
        reported_cost_usd: Option<f64>,
    ) -> super::spec::TurnCostDeltaReceipt {
        let spec = self.models.get(model_id);
        let tier = super::spec::TurnCostDeltaReceipt::classify_tier(spec, model_id, has_user_byok);
        let credits_deducted = match (pre_balance, post_balance) {
            (Some(before), Some(after)) => Some((before - after).max(0.0)),
            _ => None,
        };
        let cost_usd_delta = match tier {
            super::spec::CostProvenanceTier::LocalZeroCost
            | super::spec::CostProvenanceTier::FreeTier => 0.0,
            super::spec::CostProvenanceTier::Byok
            | super::spec::CostProvenanceTier::PlatformCredits => {
                if let Some(reported) = reported_cost_usd {
                    reported.max(0.0)
                } else if let Some(s) = spec {
                    let uncached_prompt = prompt_tokens.saturating_sub(cache_read_tokens) as f64;
                    let cached_prompt = cache_read_tokens as f64;
                    let completion = completion_tokens as f64;
                    if s.cost_per_1k_input > 0.0 || s.cost_per_1k_output > 0.0 {
                        let cache_rate = if s.cache_read_cost_per_1k > 0.0 {
                            s.cache_read_cost_per_1k
                        } else {
                            s.cost_per_1k_input * 0.1
                        };
                        (uncached_prompt / 1000.0) * s.cost_per_1k_input
                            + (cached_prompt / 1000.0) * cache_rate
                            + (completion / 1000.0) * s.cost_per_1k_output
                    } else {
                        ((prompt_tokens + completion_tokens) as f64 / 1000.0) * s.cost_per_1k
                    }
                } else {
                    0.0
                }
            }
        };
        super::spec::TurnCostDeltaReceipt {
            model_id: model_id.to_string(),
            tier,
            cost_usd_delta,
            credits_deducted,
            credits_remaining: post_balance,
            prompt_tokens,
            completion_tokens,
            cache_read_tokens,
        }
    }

    pub fn record_penalty(&mut self, model_id: String, category: TaskCategory, duration: Duration) {
        let expiry = SystemTime::now() + duration;
        self.penalty_map.insert((model_id, category), expiry);
    }

    pub fn is_penalized(&self, model_id: &str, category: TaskCategory) -> bool {
        if let Some(expiry) = self.penalty_map.get(&(model_id.to_string(), category)) {
            if *expiry > SystemTime::now() {
                return true;
            }
        }
        false
    }

    pub fn inject_scoreboard(&mut self, scores: HashMap<String, ModelScore>) {
        self.scoreboard = scores;
    }

    /// Update the deterministic `ResponseQualityAnalyzer` EMA (`0.0..=1.0`, normalized from `0..=100`)
    /// for `model_id` in the live scoreboard (`T3-ROUTER-DPO-FLY-02`).
    pub fn record_response_quality_observation(
        &mut self,
        model_id: &str,
        quality_score_0_100: f64,
    ) {
        let normalized = (quality_score_0_100 / 100.0).clamp(0.0, 1.0);
        let entry = self
            .scoreboard
            .entry(model_id.to_string())
            .or_insert_with(|| ModelScore {
                success_rate: 1.0,
                quality_score: 1.0,
                n_calls: 1,
                success_count: 1,
                ..ModelScore::default()
            });
        entry.response_quality_ema = Some(match entry.response_quality_ema {
            Some(prev) => (prev * 0.75 + normalized * 0.25).clamp(0.0, 1.0),
            None => normalized,
        });
    }

    /// Attach an [`super::bench_evidence::ExternalBenchmarkPrior`] to `model_id`'s scoreboard entry (`T3-ROUTER-DPO-FLY-01`).
    pub fn set_model_benchmark_prior(
        &mut self,
        model_id: &str,
        prior: super::bench_evidence::ExternalBenchmarkPrior,
    ) {
        let entry = self
            .scoreboard
            .entry(model_id.to_string())
            .or_insert_with(|| ModelScore {
                success_rate: 1.0,
                quality_score: 1.0,
                n_calls: 1,
                success_count: 1,
                ..ModelScore::default()
            });
        entry.benchmark_prior = Some(prior);
    }

    /// Replace the benchmark evidence and restamp every quality prior from it.
    pub fn inject_benchmark_evidence(
        &mut self,
        evidence: HashMap<String, super::bench_evidence::BenchEvidence>,
    ) {
        self.bench = evidence;
        self.apply_routing_reference();
    }

    /// Inject every persisted routing signal — scoreboard (+ observed p50), arm stats,
    /// observed pricing, and benchmark evidence — then restamp priors. Each source is
    /// best-effort: a failing query leaves that signal at its prior state. The one builder
    /// every surface (daemon, GUI, CLI `--model auto`) uses so they route identically.
    pub async fn inject_evidence_from_db(&mut self, db: &vox_db::VoxDb) {
        if let Ok(rows) = db.get_model_scoreboard(7).await {
            self.inject_scoreboard_latency(&rows);
            self.inject_scoreboard(
                rows.into_iter()
                    .map(|row| (row.model_id.clone(), ModelScore::from(row)))
                    .collect(),
            );
        }
        if let Ok(arms) = db.list_model_arm_stats(7).await {
            self.inject_arm_stats(arms);
        }
        if let Ok(pricing) = db.get_pricing_catalog().await {
            self.inject_pricing_catalog(pricing);
        }
        match super::bench_evidence::load(db, super::bench_evidence::BENCH_EVIDENCE_WINDOW_DAYS)
            .await
        {
            Ok(ev) => self.inject_benchmark_evidence(ev),
            Err(_) => {
                self.apply_routing_reference();
            }
        }
        if let Some(bundle) = super::intelligence_bundle::load_cached_bundle_if_valid(None) {
            self.apply_community_intelligence_bundle(&bundle);
        }
    }

    #[must_use]
    pub fn bench_evidence(&self, model_id: &str) -> Option<&super::bench_evidence::BenchEvidence> {
        self.bench.get(model_id)
    }

    pub fn inject_arm_stats(&mut self, stats: HashMap<String, (u32, u32)>) {
        self.arm_stats = stats;
    }

    #[must_use]
    pub fn arm_stats_snapshot(&self) -> &HashMap<String, (u32, u32)> {
        &self.arm_stats
    }

    #[must_use]
    pub fn scoreboard_snapshot(&self) -> &HashMap<String, ModelScore> {
        &self.scoreboard
    }

    /// Record a bandit outcome for `model_id` and update the in-memory Beta posterior.
    /// `success = true` increments the success counter; `false` increments failures.
    /// Used by [`crate::calibration::ContextualBandit`] to feed outcomes back.
    pub fn record_bandit_outcome(&mut self, model_id: &str, success: bool) {
        let entry = self.arm_stats.entry(model_id.to_string()).or_insert((0, 0));
        if success {
            entry.0 = entry.0.saturating_add(1);
        } else {
            entry.1 = entry.1.saturating_add(1);
        }
    }

    pub fn inject_pricing_catalog(
        &mut self,
        pricing: Vec<vox_db::store::types::ModelPricingCatalogRow>,
    ) {
        for row in pricing {
            if row.confidence == "medium" || row.confidence == "high" {
                if let Some(spec) = self.models.get_mut(&row.model_id) {
                    if let Some(blended) = row.observed_blended_per_1k {
                        tracing::info!(
                            model_id = %row.model_id,
                            catalog_price = %spec.cost_per_1k,
                            observed_price = %blended,
                            confidence = %row.confidence,
                            "Calibrating model price from telemetry loop"
                        );
                        spec.cost_per_1k = blended;
                        spec.observed_cost_per_1k = Some(blended);

                        // If provider separates inputs/outputs, override those too
                        if let Some(input) = row.observed_input_per_1k {
                            spec.cost_per_1k_input = input;
                        }
                        if let Some(output) = row.observed_output_per_1k {
                            spec.cost_per_1k_output = output;
                        }
                        spec.pricing_source = super::spec::PricingSource::Telemetry;
                    }
                }
            }
        }
        self.apply_routing_reference();
    }

    /// Inject observed p50 latency from the `model_scoreboard` into each matching
    /// [`ModelSpec`]'s `capabilities.latency_p50_ms`, closing the gap where the scorer's
    /// `crate::models::scoring::latency_score` read only the *static* catalog field and
    /// never the measured telemetry p50.
    ///
    /// Mirrors [`Self::inject_pricing_catalog`]: only rows with a positive, present
    /// `p50_latency_ms` overwrite the spec, and only for models already in the registry.
    /// Returns the number of specs updated (useful for logging / tests).
    pub fn inject_scoreboard_latency(
        &mut self,
        rows: &[vox_db::store::types::ModelScoreboardRow],
    ) -> usize {
        let mut updated = 0;
        for row in rows {
            let Some(p50) = row.p50_latency_ms else {
                continue;
            };
            if p50 <= 0 {
                continue;
            }
            if let Some(spec) = self.models.get_mut(&row.model_id) {
                let clamped = p50.min(u32::MAX as i64) as u32;
                tracing::debug!(
                    model_id = %row.model_id,
                    catalog_p50 = ?spec.capabilities.latency_p50_ms,
                    observed_p50 = clamped,
                    "Calibrating model latency p50 from scoreboard telemetry"
                );
                spec.capabilities.latency_p50_ms = Some(clamped);
                updated += 1;
            }
        }
        updated
    }

    /// Apply supplementary pricing from the LiteLLM oracle.
    ///
    /// Fills fields that OpenRouter doesn't expose (cache-hit prices, Anthropic models, etc.).
    /// Will not overwrite an entry whose `pricing_source` is already `Telemetry`.
    pub fn apply_litellm_pricing(
        &mut self,
        entries: &std::collections::HashMap<String, crate::catalog::LiteLLMPricingEntry>,
    ) {
        use super::spec::PricingSource;

        for (litellm_id, entry) in entries {
            // Build a candidate list of IDs to try (exact → prefixed → suffix).
            let suffix = litellm_id
                .split('/')
                .next_back()
                .unwrap_or(litellm_id.as_str());
            let with_provider = entry
                .litellm_provider
                .as_deref()
                .map(|p| format!("{}/{}", p, suffix));

            let found_key = if self.models.contains_key(litellm_id.as_str()) {
                Some(litellm_id.clone())
            } else if let Some(ref wp) = with_provider {
                if self.models.contains_key(wp.as_str()) {
                    Some(wp.clone())
                } else {
                    // Suffix fallback — only match when exactly one model ID ends with the
                    // bare name. If multiple models share the suffix the match is ambiguous
                    // and nondeterministic (HashMap ordering), so we skip and log instead.
                    let mut matches = self.models.keys().filter(|k| k.ends_with(suffix));
                    match (matches.next().cloned(), matches.next()) {
                        (Some(key), None) => Some(key),
                        _ => {
                            tracing::debug!(
                                target: "vox.orchestrator.models",
                                suffix,
                                litellm_id,
                                "ambiguous or missing suffix match in apply_litellm_pricing; skipping"
                            );
                            None
                        }
                    }
                }
            } else {
                // Same deterministic suffix fallback without provider prefix.
                let mut matches = self.models.keys().filter(|k| k.ends_with(suffix));
                match (matches.next().cloned(), matches.next()) {
                    (Some(key), None) => Some(key),
                    _ => {
                        tracing::debug!(
                            target: "vox.orchestrator.models",
                            suffix,
                            litellm_id,
                            "ambiguous or missing suffix match in apply_litellm_pricing; skipping"
                        );
                        None
                    }
                }
            };

            let Some(key) = found_key else { continue };
            let Some(spec) = self.models.get_mut(&key) else {
                continue;
            };

            // Telemetry is always the highest-trust source; never downgrade it.
            if spec.pricing_source == PricingSource::Telemetry {
                continue;
            }

            if let Some(cost_in) = entry.cost_per_1k_input {
                if cost_in > 0.0 {
                    spec.cost_per_1k_input = cost_in;
                }
            }
            if let Some(cost_out) = entry.cost_per_1k_output {
                if cost_out > 0.0 {
                    spec.cost_per_1k_output = cost_out;
                    spec.cost_per_1k = cost_out; // keep legacy blended field in sync
                }
            }
            if let Some(cache_create) = entry.cache_creation_cost_per_1k {
                spec.cache_creation_cost_per_1k = cache_create;
            }
            if let Some(cache_read) = entry.cache_read_cost_per_1k {
                spec.cache_read_cost_per_1k = cache_read;
            }
            if let Some(caching) = entry.supports_prompt_caching {
                spec.supports_prompt_caching = caching;
            }
            spec.pricing_source = PricingSource::LiteLLM;
        }
    }

    pub fn scoreboard_len(&self) -> usize {
        self.scoreboard.len()
    }

    pub fn get_score(&self, model_id: &str) -> Option<&ModelScore> {
        self.scoreboard.get(model_id)
    }

    pub(crate) fn matches_strength(m: &ModelSpec, strength: crate::models::StrengthTag) -> bool {
        m.strengths
            .iter()
            .any(|s| *s == strength || *s == crate::models::StrengthTag::Generalist)
    }

    /// Credential gate used by the canonical selector: true iff the provider's
    /// primary key is resolvable right now (local providers always pass).
    pub(crate) fn key_is_present_for(m: &ModelSpec) -> bool {
        // Through `selection_key_available` so a test can pin the provider set per thread
        // (`key_guard::set_test_key_availability`); with no override it is the same Clavis check.
        super::key_guard::selection_key_available(&m.provider_type)
    }

    pub(super) fn min_refresh_interval() -> Duration {
        let secs = vox_secrets::resolve_secret(
            vox_secrets::SecretId::VoxOpenRouterCatalogMinRefreshIntervalSecs,
        )
        .expose()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(3600);
        Duration::from_secs(secs.max(30))
    }

    pub(super) fn jitter_ms() -> u64 {
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxOpenRouterCatalogRefreshJitterMs)
            .expose()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
            .min(60_000)
    }
}

pub(super) use super::{
    ModelTier, bench_evidence, family, intelligence_bundle, key_guard, ranking, reference, scoring,
    select, spec, tiering,
};

mod catalog_seed;
mod conversational_routing;
mod conversational_workflows;
mod mens_spoke_routing;
mod selection_queries;
mod sota_harness_eval;

#[cfg(feature = "runtime")]
pub use selection_queries::llm_config_for_spec;

#[cfg(all(test, feature = "runtime"))]
mod tests;
