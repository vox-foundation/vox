//! Ledger + worker tick for scholarly [`external_submission_jobs`](vox_db::VoxDb) rows.

use anyhow::{Result, anyhow};
use serde_json::Value;
use vox_db::{
    ExternalSubmissionAttemptParams, ExternalSubmissionJobRow, ExternalSubmissionJobUpsertParams,
    ScholarlySubmissionRow, VoxDb,
};

use crate::publication::PublicationManifest;
use crate::scholarly::{self, ScholarlyError, ScholarlySubmissionReceipt};
use crate::scholarly_remote_status::{
    ScholarlyRemoteStatusMap, map_scholarly_remote_to_job_status,
};

/// `VOX_SCHOLARLY_ADAPTER` (default `local_ledger`), or non-empty `adapter_override` (trimmed, ASCII lowercased).
pub fn resolve_scholarly_adapter_kind(adapter_override: Option<&str>) -> String {
    if let Some(s) = adapter_override {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_ascii_lowercase();
        }
    }
    let raw = std::env::var("VOX_SCHOLARLY_ADAPTER").unwrap_or_default();
    let k = raw.trim();
    if k.is_empty() {
        "local_ledger".to_string()
    } else {
        k.to_ascii_lowercase()
    }
}

/// Full flow used by CLI / MCP: dual-approval gate, job row, submit via [`scholarly::submit_with_adapter`], ledger finish.
pub async fn publication_scholarly_submit_with_ledger(
    db: &VoxDb,
    publication_id: &str,
    adapter_override: Option<&str>,
) -> Result<ScholarlySubmissionReceipt> {
    let Some(row) = db
        .get_publication_manifest(publication_id)
        .await
        .map_err(|e| anyhow!("{e}"))?
    else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let dual = db
        .has_dual_publication_approval_for_digest(publication_id, &row.content_sha3_256)
        .await
        .map_err(|e| anyhow!("{e}"))?;
    if !dual {
        anyhow::bail!("publication requires two distinct digest-bound approvers before submission");
    }
    let digest = row.content_sha3_256.clone();
    let adapter_kind = resolve_scholarly_adapter_kind(adapter_override);
    let idem =
        scholarly::scholarly_idempotency_key(&adapter_kind, publication_id, &digest, "submit");
    let prior_job = db
        .get_external_submission_job_by_idempotency_key(&idem)
        .await
        .map_err(|e| anyhow!("{e}"))?;
    let attempt_base = prior_job.as_ref().map(|j| j.attempt_count).unwrap_or(0);
    let job_id = db
        .upsert_external_submission_job(ExternalSubmissionJobUpsertParams {
            publication_id,
            content_sha3_256: &digest,
            adapter: &adapter_kind,
            operation: "submit",
            idempotency_key: &idem,
            status: "running",
            lock_owner: None,
            lock_expires_at_ms: None,
            next_retry_at_ms: None,
            attempt_count: attempt_base,
            last_error_class: None,
            last_error_message: None,
            metadata_json: None,
        })
        .await
        .map_err(|e| anyhow!("{e}"))?;

    let manifest = PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let receipt = scholarly::submit_with_adapter(&manifest, adapter_kind.as_str()).await;
    submit_finish_ledger(
        db,
        publication_id,
        &digest,
        job_id,
        &idem,
        attempt_base,
        adapter_kind.as_str(),
        receipt,
    )
    .await
}

