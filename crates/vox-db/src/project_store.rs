use crate::VoxDb;
use crate::DbConfig;
use crate::store::{StoreError, DEFAULT_PROJECT_STORE_PATH};

/// Opens the project-local VoxDb (Arca store under `.vox/store.db`).
///
/// Discovers the repository root, ensuring `.vox/` exists, and opens the Turso
/// store.
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
