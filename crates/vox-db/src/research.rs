//! Ingest external research into **Codex** (`knowledge_nodes`, `snippets`) and maintain
//! `codex_capability_map` for competitive / capability tracking.
//!
//! Use [`crate::VoxDb::ingest_research_document_async`] or [`crate::VoxDb::ingest_research_document`]
//! with a [`ResearchIngestRequest`]. Capability-map DDL is applied when those APIs touch the map.

use crate::VoxDb;
use serde::{Deserialize, Serialize};
use turso::params;
use crate::arca_store::StoreError;

const CAP_TABLE: &str = "
CREATE TABLE IF NOT EXISTS codex_capability_map (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    topic TEXT NOT NULL,
    vendor TEXT NOT NULL,
    area TEXT NOT NULL,
    openclaw_capability TEXT NOT NULL,
    vox_evidence TEXT NOT NULL,
    status TEXT NOT NULL,
    advantage_direction TEXT NOT NULL,
    recommended_action TEXT NOT NULL,
    linked_paths_json TEXT NOT NULL,
    metadata_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_codex_cap_vendor_topic ON codex_capability_map (vendor, topic);
";

/// Serializable metadata for a captured research artifact (before DB insert).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalResearchPacket {
    /// Subject area or product topic.
    pub topic: String,
    /// Vendor or competitor name.
    pub vendor: String,
    /// Optional sub-area (e.g. “CLI”, “pricing”).
    pub area: Option<String>,
    /// Canonical URL of the source page.
    pub source_url: String,
    /// MIME-ish label: `web`, `pdf`, etc.
    pub source_type: String,
    /// Human-readable document title.
    pub title: String,
    /// ISO-8601 or similar capture timestamp string.
    pub captured_at: String,
    /// Short abstract for search / UI.
    pub summary: String,
    /// Verbatim excerpt retained for audit.
    pub raw_excerpt: String,
    /// Structured claim objects (JSON).
    pub claims: Vec<serde_json::Value>,
    /// Free-form tags for filtering.
    pub tags: Vec<String>,
    /// Heuristic confidence in `[0, 1]` (caller-defined).
    pub confidence: f64,
    /// Blake3 hex over normalized content (dedup key).
    pub content_hash: String,
    /// Extra JSON bag (provenance, tool ids, etc.).
    pub metadata: serde_json::Value,
}

/// Full ingest input: structured packet plus raw body text for chunking.
pub struct ResearchIngestRequest {
    /// Structured header fields (hashed and stored in `knowledge_nodes.metadata`).
    pub packet: ExternalResearchPacket,
    /// Full text used for chunking and embedding hooks.
    pub body: String,
    /// Optional knowledge-base id for partitioning.
    pub kb_id: Option<String>,
    /// Reserved for future vector ingest; ignored today.
    pub embeddings: Vec<Vec<f32>>,
}

/// Row ids returned from [`VoxDb::ingest_research_document`](crate::VoxDb::ingest_research_document).
#[derive(Debug, Clone)]
pub struct ResearchIngestResult {
    /// `knowledge_nodes` row id from the insert.
    pub packet_id: i64,
    /// Reserved for future normalized document table.
    pub document_id: Option<i64>,
    /// `snippets` row ids for each text chunk.
    pub chunk_ids: Vec<i64>,
    /// Echo of request `kb_id`.
    pub kb_id: Option<String>,
    /// Blake3 hex over `body` (also written into the packet).
    pub content_hash: String,
}

