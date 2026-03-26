//! Scholarly repository adapters (Zenodo, OpenReview, local/echo ledger).

mod error;
mod flags;
mod idempotency;
mod openreview;
mod zenodo;

pub use error::{
    ScholarlyError, scholarly_http_status_code, scholarly_retry_not_before_ms,
};
pub use idempotency::scholarly_idempotency_key;

use async_trait::async_trait;

use crate::publication::PublicationManifest;

/// Receipt returned by scholarly adapter submit operations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScholarlySubmissionReceipt {
    pub adapter: String,
    pub external_submission_id: String,
    pub status: String,
    pub response_fingerprint: Option<String>,
    pub metadata_json: Option<String>,
}

/// Normalized remote status snapshot (polling).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScholarlyRemoteStatus {
    pub status: String,
    pub detail_json: Option<String>,
}

/// Async adapter contract for scholarly repositories (network-capable).
#[async_trait]
pub trait ScholarlyAdapter: Send + Sync {
    fn adapter_name(&self) -> &'static str;
    async fn submit(
        &self,
        manifest: &PublicationManifest,
    ) -> Result<ScholarlySubmissionReceipt, ScholarlyError>;
    async fn fetch_status(
        &self,
        external_submission_id: &str,
    ) -> Result<ScholarlyRemoteStatus, ScholarlyError>;
}

/// First integration path: local scholarly ledger — no network I/O.
#[derive(Debug, Default, Clone)]
pub struct LocalLedgerAdapter;

#[async_trait]
impl ScholarlyAdapter for LocalLedgerAdapter {
    fn adapter_name(&self) -> &'static str {
        "local_ledger"
    }

    async fn submit(
        &self,
        manifest: &PublicationManifest,
    ) -> Result<ScholarlySubmissionReceipt, ScholarlyError> {
        let digest = manifest.content_sha3_256();
        let external_submission_id = format!("local-{}-v1", &digest[..12.min(digest.len())]);
        Ok(ScholarlySubmissionReceipt {
            adapter: self.adapter_name().to_string(),
            external_submission_id,
            status: "submitted".to_string(),
            response_fingerprint: Some(digest),
            metadata_json: Some(
                serde_json::json!({
                    "channel": "scholarly",
                    "mode": "local_ledger"
                })
                .to_string(),
            ),
        })
    }

    async fn fetch_status(
        &self,
        external_submission_id: &str,
    ) -> Result<ScholarlyRemoteStatus, ScholarlyError> {
        Ok(ScholarlyRemoteStatus {
            status: "local_synthetic".into(),
            detail_json: Some(
                serde_json::json!({ "external_submission_id": external_submission_id }).to_string(),
            ),
        })
    }
}

/// Deterministic id without external I/O (CI / tests).
#[derive(Debug, Default, Clone)]
pub struct EchoLedgerAdapter;

#[async_trait]
impl ScholarlyAdapter for EchoLedgerAdapter {
    fn adapter_name(&self) -> &'static str {
        "echo_ledger"
    }

    async fn submit(
        &self,
        manifest: &PublicationManifest,
    ) -> Result<ScholarlySubmissionReceipt, ScholarlyError> {
        let digest = manifest.content_sha3_256();
        let external_submission_id = format!("echo-{}", &digest[..16.min(digest.len())]);
        Ok(ScholarlySubmissionReceipt {
            adapter: self.adapter_name().to_string(),
            external_submission_id,
            status: "recorded".to_string(),
            response_fingerprint: Some(digest.clone()),
            metadata_json: Some(
                serde_json::json!({
                    "channel": "scholarly",
                    "mode": "echo_ledger",
                    "note": "no external repository call"
                })
                .to_string(),
            ),
        })
    }

    async fn fetch_status(
        &self,
        external_submission_id: &str,
    ) -> Result<ScholarlyRemoteStatus, ScholarlyError> {
        Ok(ScholarlyRemoteStatus {
            status: "echo_synthetic".into(),
            detail_json: Some(
                serde_json::json!({ "external_submission_id": external_submission_id }).to_string(),
            ),
        })
    }
}

