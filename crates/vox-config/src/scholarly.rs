//! Configuration for scholarly publisher platforms (Zenodo, OpenReview, etc.).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScholarlyAdapterKind {
    /// Default: no external network I/O; local ledger only.
    #[default]
    LocalLedger,
    /// Deterministic echo (CI/testing).
    EchoLedger,
    /// CERN Zenodo (Sandbox or Prod).
    Zenodo,
    /// OpenReview.net.
    OpenReview,
    /// arXiv assist (staging).
    ArxivAssist,
    /// OSF (Open Science Framework) REST v2.
    Osf,
    /// Crossref DOI deposit (metadata XML via doi.crossref.org).
    CrossrefDeposit,
}

#[must_use]
pub fn scholarly_adapter_from_env() -> ScholarlyAdapterKind {
    let raw = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxScholarlyAdapter)
        .expose()
        .map(|s| s.trim().to_ascii_lowercase());
    match raw.as_deref() {
        Some("zenodo") => ScholarlyAdapterKind::Zenodo,
        Some("openreview") => ScholarlyAdapterKind::OpenReview,
        Some("echo_ledger") | Some("echo") => ScholarlyAdapterKind::EchoLedger,
        Some("arxiv_assist") | Some("arxiv") => ScholarlyAdapterKind::ArxivAssist,
        Some("osf") => ScholarlyAdapterKind::Osf,
        Some("crossref_deposit") | Some("crossref") => ScholarlyAdapterKind::CrossrefDeposit,
        Some("local_ledger") | Some("local") | None => ScholarlyAdapterKind::LocalLedger,
        _ => ScholarlyAdapterKind::LocalLedger,
    }
}
