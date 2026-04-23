#![cfg(feature = "scholarly-external-jobs")]
use super::poll::loop_sleep_interval;
use super::submit::submit_finish_ledger;
use super::{ExternalJobsTickOutput, default_scholarly_job_lock_owner};
use crate::publication::PublicationManifest;
use crate::scholarly;
use anyhow::{Result, anyhow};
use serde_json::Value;
use vox_db::{ExternalSubmissionJobRow, ExternalSubmissionJobUpsertParams, StoreError, VoxDb};

pub async fn run_external_submit_jobs_tick(
    db: &VoxDb,
    limit: i64,
    lock_ttl_ms: i64,
    lock_owner: Option<&str>,
    now_ms: i64,
) -> Result<ExternalJobsTickOutput> {
    let lock_ttl_ms = lock_ttl_ms.clamp(5_000, 3_600_000);
    let owner = lock_owner
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(default_scholarly_job_lock_owner);
    let lim = limit.clamp(1, 500);
    let jobs = db
        .list_external_submission_jobs_due(now_ms, lim)
        .await
        .map_err(|e: StoreError| anyhow!("{e}"))?;
    let mut results: Vec<Value> = Vec::new();
    for job in jobs {
        if let Err(failure) = external_job_tick_preflight(db, &job).await {
            external_job_mark_preflight_outcome(
                db,
                &job,
                now_ms,
                failure.is_permanent(),
                &failure.message,
            )
            .await?;
            results.push(serde_json::json!({
                "job_id": job.id,
                "outcome": "preflight_rejected",
                "permanent": failure.is_permanent(),
                "message": failure.message,
                "preflight_profile": scholarly_job_preflight_profile_label(job.adapter.as_str()),
            }));
            continue;
        }
        let claimed = db
            .try_claim_external_submission_job(
                job.id,
                &owner,
                now_ms.saturating_add(lock_ttl_ms),
                now_ms,
            )
            .await
            .map_err(|e: StoreError| anyhow!("{e}"))?;
        if !claimed {
            results.push(serde_json::json!({
                "job_id": job.id,
                "outcome": "claim_lost",
                "current_lock_owner": job.lock_owner,
                "lock_expires_at_ms": job.lock_expires_at_ms,
            }));
            continue;
        }
        let Some(j2) = db
            .get_external_submission_job_by_id(job.id)
            .await
            .map_err(|e: StoreError| anyhow!("{e}"))?
        else {
            results.push(serde_json::json!({
                "job_id": job.id,
                "outcome": "error",
                "message": "job row missing after claim",
            }));
            continue;
        };
        let attempt_base = j2.attempt_count;
        let Some(mrow) = db
            .get_publication_manifest(&j2.publication_id)
            .await
            .map_err(|e: StoreError| anyhow!("{e}"))?
        else {
            external_job_mark_preflight_outcome(
                db,
                &j2,
                now_ms,
                true,
                "publication manifest not found after claim",
            )
            .await?;
            results.push(serde_json::json!({
                "job_id": job.id,
                "outcome": "manifest_missing",
            }));
            continue;
        };
        let manifest = PublicationManifest {
            publication_id: mrow.publication_id.clone(),
            content_type: mrow.content_type.clone(),
            source_ref: mrow.source_ref.clone(),
            title: mrow.title.clone(),
            author: mrow.author.clone(),
            abstract_text: mrow.abstract_text.clone(),
            body_markdown: mrow.body_markdown.clone(),
            citations_json: mrow.citations_json.clone(),
            metadata_json: mrow.metadata_json.clone(),
        };
        let receipt = scholarly::submit_with_adapter(&manifest, j2.adapter.as_str()).await;
        match submit_finish_ledger(
            db,
            j2.publication_id.as_str(),
            j2.content_sha3_256.as_str(),
            j2.id,
            j2.idempotency_key.as_str(),
            attempt_base,
            j2.adapter.as_str(),
            receipt,
        )
        .await
        {
            Ok(r) => {
                results.push(serde_json::json!({
                    "job_id": job.id,
                    "outcome": "succeeded",
                    "adapter": r.adapter,
                    "external_submission_id": r.external_submission_id,
                    "status": r.status,
                }));
            }
            Err(e) => {
                results.push(serde_json::json!({
                    "job_id": job.id,
                    "outcome": "submit_failed",
                    "message": e.to_string(),
                }));
            }
        }
    }
    Ok(ExternalJobsTickOutput {
        lock_owner: owner,
        lock_ttl_ms,
        results,
    })
}