async fn submit_for_adapter_normalized(
    manifest: &PublicationManifest,
    kind: &str,
) -> Result<ScholarlySubmissionReceipt, ScholarlyError> {
    if flags::scholarly_globally_disabled() {
        return Err(ScholarlyError::Disabled {
            reason: "VOX_SCHOLARLY_DISABLE is set".into(),
        });
    }
    let kind = kind.trim();
    if kind.is_empty() || kind.eq_ignore_ascii_case("local_ledger") {
        return LocalLedgerAdapter.submit(manifest).await;
    }
    if kind.eq_ignore_ascii_case("echo_ledger") {
        return EchoLedgerAdapter.submit(manifest).await;
    }
    if kind.eq_ignore_ascii_case("zenodo") {
        if flags::scholarly_live_globally_disabled() {
            return Err(ScholarlyError::Disabled {
                reason: "VOX_SCHOLARLY_DISABLE_LIVE is set".into(),
            });
        }
        let adapter = zenodo::zenodo_from_clavis()?;
        return adapter.submit(manifest).await;
    }
    if kind.eq_ignore_ascii_case("openreview") {
        if flags::scholarly_live_globally_disabled() {
            return Err(ScholarlyError::Disabled {
                reason: "VOX_SCHOLARLY_DISABLE_LIVE is set".into(),
            });
        }
        let adapter = openreview::openreview_adapter_from_env().await?;
        return adapter.submit(manifest).await;
    }
    Err(ScholarlyError::Config {
        message: format!(
            "unsupported scholarly adapter {kind:?} (supported: local_ledger, echo_ledger, zenodo, openreview)"
        ),
    })
}

/// Submit using an explicit adapter name (e.g. from `external_submission_jobs.adapter`), case-insensitive.
///
/// Same flags as [`submit_with_configured_adapter`]: `VOX_SCHOLARLY_DISABLE*`, live and per-adapter disables.
pub async fn submit_with_adapter(
    manifest: &PublicationManifest,
    adapter: &str,
) -> Result<ScholarlySubmissionReceipt, ScholarlyError> {
    let k = adapter.trim().to_ascii_lowercase();
    submit_for_adapter_normalized(manifest, k.as_str()).await
}

/// Resolve [`VOX_SCHOLARLY_ADAPTER`] (default `local_ledger`) and submit.
///
/// Supported: `local_ledger`, `echo_ledger`, `zenodo`, `openreview`.  
/// Live adapters honor `VOX_SCHOLARLY_DISABLE`, `VOX_SCHOLARLY_DISABLE_LIVE`, and per-adapter `VOX_SCHOLARLY_DISABLE_*`.
pub async fn submit_with_configured_adapter(
    manifest: &PublicationManifest,
) -> Result<ScholarlySubmissionReceipt, ScholarlyError> {
    let raw = std::env::var("VOX_SCHOLARLY_ADAPTER").unwrap_or_default();
    let k = raw.trim();
    let k = if k.is_empty() {
        "local_ledger".to_string()
    } else {
        k.to_ascii_lowercase()
    };
    submit_for_adapter_normalized(manifest, k.as_str()).await
}

