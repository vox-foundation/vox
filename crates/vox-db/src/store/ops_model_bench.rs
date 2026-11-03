//! Store ops for `model_benchmark_results` — graded per-case results from `vox models bench`.
//!
//! This table is the only writer of *measured* model quality. Routing reads it through
//! [`VoxDb::model_benchmark_summary`], one row per model over a recency window.

use crate::VoxDb;
use crate::store::types::StoreError;
use turso::params;

/// One graded benchmark case to insert.
#[derive(Debug, Clone, PartialEq)]
pub struct NewModelBenchmarkResult {
    pub run_id: String,
    pub model_id: String,
    pub resolved_model_id: Option<String>,
    pub provider: String,
    pub suite: String,
    pub task_category: String,
    pub case_id: String,
    pub passed: bool,
    pub score: f64,
    pub ttft_ms: Option<i64>,
    pub latency_ms: Option<i64>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub tokens_per_sec: Option<f64>,
    pub cost_usd: Option<f64>,
    pub error_class: Option<String>,
    pub recorded_at_ms: i64,
}

/// Per-model aggregate over a window of benchmark cases.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelBenchmarkSummaryRow {
    pub model_id: String,
    /// Most recent provider-reported id (alias target) seen in the window.
    pub resolved_model_id: Option<String>,
    pub cases: i64,
    pub passed: i64,
    pub mean_score: f64,
    /// Cases that ended in a transport/provider error rather than a graded answer.
    pub errors: i64,
    pub p50_ttft_ms: Option<i64>,
    pub p50_latency_ms: Option<i64>,
    pub p95_latency_ms: Option<i64>,
    pub mean_tokens_per_sec: Option<f64>,
    pub total_cost_usd: f64,
    pub total_output_tokens: i64,
    pub last_recorded_at_ms: i64,
}

impl ModelBenchmarkSummaryRow {
    /// USD spent per passed case (`None` when nothing passed).
    #[must_use]
    pub fn cost_per_solve_usd(&self) -> Option<f64> {
        (self.passed > 0).then(|| self.total_cost_usd / self.passed as f64)
    }

    /// Output tokens per passed case (`None` when nothing passed).
    #[must_use]
    pub fn tokens_per_solve(&self) -> Option<f64> {
        (self.passed > 0).then(|| self.total_output_tokens as f64 / self.passed as f64)
    }
}

fn nearest_rank(sorted: &[i64], q: f64) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

