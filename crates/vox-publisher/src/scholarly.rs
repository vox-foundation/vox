use anyhow::Result;

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

/// Minimal adapter contract for journals/repositories.
pub trait ScholarlyAdapter {
    fn adapter_name(&self) -> &'static str;
    fn submit(&self, manifest: &PublicationManifest) -> Result<ScholarlySubmissionReceipt>;
}

/// First integration path: local scholarly ledger adapter.
///
/// This adapter does not perform network I/O; it emits a deterministic submission id so
/// callers can persist lifecycle transitions in Codex immediately.
#[derive(Debug, Default, Clone)]
pub struct LocalLedgerAdapter;

impl ScholarlyAdapter for LocalLedgerAdapter {
    fn adapter_name(&self) -> &'static str {
        "local_ledger"
    }

    fn submit(&self, manifest: &PublicationManifest) -> Result<ScholarlySubmissionReceipt> {
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
}
