//! Opt-in Codex audit rows for mesh local registry publish (`VOX_MESH_CODEX_TELEMETRY`).

use std::path::Path;
use std::path::PathBuf;

/// After a successful local mesh registry write, append one `mesh_control_event` row (no secrets).
pub async fn record_local_registry_publish_opt(registry_path: &Path, node_id: Option<&str>) {
    let start = discovery_start();
    let rid = vox_repository::discover_repository_or_fallback(&start).repository_id;
    vox_db::mesh_registry_telemetry::record_local_registry_publish_opt(
        &rid,
        registry_path,
        node_id,
    )
    .await;
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
