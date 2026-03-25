//! Memory, knowledge graph, embeddings, and component registry for [`VoxDb`].
//!
//! Tables covered (all defined in V3 schema or `schema/domains/agents.rs`):
//! - **`memories`** — episodic agent/session memory rows.
//! - **`knowledge_nodes`** — labelled concept graph nodes (full-text searchable).
//! - **`embeddings`** — raw f32 vector blobs for similarity search.
//! - **`components`** — registered Vox UI/service component metadata.

use turso::params;


use crate::store::types::{EmbeddingEntry, MemoryEntry, SaveMemoryParams, StoreError};

impl crate::VoxDb {
    // ── Memories (memories) ───────────────────────────────────────────────────

    /// Append a row to `memories`. Returns the inserted `rowid`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::store_memory`.
    pub async fn save_memory(&self, p: SaveMemoryParams<'_>) -> Result<i64, StoreError> {
        self.conn
            .execute(
                "INSERT INTO memories
                     (agent_id, session_id, memory_type, content, metadata, importance, vcs_snapshot_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    p.agent_id,
                    p.session_id,
                    p.memory_type,
                    p.content,
                    p.metadata,
                    p.importance,
                    p.vcs_snapshot_id
                ],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Fetch recent `memories` for `agent_id`, newest first.
    ///
    /// Pass `memory_type = Some("…")` to filter; `_session_id` is accepted for API compatibility
    /// but not yet applied to avoid over-restricting results.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::recall_memory`.
    pub async fn recall_memory(
        &self,
        agent_id: &str,
        memory_type: Option<&str>,
        limit: i64,
        _session_id: Option<&str>,
    ) -> Result<Vec<MemoryEntry>, StoreError> {
        let lim = limit.clamp(1, 10_000);
        let mut rows = match memory_type {
            Some(t) => {
                self.conn
                    .query(
                        "SELECT id, agent_id, session_id, memory_type, content, metadata,
                                importance, created_at
                         FROM memories
                         WHERE agent_id = ?1 AND memory_type = ?2
                         ORDER BY created_at DESC LIMIT ?3",
                        params![agent_id, t, lim],
                    )
                    .await?
            }
            None => {
                self.conn
                    .query(
                        "SELECT id, agent_id, session_id, memory_type, content, metadata,
                                importance, created_at
                         FROM memories
                         WHERE agent_id = ?1
                         ORDER BY created_at DESC LIMIT ?2",
                        params![agent_id, lim],
                    )
                    .await?
            }
        };
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(MemoryEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                agent_id: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                session_id: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                memory_type: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
                content: row.get(4).map_err(|e| StoreError::Db(e.to_string()))?,
                metadata: row.get(5).map_err(|e| StoreError::Db(e.to_string()))?,
                importance: row.get(6).map_err(|e| StoreError::Db(e.to_string()))?,
                created_at: row.get(7).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }

    // ── Knowledge Nodes (knowledge_nodes) ────────────────────────────────────

    /// Upsert a knowledge node manually
    pub async fn upsert_knowledge_node(
        &self,
        id: &str,
        label: &str,
        content: &str,
        node_type: Option<&str>,
        metadata: Option<&str>,
        _vcs_snapshot_id: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO knowledge_nodes (id, label, content, node_type, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    label = excluded.label,
                    content = excluded.content,
                    node_type = excluded.node_type,
                    metadata = excluded.metadata",
                params![id, label, content, node_type, metadata],
            )
            .await?;
        Ok(())
    }

    /// Create an edge between knowledge nodes
    pub async fn create_knowledge_edge(
        &self,
        source_id: &str,
        target_id: &str,
        relation: &str,
        weight: f32,
        _metadata: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO knowledge_edges (src_id, dst_id, relation, weight)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(src_id, dst_id, relation) DO UPDATE SET
                    weight = excluded.weight",
                params![source_id, target_id, relation, weight],
            )
            .await?;
        Ok(())
    }

    /// Fetch neighboring nodes along with their relations
    pub async fn get_knowledge_neighbors(
        &self,
        node_id: &str,
    ) -> Result<Vec<(String, String, String, f32)>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT e.dst_id, n.label, e.relation, e.weight
                 FROM knowledge_edges e
                 JOIN knowledge_nodes n ON e.dst_id = n.id
                 WHERE e.src_id = ?1
                 UNION
                 SELECT e.src_id, n.label, e.relation, e.weight
                 FROM knowledge_edges e
                 JOIN knowledge_nodes n ON e.src_id = n.id
                 WHERE e.dst_id = ?1",
                params![node_id],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let label: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let rel: String = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            let w: f64 = row.get(3).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((id, label, rel, w as f32));
        }
        Ok(out)
    }