pub async fn run_external_submit_jobs_tick_loop(
    db: &VoxDb,
    limit: i64,
    lock_ttl_ms: i64,
    lock_owner: Option<&str>,
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
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let out = run_external_submit_jobs_tick(db, limit, lock_ttl_ms, lock_owner, now_ms).await?;
        runs.push(serde_json::json!({
            "now_ms": now_ms,
            "lock_owner": out.lock_owner,
            "lock_ttl_ms": out.lock_ttl_ms,
            "results": out.results,
        }));
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

pub async fn external_job_tick_preflight(
    db: &VoxDb,
    job: &ExternalSubmissionJobRow,
) -> Result<(), ExternalJobPreflightFailure> {
    if job.operation != "submit" {
        return Err(ExternalJobPreflightFailure::permanent(format!(
            "unsupported external_submission_jobs.operation={:?} (only \"submit\" is supported)",
            job.operation
        )));
    }
    let Some(row) = db
        .get_publication_manifest(&job.publication_id)
        .await
        .map_err(|e: StoreError| ExternalJobPreflightFailure::retryable(e.to_string()))?
    else {
        return Err(ExternalJobPreflightFailure::permanent(
            "publication manifest not found".into(),
        ));
    };
    if row.content_sha3_256 != job.content_sha3_256 {
        return Err(ExternalJobPreflightFailure::permanent(
            "job content_sha3_256 does not match current publication manifest digest (re-submit from CLI or delete job)"
                .into(),
        ));
    }
    let dual = db
        .has_dual_publication_approval_for_digest(&job.publication_id, &job.content_sha3_256)
        .await
        .map_err(|e: StoreError| ExternalJobPreflightFailure::retryable(e.to_string()))?;
    if !dual {
        return Err(ExternalJobPreflightFailure::retryable(
            "dual digest-bound approvals required before scholarly submit or retry".into(),
        ));
    }
    let manifest = PublicationManifest {
        publication_id: row.publication_id,
        content_type: row.content_type,
        source_ref: row.source_ref,
        title: row.title,
        author: row.author,
        abstract_text: row.abstract_text,
        body_markdown: row.body_markdown,
        citations_json: row.citations_json,
        metadata_json: row.metadata_json,
    };
    let profile = scholarly_job_preflight_profile(job.adapter.as_str());
    let report = crate::publication_preflight::run_preflight(&manifest, profile);
    if !report.ok {
        return Err(ExternalJobPreflightFailure::permanent(format!(
            "preflight {:?} rejected submit: readiness={} errors={} warnings={}",
            profile,
            report.readiness_score,
            report
                .findings
                .iter()
                .filter(|f| f.severity == crate::publication_preflight::PreflightSeverity::Error)
                .count(),
            report
                .findings
                .iter()
                .filter(|f| f.severity == crate::publication_preflight::PreflightSeverity::Warning)
                .count(),
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalJobPreflightDisposition {
    Permanent,
    Retryable,
}

#[derive(Debug, Clone)]
pub struct ExternalJobPreflightFailure {
    pub disposition: ExternalJobPreflightDisposition,
    pub message: String,
}

impl ExternalJobPreflightFailure {
    pub fn permanent(message: String) -> Self {
        Self {
            disposition: ExternalJobPreflightDisposition::Permanent,
            message,
        }
    }

    pub fn retryable(message: String) -> Self {
        Self {
            disposition: ExternalJobPreflightDisposition::Retryable,
            message,
        }
    }

    pub fn is_permanent(&self) -> bool {
        matches!(self.disposition, ExternalJobPreflightDisposition::Permanent)
    }
}

pub fn scholarly_job_preflight_profile(
    adapter: &str,
) -> crate::publication_preflight::PreflightProfile {
    match adapter.trim().to_ascii_lowercase().as_str() {
        "zenodo" | "openreview" => crate::publication_preflight::PreflightProfile::MetadataComplete,
        _ => crate::publication_preflight::PreflightProfile::Default,
    }
}

pub fn scholarly_job_preflight_profile_label(adapter: &str) -> &'static str {
    match scholarly_job_preflight_profile(adapter) {
        crate::publication_preflight::PreflightProfile::Default => "default",
        crate::publication_preflight::PreflightProfile::DoubleBlind => "double_blind",
        crate::publication_preflight::PreflightProfile::MetadataComplete => "metadata_complete",
        crate::publication_preflight::PreflightProfile::ArxivAssist => "arxiv_assist",
        crate::publication_preflight::PreflightProfile::NewsInbound => "news_inbound",
    }
}

pub async fn external_job_mark_preflight_outcome(
    db: &VoxDb,
    job: &ExternalSubmissionJobRow,
    now_ms: i64,
    permanent: bool,
    message: &str,
) -> Result<()> {
    let status = if permanent {
        "failed"
    } else {
        "retryable_failed"
    };
    let next_retry_at_ms = if permanent {
        None
    } else {
        Some(now_ms.saturating_add(300_000))
    };
    db.upsert_external_submission_job(ExternalSubmissionJobUpsertParams {
        publication_id: job.publication_id.as_str(),
        content_sha3_256: job.content_sha3_256.as_str(),
        adapter: job.adapter.as_str(),
        operation: job.operation.as_str(),
        idempotency_key: job.idempotency_key.as_str(),
        status,
        lock_owner: None,
        lock_expires_at_ms: None,
        next_retry_at_ms,
        attempt_count: job.attempt_count,
        last_error_class: Some("preflight"),
        last_error_message: Some(message),
        metadata_json: None,
    })
    .await
    .map_err(|e: StoreError| anyhow!("{e}"))?;
    Ok(())
}
