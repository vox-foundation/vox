//! Open a project-scoped [`CodeStore`] with canonical `VOX_DB_*` precedence (see [`DbConfig::resolve_project_code_store_config`]).

use crate::DbConfig;
use vox_pm::store::{CodeStore, StoreError};

/// Open the Arca [`CodeStore`] for CLI helpers (`vox snippet`, `vox share`, etc.).
///
/// Honors `VOX_DB_URL`/`VOX_DB_TOKEN`, `VOX_DB_PATH`, compatibility `VOX_TURSO_*` / `TURSO_*`,
/// and defaults to [`vox_pm::store::DEFAULT_PROJECT_STORE_PATH`] when unset.
pub async fn open_project_code_store() -> Result<CodeStore, StoreError> {
    let cfg = DbConfig::resolve_project_code_store_config().map_err(StoreError::Db)?;
    match cfg {
        DbConfig::Remote { url, token } => CodeStore::open_remote(&url, &token).await,
        #[cfg(feature = "local")]
        DbConfig::Local { path } => {
            let p = std::path::Path::new(&path);
            if let Some(parent) = p.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).map_err(|e| StoreError::Db(e.to_string()))?;
                }
            }
            CodeStore::open(&path).await
        }
        #[cfg(feature = "local")]
        DbConfig::Memory => CodeStore::open_memory().await,
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
            CodeStore::open_embedded_replica(&local_path, &url, &token).await
        }
    }
}
