//! Ingest `mcp-invocables.json` (JSON array) into **Codex** content-addressed storage + `names`.
//!
//! Each entry is stored via [`crate::VoxDb::store`] under kind `mcp_invocable` and bound with
//! prefix `invocable.`.

use crate::VoxDb;
use crate::store::StoreError;
use std::path::Path;

/// Thin wrapper around [`crate::VoxDb`] for batch invocable import.
pub struct InvocableSyncEngine<'a> {
    db: &'a VoxDb,
}

impl<'a> InvocableSyncEngine<'a> {
    /// Borrow a [`VoxDb`] for the lifetime of the sync pass.
    pub fn new(db: &'a VoxDb) -> Self {
        Self { db }
    }

    /// Each array element is stored as a `mcp_invocable` object; `name` or `id` becomes `invocable.<slug>`.
    pub fn sync_from_file(&mut self, path: &Path) -> Result<usize, StoreError> {
        let data = std::fs::read_to_string(path)
            .map_err(|e| StoreError::Db(format!("read invocables file {}: {e}", path.display())))?;
        let v: serde_json::Value =
            serde_json::from_str(&data).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let arr = v
            .as_array()
            .ok_or_else(|| StoreError::Db("expected JSON array root for invocables".into()))?;
        let mut n = 0usize;
        for item in arr {
            let slug = item
                .get("name")
                .or_else(|| item.get("id"))
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let json =
                serde_json::to_vec(item).map_err(|e| StoreError::Serialization(e.to_string()))?;
            let db = self.db;
            let res: Result<(), StoreError> = db.block_on(async {
                let hash = db.store("mcp_invocable", &json).await?;
                db.bind_name("mcp_invocable", &slug, &hash).await?;
                Ok(())
            });
            res?;
            n += 1;
        }
        Ok(n)
    }
}