/// After polling remote status, refresh [`external_submission_jobs`] for the submit idempotency key when a row exists.
///
/// Updates `metadata_json` from `remote_detail_json` when provided. Uses [`map_scholarly_remote_to_job_status`] for
/// adapter-aware terminal detection; preserves the existing job `status` when the remote is non-terminal or unknown.
pub async fn sync_external_job_after_remote_status(
    db: &VoxDb,
    publication_id: &str,
    content_sha3_256: &str,
    adapter: &str,
    remote_status: &str,
    remote_detail_json: Option<&str>,
) -> Result<(bool, Option<ScholarlyRemoteStatusMap>)> {
    let adapter_norm = adapter.trim().to_ascii_lowercase();
    let idem = scholarly::scholarly_idempotency_key(
        &adapter_norm,
        publication_id,
        content_sha3_256,
        "submit",
    );
    let Some(job) = db
        .get_external_submission_job_by_idempotency_key(&idem)
        .await
        .map_err(|e| anyhow!("{e}"))?
    else {
        return Ok((false, None));
    };
    let mapped = map_scholarly_remote_to_job_status(
        adapter_norm.as_str(),
        remote_status,
        job.status.as_str(),
    );
    let job_status = if mapped.preserve_prior_job_status {
        job.status.as_str()
    } else {
        mapped.job_status.as_str()
    };
    let next_retry = if job_status == "succeeded" || job_status == "failed" {
        None
    } else {
        job.next_retry_at_ms
    };
    let meta = remote_detail_json.or(job.metadata_json.as_deref());
    db.upsert_external_submission_job(ExternalSubmissionJobUpsertParams {
        publication_id,
        content_sha3_256,
        adapter: adapter_norm.as_str(),
        operation: "submit",
        idempotency_key: idem.as_str(),
        status: job_status,
        lock_owner: None,
        lock_expires_at_ms: None,
        next_retry_at_ms: next_retry,
        attempt_count: job.attempt_count,
        last_error_class: job.last_error_class.as_deref(),
        last_error_message: job.last_error_message.as_deref(),
        metadata_json: meta,
    })
    .await
    .map_err(|e| anyhow!("{e}"))?;
    Ok((true, Some(mapped)))
}

/// Poll adapter remote status, persist snapshot, patch `scholarly_submissions`, and sync `external_submission_jobs` when present.
pub async fn poll_scholarly_remote_status_persist(
    db: &VoxDb,
    publication_id: &str,
    sub_row: &ScholarlySubmissionRow,
) -> Result<Value> {
    let adapter = sub_row.adapter.as_str();
    let ext_id = sub_row.external_submission_id.as_str();
    let remote = scholarly::fetch_scholarly_remote_status_for_adapter(adapter, ext_id)
        .await
        .map_err(|e| anyhow!("{e}"))?;
    let snapshot = serde_json::to_string(&serde_json::json!({
        "status": remote.status,
        "detail_json": remote.detail_json,
    }))?;
    db.insert_external_status_snapshot(vox_db::ExternalStatusSnapshotParams {
        adapter,
        external_submission_id: ext_id,
        publication_id,
        content_sha3_256: sub_row.content_sha3_256.as_str(),
        snapshot_json: snapshot.as_str(),
    })
    .await
    .map_err(|e| anyhow!("{e}"))?;
    db.patch_scholarly_submission_status(
        publication_id,
        adapter,
        ext_id,
        remote.status.as_str(),
        None,
    )
    .await
    .map_err(|e| anyhow!("{e}"))?;
    let (job_synced, job_status_map) = sync_external_job_after_remote_status(
        db,
        publication_id,
        sub_row.content_sha3_256.as_str(),
        adapter,
        remote.status.as_str(),
        remote.detail_json.as_deref(),
    )
    .await?;
    Ok(serde_json::json!({
        "publication_id": publication_id,
        "adapter": adapter,
        "external_submission_id": ext_id,
        "remote": remote,
        "external_status_snapshot_saved": true,
        "external_submission_job_synced": job_synced,
        "external_job_status_map": job_status_map,
    }))
}

