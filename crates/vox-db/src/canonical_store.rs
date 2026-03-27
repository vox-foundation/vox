//! Canonical Codex / VoxDB storage policy (single source of truth).
//!
//! # Store tiers
//!
//! | Tier | How | Purpose |
//! |------|-----|---------|
//! | **Canonical user-global** | [`resolve_canonical_config`] / [`crate::DbConfig::resolve_canonical`] | Authoritative relational data: Codex, publication, research, mesh defaults, training telemetry when the main file is on the current baseline. |
//! | **Project-local artifacts** | [`crate::open_project_db`] → `.vox/store.db` | Repo-scoped cache only (snippets, share, agent tooling, LSP); optional; not cross-repo authoritative. |
//! | **Training telemetry fallback** | [`crate::paths::training_telemetry_db_path`] (`vox_training_telemetry.db` next to `vox.db`) | Transitional SQLite when canonical `vox.db` is still on a legacy [`crate::StoreError::LegacySchemaChain`]. Automatically reset to baseline if stale. Converge by migrating the main DB (`vox codex export-legacy` → fresh baseline → `vox codex import-legacy`). |
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
#[must_use]
pub fn resolve_canonical_config() -> Result<DbConfig, String> {
    DbConfig::resolve_canonical()
}

/// Default local path to `vox.db` when the platform data dir is known (`VOX_DATA_DIR` / defaults).
#[must_use]
pub fn user_global_sqlite_path() -> Option<PathBuf> {
    vox_config::paths::default_db_path()
}
