use serde_json::Value;

pub fn resolve_scholarly_adapter_kind(adapter_override: Option<&str>) -> String {
    let k = if let Some(s) = adapter_override {
        match s.trim().to_ascii_lowercase().as_str() {
            "zenodo" => vox_config::scholarly::ScholarlyAdapterKind::Zenodo,
            "openreview" => vox_config::scholarly::ScholarlyAdapterKind::OpenReview,
            "echo_ledger" | "echo" => vox_config::scholarly::ScholarlyAdapterKind::EchoLedger,
            "arxiv_assist" | "arxiv" => vox_config::scholarly::ScholarlyAdapterKind::ArxivAssist,
            _ => vox_config::scholarly::ScholarlyAdapterKind::LocalLedger,
        }
    } else {
        vox_config::scholarly::scholarly_adapter_from_env()
    };
    match k {
        vox_config::scholarly::ScholarlyAdapterKind::Zenodo => "zenodo".to_string(),
        vox_config::scholarly::ScholarlyAdapterKind::OpenReview => "openreview".to_string(),
        vox_config::scholarly::ScholarlyAdapterKind::EchoLedger => "echo_ledger".to_string(),
        vox_config::scholarly::ScholarlyAdapterKind::ArxivAssist => "arxiv_assist".to_string(),
        vox_config::scholarly::ScholarlyAdapterKind::LocalLedger => "local_ledger".to_string(),
    }
}

pub struct ExternalJobsTickOutput {
    pub lock_owner: String,
    pub lock_ttl_ms: i64,
    pub results: Vec<Value>,
}

pub fn default_scholarly_job_lock_owner() -> String {
    if let Some(s) =
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxScholarlyJobLockOwner).expose()
    {
        let t = s.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    format!("vox:{}", std::process::id())
}

pub mod poll;
pub mod submit;
pub mod sync;
pub mod tick;

pub use poll::*;
pub use submit::*;
pub use sync::*;
pub use tick::*;