impl VoxDb {
    /// Insert one graded benchmark case.
    pub async fn record_model_benchmark_result(
        &self,
        r: &NewModelBenchmarkResult,
    ) -> Result<(), StoreError> {
        if !(0.0..=1.0).contains(&r.score) {
            return Err(StoreError::Db(format!(
                "model_benchmark_results.score must be within 0..=1, got {}",
                r.score
            )));
        }
        self.conn
            .execute(
                "INSERT INTO model_benchmark_results \
                 (run_id, model_id, resolved_model_id, provider, suite, task_category, case_id, passed, score, \
                  ttft_ms, latency_ms, input_tokens, output_tokens, tokens_per_sec, cost_usd, error_class, recorded_at_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    r.run_id.as_str(),
                    r.model_id.as_str(),
                    r.resolved_model_id.as_deref(),
                    r.provider.as_str(),
                    r.suite.as_str(),
                    r.task_category.as_str(),
                    r.case_id.as_str(),
                    i64::from(r.passed),
                    r.score,
                    r.ttft_ms,
                    r.latency_ms,
                    r.input_tokens,
                    r.output_tokens,
                    r.tokens_per_sec,
                    r.cost_usd,
                    r.error_class.as_deref(),
                    r.recorded_at_ms,
                ],
            )
            .await
            .map_err(StoreError::Turso)?;
        Ok(())
    }

    /// Per-model summary of cases recorded at or after `since_ms`, sorted by model id.
    pub async fn model_benchmark_summary(
        &self,
        since_ms: i64,
    ) -> Result<Vec<ModelBenchmarkSummaryRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT model_id, resolved_model_id, passed, score, ttft_ms, latency_ms, \
                        tokens_per_sec, cost_usd, output_tokens, error_class, recorded_at_ms \
                 FROM model_benchmark_results WHERE recorded_at_ms >= ?1 \
                 ORDER BY model_id, recorded_at_ms",
                params![since_ms],
            )
            .await
            .map_err(StoreError::Turso)?;

        #[derive(Default)]
        struct Acc {
            resolved: Option<String>,
            cases: i64,
            passed: i64,
            score_sum: f64,
            errors: i64,
            ttft: Vec<i64>,
            latency: Vec<i64>,
            tps: Vec<f64>,
            cost: f64,
            out_tokens: i64,
            last: i64,
        }
        let mut by_model: std::collections::BTreeMap<String, Acc> = Default::default();
        while let Some(row) = rows.next().await.map_err(StoreError::Turso)? {
            let model_id: String = row.get(0).map_err(StoreError::Turso)?;
            let a = by_model.entry(model_id).or_default();
            let resolved: Option<String> = row.get(1).map_err(StoreError::Turso)?;
            if resolved.is_some() {
                a.resolved = resolved;
            }
            a.cases += 1;
            a.passed += row.get::<i64>(2).map_err(StoreError::Turso)?;
            a.score_sum += row.get::<f64>(3).map_err(StoreError::Turso)?;
            if let Some(t) = row.get::<Option<i64>>(4).map_err(StoreError::Turso)? {
                a.ttft.push(t);
            }
            if let Some(l) = row.get::<Option<i64>>(5).map_err(StoreError::Turso)? {
                a.latency.push(l);
            }
            if let Some(t) = row.get::<Option<f64>>(6).map_err(StoreError::Turso)? {
                a.tps.push(t);
            }
            a.cost += row
                .get::<Option<f64>>(7)
                .map_err(StoreError::Turso)?
                .unwrap_or(0.0);
            a.out_tokens += row
                .get::<Option<i64>>(8)
                .map_err(StoreError::Turso)?
                .unwrap_or(0);
            if row
                .get::<Option<String>>(9)
                .map_err(StoreError::Turso)?
                .is_some()
            {
                a.errors += 1;
            }
            a.last = row.get(10).map_err(StoreError::Turso)?;
        }

        Ok(by_model
            .into_iter()
            .map(|(model_id, mut a)| {
                a.ttft.sort_unstable();
                a.latency.sort_unstable();
                ModelBenchmarkSummaryRow {
                    model_id,
                    resolved_model_id: a.resolved,
                    cases: a.cases,
                    passed: a.passed,
                    mean_score: a.score_sum / a.cases.max(1) as f64,
                    errors: a.errors,
                    p50_ttft_ms: nearest_rank(&a.ttft, 0.5),
                    p50_latency_ms: nearest_rank(&a.latency, 0.5),
                    p95_latency_ms: nearest_rank(&a.latency, 0.95),
                    mean_tokens_per_sec: (!a.tps.is_empty())
                        .then(|| a.tps.iter().sum::<f64>() / a.tps.len() as f64),
                    total_cost_usd: a.cost,
                    total_output_tokens: a.out_tokens,
                    last_recorded_at_ms: a.last,
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DbConfig;

    fn case(
        model: &str,
        passed: bool,
        score: f64,
        latency: i64,
        cost: f64,
    ) -> NewModelBenchmarkResult {
        NewModelBenchmarkResult {
            run_id: "run-1".into(),
            model_id: model.into(),
            resolved_model_id: Some(format!("{model}-20261001")),
            provider: "openrouter".into(),
            suite: "codegen".into(),
            task_category: "CodeGen".into(),
            case_id: format!("c-{latency}"),
            passed,
            score,
            ttft_ms: Some(latency / 4),
            latency_ms: Some(latency),
            input_tokens: Some(100),
            output_tokens: Some(50),
            tokens_per_sec: Some(40.0),
            cost_usd: Some(cost),
            error_class: None,
            recorded_at_ms: 1_000 + latency,
        }
    }

    #[tokio::test]
    async fn summary_aggregates_quality_latency_and_cost_per_solve() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        db.record_model_benchmark_result(&case("acme/a", true, 1.0, 100, 0.01))
            .await
            .unwrap();
        db.record_model_benchmark_result(&case("acme/a", true, 0.5, 300, 0.01))
            .await
            .unwrap();
        db.record_model_benchmark_result(&case("acme/a", false, 0.0, 200, 0.02))
            .await
            .unwrap();
        db.record_model_benchmark_result(&case("acme/b", true, 1.0, 50, 0.0))
            .await
            .unwrap();

        let rows = db.model_benchmark_summary(0).await.unwrap();
        assert_eq!(rows.len(), 2);
        let a = &rows[0];
        assert_eq!(a.model_id, "acme/a");
        assert_eq!((a.cases, a.passed, a.errors), (3, 2, 0));
        assert!((a.mean_score - 0.5).abs() < 1e-9);
        assert_eq!(a.p50_latency_ms, Some(200));
        assert_eq!(a.p95_latency_ms, Some(300));
        assert!((a.cost_per_solve_usd().unwrap() - 0.02).abs() < 1e-9);
        assert_eq!(a.tokens_per_solve(), Some(75.0));
        assert_eq!(a.resolved_model_id.as_deref(), Some("acme/a-20261001"));
    }

    #[tokio::test]
    async fn summary_respects_the_window_and_rejects_out_of_range_scores() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("db");
        db.record_model_benchmark_result(&case("acme/a", true, 1.0, 100, 0.0))
            .await
            .unwrap();
        assert!(db.model_benchmark_summary(10_000).await.unwrap().is_empty());
        assert!(
            db.record_model_benchmark_result(&case("acme/a", true, 1.5, 100, 0.0))
                .await
                .is_err()
        );
    }
}