/// One capability comparison row for `codex_capability_map`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityMapRecord {
    /// Subject or product under comparison.
    pub topic: String,
    /// External vendor or competitor id.
    pub vendor: String,
    /// Sub-domain (e.g. `pricing`, `security`).
    pub area: String,
    /// Capability label in the comparison framework.
    pub openclaw_capability: String,
    /// Vox-side evidence or counterpoint text.
    pub vox_evidence: String,
    /// Workflow status (e.g. `open`, `closed`).
    pub status: String,
    /// Who leads: `vox`, `peer`, `tie`, etc.
    pub advantage_direction: String,
    /// Suggested next engineering or research step.
    pub recommended_action: String,
    /// Repo paths or URLs cited in the analysis.
    pub linked_paths: Vec<String>,
    /// Free-form JSON for tooling-specific fields.
    pub metadata: serde_json::Value,
}

fn blake3_hex(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

fn chunk_text(body: &str, max_chunk: usize) -> Vec<String> {
    if body.is_empty() {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    for word in body.split_whitespace() {
        if cur.len() + word.len() + 1 > max_chunk && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(word);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

async fn ensure_cap_table(db: &VoxDb) -> Result<(), StoreError> {
    db.store().connection().execute_batch(CAP_TABLE).await?;
    Ok(())
}

impl VoxDb {
    /// Persist a fetched document: metadata JSON in `knowledge_nodes`, text chunks in `snippets`.
    ///
    /// Overwrites [`ResearchIngestRequest::packet`].`content_hash` from `body`. Mutates `req` in place.
    pub async fn ingest_research_document_async(
        &self,
        req: &mut ResearchIngestRequest,
    ) -> Result<ResearchIngestResult, StoreError> {
        let hash = blake3_hex(req.body.as_bytes());
        req.packet.content_hash = hash.clone();
        let node_id = format!("research:{hash}");
        let meta = serde_json::to_string(&req.packet)
            .map_err(|e| StoreError::Serialization(e.to_string()))?;

        self.store()
            .connection()
            .execute(
                "INSERT OR REPLACE INTO knowledge_nodes (id, label, content, category, metadata, created_at)
                 VALUES (?1, ?2, ?3, 'external_research', ?4, datetime('now'))",
                params![
                    node_id.clone(),
                    req.packet.title.as_str(),
                    req.body.as_str(),
                    meta.as_str()
                ],
            )
            .await?;

        let packet_rowid = self.store().connection().last_insert_rowid();

        let chunks = chunk_text(&req.body, 2048);
        let mut chunk_ids = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let title = format!("{}#{}", req.packet.topic, i);
            self.store()
                .connection()
                .execute(
                    "INSERT INTO snippets (language, title, code, description, tags, author_id, source_ref, embedding_ref)
                     VALUES ('research_chunk', ?1, ?2, ?3, ?4, NULL, ?5, NULL)",
                    params![
                        title,
                        chunk.as_str(),
                        req.packet.summary.as_str(),
                        req.packet.tags.join(","),
                        req.packet.source_url.as_str(),
                    ],
                )
                .await?;
            chunk_ids.push(self.store().connection().last_insert_rowid());
        }

        Ok(ResearchIngestResult {
            packet_id: packet_rowid,
            document_id: None,
            chunk_ids,
            kb_id: req.kb_id.clone(),
            content_hash: hash,
        })
    }

    /// Same as [`Self::ingest_research_document_async`] but blocks on the store’s internal runtime.
    pub fn ingest_research_document(
        &self,
        req: &mut ResearchIngestRequest,
    ) -> Result<ResearchIngestResult, StoreError> {
        self.store()
            .block_on(self.ingest_research_document_async(req))
    }

    /// Deserialize recent `external_research` rows from `knowledge_nodes` (optional vendor/topic filter).
    pub fn list_research_packets(
        &self,
        vendor: Option<&str>,
        topic: Option<&str>,
        limit: i64,
    ) -> Result<Vec<ExternalResearchPacket>, StoreError> {
        let limit = limit.clamp(1, 50_000);
        let vendor = vendor.map(str::to_string);
        let topic = topic.map(str::to_string);
        self.store().block_on(async {
            let lim = limit;
            let mut rows = match (&vendor, &topic) {
                (Some(v), Some(t)) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT metadata FROM knowledge_nodes WHERE category = 'external_research'
                             AND json_extract(metadata, '$.vendor') = ?1 AND json_extract(metadata, '$.topic') = ?2
                             ORDER BY created_at DESC LIMIT ?3",
                            params![v.as_str(), t.as_str(), lim],
                        )
                        .await?
                }
                (Some(v), None) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT metadata FROM knowledge_nodes WHERE category = 'external_research'
                             AND json_extract(metadata, '$.vendor') = ?1
                             ORDER BY created_at DESC LIMIT ?2",
                            params![v.as_str(), lim],
                        )
                        .await?
                }
                (None, Some(t)) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT metadata FROM knowledge_nodes WHERE category = 'external_research'
                             AND json_extract(metadata, '$.topic') = ?1
                             ORDER BY created_at DESC LIMIT ?2",
                            params![t.as_str(), lim],
                        )
                        .await?
                }
                (None, None) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT metadata FROM knowledge_nodes WHERE category = 'external_research'
                             ORDER BY created_at DESC LIMIT ?1",
                            params![lim],
                        )
                        .await?
                }
            };
            let mut out = Vec::new();
            while let Some(row) = rows.next().await? {
                let s: String = row.get::<String>(0)?;
                let p: ExternalResearchPacket =
                    serde_json::from_str(&s).map_err(|e| StoreError::Serialization(e.to_string()))?;
                out.push(p);
            }
            Ok(out)
        })
    }

    /// Insert one row into `codex_capability_map` (ensures table exists). Returns SQLite rowid.
    pub fn store_capability_map_record(
        &self,
        rec: &CapabilityMapRecord,
    ) -> Result<i64, StoreError> {
        self.store().block_on(async {
            ensure_cap_table(self).await?;
            let linked = serde_json::to_string(&rec.linked_paths)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            let meta = serde_json::to_string(&rec.metadata)
                .map_err(|e| StoreError::Serialization(e.to_string()))?;
            self.store()
                .connection()
                .execute(
                    "INSERT INTO codex_capability_map (
                        topic, vendor, area, openclaw_capability, vox_evidence, status,
                        advantage_direction, recommended_action, linked_paths_json, metadata_json
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        rec.topic.as_str(),
                        rec.vendor.as_str(),
                        rec.area.as_str(),
                        rec.openclaw_capability.as_str(),
                        rec.vox_evidence.as_str(),
                        rec.status.as_str(),
                        rec.advantage_direction.as_str(),
                        rec.recommended_action.as_str(),
                        linked.as_str(),
                        meta.as_str(),
                    ],
                )
                .await?;
            Ok(self.store().connection().last_insert_rowid())
        })
    }

    /// List capability-map rows newest-first, optionally filtered by vendor and/or topic.
    pub fn list_capability_map_records(
        &self,
        vendor: Option<&str>,
        topic: Option<&str>,
        limit: i64,
    ) -> Result<Vec<CapabilityMapRecord>, StoreError> {
        let limit = limit.clamp(1, 50_000);
        let vendor = vendor.map(str::to_string);
        let topic = topic.map(str::to_string);
        self.store().block_on(async {
            ensure_cap_table(self).await?;
            let mut rows = match (&vendor, &topic) {
                (Some(v), Some(t)) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT topic, vendor, area, openclaw_capability, vox_evidence, status,
                                    advantage_direction, recommended_action, linked_paths_json, metadata_json
                             FROM codex_capability_map WHERE vendor = ?1 AND topic = ?2
                             ORDER BY id DESC LIMIT ?3",
                            params![v.as_str(), t.as_str(), limit],
                        )
                        .await?
                }
                (Some(v), None) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT topic, vendor, area, openclaw_capability, vox_evidence, status,
                                    advantage_direction, recommended_action, linked_paths_json, metadata_json
                             FROM codex_capability_map WHERE vendor = ?1
                             ORDER BY id DESC LIMIT ?2",
                            params![v.as_str(), limit],
                        )
                        .await?
                }
                (None, Some(t)) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT topic, vendor, area, openclaw_capability, vox_evidence, status,
                                    advantage_direction, recommended_action, linked_paths_json, metadata_json
                             FROM codex_capability_map WHERE topic = ?1
                             ORDER BY id DESC LIMIT ?2",
                            params![t.as_str(), limit],
                        )
                        .await?
                }
                (None, None) => {
                    self.store()
                        .connection()
                        .query(
                            "SELECT topic, vendor, area, openclaw_capability, vox_evidence, status,
                                    advantage_direction, recommended_action, linked_paths_json, metadata_json
                             FROM codex_capability_map
                             ORDER BY id DESC LIMIT ?1",
                            params![limit],
                        )
                        .await?
                }
            };
            let mut out = Vec::new();
            while let Some(row) = rows.next().await? {
                let linked: String = row.get::<String>(8)?;
                let meta: String = row.get::<String>(9)?;
                out.push(CapabilityMapRecord {
                    topic: row.get::<String>(0)?,
                    vendor: row.get::<String>(1)?,
                    area: row.get::<String>(2)?,
                    openclaw_capability: row.get::<String>(3)?,
                    vox_evidence: row.get::<String>(4)?,
                    status: row.get::<String>(5)?,
                    advantage_direction: row.get::<String>(6)?,
                    recommended_action: row.get::<String>(7)?,
                    linked_paths: serde_json::from_str(&linked).unwrap_or_default(),
                    metadata: serde_json::from_str(&meta).unwrap_or_else(|_| serde_json::json!({})),
                });
            }
            Ok(out)
        })
    }
}

