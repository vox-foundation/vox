#![cfg(feature = "scholarly-external-jobs")]
use anyhow::{Result, anyhow};
use vox_db::{
    ExternalSubmissionAttemptParams, ExternalSubmissionJobUpsertParams,
    VoxDb, StoreError,
};
use crate::publication::PublicationManifest;
use crate::scholarly::{self, ScholarlyError, ScholarlySubmissionReceipt};
use super::resolve_scholarly_adapter_kind;

pub async fn publication_scholarly_submit_with_ledger(
    db: &VoxDb,
    publication_id: &str,
    adapter_override: Option<&str>,
) -> Result<ScholarlySubmissionReceipt> {
    let Some(row) = db
        .get_publication_manifest(publication_id)
        .await
        .map_err(|e: StoreError| anyhow!("{e}"))?
    else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let dual = db
        .has_dual_publication_approval_for_digest(publication_id, &row.content_sha3_256)
        .await
        .map_err(|e: StoreError| anyhow!("{e}"))?;
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
        .map_err(|e: StoreError| anyhow!("{e}"))?;
    let attempt_base = prior_job.as_ref().map(|j| j.attempt_count).unwrap_or(0);
    let job_id = db
        .upsert_external_submission_job(ExternalSubmissionJobUpsertParams {
            // ... (keep fields same)
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
        .map_err(|e: StoreError| anyhow!("{e}"))?;

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
            .map_err(|e: StoreError| anyhow!("{e}"))?;
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
            .map_err(|e: StoreError| anyhow!("{e}"))?;
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
            .map_err(|e: StoreError| anyhow!("{e}"))?;
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
            .map_err(|e: vox_db::StoreError| anyhow!("{e}"))?;
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
            .map_err(|e: StoreError| anyhow!("{e}"))?;
            Err(anyhow!("{e}"))
        }
    }
}
