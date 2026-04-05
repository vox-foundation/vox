//! Repository discovery for VoxDb lineage rows (orchestrator ↔ workspace SSOT).

use std::path::PathBuf;

/// When `VOX_ORCH_LINEAGE_OFF` is truthy, skip `orchestration_lineage_events` writes (rollback toggle).
pub(crate) fn orchestration_lineage_persist_enabled() -> bool {
    vox_config::orchestration_lineage_persist_enabled()
}

/// Optional cross-plan grouping id from `VOX_ORCH_CAMPAIGN_ID` (trimmed; omitted when unset).
pub(crate) fn orchestration_campaign_id() -> Option<String> {
    std::env::var("VOX_ORCH_CAMPAIGN_ID").ok().and_then(|s| {
        let t = s.trim().to_string();
        (!t.is_empty()).then_some(t)
    })
}

pub fn repository_id() -> String {
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