/// Best-effort status poll for the configured adapter.
pub async fn fetch_status_with_configured_adapter(
    external_submission_id: &str,
) -> Result<ScholarlyRemoteStatus, ScholarlyError> {
    if flags::scholarly_globally_disabled() {
        return Err(ScholarlyError::Disabled {
            reason: "VOX_SCHOLARLY_DISABLE is set".into(),
        });
    }
    let raw = std::env::var("VOX_SCHOLARLY_ADAPTER").unwrap_or_default();
    let kind = raw.trim();
    if kind.is_empty() || kind.eq_ignore_ascii_case("local_ledger") {
        return LocalLedgerAdapter
            .fetch_status(external_submission_id)
            .await;
    }
    if kind.eq_ignore_ascii_case("echo_ledger") {
        return EchoLedgerAdapter
            .fetch_status(external_submission_id)
            .await;
    }
    if kind.eq_ignore_ascii_case("zenodo") {
        if flags::scholarly_live_globally_disabled() {
            return Err(ScholarlyError::Disabled {
                reason: "VOX_SCHOLARLY_DISABLE_LIVE is set".into(),
            });
        }
        let adapter = zenodo::zenodo_from_clavis()?;
        return adapter.fetch_status(external_submission_id).await;
    }
    if kind.eq_ignore_ascii_case("openreview") {
        if flags::scholarly_live_globally_disabled() {
            return Err(ScholarlyError::Disabled {
                reason: "VOX_SCHOLARLY_DISABLE_LIVE is set".into(),
            });
        }
        let adapter = openreview::openreview_adapter_from_env().await?;
        return adapter.fetch_status(external_submission_id).await;
    }
    Err(ScholarlyError::Config {
        message: format!("unsupported VOX_SCHOLARLY_ADAPTER for status: {kind:?}"),
    })
}

/// Poll remote status for a scholarly adapter name (e.g. from `scholarly_submissions.adapter`),
/// honoring the same global and per-adapter disable flags as submit/status helpers.
pub async fn fetch_scholarly_remote_status_for_adapter(
    adapter: &str,
    external_submission_id: &str,
) -> Result<ScholarlyRemoteStatus, ScholarlyError> {
    if flags::scholarly_globally_disabled() {
        return Err(ScholarlyError::Disabled {
            reason: "VOX_SCHOLARLY_DISABLE is set".into(),
        });
    }
    let kind = adapter.trim();
    if kind.is_empty() || kind.eq_ignore_ascii_case("local_ledger") {
        return LocalLedgerAdapter
            .fetch_status(external_submission_id)
            .await;
    }
    if kind.eq_ignore_ascii_case("echo_ledger") {
        return EchoLedgerAdapter
            .fetch_status(external_submission_id)
            .await;
    }
    if kind.eq_ignore_ascii_case("zenodo") {
        if flags::scholarly_live_globally_disabled() {
            return Err(ScholarlyError::Disabled {
                reason: "VOX_SCHOLARLY_DISABLE_LIVE is set".into(),
            });
        }
        let z = zenodo::zenodo_from_clavis()?;
        return z.fetch_status(external_submission_id).await;
    }
    if kind.eq_ignore_ascii_case("openreview") {
        if flags::scholarly_live_globally_disabled() {
            return Err(ScholarlyError::Disabled {
                reason: "VOX_SCHOLARLY_DISABLE_LIVE is set".into(),
            });
        }
        let o = openreview::openreview_adapter_from_env().await?;
        return o.fetch_status(external_submission_id).await;
    }
    Err(ScholarlyError::Config {
        message: format!(
            "unsupported scholarly adapter for remote status: {kind:?} (supported: local_ledger, echo_ledger, zenodo, openreview)"
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn echo_ledger_is_deterministic_and_matches_content_digest() {
        let m = PublicationManifest {
            publication_id: "p1".into(),
            content_type: "paper".into(),
            source_ref: None,
            title: "t".into(),
            author: "a".into(),
            abstract_text: None,
            body_markdown: "b".into(),
            citations_json: None,
            metadata_json: None,
        };
        let digest = m.content_sha3_256();
        let r = EchoLedgerAdapter.submit(&m).await.expect("submit");
        assert_eq!(r.adapter, "echo_ledger");
        assert!(r.external_submission_id.starts_with("echo-"));
        assert_eq!(r.response_fingerprint.as_deref(), Some(digest.as_str()));
    }
}
