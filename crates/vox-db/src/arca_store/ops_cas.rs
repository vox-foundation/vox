//! Content-addressed storage (`objects`, `names`) and schema introspection for [`VoxDb`].
//!
//! The `objects` table (V1 schema) stores arbitrary blobs keyed by SHA3-512 Base32Hex hash.
//! The `names` table (V1 schema) maps `(namespace, name)` pairs to object hashes.
//! `schema_version` is created by [`super::open`] migrations and queried here.

use turso::params;

use crate::hash::content_hash;

use crate::arca_store::types::StoreError;

impl crate::VoxDb {
    /// Write `data` as a `kind`-tagged blob into `objects` using its SHA3-512 Base32Hex hash as
    /// the primary key. Duplicate writes (`INSERT OR IGNORE`) are a no-op. Returns the hash.
    pub async fn store(&self, kind: &str, data: &[u8]) -> Result<String, StoreError> {
        let hash = content_hash(data);
        self.conn
            .execute(
                "INSERT OR IGNORE INTO objects (hash, kind, data) VALUES (?1, ?2, ?3)",
                params![hash.as_str(), kind, data],
            )
            .await?;
        Ok(hash)
    }

    /// Read the `data` blob for `hash` from `objects`. Returns `NotFound` if absent.
    pub async fn get(&self, hash: &str) -> Result<Vec<u8>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT data FROM objects WHERE hash = ?1 LIMIT 1",
                params![hash],
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::NotFound(format!("object {hash}")))?;
        let data: Vec<u8> = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(data)
    }

    /// Bind (or rebind) a logical `name` in `namespace` to a content hash in the `names` table.
    ///
    /// The `hash` must already exist in `objects`; the schema enforces the FK constraint.
    pub async fn bind_name(&self, namespace: &str, name: &str, hash: &str) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO names (namespace, name, hash, updated_at)
                 VALUES (?1, ?2, ?3, datetime('now'))
                 ON CONFLICT(namespace, name)
                 DO UPDATE SET hash = excluded.hash, updated_at = datetime('now')",
                params![namespace, name, hash],
            )
            .await?;
        Ok(())
    }

    /// Return `MAX(version)` from `schema_version`, or `0` if the table is empty.
    pub async fn schema_version(&self) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                (),
            )
            .await?;
        let row = rows
            .next()
            .await?
            .ok_or_else(|| StoreError::Db("schema_version query returned no rows".into()))?;
        let v: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
        Ok(v)
    }

    /// Serialize the live SQLite schema into a `db_snapshots` row, keyed by `snap_id`
    /// (the `db_snapshots.id` primary key). Returns `Ok(())` on success.
    ///
    /// Called from `vox-orchestrator` `Orchestrator::take_db_snapshot`.
    pub async fn take_db_snapshot(
        &self,
        snap_id: u64,
        agent_id: &str,
        description: &str,
    ) -> Result<(), StoreError> {
        // Capture a lightweight JSON-encoded snapshot of all table names (schema audit only).
        let mut rows = self
            .conn
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
                (),
            )
            .await?;
        let mut names: Vec<String> = Vec::new();
        while let Some(row) = rows.next().await? {
            let n: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            names.push(n);
        }
        let payload = serde_json::to_string(&names)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO db_snapshots (id, agent_id, description, payload)
                 VALUES (?1, ?2, ?3, ?4)",
                params![snap_id as i64, agent_id, description, payload],
            )
            .await?;
        Ok(())
    }

    /// Restore (replay) a db snapshot identified by `snap_id`.
    ///
    /// Validates the snapshot row exists; a full byte-for-byte restore would require an
    /// out-of-band database swap beyond libSQL's in-connection capabilities. Returns
    /// `NotFound` if the snapshot is absent.
    ///
    /// Called from `vox-orchestrator` `Orchestrator::undo_operation` / `redo_operation`.
    pub async fn restore_db_snapshot(&self, snap_id: u64) -> Result<(), StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id FROM db_snapshots WHERE id = ?1 LIMIT 1",
                params![snap_id as i64],
            )
            .await?;
        rows.next()
            .await?
            .ok_or_else(|| StoreError::NotFound(format!("db_snapshot {snap_id}")))?;
        Ok(())
    }
}
