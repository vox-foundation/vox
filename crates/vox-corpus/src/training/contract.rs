//! Workspace discovery for training SSOT paths (`training_contract.yaml`, system prompt, presets).

use std::path::{Path, PathBuf};

/// Walk up from [`std::env::current_dir`] until a `Cargo.toml` containing a `[workspace]` section is found.
///
/// Returns [`None`] if the current directory cannot be read or no workspace manifest is found.
pub fn find_workspace_root() -> Option<PathBuf> {
    vox_repository::find_cargo_workspace_root()
}

/// Resolve `path` relative to the workspace root when possible; otherwise return `path` as-is.
pub fn resolve_from_workspace(path: &Path) -> PathBuf {
    vox_repository::resolve_from_cargo_workspace(path)
}
