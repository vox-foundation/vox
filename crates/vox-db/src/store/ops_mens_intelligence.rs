//! Arca CRUD for MENS intelligence tables.

use crate::store::types::StoreError;
use crate::store::types::{ObservationReport, TestDecision, VictoryVerdict};
use turso::params;

/// Aggregated summary of corpus quality from the `mens_corpus_quality` table.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CorpusQualitySummary {
    /// Total rows tracked.
    pub total_pairs: u64,
    /// Fraction that parsed successfully (0.0–1.0).
    pub parse_rate: f64,
    /// Average AST depth across all pairs.
    pub avg_ast_depth: f64,
    /// Average construct count per pair.
    pub avg_construct_count: f64,
    /// Average reward score.
    pub avg_reward_score: f64,
}

/// A single GRPO training step row from `grpo_training_run`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpoStepRow {
    pub run_id: String,
    pub step: u32,
    pub mean_reward: f32,
    pub policy_loss: f32,
    pub clip_fraction: f32,
    pub parse_rate: f32,
}

impl crate::VoxDb {
    /// Insert an observer event into the Arca database.
    pub async fn insert_observer_event(
        &self,
        session_id: &str,
        task_id: &str,
        report: &ObservationReport,
    ) -> Result<(), StoreError> {
        let session_id = session_id.to_string();
        let task_id = task_id.to_string();
        let report_json = serde_json::to_string(report).unwrap_or_default();
        let action = format!("{:?}", report.recommended_action);
        let observed_at_ms = report.observed_at.timestamp_millis();
        let file_path = report.file_path.clone();

        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker.call(|| {
            let session_id = session_id.clone();
            let task_id = task_id.clone();
            let report_json = report_json.clone();
            let action = action.clone();
            let file_path = file_path.clone();
            async move {
                conn.execute(
                    "INSERT INTO observer_events (session_id, task_id, observed_at_ms, file_path, lsp_errors, parse_rate, construct_coverage, action, raw_json)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        session_id.as_str(),
                        task_id.as_str(),
                        observed_at_ms,
                        file_path.as_str(),
                        report.lsp_error_count as i64,
                        report.parse_rate as f64,
                        report.construct_coverage as f64,
                        action.as_str(),
                        report_json.as_str()
                    ]
                ).await?;
                Ok(())
            }
        }).await
    }

    /// Insert a testing decision for a task.
    pub async fn insert_test_decision(
        &self,
        task_id: &str,
        decision: &TestDecision,
        rationale: Option<&str>,
        complexity: u8,
        file_count: usize,
    ) -> Result<(), StoreError> {
        let task_id = task_id.to_string();
        let decision_str = format!("{:?}", decision);
        let rationale = rationale.map(|s| s.to_string());

        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker.call(|| {
            let task_id = task_id.clone();
            let decision_str = decision_str.clone();
            let rationale = rationale.clone();
            async move {
                conn.execute(
                    "INSERT INTO test_decisions (task_id, decision, rationale, complexity_score, file_count)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        task_id.as_str(),
                        decision_str.as_str(),
                        rationale.as_deref(),
                        complexity as i64,
                        file_count as i64
                    ]
                ).await?;
                Ok(())
            }
        }).await
    }

    /// Insert a victory verdict for a task.
    pub async fn insert_victory_verdict(
        &self,
        task_id: &str,
        verdict: &VictoryVerdict,
    ) -> Result<(), StoreError> {
        let task_id = task_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        let first_failure = verdict.first_failure.clone();
        let report = verdict.report.clone();
        let passed = verdict.passed;

        breaker
            .call(|| {
                let task_id = task_id.clone();
                let _first_failure = first_failure.clone();
                let report = report.clone();
                async move {
                    // Simplified insert for Wave 0: just log the roll-up.
                    // Future waves will expand into individual tier rows if needed.
                    conn.execute(
                        "INSERT INTO victory_verdicts (task_id, tier, passed, error_count, report)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![
                            task_id.as_str(),
                            "Full",
                            if passed { 1 } else { 0 },
                            0i64, // simplified
                            report.as_str()
                        ],
                    )
                    .await?;
                    Ok(())
                }
            })
            .await
    }

    /// Upsert corpus quality metrics for a training pair.
    pub async fn upsert_corpus_quality(
        &self,
        pair_hash: &str,
        source: &str,
        parse_valid: bool,
        ast_depth: usize,
        construct_count: usize,
        reward_score: f32,
        split: &str,
    ) -> Result<(), StoreError> {
        let pair_hash = pair_hash.to_string();
        let source = source.to_string();
        let split = split.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker.call(|| {
            let pair_hash = pair_hash.clone();
            let source = source.clone();
            let split = split.clone();
            async move {
                conn.execute(
                    "INSERT INTO mens_corpus_quality (pair_hash, source, parse_valid, ast_depth, construct_count, reward_score, split)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                     ON CONFLICT(pair_hash) DO UPDATE SET
                        parse_valid = excluded.parse_valid,
                        ast_depth = excluded.ast_depth,
                        construct_count = excluded.construct_count,
                        reward_score = excluded.reward_score,
                        split = excluded.split",
                    params![
                        pair_hash.as_str(),
                        source.as_str(),
                        if parse_valid { 1 } else { 0 },
                        ast_depth as i64,
                        construct_count as i64,
                        reward_score as f64,
                        split.as_str()
                    ]
                ).await?;
                Ok(())
            }
        }).await
    }

    /// Insert a MENS/GRPO training step.
    pub async fn insert_grpo_step(
        &self,
        run_id: &str,
        step: u32,
        mean_reward: f32,
        policy_loss: f32,
        clip_fraction: f32,
        parse_rate: f32,
    ) -> Result<(), StoreError> {
        let run_id = run_id.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();

        breaker.call(|| {
            let run_id = run_id.clone();
            async move {
                conn.execute(
                    "INSERT INTO grpo_training_run (run_id, step, mean_reward, policy_loss, clip_fraction, parse_rate)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        run_id.as_str(),
                        step as i64,
                        mean_reward as f64,
                        policy_loss as f64,
                        clip_fraction as f64,
                        parse_rate as f64
                    ]
                ).await?;
                Ok(())
            }
        }).await
    }

    /// Return an aggregated summary of corpus quality across all tracked pairs.
    pub async fn query_corpus_quality_summary(&self) -> Result<CorpusQualitySummary, StoreError> {
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                let mut rows = conn
                    .query(
                        "SELECT COUNT(*), \
                        AVG(CAST(parse_valid AS REAL)), \
                        AVG(ast_depth), \
                        AVG(construct_count), \
                        AVG(reward_score) \
                 FROM mens_corpus_quality",
                        params![],
                    )
                    .await?;
                let mut total_pairs = 0u64;
                let mut parse_rate = 0.0f64;
                let mut avg_ast_depth = 0.0f64;
                let mut avg_construct_count = 0.0f64;
                let mut avg_reward_score = 0.0f64;

                if let Some(row) = rows.next().await? {
                    total_pairs = row.get::<i64>(0).unwrap_or(0) as u64;
                    parse_rate = row.get::<f64>(1).unwrap_or(0.0);
                    avg_ast_depth = row.get::<f64>(2).unwrap_or(0.0);
                    avg_construct_count = row.get::<f64>(3).unwrap_or(0.0);
                    avg_reward_score = row.get::<f64>(4).unwrap_or(0.0);
                }
                Ok(CorpusQualitySummary {
                    total_pairs,
                    parse_rate,
                    avg_ast_depth,
                    avg_construct_count,
                    avg_reward_score,
                })
            })
            .await
    }

    /// Return the most recent `limit` GRPO training steps, newest first.
    pub async fn query_recent_grpo_steps(
        &self,
        run_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<GrpoStepRow>, StoreError> {
        let run_id = run_id.map(|s| s.to_string());
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(move || {
                let run_id = run_id.clone();
                async move {
                    let mut rows = if let Some(rid) = &run_id {
                        conn.query(
                        "SELECT run_id, step, mean_reward, policy_loss, clip_fraction, parse_rate \
                         FROM grpo_training_run WHERE run_id = ?1 ORDER BY step DESC LIMIT ?2",
                        params![rid.as_str(), limit as i64],
                    ).await?
                    } else {
                        conn.query(
                        "SELECT run_id, step, mean_reward, policy_loss, clip_fraction, parse_rate \
                         FROM grpo_training_run ORDER BY step DESC LIMIT ?1",
                        params![limit as i64],
                    ).await?
                    };
                    let mut out = Vec::new();
                    while let Some(row) = rows.next().await? {
                        out.push(GrpoStepRow {
                            run_id: row.get::<String>(0).unwrap_or_default(),
                            step: row.get::<i64>(1).unwrap_or(0) as u32,
                            mean_reward: row.get::<f64>(2).unwrap_or(0.0) as f32,
                            policy_loss: row.get::<f64>(3).unwrap_or(0.0) as f32,
                            clip_fraction: row.get::<f64>(4).unwrap_or(0.0) as f32,
                            parse_rate: row.get::<f64>(5).unwrap_or(0.0) as f32,
                        });
                    }
                    Ok(out)
                }
            })
            .await
    }
}
