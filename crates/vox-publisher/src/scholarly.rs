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

/// Second integration path: deterministic id without claiming external network I/O (tests, dry CI).
#[derive(Debug, Default, Clone)]
pub struct EchoLedgerAdapter;

impl ScholarlyAdapter for EchoLedgerAdapter {
    fn adapter_name(&self) -> &'static str {
        "echo_ledger"
    }

    fn submit(&self, manifest: &PublicationManifest) -> Result<ScholarlySubmissionReceipt> {
        let digest = manifest.content_sha3_256();
        let external_submission_id =
            format!("echo-{}", &digest[..16.min(digest.len())]);
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
}

/// Resolve [`VOX_SCHOLARLY_ADAPTER`] (default `local_ledger`) and submit.
///
/// Supported values: `local_ledger` (default), `echo_ledger`. Any other value is rejected so stubs cannot silently no-op.
pub fn submit_with_configured_adapter(manifest: &PublicationManifest) -> Result<ScholarlySubmissionReceipt> {
    let raw = std::env::var("VOX_SCHOLARLY_ADAPTER").unwrap_or_default();
    let kind = raw.trim();
    if kind.is_empty() || kind.eq_ignore_ascii_case("local_ledger") {
        return LocalLedgerAdapter.submit(manifest);
    }
    if kind.eq_ignore_ascii_case("echo_ledger") {
        return EchoLedgerAdapter.submit(manifest);
    }
    anyhow::bail!(
        "unsupported VOX_SCHOLARLY_ADAPTER={kind:?} (supported: local_ledger, echo_ledger)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn echo_ledger_is_deterministic_and_matches_content_digest() {
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
        let r = EchoLedgerAdapter.submit(&m).expect("submit");
        assert_eq!(r.adapter, "echo_ledger");
        assert!(r.external_submission_id.starts_with("echo-"));
        assert_eq!(r.response_fingerprint.as_deref(), Some(digest.as_str()));
    }
}
