use anyhow::Result;

/// Poll the remote scholarly repository for the latest stored submission (or one matching `external_submission_id`).
pub async fn publication_scholarly_remote_status(
    publication_id: &str,
    external_submission_id: Option<&str>,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let submissions = db.list_scholarly_submissions(publication_id).await?;
    let sub_row: &vox_db::ScholarlySubmissionRow = match external_submission_id {
        Some(e) => {
            let e = e.trim();
            if e.is_empty() {
                anyhow::bail!("--external-submission-id must not be empty when provided");
            }
            submissions
                .iter()
                .find(|r| r.external_submission_id == e)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "no scholarly submission for publication {publication_id} with external_submission_id {e}"
                    )
                })?
        }
        None => submissions.first().ok_or_else(|| {
            anyhow::anyhow!("no scholarly submissions for publication {publication_id}")
        })?,
    };
    let v = vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_persist(
        &db,
        publication_id,
        sub_row,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
/// Poll remote status for **every** `scholarly_submissions` row for this publication (continues on per-row errors).
pub async fn publication_scholarly_remote_status_sync_all(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let v = vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_all_submissions_for_publication(
        &db,
        publication_id,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
/// Batch remote status poll across publications (distinct ids by recent `scholarly_submissions` activity). For cron/operators.
pub async fn publication_scholarly_remote_status_sync_batch(
    limit: i64,
    iterations: u32,
    interval_secs: u64,
    max_runtime_secs: Option<u64>,
    jitter_secs: u64,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let v = if iterations <= 1
        && interval_secs == 0
        && max_runtime_secs.is_none()
        && jitter_secs == 0
    {
        vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_batch(&db, limit).await
    } else {
        vox_publisher::scholarly_external_jobs::poll_scholarly_remote_status_batch_loop(
            &db,
            limit,
            iterations,
            interval_secs,
            max_runtime_secs,
            jitter_secs,
        )
        .await
    }
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
/// Read-only metrics rollup for the scholarly external pipeline and related publication attempt channels.
pub async fn publication_external_pipeline_metrics(since_hours: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let hours = since_hours.clamp(0, 8_760);
    let since_ms = if hours == 0 {
        0_i64
    } else {
        now_ms.saturating_sub(hours.saturating_mul(3_600_000))
    };
    let v = db
        .summarize_scholarly_external_pipeline_metrics(since_ms)
        .await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
/// Operator view of scholarly outbound jobs eligible for a retry worker (`queued` / due `retryable_failed`).
pub async fn publication_external_jobs_due(limit: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let before_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let jobs = db
        .list_external_submission_jobs_due(before_ms, limit)
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "due_before_ms_inclusive": before_ms,
            "jobs": jobs,
        }))?
    );
    Ok(())
}
/// List `external_submission_jobs` in terminal **`failed`** state (not scheduled for retry).
pub async fn publication_external_jobs_dead_letter(limit: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let jobs = db.list_external_submission_jobs_failed(limit).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "jobs": jobs }))?
    );
    Ok(())
}
/// Requeue one dead-letter job (`status = failed`) to `queued` for the next `publication-external-jobs-tick`.
pub async fn publication_external_jobs_replay(job_id: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let row = db
        .replay_failed_external_submission_job_to_queued(job_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "replayed": true,
            "job": row,
        }))?
    );
    Ok(())
}
/// Process one batch of due `external_submission_jobs`: preflight, lease, scholarly `submit` using the job's adapter.
pub async fn publication_external_jobs_tick(
    limit: i64,
    lock_ttl_ms: i64,
    lock_owner: Option<&str>,
    iterations: u32,
    interval_secs: u64,
    max_runtime_secs: Option<u64>,
    jitter_secs: u64,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    if iterations <= 1 && interval_secs == 0 && max_runtime_secs.is_none() && jitter_secs == 0 {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let out = vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick(
            &db,
            limit,
            lock_ttl_ms,
            lock_owner,
            now_ms,
        )
        .await?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "now_ms": now_ms,
                "lock_owner": out.lock_owner,
                "lock_ttl_ms": out.lock_ttl_ms,
                "results": out.results,
            }))?
        );
        return Ok(());
    }
    let v = vox_publisher::scholarly_external_jobs::run_external_submit_jobs_tick_loop(
        &db,
        limit,
        lock_ttl_ms,
        lock_owner,
        iterations,
        interval_secs,
        max_runtime_secs,
        jitter_secs,
    )
    .await?;
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}
