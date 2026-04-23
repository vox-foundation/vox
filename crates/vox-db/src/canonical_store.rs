//! Canonical Codex / VoxDB storage policy (single source of truth).
//!
//! # Store tiers
//!
//! | Tier | How | Purpose |
//! |------|-----|---------|
//! | **Canonical user-global** | [`resolve_canonical_config`] / [`crate::DbConfig::resolve_canonical`] | Authoritative relational data: Codex, publication, research, mesh defaults, training telemetry when the main file is on the current baseline. |
//! | **Workspace journey store** | [`crate::connect_workspace_journey_optional`] → default `.vox/store.db` | Repo-backed **interactive** MCP/daemon journeys use this as the primary Codex handle unless `VOX_WORKSPACE_JOURNEY_STORE=canonical`. Operator/global workflows without a single repo still use **Canonical user-global**. |
//! | **Project-local artifacts** | [`crate::open_project_db`] / [`crate::open_project_db_at_root`] | Same `.vox/store.db` file as the workspace journey default; also used for explicit repo-scoped tooling. |
//! | **Historical `vox_training_telemetry.db`** | [`crate::paths::training_telemetry_db_path`] | May remain from older releases; training telemetry uses the canonical file via [`crate::VoxDb::connect_default`]. A legacy primary still yields [`crate::StoreError::LegacySchemaChain`] until migration (`vox codex export-legacy` → fresh baseline → `vox codex import-legacy`). |
//!
//! # Environment
//!
//! Resolution order matches [`crate::DbConfig::resolve_standalone`]: `VOX_DB_URL` + `VOX_DB_TOKEN` (remote),
//! embedded replica triple when enabled, `VOX_DB_PATH`, then
//! [`vox_config::paths::default_db_path`] (`<data_dir>/vox.db`), then Turso compatibility env aliases.
//!
//! Operator guide: `docs/src/how-to/how-to-voxdb-canonical-store.md`.

use std::path::PathBuf;

use crate::DbConfig;

/// Resolves the canonical user-global [`DbConfig`] for Codex / VoxDB.
///
/// Alias for [`DbConfig::resolve_canonical`]; use either name for clarity at callsites.
pub fn resolve_canonical_config() -> Result<DbConfig, String> {
    DbConfig::resolve_canonical()
}

/// Default local path to `vox.db` when the platform data dir is known (`VOX_DATA_DIR` / defaults).
#[must_use]
pub fn user_global_sqlite_path() -> Option<PathBuf> {
    vox_config::paths::default_db_path()
}
