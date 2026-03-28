//! Repository discovery for VoxDb lineage rows (orchestrator ↔ workspace SSOT).

use std::path::PathBuf;

/// When `VOX_ORCH_LINEAGE_OFF` is truthy, skip `orchestration_lineage_events` writes (rollback toggle).
pub(crate) fn orchestration_lineage_persist_enabled() -> bool {
    vox_config::orchestration_lineage_persist_enabled()
}

pub(crate) fn repository_id() -> String {
    let start = if let Ok(p) = std::env::var("VOX_REPOSITORY_ROOT") {
        let p = p.trim();
        if !p.is_empty() {
            PathBuf::from(p)
        } else {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        }
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    vox_repository::discover_repository_or_fallback(&start).repository_id
}
