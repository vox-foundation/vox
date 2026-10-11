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
}