/// Aggregate counts for `vox db retrieval-status` and similar diagnostics.
#[derive(Debug, Clone)]
pub struct RetrievalDiagnostics {
    /// Rows in `embeddings`.
    pub embeddings_count: usize,
    /// Rows in `knowledge_nodes`.
    pub knowledge_nodes_count: usize,
    /// Rows in `knowledge_edges`.
    pub knowledge_edges_count: usize,
    /// Default vector weight for hybrid fusion (informational; not persisted).
    pub vector_weight: f64,
    /// Placeholder for future latency tracking.
    pub last_retrieval_latency_ms: Option<u64>,
    /// Placeholder labels for modality usage splits.
    pub retrieval_mode_splits: Vec<String>,
}

/// Cheap `COUNT(*)` snapshots over retrieval-related tables.
pub fn retrieval_diagnostics(
    store: &crate::arca_store::CodeStore,
) -> Result<RetrievalDiagnostics, StoreError> {
    store.block_on(async {
        let mut r1 = store
            .connection()
            .query("SELECT COUNT(*) FROM embeddings", ())
            .await?;
        let embeddings_count = r1
            .next()
            .await?
            .map(|row| row.get::<i64>(0).unwrap_or(0) as usize)
            .unwrap_or(0);

        let mut r2 = store
            .connection()
            .query("SELECT COUNT(*) FROM knowledge_nodes", ())
            .await?;
        let knowledge_nodes_count = r2
            .next()
            .await?
            .map(|row| row.get::<i64>(0).unwrap_or(0) as usize)
            .unwrap_or(0);

        let mut r3 = store
            .connection()
            .query("SELECT COUNT(*) FROM knowledge_edges", ())
            .await?;
        let knowledge_edges_count = r3
            .next()
            .await?
            .map(|row| row.get::<i64>(0).unwrap_or(0) as usize)
            .unwrap_or(0);

        Ok(RetrievalDiagnostics {
            embeddings_count,
            knowledge_nodes_count,
            knowledge_edges_count,
            vector_weight: 0.65,
            last_retrieval_latency_ms: None,
            retrieval_mode_splits: vec!["hybrid".into(), "vector".into(), "fulltext".into()],
        })
    })
}
