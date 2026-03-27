//! Opt-in Codex rows after mens local registry publish (`VOX_MESH_CODEX_TELEMETRY`).
//!
//! Shared by `vox-cli` (`vox run`) and `vox-mcp` so operators get one code path.

use std::path::Path;

use serde_json::json;

use crate::{DbConfig, VoxDb};

/// True when **`VOX_MESH_CODEX_TELEMETRY`** is `1` or `true`.
#[must_use]
pub fn mesh_codex_telemetry_enabled() -> bool {
    std::env::var("VOX_MESH_CODEX_TELEMETRY")
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

/// After a successful local mens registry write, append one `populi_control_event` row (no secrets).
///
/// `repository_id` should match MCP / CLI discovery (e.g. [`vox_repository::RepositoryContext::repository_id`]).
pub async fn record_local_registry_publish_opt(
    repository_id: &str,
    registry_path: &Path,
    node_id: Option<&str>,
) {
    if !mesh_codex_telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_canonical() else {
        tracing::debug!(
            target: "vox.populi_registry_telemetry",
            "skip mens telemetry: db config unresolved"
        );
        return;
    };
    let Ok(db) = VoxDb::connect(cfg).await else {
        tracing::debug!(
            target: "vox.populi_registry_telemetry",
            "skip: VoxDb::connect failed"
        );
        return;
    };
    let scope_id = std::env::var("VOX_MESH_SCOPE_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    let details = json!({
        "registry_path": registry_path.display().to_string(),
        "node_id": node_id,
        "scope_id": scope_id,
    });
    if let Err(e) = db
        .record_populi_control_event(repository_id, "local_registry_publish", Some(details))
        .await
    {
        tracing::debug!(
            target: "vox.populi_registry_telemetry",
            error = %e,
            "record_populi_control_event failed"
        );
    }
}

/// After HTTP mens control-plane join (or failure), append one `populi_control_event` when telemetry is on.
///
/// Event names: `mesh_http_join_ok` / `mesh_http_join_err` (requires [`mesh_codex_telemetry_enabled`]).
pub async fn record_populi_http_join_opt(
    repository_id: &str,
    ok: bool,
    control_base: &str,
    node_id: Option<&str>,
    error: Option<&str>,
) {
    if !mesh_codex_telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_canonical() else {
        tracing::debug!(
            target: "vox.populi_registry_telemetry",
            "skip mens http join telemetry: db config unresolved"
        );
        return;
    };
    let Ok(db) = VoxDb::connect(cfg).await else {
        tracing::debug!(
            target: "vox.populi_registry_telemetry",
            "skip: VoxDb::connect failed"
        );
        return;
    };
    let scope_id = std::env::var("VOX_MESH_SCOPE_ID")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_string());
    let details = json!({
        "control_base": control_base,
        "ok": ok,
        "node_id": node_id,
        "scope_id": scope_id,
        "error": error,
    });
    let name = if ok {
        "mesh_http_join_ok"
    } else {
        "mesh_http_join_err"
    };
    if let Err(e) = db
        .record_populi_control_event(repository_id, name, Some(details))
        .await
    {
        tracing::debug!(
            target: "vox.populi_registry_telemetry",
            error = %e,
            "record_populi_control_event (http join) failed"
        );
    }
}
