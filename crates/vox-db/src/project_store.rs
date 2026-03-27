use crate::DbConfig;
use crate::VoxDb;
use crate::store::{DEFAULT_PROJECT_STORE_PATH, StoreError};

/// Opens the **project-local** VoxDb (`.vox/store.db`) for repo-scoped artifacts only.
///
/// This is **not** the canonical user-global Codex store ([`crate::canonical_store`]); use
/// [`crate::DbConfig::resolve_canonical`] + [`crate::VoxDb::connect`] for authoritative relational data.
/// Discovers the repository root, ensures `.vox/` exists, and opens Turso.
pub async fn open_project_db() -> Result<VoxDb, StoreError> {
    let cwd = std::env::current_dir().map_err(StoreError::Io)?;
    let repo = vox_repository::discover_repository_or_fallback(&cwd);

    let db_path = repo.root.join(DEFAULT_PROJECT_STORE_PATH);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent).map_err(StoreError::Io)?;
    }

    let config = DbConfig::Local {
        path: db_path.to_string_lossy().to_string(),
    };
    VoxDb::connect(config).await
}

#[deprecated(note = "renamed to open_project_db")]
pub async fn open_project_code_store() -> Result<VoxDb, StoreError> {
    open_project_db().await
}