/// Poll remote status for every `scholarly_submissions` row on `publication_id` (best-effort per row).
pub async fn poll_scholarly_remote_status_all_submissions_for_publication(
    db: &VoxDb,
    publication_id: &str,
) -> Result<Value> {
    let submissions = db
        .list_scholarly_submissions(publication_id)
        .await
        .map_err(|e| anyhow!("{e}"))?;
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

/// Cron/worker helper: for each publication id from `db.list_publication_ids_with_scholarly_submissions`, run [`poll_scholarly_remote_status_all_submissions_for_publication`].
pub async fn poll_scholarly_remote_status_batch(
    db: &VoxDb,
    publication_limit: i64,
) -> Result<Value> {
    let pub_ids = db
        .list_publication_ids_with_scholarly_submissions(publication_limit)
        .await
        .map_err(|e| anyhow!("{e}"))?;
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

fn loop_sleep_interval(base_secs: u64, jitter_secs: u64) -> std::time::Duration {
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

/// Run [`poll_scholarly_remote_status_batch`] multiple times with an optional pause (supervised worker / cron alternative).
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

/// Run [`run_external_submit_jobs_tick`] multiple times with an optional pause between batches.
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

/// Default lock owner: `VOX_SCHOLARLY_JOB_LOCK_OWNER` or `vox:<pid>`.
pub fn default_scholarly_job_lock_owner() -> String {
    if let Ok(s) = std::env::var("VOX_SCHOLARLY_JOB_LOCK_OWNER") {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    format!("vox:{}", std::process::id())
}

/// Record adapter HTTP outcome on the job + `scholarly_submissions` when `Ok`.
pub async fn submit_finish_ledger(
    db: &VoxDb,
    publication_id: &str,
    digest: &str,
    job_id: i64,
    idempotency_key: &str,
    attempt_base: i64,
    adapter_for_failed_upsert: &str,
    receipt: Result<ScholarlySubmissionReceipt, ScholarlyError>,
) -> Result<ScholarlySubmissionReceipt> {
    match receipt {
        Ok(receipt) => {
            let detail_ok = serde_json::json!({
                "external_submission_id": receipt.external_submission_id,
                "status": receipt.status,
            })
            .to_string();
            db.record_external_submission_attempt(ExternalSubmissionAttemptParams {
                job_id,
                http_status: Some(200),
                error_class: None,
                retryable: false,
                request_fingerprint: None,
                response_fingerprint: receipt.response_fingerprint.as_deref(),
                detail_json: Some(&detail_ok),
            })
            .await
            .map_err(|e| anyhow!("{e}"))?;
            let post_attempt = attempt_base + 1;
            db.upsert_external_submission_job(ExternalSubmissionJobUpsertParams {
                publication_id,
                content_sha3_256: digest,
                adapter: receipt.adapter.as_str(),
                operation: "submit",
                idempotency_key,
                status: "succeeded",
                lock_owner: None,
                lock_expires_at_ms: None,
                next_retry_at_ms: None,
                attempt_count: post_attempt,
                last_error_class: None,
                last_error_message: None,
                metadata_json: receipt.metadata_json.as_deref(),
            })
            .await
            .map_err(|e| anyhow!("{e}"))?;
            db.upsert_scholarly_submission(
                publication_id,
                digest,
                &receipt.adapter,
                &receipt.external_submission_id,
                &receipt.status,
                receipt.response_fingerprint.as_deref(),
                receipt.metadata_json.as_deref(),
            )
            .await
            .map_err(|e| anyhow!("{e}"))?;
            Ok(receipt)
        }
        Err(e) => {
            let err_msg = e.to_string();
            let detail = serde_json::json!({
                "error": err_msg,
                "error_class": e.error_class(),
            })
            .to_string();
            db.record_external_submission_attempt(ExternalSubmissionAttemptParams {
                job_id,
                http_status: scholarly::scholarly_http_status_code(&e),
                error_class: Some(e.error_class()),
                retryable: e.retryable(),
                request_fingerprint: None,
                response_fingerprint: None,
                detail_json: Some(&detail),
            })
            .await
            .map_err(|e| anyhow!("{e}"))?;
            let post_attempt = attempt_base + 1;
            let st = if e.retryable() {
                "retryable_failed"
            } else {
                "failed"
            };
            db.upsert_external_submission_job(ExternalSubmissionJobUpsertParams {
                publication_id,
                content_sha3_256: digest,
                adapter: adapter_for_failed_upsert,
                operation: "submit",
                idempotency_key,
                status: st,
                lock_owner: None,
                lock_expires_at_ms: None,
                next_retry_at_ms: scholarly::scholarly_retry_not_before_ms(&e),
                attempt_count: post_attempt,
                last_error_class: Some(e.error_class()),
                last_error_message: Some(err_msg.as_str()),
                metadata_json: None,
            })
            .await
            .map_err(|e| anyhow!("{e}"))?;
            Err(anyhow!("{e}"))
        }
    }
}

async fn external_job_tick_preflight(
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
        .map_err(|e| ExternalJobPreflightFailure::retryable(e.to_string()))?
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
        .map_err(|e| ExternalJobPreflightFailure::retryable(e.to_string()))?;
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
enum ExternalJobPreflightDisposition {
    Permanent,
    Retryable,
}

#[derive(Debug, Clone)]
struct ExternalJobPreflightFailure {
    disposition: ExternalJobPreflightDisposition,
    message: String,
}

impl ExternalJobPreflightFailure {
    fn permanent(message: String) -> Self {
        Self {
            disposition: ExternalJobPreflightDisposition::Permanent,
            message,
        }
    }

    fn retryable(message: String) -> Self {
        Self {
            disposition: ExternalJobPreflightDisposition::Retryable,
            message,
        }
    }

    fn is_permanent(&self) -> bool {
        matches!(self.disposition, ExternalJobPreflightDisposition::Permanent)
    }
}

fn scholarly_job_preflight_profile(
    adapter: &str,
) -> crate::publication_preflight::PreflightProfile {
    match adapter.trim().to_ascii_lowercase().as_str() {
        "zenodo" | "openreview" => crate::publication_preflight::PreflightProfile::MetadataComplete,
        _ => crate::publication_preflight::PreflightProfile::Default,
    }
}

fn scholarly_job_preflight_profile_label(adapter: &str) -> &'static str {
    match scholarly_job_preflight_profile(adapter) {
        crate::publication_preflight::PreflightProfile::Default => "default",
        crate::publication_preflight::PreflightProfile::DoubleBlind => "double_blind",
        crate::publication_preflight::PreflightProfile::MetadataComplete => "metadata_complete",
        crate::publication_preflight::PreflightProfile::ArxivAssist => "arxiv_assist",
    }
}

async fn external_job_mark_preflight_outcome(
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
    .map_err(|e| anyhow!("{e}"))?;
    Ok(())
}

/// Tick output for CLI / MCP (JSON payload).
pub struct ExternalJobsTickOutput {
    pub lock_owner: String,
    pub lock_ttl_ms: i64,
    pub results: Vec<Value>,
}

/// One batch: preflight due jobs, lease, submit with `job.adapter`.
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
        .map_err(|e| anyhow!("{e}"))?;
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
            .map_err(|e| anyhow!("{e}"))?;
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
            .map_err(|e| anyhow!("{e}"))?
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
            .map_err(|e| anyhow!("{e}"))?
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scholarly_job_preflight_profile_matches_adapter_capability() {
        assert_eq!(
            scholarly_job_preflight_profile("zenodo"),
            crate::publication_preflight::PreflightProfile::MetadataComplete
        );
        assert_eq!(
            scholarly_job_preflight_profile("openreview"),
            crate::publication_preflight::PreflightProfile::MetadataComplete
        );
        assert_eq!(
            scholarly_job_preflight_profile("local_ledger"),
            crate::publication_preflight::PreflightProfile::Default
        );
    }

    #[test]
    fn preflight_failure_tracks_retryability() {
        assert!(ExternalJobPreflightFailure::permanent("x".into()).is_permanent());
        assert!(!ExternalJobPreflightFailure::retryable("x".into()).is_permanent());
    }
}
