//! Repo-backed CLI Codex connect path (Unified Vox Request Journey).
//!
//! Uses [`vox_db::connect_workspace_journey_optional`] so `VOX_WORKSPACE_JOURNEY_STORE` /
//! `VOX_WORKSPACE_JOURNEY_FALLBACK_CANONICAL` match MCP and `vox-orchestrator-d`.

use anyhow::Context;
use vox_db::VoxDb;

/// Open the authoritative VoxDb for workspace-scoped CLI commands (`vox agent`, `vox snippet`, …).
pub async fn connect_cli_workspace_voxdb() -> anyhow::Result<VoxDb> {
    connect_cli_workspace_voxdb_with_overrides(false).await
}

/// Like [`connect_cli_workspace_voxdb`]; `skip_log` mirrors MCP embed startup noise control.
pub async fn connect_cli_workspace_voxdb_with_overrides(skip_log: bool) -> anyhow::Result<VoxDb> {
    vox_db::connect_workspace_journey_optional(
        vox_db::DbConnectSurface::CliWorkspace,
        skip_log,
    )
    .await
    .context(
        "Failed to open VoxDb for this repository (workspace journey). \
         See VOX_WORKSPACE_JOURNEY_STORE / VOX_WORKSPACE_JOURNEY_FALLBACK_CANONICAL, \
         VOX_DB_PATH, VOX_DB_URL, and project `.vox/store.db`.",
    )
}
