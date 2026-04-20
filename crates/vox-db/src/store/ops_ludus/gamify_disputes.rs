//! Gamification CRUD for Disputes (`gamify_disputes` and `gamify_dispute_votes`).

use turso::params;
use crate::store::types::StoreError;

impl crate::VoxDb {
    /// Insert a new dispute.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_gamify_dispute(
        &self,
        id: &str,
        accused_user_id: &str,
        accuser_user_id: &str,
        github_event_id: Option<&str>,
        snapshot_id: Option<i64>,
        evidence_json: &str,
        malice_score: f64,
        status: &str,
        created_at: i64,
        appeal_deadline_ts: i64,
    ) -> Result<(), StoreError> {
        let id = id.to_string();
        let accused_user_id = accused_user_id.to_string();
        let accuser_user_id = accuser_user_id.to_string();
        let github_event_id = github_event_id.map(|s| s.to_string());
        let evidence_json = evidence_json.to_string();
        let status = status.to_string();

        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gamify_disputes
                     (id, accused_user_id, accuser_user_id, github_event_id, snapshot_id,
                      evidence_json, malice_score, status, created_at, appeal_deadline_ts, penalty_applied)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)",
                    params![
                        id.as_str(),
                        accused_user_id.as_str(),
                        accuser_user_id.as_str(),
                        github_event_id.as_deref(),
                        snapshot_id,
                        evidence_json.as_str(),
                        malice_score,
                        status.as_str(),
                        created_at,
                        appeal_deadline_ts
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Update dispute status and penalty.
    pub async fn update_gamify_dispute_status(
        &self,
        id: &str,
        status: &str,
        penalty_applied: i64,
        resolved_at: Option<i64>,
    ) -> Result<(), StoreError> {
        let id = id.to_string();
        let status = status.to_string();
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "UPDATE gamify_disputes
                     SET status = ?1, penalty_applied = ?2, resolved_at = ?3
                     WHERE id = ?4",
                    params![status.as_str(), penalty_applied, resolved_at, id.as_str()],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Insert a vote on a dispute.
    pub async fn insert_gamify_dispute_vote(
        &self,
        dispute_id: &str,
        juror_user_id: &str,
        verdict: &str,
        rationale: Option<&str>,
        cast_at: i64,
    ) -> Result<(), StoreError> {
        let dispute_id = dispute_id.to_string();
        let juror_user_id = juror_user_id.to_string();
        let verdict = verdict.to_string();
        let rationale = rationale.map(|s| s.to_string());
        
        let breaker = self.breaker.clone();
        let conn = self.conn.clone();
        breaker
            .call(|| async move {
                conn.execute(
                    "INSERT INTO gamify_dispute_votes
                     (dispute_id, juror_user_id, verdict, rationale, cast_at)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(dispute_id, juror_user_id) DO UPDATE SET
                     verdict = excluded.verdict, rationale = excluded.rationale, cast_at = excluded.cast_at",
                    params![
                        dispute_id.as_str(),
                        juror_user_id.as_str(),
                        verdict.as_str(),
                        rationale.as_deref(),
                        cast_at
                    ],
                )
                .await?;
                Ok::<(), StoreError>(())
            })
            .await
    }

    /// Get disputes by status. Returns vector of strings representing disputes.
    pub async fn get_gamify_disputes_by_status(
        &self,
        status: &str,
    ) -> Result<Vec<Vec<Option<String>>>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT id, accused_user_id, accuser_user_id, github_event_id,
                    CAST(snapshot_id AS TEXT), evidence_json, CAST(malice_score AS TEXT),
                    status, CAST(created_at AS TEXT), CAST(resolved_at AS TEXT),
                    CAST(appeal_deadline_ts AS TEXT), CAST(penalty_applied AS TEXT)
             FROM gamify_disputes WHERE status = ?1",
            params![status],
        ).await?;
        
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let cols: Vec<Option<String>> = (0..12)
                .map(|i| row.get::<Option<String>>(i).unwrap_or(None))
                .collect();
            results.push(cols);
        }
        Ok(results)
    }

    /// Fetch up to `limit` available Master-tier jurors.
    pub async fn get_available_jurors(&self, limit: i64) -> Result<Vec<String>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT user_id FROM gamify_profiles WHERE trust_tier = 3 ORDER BY RANDOM() LIMIT ?1",
            params![limit],
        ).await?;
        
        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            if let Ok(user_id) = row.get::<String>(0) {
                results.push(user_id);
            }
        }
        Ok(results)
    }
}
