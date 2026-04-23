#![cfg(feature = "scholarly-external-jobs")]
use super::sync::poll_scholarly_remote_status_persist;
use anyhow::{Result, anyhow};
use serde_json::Value;
use vox_db::{StoreError, VoxDb};

pub async fn poll_scholarly_remote_status_all_submissions_for_publication(
    db: &VoxDb,
    publication_id: &str,
) -> Result<Value> {
    let submissions = db
        .list_scholarly_submissions(publication_id)
        .await
        .map_err(|e: StoreError| anyhow!("{e}"))?;
    if submissions.is_empty() {
        anyhow::bail!("no scholarly submissions for publication {publication_id}");
    }
    let mut results: Vec<Value> = Vec::new();
    for sub in &submissions {
        match poll_scholarly_remote_status_persist(db, publication_id, sub).await {
            Ok(v) => results.push(v),
            Err(e) => {
                results.push(serde_json::json!({
                    "publication_id": publication_id,
                    "adapter": sub.adapter,
                    "external_submission_id": sub.external_submission_id,
                    "error": e.to_string(),
                }));
            }
        }
    }
    Ok(serde_json::json!({
        "publication_id": publication_id,
        "polled": results.len(),
        "results": results,
    }))
}

pub async fn poll_scholarly_remote_status_batch(
    db: &VoxDb,
    publication_limit: i64,
) -> Result<Value> {
    let pub_ids = db
        .list_publication_ids_with_scholarly_submissions(publication_limit)
        .await
        .map_err(|e: StoreError| anyhow!("{e}"))?;
    let mut publications: Vec<Value> = Vec::new();
    for publication_id in &pub_ids {
        match poll_scholarly_remote_status_all_submissions_for_publication(db, publication_id).await
        {
            Ok(v) => publications.push(v),
            Err(e) => {
                publications.push(serde_json::json!({
                    "publication_id": publication_id,
                    "error": e.to_string(),
                }));
            }
        }
    }
    Ok(serde_json::json!({
        "publication_count": pub_ids.len(),
        "publications": publications,
    }))
}

pub async fn poll_scholarly_remote_status_batch_loop(
    db: &VoxDb,
    publication_limit: i64,
    iterations: u32,
    interval_secs: u64,
    max_runtime_secs: Option<u64>,
    jitter_secs: u64,
) -> Result<Value> {
    let iterations = iterations.clamp(1, 10_000);
    let interval_secs = interval_secs.min(3_600);
    let jitter_secs = jitter_secs.min(3_600);
    let max_runtime = max_runtime_secs.map(|m| m.min(86_400));
    let started = std::time::Instant::now();
    let mut runs: Vec<Value> = Vec::new();
    let mut done = 0_u32;
    for i in 0..iterations {
        if let Some(limit_s) = max_runtime
            && started.elapsed().as_secs() >= limit_s
        {
            break;
        }
        if i > 0 && interval_secs > 0 {
            tokio::time::sleep(loop_sleep_interval(interval_secs, jitter_secs)).await;
        }
        let v = poll_scholarly_remote_status_batch(db, publication_limit).await?;
        runs.push(v);
        done += 1;
    }
    Ok(serde_json::json!({
        "iterations_requested": iterations,
        "iterations_completed": done,
        "interval_secs": interval_secs,
        "jitter_secs": jitter_secs,
        "max_runtime_secs": max_runtime,
        "elapsed_ms": started.elapsed().as_millis() as u64,
        "runs": runs,
    }))
}

pub(crate) fn loop_sleep_interval(base_secs: u64, jitter_secs: u64) -> std::time::Duration {
    let base = base_secs.min(3_600);
    let jcap = jitter_secs.min(base);
    let jitter = if jcap == 0 {
        0_u64
    } else {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 % (jcap + 1))
            .unwrap_or(0)
    };
    std::time::Duration::from_secs(base.saturating_add(jitter.min(base)))
}
