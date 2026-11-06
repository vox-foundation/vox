//! Measured model quality from `vox models bench` (`model_benchmark_results`).
//!
//! A graded benchmark run is the only signal that *raises or lowers* a model's quality on
//! Vox's own tasks. It does two things in routing:
//!
//! - [`BenchEvidence::blended_quality`] shrinks the measured mean score toward the model's
//!   external prior (Artificial Analysis index / family / proxy), so a handful of cases
//!   cannot overturn a well-established index but a full suite dominates it.
//! - [`BenchEvidence::verdict`] confirms or holds back a model for production routing on the
//!   95% Wilson interval of its pass rate — eligibility moves on evidence, not on price
//!   provenance.

use super::autonomic::ModelConfidence;
use super::registry::wilson_score_interval;

/// Pseudo-cases the external prior is worth when blending measured quality.
pub const BENCH_PRIOR_PSEUDO_CASES: f64 = 8.0;
/// Fewest graded cases before a benchmark verdict can move eligibility.
pub const MIN_VERDICT_CASES: i64 = 8;
/// Pass rate a model must clear (Wilson lower bound) to be confirmed, and must fall
/// below (Wilson upper bound) to be held back.
pub const PASS_FLOOR: f64 = 0.3;

/// Recency window of benchmark cases routing reads.
pub const BENCH_EVIDENCE_WINDOW_DAYS: i64 = 30;

/// Load per-model evidence from the last `window_days` of `model_benchmark_results`.
pub async fn load(
    db: &vox_db::VoxDb,
    window_days: i64,
) -> Result<std::collections::HashMap<String, BenchEvidence>, vox_db::StoreError> {
    let since = vox_db::now_unix_ms() as i64 - window_days.max(0) * 86_400_000;
    Ok(db
        .model_benchmark_summary(since)
        .await?
        .iter()
        .map(|r| (r.model_id.clone(), BenchEvidence::from(r)))
        .collect())
}

/// Per-model benchmark evidence, as injected into the registry.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchEvidence {
    pub cases: i64,
    pub passed: i64,
    pub errors: i64,
    pub mean_score: f64,
    pub resolved_model_id: Option<String>,
    pub p50_ttft_ms: Option<i64>,
    pub p50_latency_ms: Option<i64>,
    pub p95_latency_ms: Option<i64>,
    pub mean_tokens_per_sec: Option<f64>,
    pub cost_per_solve_usd: Option<f64>,
    pub tokens_per_solve: Option<f64>,
    pub last_recorded_at_ms: i64,
}

impl From<&vox_db::ModelBenchmarkSummaryRow> for BenchEvidence {
    fn from(r: &vox_db::ModelBenchmarkSummaryRow) -> Self {
        Self {
            cases: r.cases,
            passed: r.passed,
            errors: r.errors,
            mean_score: r.mean_score,
            resolved_model_id: r.resolved_model_id.clone(),
            p50_ttft_ms: r.p50_ttft_ms,
            p50_latency_ms: r.p50_latency_ms,
            p95_latency_ms: r.p95_latency_ms,
            mean_tokens_per_sec: r.mean_tokens_per_sec,
            cost_per_solve_usd: r.cost_per_solve_usd(),
            tokens_per_solve: r.tokens_per_solve(),
            last_recorded_at_ms: r.last_recorded_at_ms,
        }
    }
}

impl BenchEvidence {
    /// Measured mean score shrunk toward `prior` (both 0–1).
    #[must_use]
    pub fn blended_quality(&self, prior: f64) -> f64 {
        let n = self.cases.max(0) as f64;
        ((prior * BENCH_PRIOR_PSEUDO_CASES + self.mean_score * n) / (BENCH_PRIOR_PSEUDO_CASES + n))
            .clamp(0.0, 1.0)
    }

    /// 95% Wilson interval of the pass rate (errors count as failures).
    #[must_use]
    pub fn pass_interval(&self) -> Option<(f64, f64)> {
        wilson_score_interval(self.passed, self.cases)
    }

    /// `Confirmed` when the pass rate is confidently above [`PASS_FLOOR`], `Shadowed` when
    /// confidently below it, `None` when too few cases or the interval straddles the floor.
    #[must_use]
    pub fn verdict(&self) -> Option<ModelConfidence> {
        if self.cases < MIN_VERDICT_CASES {
            return None;
        }
        let (lo, hi) = self.pass_interval()?;
        if lo >= PASS_FLOOR {
            Some(ModelConfidence::Confirmed)
        } else if hi < PASS_FLOOR {
            Some(ModelConfidence::Shadowed)
        } else {
            None
        }
    }
}

/// Multi-benchmark Bayesian prior combining SWE-bench Verified, LiveBench Coding,
/// Artificial Analysis Index, SkateBench, and SnitchBench safety penalty (`T3-BATTERY-LEDGER-02`).
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExternalBenchmarkPrior {
    /// SWE-bench Verified score (`0.0..=1.0`). Weight: `0.35`.
    pub swe_bench_verified: Option<f64>,
    /// LiveBench Coding score (`0.0..=1.0`). Weight: `0.25`.
    pub live_bench_coding: Option<f64>,
    /// Artificial Analysis Intelligence Index normalized (`0.0..=1.0`). Weight: `0.20`.
    pub artificial_analysis_index: Option<f64>,
    /// Theo's SkateBench domain/tool reasoning score (`0.0..=1.0`). Weight: `0.15`.
    pub skate_bench_score: Option<f64>,
    /// Theo's SnitchBench whistleblowing/safety over-refusal rate (`0.0..=1.0`). Penalty weight: `-0.10`.
    pub snitch_bench_safety_penalty: Option<f64>,
}