    /// Full-text LIKE search over `knowledge_nodes` (label + content).
    ///
    /// Returns `(id, label, snippet)` — snippet is the first 200 chars of `content`.
    /// Called from `vox-db/src/lib.rs` `VoxDb::search_memories`.
    pub async fn query_knowledge_nodes(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let lim = limit.clamp(1, 1_000);
        let pat = format!("%{query}%");
        let mut rows = self
            .conn
            .query(
                "SELECT id, label, COALESCE(SUBSTR(content, 1, 200), '')
                 FROM knowledge_nodes
                 WHERE label LIKE ?1 OR content LIKE ?1
                 ORDER BY created_at DESC LIMIT ?2",
                params![pat, lim],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let label: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let snippet: String = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((id, label, snippet));
        }
        Ok(out)
    }

    // ── Embeddings (embeddings) ───────────────────────────────────────────────

    /// Store a raw embedding vector.
    pub async fn store_embedding(
        &self,
        source_type: &str,
        source_id: &str,
        _model: &str,
        vector: &[f32],
        metadata: Option<&str>,
        _vcs_snapshot_id: Option<&str>,
    ) -> Result<i64, StoreError> {
        let mut blob = Vec::with_capacity(vector.len() * 4);
        for &v in vector {
            blob.extend_from_slice(&v.to_le_bytes());
        }
        self.conn
            .execute(
                "INSERT INTO embeddings (source_type, source_id, dim, vector, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![source_type, source_id, vector.len() as i64, blob, metadata],
            )
            .await?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Brute-force cosine similarity search over the `embeddings` table.
    ///
    /// Fetches up to `limit * 10` candidate rows, scores each, and returns the top `limit`
    /// sorted by similarity descending. Suitable for small tables (< 10 k rows).
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::search_embeddings`.
    pub async fn search_similar_embeddings(
        &self,
        vector: &[f32],
        source_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(EmbeddingEntry, f32)>, StoreError> {
        let lim = limit.clamp(1, 500);
        let candidate_cap = lim * 10;
        let mut rows = match source_type {
            Some(st) => {
                self.conn
                    .query(
                        "SELECT id, source_type, source_id, dim, vector, metadata
                         FROM embeddings WHERE source_type = ?1
                         ORDER BY created_at DESC LIMIT ?2",
                        params![st, candidate_cap],
                    )
                    .await?
            }
            None => {
                self.conn
                    .query(
                        "SELECT id, source_type, source_id, dim, vector, metadata
                         FROM embeddings ORDER BY created_at DESC LIMIT ?1",
                        params![candidate_cap],
                    )
                    .await?
            }
        };

        let mut scored: Vec<(EmbeddingEntry, f32)> = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: i64 = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let st: Option<String> = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let source_id: String = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            let dim: i64 = row.get(3).map_err(|e| StoreError::Db(e.to_string()))?;
            let blob: Vec<u8> = row.get(4).map_err(|e| StoreError::Db(e.to_string()))?;
            let metadata: Option<String> = row.get(5).map_err(|e| StoreError::Db(e.to_string()))?;
            // Deserialise little-endian f32 bytes
            let stored: Vec<f32> = blob
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let dot: f32 = vector.iter().zip(stored.iter()).map(|(a, b)| a * b).sum();
            let mag_a: f32 = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
            let mag_b: f32 = stored.iter().map(|x| x * x).sum::<f32>().sqrt();
            let sim = if mag_a > 0.0 && mag_b > 0.0 {
                dot / (mag_a * mag_b)
            } else {
                0.0
            };
            scored.push((
                EmbeddingEntry {
                    id,
                    source_type: st,
                    source_id,
                    dim,
                    metadata,
                },
                sim,
            ));
        }
        scored.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(lim as usize);
        Ok(scored)
    }

    // ── Components (components) ───────────────────────────────────────────────

    /// Upsert a row in `components`.
    ///
    /// Called from `vox-db/src/lib.rs` `VoxDb::register_local_project`.
    pub async fn register_component(
        &self,
        name: &str,
        namespace: &str,
        schema_hash: Option<&str>,
        description: Option<&str>,
        version: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO components (name, namespace, schema_hash, version, description)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(name)
                 DO UPDATE SET namespace   = excluded.namespace,
                               schema_hash = COALESCE(excluded.schema_hash, components.schema_hash),
                               version     = excluded.version,
                               description = COALESCE(excluded.description, components.description)",
                params![name, namespace, schema_hash, version, description],
            )
            .await?;
        Ok(())
    }
}
