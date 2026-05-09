//! Codex persistence for interpreted workflow journals.

use serde_json::Value;
use std::path::PathBuf;
use vox_db::{DbConfig, VoxDb};

fn journal_codex_disabled() -> bool {
    std::env::var("VOX_WORKFLOW_JOURNAL_CODEX_OFF")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn discovery_start() -> PathBuf {
    if let Some(p) = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxRepositoryRoot).expose()
    {
        let p = p.trim();
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

async fn persist_workflow_journal_rows(workflow_name: &str, journal: &[Value]) {
    if journal_codex_disabled() || journal.is_empty() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_canonical() else {
        return;
    };
    let start = discovery_start();
    let rid = vox_repository::discover_repository_or_fallback(&start).repository_id;
    let Ok(db) = VoxDb::connect(cfg).await else {
        return;
    };
    for entry in journal {
        let _ = db
            .record_workflow_journal_entry(&rid, workflow_name, entry)
            .await;
    }
}

pub async fn persist_workflow_journal_entry_opt(workflow_name: &str, entry: &Value) {
    persist_workflow_journal_rows(workflow_name, std::slice::from_ref(entry)).await;
}

pub async fn persist_workflow_journal_opt(workflow_name: &str, journal: &[Value]) {
    persist_workflow_journal_rows(workflow_name, journal).await;
}
