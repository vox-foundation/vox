use crate::util::now_unix;
use anyhow::Result;
use turso::params;
use vox_db::Codex;

/// File a dispute against a user for potentially malicious behavior.
pub async fn file_dispute(
    db: &Codex,
    dispute_id: &str,
    accused_user_id: &str,
    accuser_user_id: &str,
    github_event_id: Option<&str>,
    snapshot_id: Option<i64>,
    evidence_json: &str,
    malice_score: f64,
) -> Result<()> {
    let now = now_unix();
    let appeal_deadline = now + 86400 * 7; // 7 days

    let id = dispute_id.to_string();
    let accused = accused_user_id.to_string();
    let accuser = accuser_user_id.to_string();
    let github_ev = github_event_id.map(str::to_string);
    let evidence = evidence_json.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_disputes
                 (id, accused_user_id, accuser_user_id, github_event_id, snapshot_id,
                  evidence_json, malice_score, status, created_at, appeal_deadline_ts, penalty_applied)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0)",
                params![
                    id.as_str(),
                    accused.as_str(),
                    accuser.as_str(),
                    github_ev.as_deref(),
                    snapshot_id,
                    evidence.as_str(),
                    malice_score,
                    "pending",
                    now,
                    appeal_deadline
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Auto-assign jurors
    let available_jurors = get_available_jurors(db, 3).await?;
    if !available_jurors.is_empty() {
        assign_jury(db, dispute_id, &available_jurors).await?;
    }

    Ok(())
}

/// Cast a vote as a juror on a dispute.
pub async fn cast_vote(
    db: &Codex,
    dispute_id: &str,
    juror_user_id: &str,
    verdict: &str,
    rationale: Option<&str>,
) -> Result<()> {
    let now = now_unix();
    let dispute_id = dispute_id.to_string();
    let juror = juror_user_id.to_string();
    let verdict = verdict.to_string();
    let rationale = rationale.map(str::to_string);
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
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
                    juror.as_str(),
                    verdict.as_str(),
                    rationale.as_deref(),
                    now
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Assign a jury to a dispute.
pub async fn assign_jury(db: &Codex, dispute_id: &str, juror_ids: &[String]) -> Result<()> {
    let now = now_unix();
    let conn = db.connection().clone();
    let breaker = db.breaker().clone();

    for juror in juror_ids {
        let dispute_id = dispute_id.to_string();
        let juror = juror.to_string();
        let conn_clone = conn.clone();
        breaker
            .call(|| async move {
                conn_clone
                    .execute(
                        "INSERT INTO gamify_dispute_jury (dispute_id, juror_user_id, assigned_at)
                 VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                        params![dispute_id.as_str(), juror.as_str(), now],
                    )
                    .await?;
                Ok::<(), vox_db::StoreError>(())
            })
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }
    Ok(())
}

/// Appeal a dispute.
pub async fn appeal_dispute(db: &Codex, dispute_id: &str) -> Result<()> {
    let id = dispute_id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "UPDATE gamify_disputes
                 SET status = ?1, penalty_applied = ?2, resolved_at = ?3
                 WHERE id = ?4",
                params!["appealed", 0i64, Option::<i64>::None, id.as_str()],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Fetch up to `limit` available Master-tier jurors.
async fn get_available_jurors(db: &Codex, limit: i64) -> Result<Vec<String>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT user_id FROM gamify_profiles WHERE trust_tier = 3 ORDER BY RANDOM() LIMIT ?1",
            params![limit],
        )
        .await?;
    let mut results = Vec::new();
    while let Some(row) = rows.next().await? {
        if let Ok(user_id) = row.get::<String>(0) {
            results.push(user_id);
        }
    }
    Ok(results)
}
