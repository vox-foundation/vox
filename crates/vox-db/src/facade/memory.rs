use crate::{EmbeddingEntry, StoreError, learning, memory::MemoryParams};
use turso::params;

impl crate::VoxDb {
    // ── Memory Convenience Methods ──────────────────────

    /// Persist an agent memory row (`memories` table). See [`MemoryParams`] for fields.
    pub async fn store_memory(&self, params: MemoryParams<'_>) -> Result<i64, StoreError> {
        self.save_memory(params).await
    }

    /// Full-text-ish search over knowledge nodes (delegates to `VoxDb::query_knowledge_nodes`).
    ///
    /// Returns `(id, title, snippet)` tuples as produced by the store layer.
    pub async fn search_memories(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        self.query_knowledge_nodes(query, limit).await
    }

    /// Full-text search over `search_document_chunks` (see [`crate::VoxDb::query_search_document_chunks`]).
    ///
    /// Returns `(chunk_id, document_id, body_snippet, document_title)` tuples.
    pub async fn search_ingested_chunks(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(i64, i64, String, String)>, StoreError> {
        self.query_search_document_chunks(query, limit).await
    }

    /// Vector similarity search in `embeddings` (optional `source_type` filter).
    pub async fn search_embeddings(
        &self,
        vector: &[f32],
        source_type: Option<&str>,
        limit: i64,
    ) -> Result<Vec<(EmbeddingEntry, f32)>, StoreError> {
        self.search_similar_embeddings(vector, source_type, limit)
            .await
    }

    /// Return a behavioral learner for this database.
    pub fn learner(&self) -> learning::BehavioralLearner<'_> {
        learning::BehavioralLearner::new(self)
    }

    /// Run a parameterized `SELECT` and collect all rows (for small result sets).
    pub async fn query_all(
        &self,
        sql: &str,
        params: impl turso::IntoParams + Send,
    ) -> Result<Vec<turso::Row>, StoreError> {
        let mut cursor = self.conn.query(sql, params).await?;
        let mut rows = Vec::new();
        while let Some(row) = cursor.next().await? {
            rows.push(row);
        }
        Ok(rows)
    }

    /// Search for symbols across the project by label.
    pub async fn search_project_symbols(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(String, String, String)>, StoreError> {
        let pat = format!("%{query}%");
        let mut rows = self
            .conn
            .query(
                "SELECT id, label, COALESCE(metadata, '')
                 FROM knowledge_nodes
                 WHERE node_type = 'symbol' AND (label LIKE ?1 OR id LIKE ?1)
                 ORDER BY created_at DESC LIMIT ?2",
                params![pat, limit],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            let id: String = row.get(0).map_err(|e| StoreError::Db(e.to_string()))?;
            let label: String = row.get(1).map_err(|e| StoreError::Db(e.to_string()))?;
            let meta: String = row.get(2).map_err(|e| StoreError::Db(e.to_string()))?;
            out.push((id, label, meta));
        }
        Ok(out)
    }
}
