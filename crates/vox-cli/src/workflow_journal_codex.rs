//! Codex persistence for interpreted workflow journals.
//!
//! **Default:** persistence runs when DB config resolves. Set **`VOX_WORKFLOW_JOURNAL_CODEX_OFF=1`**
//! to skip writes (escape hatch).

use std::path::PathBuf;

use serde_json::Value;
use vox_config::workflow_journal_codex_persist_enabled;
use vox_db::{DbConfig, VoxDb};

fn journal_codex_disabled() -> bool {
    !workflow_journal_codex_persist_enabled()
}

fn discovery_start() -> PathBuf {
    if let Ok(p) = std::env::var("VOX_REPOSITORY_ROOT") {
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
        tracing::debug!(
            target: "vox.workflow_journal",
            "skip Codex persist: db config unresolved"
        );
        return;
    };
    let start = discovery_start();
    let rid = vox_repository::discover_repository_or_fallback(&start).repository_id;
    let Ok(db) = VoxDb::connect(cfg).await else {
        tracing::debug!(target: "vox.workflow_journal", "skip: VoxDb::connect failed");
        return;
    };
    for entry in journal {
        if let Err(e) = db
            .record_workflow_journal_entry(&rid, workflow_name, entry)
            .await
        {
            tracing::debug!(
                target: "vox.workflow_journal",
                error = %e,
                "record_workflow_journal_entry failed (best-effort)"
            );
        }
    }
}

/// When workflow journal persistence is not disabled and DB config resolves, append one row to Codex.
pub async fn persist_workflow_journal_entry_opt(workflow_name: &str, entry: &Value) {
    persist_workflow_journal_rows(workflow_name, std::slice::from_ref(entry)).await;
}

/// Batch append when persistence is enabled and DB config resolves.
#[allow(dead_code)] // Public batch API; CLI uses [`persist_workflow_journal_entry_opt`] for row-by-row control.
pub async fn persist_workflow_journal_opt(workflow_name: &str, journal: &[Value]) {
    persist_workflow_journal_rows(workflow_name, journal).await;
}
