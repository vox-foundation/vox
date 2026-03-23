//! Global **Codex** connection helper (`vox_db::VoxDb`).
//!
//! Enable with the `database` feature on `vox-runtime`.
//!
//! # Environment (canonical)
//!
//! - [`vox_db::DbConfig::from_env`] — `VOX_DB_URL` + `VOX_DB_TOKEN`, or `VOX_DB_PATH`, or embedded replica when all three are set (with `replication`).
//!
//! # Legacy fallback
//!
//! - `TURSO_URL` + `TURSO_AUTH_TOKEN` — deprecated; use `VOX_DB_URL` + `VOX_DB_TOKEN`.

use std::sync::Arc;
use tokio::sync::OnceCell;
pub use vox_db::StoreError;
use vox_db::{DbConfig, SchemaDigest, VoxDb};

static DB: OnceCell<Arc<VoxDb>> = OnceCell::const_new();

fn resolve_db_config() -> Result<DbConfig, StoreError> {
    DbConfig::resolve_standalone().map_err(|msg| StoreError::Db(msg))
}

/// Initialize the global Codex instance using environment variables.
pub async fn get_db() -> Result<Arc<VoxDb>, StoreError> {
    DB.get_or_try_init(|| async {
        let config = resolve_db_config()?;
        let db = VoxDb::connect(config).await?;
        Ok(Arc::new(db))
    })
    .await
    .map(|db| db.clone())
}

/// Ensure the database schema matches the provided digest.
///
/// Call during application startup (e.g. generated `main()`).
pub async fn ensure_schema(digest: &SchemaDigest) -> Result<(), StoreError> {
    let db = get_db().await?;
    db.sync_schema_from_digest(digest).await?;
    Ok(())
}
