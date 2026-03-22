//! Connection helpers and built-in schema migration.
//!
//! # Turso `execute_batch` constraint
//!
//! [`turso::Connection::execute_batch`] executes each statement with `execute`, which errors if a
//! statement returns rows. Assignment `PRAGMA` values are applied with `pragma_update` in
//! [`CodeStore::apply_pragmas`]. Migration bodies must be DDL/DML only; empty strings in a fragment
//! are skipped.

use crate::schema::{BASELINE_VERSION, baseline_sql};
use crate::store::CodeStore;
use crate::store::types::StoreError;

impl CodeStore {
    pub(crate) async fn apply_pragmas(conn: &turso::Connection) -> Result<(), StoreError> {
        // `execute_batch` runs each statement via `execute()`, which errors on any row-producing
        // statement. Assignment-style `PRAGMA x = y` returns the new value as a row; use
        // `pragma_update` so those rows are consumed (see turso `Connection::prepare_execute_batch`).
        let _ = conn.pragma_update("journal_mode", "WAL").await?;
        let _ = conn.pragma_update("busy_timeout", 5000).await?;
        let _ = conn.pragma_update("synchronous", "NORMAL").await?;
        let _ = conn.pragma_update("foreign_keys", "ON").await?;
        let _ = conn.pragma_update("cache_size", -8000).await?;
        Ok(())
    }

    /// Apply the manifest **baseline** DDL once and ensure `schema_version` records baseline **1**.
    ///
    /// Legacy databases with `MAX(schema_version.version) > 1` return [`StoreError::LegacySchemaChain`].
    pub(crate) async fn migrate(conn: &turso::Connection) -> Result<(), StoreError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .await?;

        let mut rows = conn
            .query("SELECT COALESCE(MAX(version), 0) FROM schema_version", ())
            .await?;
        let current_version: i64 = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            0
        };

        if current_version > BASELINE_VERSION {
            return Err(StoreError::LegacySchemaChain {
                max_version: current_version,
            });
        }

        if current_version == 0 {
            let sql = baseline_sql().trim();
            if !sql.is_empty() {
                conn.execute_batch(sql).await?;
            }
            conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                (BASELINE_VERSION,),
            )
            .await?;
        }

        Ok(())
    }

    /// Open local file **without** running `migrate` (export tools only — reads legacy multi-version DBs).
    #[cfg(feature = "local")]
    pub async fn open_local_legacy_export(path: &str) -> Result<Self, StoreError> {
        let db = turso::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        Self::apply_pragmas(&conn).await?;
        Ok(Self {
            conn,
            sync_db: None,
        })
    }

    /// Open or create a local database file (requires `local` feature on `vox-db`).
    #[cfg(feature = "local")]
    pub async fn open(path: &str) -> Result<Self, StoreError> {
        let db = turso::Builder::new_local(path).build().await?;
        let conn = db.connect()?;
        Self::apply_pragmas(&conn).await?;
        Self::migrate(&conn).await?;
        Ok(Self {
            conn,
            sync_db: None,
        })
    }

    /// In-memory store (requires `local` feature on `vox-db`).
    #[cfg(feature = "local")]
    pub async fn open_memory() -> Result<Self, StoreError> {
        Self::open(":memory:").await
    }

    /// Remote Turso / libSQL (sync client).
    pub async fn open_remote(url: &str, auth_token: &str) -> Result<Self, StoreError> {
        let db = turso::sync::Builder::new_remote(":memory:")
            .with_remote_url(url)
            .with_auth_token(auth_token)
            .build()
            .await?;
        let conn = db.connect().await?;
        Self::apply_pragmas(&conn).await?;
        Self::migrate(&conn).await?;
        Ok(Self {
            conn,
            sync_db: Some(std::sync::Arc::new(db)),
        })
    }

    /// Remote sync client **without** running `migrate` (export tools only).
    pub async fn open_remote_legacy_export(
        url: &str,
        auth_token: &str,
    ) -> Result<Self, StoreError> {
        let db = turso::sync::Builder::new_remote(":memory:")
            .with_remote_url(url)
            .with_auth_token(auth_token)
            .build()
            .await?;
        let conn = db.connect().await?;
        Self::apply_pragmas(&conn).await?;
        Ok(Self {
            conn,
            sync_db: Some(std::sync::Arc::new(db)),
        })
    }

    /// Embedded replica: local path with remote sync.
    #[cfg(feature = "replication")]
    pub async fn open_embedded_replica(
        local_path: &str,
        url: &str,
        auth_token: &str,
    ) -> Result<Self, StoreError> {
        let db = turso::sync::Builder::new_remote(local_path)
            .with_remote_url(url)
            .with_auth_token(auth_token)
            .build()
            .await?;
        let conn = db.connect().await?;
        Self::apply_pragmas(&conn).await?;
        Self::migrate(&conn).await?;
        Ok(Self {
            conn,
            sync_db: Some(std::sync::Arc::new(db)),
        })
    }

    /// Default local project store at [`super::DEFAULT_PROJECT_STORE_PATH`].
    #[cfg(feature = "local")]
    pub async fn open_default() -> Result<Self, StoreError> {
        let path = super::DEFAULT_PROJECT_STORE_PATH;
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent).map_err(|e| StoreError::Db(e.to_string()))?;
        }
        Self::open(path).await
    }

    /// Pull remote changes when using a sync-backed store; no-op for pure local files.
    pub async fn sync(&self) -> Result<(), StoreError> {
        if let Some(db) = &self.sync_db {
            let _ = db.pull().await?;
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "local"))]
mod tests {
    use super::*;
    use crate::store::types::StoreError;
    use turso::Builder;

    #[tokio::test]
    async fn fresh_db_schema_version_is_baseline_one() {
        let db = Builder::new_local(":memory:").build().await.expect("mem");
        let conn = db.connect().expect("conn");
        CodeStore::apply_pragmas(&conn).await.expect("pragmas");
        CodeStore::migrate(&conn).await.expect("migrate");
        let mut rows = conn
            .query("SELECT MAX(version) FROM schema_version", ())
            .await
            .expect("q");
        let v: i64 = rows
            .next()
            .await
            .expect("next")
            .expect("row")
            .get(0)
            .expect("v");
        assert_eq!(v, 1);
    }

    #[tokio::test]
    async fn legacy_multi_version_chain_rejected() {
        let db = Builder::new_local(":memory:").build().await.expect("mem");
        let conn = db.connect().expect("conn");
        CodeStore::apply_pragmas(&conn).await.expect("pragmas");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .await
        .expect("sv");
        conn.execute("INSERT INTO schema_version (version) VALUES (17)", ())
            .await
            .expect("ins");
        let err = CodeStore::migrate(&conn).await.expect_err("legacy");
        assert!(matches!(
            err,
            StoreError::LegacySchemaChain { max_version: 17 }
        ));
    }
}
