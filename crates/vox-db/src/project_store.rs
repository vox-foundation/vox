use crate::VoxDb;
// Open a project-scoped [`VoxDb`] with canonical `VOX_DB_*` precedence (see [`DbConfig::resolve_project_code_store_config`]).

use crate::DbConfig;
use crate::arca_store::{StoreError};

/// Open the Arca [`VoxDb`] for CLI helpers (`vox snippet`, `vox share`, etc.).
///
/// Honors `VOX_DB_URL`/`VOX_DB_TOKEN`, `VOX_DB_PATH`, compatibility `VOX_TURSO_*` / `TURSO_*`,
/// and defaults to [`crate::arca_store::DEFAULT_PROJECT_STORE_PATH`] when unset.
pub async fn open_project_code_store() -> Result<VoxDb, StoreError> {
    let cfg = DbConfig::resolve_project_code_store_config().map_err(StoreError::Db)?;
    match cfg {
        DbConfig::Remote { url, token } => VoxDb::open_remote(&url, &token).await,
        #[cfg(feature = "local")]
        DbConfig::Local { path } => {
            let p = std::path::Path::new(&path);
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| StoreError::Db(e.to_string()))?;
                }
            }
            VoxDb::open(&path).await
        }
        #[cfg(feature = "local")]
        DbConfig::Memory => VoxDb::open_memory().await,
        #[cfg(feature = "replication")]
        DbConfig::EmbeddedReplica {
            local_path,
            url,
            token,
        } => {
            let p = std::path::Path::new(&local_path);
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| StoreError::Db(e.to_string()))?;
                }
            }
            VoxDb::open_embedded_replica(&local_path, &url, &token).await
        }
    }
}
