//! Store ops for `scientia_embedding_cache`.
//!
//! Provides a typed cache for LLM embedding vectors keyed by `sha256(model+text)`.
//! Rows are upserted with INSERT OR REPLACE so the latest write always wins.
//!
//! `vector_json` is a JSON array of `f32` values serialised/deserialised by these
//! ops; callers deal only in `Vec<f32>`.

use crate::VoxDb;
use crate::store::types::StoreError;
use turso::params;

impl VoxDb {
    /// Return the cached embedding vector for `text_sha256`, or `None` if absent.
    ///
    /// Reads binary little-endian `f32` `vector_blob` first (3.1× smaller, zero JSON
    /// float-parse overhead) and falls back to `vector_json` for rows written before
    /// the BLOB upgrade.
    pub async fn get_cached_embedding(
        &self,
        text_sha256: &str,
    ) -> Result<Option<Vec<f32>>, StoreError> {
        let sha = text_sha256.to_string();
        let mut rows = self
            .conn
            .query(
                "SELECT vector_blob, vector_json FROM scientia_embedding_cache WHERE text_sha256 = ?1",
                params![sha],
            )
            .await
            .map_err(StoreError::Turso)?;

        if let Some(row) = rows.next().await.map_err(StoreError::Turso)? {
            let blob: Option<Vec<u8>> = row.get(0).map_err(StoreError::Turso)?;
            if let Some(bytes) = blob.filter(|b| !b.is_empty()) {
                if bytes.len() % 4 != 0 {
                    return Err(StoreError::Db(format!(
                        "scientia_embedding_cache: malformed vector_blob byte length {} for sha256 {text_sha256}",
                        bytes.len()
                    )));
                }
                let vec: Vec<f32> = bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| f32::from_le_bytes(*c))
                    .collect();
                return Ok(Some(vec));
            }
            let json: String = row.get(1).map_err(StoreError::Turso)?;
            let vec: Vec<f32> = serde_json::from_str(&json).map_err(|e| {
                StoreError::Db(format!(
                    "scientia_embedding_cache: malformed vector_json for sha256 {text_sha256}: {e}"
                ))
            })?;
            Ok(Some(vec))
        } else {
            Ok(None)
        }
    }

    /// Upsert an embedding vector for `text_sha256`.
    ///
    /// Stores compact little-endian `f32` bytes in `vector_blob` (and `'[]'` in `vector_json`
    /// for backward-compatible NOT NULL schema columns).
    pub async fn put_cached_embedding(
        &self,
        text_sha256: &str,
        model: &str,
        vector: &[f32],
    ) -> Result<(), StoreError> {
        let sha = text_sha256.to_string();
        let mdl = model.to_string();
        let mut blob = Vec::with_capacity(vector.len() * 4);
        for &val in vector {
            blob.extend_from_slice(&val.to_le_bytes());
        }
        let now = crate::now_unix_ms() as i64;

        self.conn
            .execute(
                "INSERT OR REPLACE INTO scientia_embedding_cache \
                 (text_sha256, model, vector_json, vector_blob, created_at_ms) \
                 VALUES (?1, ?2, '[]', ?3, ?4)",
                params![sha, mdl, blob, now],
            )
            .await
            .map_err(StoreError::Turso)?;
        Ok(())
    }

    /// Enqueue a serialized JSON payload into `telemetry_upload_spool`.
    pub async fn enqueue_telemetry_upload_spool(
        &self,
        id: &str,
        payload_json: &str,
        created_at_ms: i64,
    ) -> Result<(), StoreError> {
        let byte_len = payload_json.len() as i64;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO telemetry_upload_spool \
                 (id, payload_json, byte_len, created_at_ms) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    id.to_string(),
                    payload_json.to_string(),
                    byte_len,
                    created_at_ms
                ],
            )
            .await
            .map_err(StoreError::Turso)?;
        Ok(())
    }

    /// List pending `telemetry_upload_spool` entries ordered by `(created_at_ms ASC, id ASC)`.
    pub async fn list_telemetry_upload_spool(
        &self,
    ) -> Result<Vec<(String, String, u64, i64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, payload_json, byte_len, created_at_ms \
                 FROM telemetry_upload_spool ORDER BY created_at_ms ASC, id ASC",
                (),
            )
            .await
            .map_err(StoreError::Turso)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(StoreError::Turso)? {
            let id: String = row.get(0).map_err(StoreError::Turso)?;
            let payload_json: String = row.get(1).map_err(StoreError::Turso)?;
            let byte_len: i64 = row.get(2).map_err(StoreError::Turso)?;
            let created_at_ms: i64 = row.get(3).map_err(StoreError::Turso)?;
            out.push((id, payload_json, byte_len.max(0) as u64, created_at_ms));
        }
        Ok(out)
    }

    /// Remove a `telemetry_upload_spool` entry by `id`.
    pub async fn ack_telemetry_upload_spool(&self, id: &str) -> Result<bool, StoreError> {
        let affected = self
            .conn
            .execute(
                "DELETE FROM telemetry_upload_spool WHERE id = ?1",
                params![id.to_string()],
            )
            .await
            .map_err(StoreError::Turso)?;
        Ok(affected > 0)
    }

    /// Upsert a replay nonce (`jwt_jti` or `idempotency`) in `mesh_replay_nonces`.
    pub async fn upsert_mesh_replay_nonce(
        &self,
        kind: &str,
        nonce_key: &str,
        recorded_at_ms: u64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO mesh_replay_nonces (kind, nonce_key, recorded_at_ms) \
                 VALUES (?1, ?2, ?3)",
                params![
                    kind.to_string(),
                    nonce_key.to_string(),
                    recorded_at_ms as i64
                ],
            )
            .await
            .map_err(StoreError::Turso)?;
        Ok(())
    }

    /// Load all replay nonces of `kind` (`jwt_jti` or `idempotency`) from `mesh_replay_nonces`.
    pub async fn list_mesh_replay_nonces(
        &self,
        kind: &str,
    ) -> Result<Vec<(String, u64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT nonce_key, recorded_at_ms FROM mesh_replay_nonces WHERE kind = ?1",
                params![kind.to_string()],
            )
            .await
            .map_err(StoreError::Turso)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(StoreError::Turso)? {
            let key: String = row.get(0).map_err(StoreError::Turso)?;
            let ts: i64 = row.get(1).map_err(StoreError::Turso)?;
            out.push((key, ts.max(0) as u64));
        }
        Ok(out)
    }

    /// Upsert a local mesh node JSON record in `mesh_local_nodes`.
    pub async fn upsert_mesh_local_node(
        &self,
        node_id: &str,
        record_json: &str,
        updated_at_ms: u64,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO mesh_local_nodes (node_id, record_json, updated_at_ms) \
                 VALUES (?1, ?2, ?3)",
                params![
                    node_id.to_string(),
                    record_json.to_string(),
                    updated_at_ms as i64
                ],
            )
            .await
            .map_err(StoreError::Turso)?;
        Ok(())
    }

    /// List all local mesh node JSON records from `mesh_local_nodes`.
    pub async fn list_mesh_local_nodes(&self) -> Result<Vec<(String, String, u64)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT node_id, record_json, updated_at_ms \
                 FROM mesh_local_nodes ORDER BY node_id ASC",
                (),
            )
            .await
            .map_err(StoreError::Turso)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await.map_err(StoreError::Turso)? {
            let id: String = row.get(0).map_err(StoreError::Turso)?;
            let json: String = row.get(1).map_err(StoreError::Turso)?;
            let ts: i64 = row.get(2).map_err(StoreError::Turso)?;
            out.push((id, json, ts.max(0) as u64));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::{DbConfig, VoxDb};

    fn approx_eq_vecs(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-6)
    }

    /// Round-trip: `put_cached_embedding` then `get_cached_embedding` returns
    /// the same vector (same length and values within float precision).
    #[tokio::test]
    async fn round_trip_put_then_get() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
        let sha = "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899";
        let model = "text-embedding-3-small";
        let vec: Vec<f32> = vec![0.1, -0.5, 0.9, 1.0, 0.0];

        db.put_cached_embedding(sha, model, &vec)
            .await
            .expect("put");

        let got = db
            .get_cached_embedding(sha)
            .await
            .expect("get")
            .expect("row present");

        assert_eq!(got.len(), vec.len(), "vector length must match");
        assert!(
            approx_eq_vecs(&got, &vec),
            "round-tripped values must match: got {got:?}, want {vec:?}"
        );
    }

    /// `get_cached_embedding` returns `None` for an unknown sha256.
    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
        let result = db
            .get_cached_embedding(
                "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
            )
            .await
            .expect("get");
        assert!(result.is_none(), "missing sha256 must return None");
    }

    /// Calling `put_cached_embedding` twice with the same key stores the
    /// SECOND vector (upsert / INSERT OR REPLACE semantics).
    #[tokio::test]
    async fn upsert_replaces_previous_vector() {
        let db = VoxDb::connect(DbConfig::Memory).await.expect("open db");
        let sha = "1111111111111111111111111111111111111111111111111111111111111111";
        let model = "text-embedding-3-small";
        let first: Vec<f32> = vec![1.0, 2.0, 3.0];
        let second: Vec<f32> = vec![9.0, 8.0, 7.0];

        db.put_cached_embedding(sha, model, &first)
            .await
            .expect("first put");
        db.put_cached_embedding(sha, model, &second)
            .await
            .expect("second put");

        let got = db
            .get_cached_embedding(sha)
            .await
            .expect("get")
            .expect("row present after upsert");

        assert!(
            approx_eq_vecs(&got, &second),
            "upsert must return the latest vector: got {got:?}, want {second:?}"
        );
    }
}
