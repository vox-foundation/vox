#![cfg(feature = "scholarly-external-jobs")]
use crate::scholarly::{self, ScholarlyError};
use crate::scholarly_remote_status::{
    ScholarlyRemoteStatusMap, map_scholarly_remote_to_job_status,
};
use anyhow::{Result, anyhow};
use serde_json::Value;
use vox_db::{ExternalSubmissionJobUpsertParams, ScholarlySubmissionRow, StoreError, VoxDb};

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
        .map_err(|e: StoreError| anyhow!("{e}"))?
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
    .map_err(|e: StoreError| anyhow!("{e}"))?;
    Ok((true, Some(mapped)))
}

pub async fn poll_scholarly_remote_status_persist(
    db: &VoxDb,
    publication_id: &str,
    sub_row: &ScholarlySubmissionRow,
) -> Result<Value> {
    let adapter = sub_row.adapter.as_str();
    let ext_id = sub_row.external_submission_id.as_str();
    let remote = scholarly::fetch_scholarly_remote_status_for_adapter(adapter, ext_id)
        .await
        .map_err(|e: ScholarlyError| anyhow!("{e}"))?;
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
    .map_err(|e: StoreError| anyhow!("{e}"))?;
    db.patch_scholarly_submission_status(
        publication_id,
        adapter,
        ext_id,
        remote.status.as_str(),
        None,
    )
    .await
    .map_err(|e: StoreError| anyhow!("{e}"))?;
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