impl ExternalBenchmarkPrior {
    /// Compute the composite weighted prior (`0.0..=1.0`), falling back to `0.5` when no external metrics exist.
    #[must_use]
    pub fn composite_prior_score(&self) -> f64 {
        let mut weighted_sum = 0.0;
        let mut total_positive_weight = 0.0;

        if let Some(v) = self.swe_bench_verified {
            weighted_sum += v.clamp(0.0, 1.0) * 0.35;
            total_positive_weight += 0.35;
        }
        if let Some(v) = self.live_bench_coding {
            weighted_sum += v.clamp(0.0, 1.0) * 0.25;
            total_positive_weight += 0.25;
        }
        if let Some(v) = self.artificial_analysis_index {
            weighted_sum += v.clamp(0.0, 1.0) * 0.20;
            total_positive_weight += 0.20;
        }
        if let Some(v) = self.skate_bench_score {
            weighted_sum += v.clamp(0.0, 1.0) * 0.15;
            total_positive_weight += 0.15;
        }

        let base = if total_positive_weight > 0.0 {
            weighted_sum / total_positive_weight
        } else {
            0.5
        };
        let penalty = self
            .snitch_bench_safety_penalty
            .map(|p| p.clamp(0.0, 1.0) * 0.10)
            .unwrap_or(0.0);
        (base - penalty).clamp(0.05, 1.0)
    }

    /// Alias for [`Self::composite_prior_score`] used by `scoreboard_feedback_boost` (`T3-ROUTER-DPO-FLY-01`).
    #[must_use]
    pub fn composite_routing_prior(&self) -> f64 {
        self.composite_prior_score()
    }
}

/// Evaluate per-token cost expressions scraped from JS bundles (e.g. `"3/1e6"`, `"0.15/1000000"`, `"0.000003"`).
/// Adapted from `vibheksoni/t3router` `src/t3/models.rs` `eval_cost_expr`.
#[must_use]
pub fn eval_cost_per_token_expr(expr: &str) -> Option<f64> {
    let trimmed = expr.trim().trim_matches('"').trim_matches('\'').trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((num_str, den_str)) = trimmed.split_once('/') {
        let num: f64 = num_str.trim().parse().ok()?;
        let den: f64 = den_str.trim().parse().ok()?;
        if den == 0.0 || !num.is_finite() || !den.is_finite() {
            return None;
        }
        Some(num / den)
    } else {
        let val: f64 = trimmed.parse().ok()?;
        val.is_finite().then_some(val)
    }
}

#[cfg(test)]
pub(crate) fn evidence(cases: i64, passed: i64, mean_score: f64) -> BenchEvidence {
    BenchEvidence {
        cases,
        passed,
        errors: 0,
        mean_score,
        resolved_model_id: None,
        p50_ttft_ms: None,
        p50_latency_ms: None,
        p95_latency_ms: None,
        mean_tokens_per_sec: None,
        cost_per_solve_usd: None,
        tokens_per_solve: None,
        last_recorded_at_ms: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn few_cases_barely_move_the_prior_and_a_full_suite_dominates_it() {
        let prior = 0.8;
        let two = evidence(2, 0, 0.0).blended_quality(prior);
        assert!((two - 0.64).abs() < 1e-9, "2 failed cases: {two}");
        let many = evidence(72, 72, 1.0).blended_quality(0.2);
        assert!(
            many > 0.9,
            "a perfect 72-case suite should dominate a weak prior: {many}"
        );
    }

    #[test]
    fn verdict_needs_enough_cases_and_a_clear_interval() {
        assert_eq!(evidence(4, 4, 1.0).verdict(), None, "too few cases");
        assert_eq!(
            evidence(20, 18, 0.9).verdict(),
            Some(ModelConfidence::Confirmed)
        );
        assert_eq!(
            evidence(30, 0, 0.0).verdict(),
            Some(ModelConfidence::Shadowed)
        );
        assert_eq!(evidence(10, 3, 0.3).verdict(), None, "straddles the floor");
    }

    #[test]
    fn external_benchmark_prior_and_cost_expr_parser_work() {
        let prior = ExternalBenchmarkPrior {
            swe_bench_verified: Some(0.80),
            live_bench_coding: Some(0.75),
            artificial_analysis_index: Some(0.85),
            skate_bench_score: Some(0.90),
            snitch_bench_safety_penalty: Some(0.20),
        };
        let score = prior.composite_prior_score();
        assert!(score > 0.75 && score < 0.85, "composite prior: {score}");

        assert!((eval_cost_per_token_expr("3/1e6").unwrap() - 0.000003).abs() < 1e-12);
        assert!((eval_cost_per_token_expr("15 / 1000000").unwrap() - 0.000015).abs() < 1e-12);
        assert_eq!(eval_cost_per_token_expr("1/0"), None);
    }
}
