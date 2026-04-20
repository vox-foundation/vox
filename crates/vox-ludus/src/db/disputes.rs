use anyhow::Result;
use vox_db::Codex;
use crate::util::now_unix;

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
    db.insert_gamify_dispute(
        dispute_id,
        accused_user_id,
        accuser_user_id,
        github_event_id,
        snapshot_id,
        evidence_json,
        malice_score,
        "pending",
        now,
        appeal_deadline,
    )
    .await?;
    
    // Auto-assign jurors
    let available_jurors = db.get_available_jurors(3).await?;
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
    db.insert_gamify_dispute_vote(dispute_id, juror_user_id, verdict, rationale, now)
        .await?;
    Ok(())
}

/// Assign a jury to a dispute.
pub async fn assign_jury(
    db: &Codex,
    dispute_id: &str,
    juror_ids: &[String],
) -> Result<()> {
    let now = now_unix();
    // Use the inner VoxDb connection via db object to insert raw assignments
    let conn = db.connection().clone();
    let breaker = db.breaker().clone();
    
    // Instead of doing raw SQL here which violates the pattern, 
    // we should ideally add an insert_gamify_jury method to VoxDb.
    // For now, we will execute it inside a breaker block since we need batch insert.
    for juror in juror_ids {
        let dispute_id = dispute_id.to_string();
        let juror = juror.to_string();
        let conn_clone = conn.clone();
        breaker.call(|| async move {
            conn_clone.execute(
                "INSERT INTO gamify_dispute_jury (dispute_id, juror_user_id, assigned_at)
                 VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                turso::params![dispute_id.as_str(), juror.as_str(), now]
            ).await?;
            Ok::<(), vox_db::store::types::StoreError>(())
        }).await?;
    }
    Ok(())
}

/// Appeal a dispute.
pub async fn appeal_dispute(
    db: &Codex,
    dispute_id: &str,
) -> Result<()> {
    // Basic implementation for MVP. Should technically check if `now < appeal_deadline_ts`
    db.update_gamify_dispute_status(dispute_id, "appealed", 0, None).await?;
    Ok(())
}
