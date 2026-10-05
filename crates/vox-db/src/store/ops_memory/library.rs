//! KB Library store: hash-keyed chunk rows over `search_documents` / `search_document_chunks`
//! (baseline 95). The AUTOINCREMENT rowid is a storage detail; `chunk_hash` is the identity that
//! embeddings (`source_type = 'kb_chunk'`), citations and `kb://chunk/{hash}` use.

use crate::store::types::StoreError;
use turso::params;

/// `search_documents.corpus` value for the docs/src library index.
pub const KB_LIBRARY_CORPUS: &str = "library";
/// `embeddings.source_type` for library chunk vectors (keyed by `chunk_hash`, never rowid).
pub const KB_CHUNK_SOURCE_TYPE: &str = "kb_chunk";
/// The four `ContextTrustTier` wire strings (snake_case serde). vox-db sits below
/// vox-orchestrator, so the wire form is validated here; Phase 3 swaps the type.
pub const TRUST_TIER_WIRE: [&str; 4] = ["untrusted", "advisory", "trusted", "system_verified"];

/// Reject anything that is not one of [`TRUST_TIER_WIRE`] (case-sensitive).
pub fn validate_trust_tier_wire(tier: &str) -> Result<(), StoreError> {
    if TRUST_TIER_WIRE.contains(&tier) {
        Ok(())
    } else {
        Err(StoreError::Db(format!(
            "invalid trust tier {tier:?}; expected one of {TRUST_TIER_WIRE:?}"
        )))
    }
}

/// One library document row. No `Default` on purpose: every writer must choose a trust tier.
#[derive(Debug, Clone)]
pub struct LibraryDocUpsert<'a> {
    pub corpus: &'a str,
    pub rel_path: &'a str,
    pub title: &'a str,
    pub file_hash: &'a str,
    pub frontmatter_json: &'a str,
    pub indexed_commit: Option<&'a str>,
    pub index_error: Option<&'a str>,
    pub trust_tier: &'a str,
}

/// One chunk to store for a document, in document order.
#[derive(Debug, Clone)]
pub struct LibraryChunkInput {
    pub chunk_hash: String,
    pub chunk_kind: String,
    pub heading_path: String,
    pub anchor: Option<String>,
    pub token_estimate: i64,
    pub body: String,
}

/// What [`crate::VoxDb::apply_library_document`] changed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryChunkDiff {
    pub inserted: usize,
    pub deleted: usize,
    pub kept: usize,
    pub deleted_hashes: Vec<String>,
}

/// A stored chunk joined with its document.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryChunkRow {
    pub chunk_hash: String,
    pub doc_path: String,
    pub title: String,
    pub heading_path: String,
    pub anchor: Option<String>,
    pub chunk_kind: String,
    pub body: String,
    pub token_estimate: i64,
}

/// Storage position of one chunk: the AUTOINCREMENT rowid, owning document and `chunk_index`.
/// For index-stability checks (kept chunks keep rowid and hash); not an identity (use the hash).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryChunkPos {
    pub rowid: i64,
    pub rel_path: String,
    pub chunk_index: i64,
    pub chunk_hash: String,
}

/// One `kb_index_state` row: what the last index run built and from what (IDX-03).
/// `indexed_at` is assigned by the store on write; the value passed to
/// [`crate::VoxDb::put_kb_index_state`] is ignored. `errors_json` is a JSON array of
/// `{path, error}` objects, kept as text.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KbIndexState {
    pub corpus: String,
    pub corpus_root: String,
    pub corpus_commit: Option<String>,
    pub indexed_at: String,
    pub doc_count: i64,
    pub chunk_count: i64,
    pub card_count: i64,
    pub chunker_version: String,
    pub lexical_backend: String,
    pub trust_tier: String,
    pub embed_model_id: Option<String>,
    pub embed_revision: Option<String>,
    pub embed_dim: Option<i64>,
    pub errors_json: String,
    pub duration_ms: i64,
}

/// A library document's metadata (no chunk bodies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryDocRow {
    pub rel_path: String,
    pub title: String,
    pub frontmatter_json: String,
    pub indexed_commit: Option<String>,
    pub index_error: Option<String>,
    pub file_hash: String,
}

const CHUNK_SELECT: &str = "SELECT c.chunk_hash, d.rel_path, d.title, c.heading_path, c.anchor, \
     c.chunk_kind, c.body_text, c.token_estimate \
     FROM search_document_chunks c JOIN search_documents d ON d.id = c.document_id";

fn chunk_row(r: &turso::Row) -> Result<LibraryChunkRow, StoreError> {
    Ok(LibraryChunkRow {
        chunk_hash: r.get::<String>(0)?,
        doc_path: r.get::<Option<String>>(1)?.unwrap_or_default(),
        title: r.get::<String>(2)?,
        heading_path: r.get::<String>(3)?,
        anchor: r.get::<Option<String>>(4)?,
        chunk_kind: r.get::<String>(5)?,
        body: r.get::<String>(6)?,
        token_estimate: r.get::<Option<i64>>(7)?.unwrap_or(0),
    })
}

impl crate::VoxDb {
    /// Upsert one library document and diff its chunks by `chunk_hash`, in ONE transaction.
    ///
    /// Rows whose hash is absent from `chunks` are deleted, new hashes are inserted, kept rows
    /// are renumbered to their new `chunk_index` (two passes through negative temporaries so
    /// `UNIQUE(document_id, chunk_index)` never collides) and never have their body rewritten,
    /// so rowids (and anything keyed to them) survive an edit elsewhere in the file.
    pub async fn apply_library_document(
        &self,
        doc: &LibraryDocUpsert<'_>,
        chunks: &[LibraryChunkInput],
    ) -> Result<LibraryChunkDiff, StoreError> {
        validate_trust_tier_wire(doc.trust_tier)?;
        let mut seen = std::collections::HashSet::new();
        if let Some(dup) = chunks.iter().find(|c| !seen.insert(c.chunk_hash.as_str())) {
            return Err(StoreError::Db(format!(
                "duplicate chunk_hash {} within {}",
                dup.chunk_hash, doc.rel_path
            )));
        }
        let conn = self.conn.clone();
        let source_uri = format!("{KB_LIBRARY_CORPUS}:{}", doc.rel_path);
        self.transaction(async move {
            conn.execute(
                "INSERT INTO search_documents \
                 (source_uri, title, mime_type, content_hash, corpus, rel_path, frontmatter_json, \
                  indexed_commit, index_error) \
                 VALUES (?1, ?2, 'text/markdown', ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT(source_uri) DO UPDATE SET \
                    title = excluded.title, content_hash = excluded.content_hash, \
                    corpus = excluded.corpus, rel_path = excluded.rel_path, \
                    frontmatter_json = excluded.frontmatter_json, \
                    indexed_commit = excluded.indexed_commit, index_error = excluded.index_error, \
                    ingested_at = datetime('now')",
                params![
                    source_uri.as_str(),
                    doc.title,
                    doc.file_hash,
                    doc.corpus,
                    doc.rel_path,
                    doc.frontmatter_json,
                    doc.indexed_commit,
                    doc.index_error
                ],
            )
            .await?;
            let mut rows = conn
                .query(
                    "SELECT id FROM search_documents WHERE source_uri = ?1",
                    params![source_uri.as_str()],
                )
                .await?;
            let doc_id: i64 = rows
                .next()
                .await?
                .ok_or_else(|| StoreError::Db("library document upsert returned no id".into()))?
                .get(0)?;
            drop(rows);

            let mut rows = conn
                .query(
                    "SELECT id, chunk_hash, chunk_index FROM search_document_chunks \
                     WHERE document_id = ?1",
                    params![doc_id],
                )
                .await?;
            let mut existing: Vec<(i64, Option<String>, i64)> = Vec::new();
            while let Some(r) = rows.next().await? {
                existing.push((r.get(0)?, r.get::<Option<String>>(1)?, r.get(2)?));
            }
            drop(rows);

            let new_pos: std::collections::HashMap<&str, i64> = chunks
                .iter()
                .enumerate()
                .map(|(i, c)| (c.chunk_hash.as_str(), i as i64))
                .collect();
            let mut diff = LibraryChunkDiff::default();
            let mut kept_hashes = std::collections::HashSet::new();
            let mut moves: Vec<(i64, i64)> = Vec::new(); // (rowid, final index)
            for (rowid, hash, idx) in &existing {
                match hash.as_deref().and_then(|h| new_pos.get_key_value(h)) {
                    Some((h, &pos)) => {
                        kept_hashes.insert(*h);
                        diff.kept += 1;
                        if pos != *idx {
                            moves.push((*rowid, pos));
                        }
                    }
                    None => {
                        conn.execute(
                            "DELETE FROM search_document_chunks WHERE id = ?1",
                            params![*rowid],
                        )
                        .await?;
                        diff.deleted += 1;
                        if let Some(h) = hash {
                            diff.deleted_hashes.push(h.clone());
                        }
                    }
                }
            }
            for (n, (rowid, _)) in moves.iter().enumerate() {
                conn.execute(
                    "UPDATE search_document_chunks SET chunk_index = ?1 WHERE id = ?2",
                    params![-(n as i64) - 1, *rowid],
                )
                .await?;
            }
            for (rowid, pos) in &moves {
                conn.execute(
                    "UPDATE search_document_chunks SET chunk_index = ?1 WHERE id = ?2",
                    params![*pos, *rowid],
                )
                .await?;
            }
            for (pos, c) in chunks.iter().enumerate() {
                if kept_hashes.contains(c.chunk_hash.as_str()) {
                    continue;
                }
                conn.execute(
                    "INSERT INTO search_document_chunks \
                     (document_id, chunk_index, body_text, chunk_hash, chunk_kind, heading_path, \
                      anchor, token_estimate) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        doc_id,
                        pos as i64,
                        c.body.as_str(),
                        c.chunk_hash.as_str(),
                        c.chunk_kind.as_str(),
                        c.heading_path.as_str(),
                        c.anchor.as_deref(),
                        c.token_estimate
                    ],
                )
                .await?;
                diff.inserted += 1;
            }
            Ok(diff)
        })
        .await
    }

    /// Stored whole-file content hash of a library document, if indexed (IDX-01 skip key).
    pub async fn library_document_file_hash(
        &self,
        corpus: &str,
        rel_path: &str,
    ) -> Result<Option<String>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT content_hash FROM search_documents WHERE corpus = ?1 AND rel_path = ?2",
                params![corpus, rel_path],
            )
            .await?;
        match rows.next().await? {
            Some(r) => Ok(Some(r.get::<String>(0)?)),
            None => Ok(None),
        }
    }

    /// Number of hash-keyed chunk rows in `corpus`.
    pub async fn library_chunk_count(&self, corpus: &str) -> Result<i64, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*) FROM search_document_chunks c \
                 JOIN search_documents d ON d.id = c.document_id \
                 WHERE d.corpus = ?1 AND c.chunk_hash IS NOT NULL",
                params![corpus],
            )
            .await?;
        Ok(match rows.next().await? {
            Some(r) => r.get::<i64>(0)?,
            None => 0,
        })
    }

    /// Delete every `corpus` document whose `rel_path` is not in `keep`, with its chunks (explicit
    /// DELETEs, not FK cascade), in one transaction. Returns the pruned paths, sorted.
    pub async fn prune_library_documents_not_in(
        &self,
        corpus: &str,
        keep: &[String],
    ) -> Result<Vec<String>, StoreError> {
        let conn = self.conn.clone();
        let corpus = corpus.to_string();
        let keep: std::collections::HashSet<String> = keep.iter().cloned().collect();
        self.transaction(async move {
            let mut rows = conn
                .query(
                    "SELECT id, rel_path FROM search_documents WHERE corpus = ?1",
                    params![corpus.as_str()],
                )
                .await?;
            let mut doomed: Vec<(i64, String)> = Vec::new();
            while let Some(r) = rows.next().await? {
                let path = r.get::<Option<String>>(1)?.unwrap_or_default();
                if !keep.contains(&path) {
                    doomed.push((r.get(0)?, path));
                }
            }
            drop(rows);
            for (id, _) in &doomed {
                conn.execute(
                    "DELETE FROM search_document_chunks WHERE document_id = ?1",
                    params![*id],
                )
                .await?;
                conn.execute("DELETE FROM search_documents WHERE id = ?1", params![*id])
                    .await?;
            }
            let mut pruned: Vec<String> = doomed.into_iter().map(|d| d.1).collect();
            pruned.sort();
            Ok(pruned)
        })
        .await
    }

    /// Delete `kb_chunk` embeddings whose `source_id` is no longer any chunk's `chunk_hash`.
    /// Other `source_type`s are never touched. Returns the number of rows removed.
    pub async fn prune_orphan_kb_chunk_embeddings(&self) -> Result<usize, StoreError> {
        let n = self
            .conn
            .execute(
                "DELETE FROM embeddings WHERE source_type = ?1 AND source_id NOT IN \
                 (SELECT chunk_hash FROM search_document_chunks WHERE chunk_hash IS NOT NULL)",
                params![KB_CHUNK_SOURCE_TYPE],
            )
            .await?;
        Ok(n as usize)
    }

    /// Insert or replace the manifest row for `state.corpus`; `indexed_at` is set to now.
    pub async fn put_kb_index_state(&self, state: &KbIndexState) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT INTO kb_index_state \
                 (corpus, corpus_root, corpus_commit, indexed_at, doc_count, chunk_count, \
                  card_count, chunker_version, lexical_backend, trust_tier, embed_model_id, \
                  embed_revision, embed_dim, errors_json, duration_ms) \
                 VALUES (?1, ?2, ?3, datetime('now'), ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14) \
                 ON CONFLICT(corpus) DO UPDATE SET \
                    corpus_root = excluded.corpus_root, corpus_commit = excluded.corpus_commit, \
                    indexed_at = excluded.indexed_at, doc_count = excluded.doc_count, \
                    chunk_count = excluded.chunk_count, card_count = excluded.card_count, \
                    chunker_version = excluded.chunker_version, \
                    lexical_backend = excluded.lexical_backend, trust_tier = excluded.trust_tier, \
                    embed_model_id = excluded.embed_model_id, \
                    embed_revision = excluded.embed_revision, embed_dim = excluded.embed_dim, \
                    errors_json = excluded.errors_json, duration_ms = excluded.duration_ms",
                params![
                    state.corpus.as_str(),
                    state.corpus_root.as_str(),
                    state.corpus_commit.as_deref(),
                    state.doc_count,
                    state.chunk_count,
                    state.card_count,
                    state.chunker_version.as_str(),
                    state.lexical_backend.as_str(),
                    state.trust_tier.as_str(),
                    state.embed_model_id.as_deref(),
                    state.embed_revision.as_deref(),
                    state.embed_dim,
                    state.errors_json.as_str(),
                    state.duration_ms
                ],
            )
            .await?;
        Ok(())
    }

    /// The manifest row of the last index run for `corpus`, if any.
    pub async fn get_kb_index_state(
        &self,
        corpus: &str,
    ) -> Result<Option<KbIndexState>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT corpus, corpus_root, corpus_commit, indexed_at, doc_count, chunk_count, \
                 card_count, chunker_version, lexical_backend, trust_tier, embed_model_id, \
                 embed_revision, embed_dim, errors_json, duration_ms \
                 FROM kb_index_state WHERE corpus = ?1",
                params![corpus],
            )
            .await?;
        let Some(r) = rows.next().await? else {
            return Ok(None);
        };
        Ok(Some(KbIndexState {
            corpus: r.get(0)?,
            corpus_root: r.get(1)?,
            corpus_commit: r.get(2)?,
            indexed_at: r.get(3)?,
            doc_count: r.get(4)?,
            chunk_count: r.get(5)?,
            card_count: r.get(6)?,
            chunker_version: r.get(7)?,
            lexical_backend: r.get(8)?,
            trust_tier: r.get(9)?,
            embed_model_id: r.get(10)?,
            embed_revision: r.get(11)?,
            embed_dim: r.get(12)?,
            errors_json: r.get(13)?,
            duration_ms: r.get(14)?,
        }))
    }

    /// Storage position of every hash-keyed chunk of `corpus`, ordered by rowid.
    pub async fn library_chunk_positions(
        &self,
        corpus: &str,
    ) -> Result<Vec<LibraryChunkPos>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT c.id, d.rel_path, c.chunk_index, c.chunk_hash \
                 FROM search_document_chunks c JOIN search_documents d ON d.id = c.document_id \
                 WHERE d.corpus = ?1 AND c.chunk_hash IS NOT NULL ORDER BY c.id",
                params![corpus],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(LibraryChunkPos {
                rowid: r.get(0)?,
                rel_path: r.get(1)?,
                chunk_index: r.get(2)?,
                chunk_hash: r.get(3)?,
            });
        }
        Ok(out)
    }

    /// Every chunk of `corpus`, ordered by document path then `chunk_index`.
    pub async fn library_chunk_rows(
        &self,
        corpus: &str,
    ) -> Result<Vec<LibraryChunkRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                &format!(
                    "{CHUNK_SELECT} WHERE d.corpus = ?1 AND c.chunk_hash IS NOT NULL \
                     ORDER BY d.rel_path, c.chunk_index"
                ),
                params![corpus],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(chunk_row(&r)?);
        }
        Ok(out)
    }

    /// The library chunk with this hash, if any. Corpus-filtered: a row that exists only in a
    /// non-library corpus is never returned (T-02-10).
    pub async fn library_chunk_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<LibraryChunkRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                &format!("{CHUNK_SELECT} WHERE d.corpus = ?1 AND c.chunk_hash = ?2 LIMIT 1"),
                params![KB_LIBRARY_CORPUS, hash],
            )
            .await?;
        match rows.next().await? {
            Some(r) => Ok(Some(chunk_row(&r)?)),
            None => Ok(None),
        }
    }

    /// Hydrate library chunks by hash, preserving input order and skipping unknown hashes.
    // ponytail: one query per hash; callers cap at ~50 hits. Batch with IN (...) if that grows.
    pub async fn library_chunks_by_hashes(
        &self,
        hashes: &[String],
    ) -> Result<Vec<LibraryChunkRow>, StoreError> {
        let mut out = Vec::with_capacity(hashes.len());
        for h in hashes {
            out.extend(self.library_chunk_by_hash(h).await?);
        }
        Ok(out)
    }

    /// Metadata of one library document by `rel_path`; `None` for unknown or non-library paths.
    pub async fn library_document_by_path(
        &self,
        rel_path: &str,
    ) -> Result<Option<LibraryDocRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT rel_path, title, frontmatter_json, indexed_commit, index_error, \
                 content_hash FROM search_documents WHERE corpus = ?1 AND rel_path = ?2",
                params![KB_LIBRARY_CORPUS, rel_path],
            )
            .await?;
        let Some(r) = rows.next().await? else {
            return Ok(None);
        };
        Ok(Some(LibraryDocRow {
            rel_path: r.get(0)?,
            title: r.get(1)?,
            frontmatter_json: r.get::<Option<String>>(2)?.unwrap_or_default(),
            indexed_commit: r.get(3)?,
            index_error: r.get(4)?,
            file_hash: r.get(5)?,
        }))
    }

    /// One document's chunks in `chunk_index` order (the card first).
    pub async fn library_doc_chunks(
        &self,
        corpus: &str,
        rel_path: &str,
    ) -> Result<Vec<LibraryChunkRow>, StoreError> {
        let mut rows = self
            .conn
            .query(
                &format!(
                    "{CHUNK_SELECT} WHERE d.corpus = ?1 AND d.rel_path = ?2 \
                     AND c.chunk_hash IS NOT NULL ORDER BY c.chunk_index"
                ),
                params![corpus, rel_path],
            )
            .await?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().await? {
            out.push(chunk_row(&r)?);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VoxDb;

    fn chunk(hash: &str, heading: &str, body: &str) -> LibraryChunkInput {
        LibraryChunkInput {
            chunk_hash: hash.into(),
            chunk_kind: "section".into(),
            heading_path: heading.into(),
            anchor: None,
            token_estimate: 1,
            body: body.into(),
        }
    }

    fn doc(rel_path: &'static str) -> LibraryDocUpsert<'static> {
        LibraryDocUpsert {
            corpus: KB_LIBRARY_CORPUS,
            rel_path,
            title: "Guide",
            file_hash: "f",
            frontmatter_json: "{}",
            indexed_commit: None,
            index_error: None,
            trust_tier: "trusted",
        }
    }

    async fn ids(db: &VoxDb) -> Vec<(i64, String, i64)> {
        let mut rows = db
            .connection()
            .query(
                "SELECT id, chunk_hash, chunk_index FROM search_document_chunks ORDER BY chunk_index",
                (),
            )
            .await
            .unwrap();
        let mut out = Vec::new();
        while let Some(r) = rows.next().await.unwrap() {
            out.push((r.get(0).unwrap(), r.get(1).unwrap(), r.get(2).unwrap()));
        }
        out
    }

    #[test]
    fn trust_tier_wire_accepts_the_four_tiers_only() {
        for t in TRUST_TIER_WIRE {
            validate_trust_tier_wire(t).unwrap();
        }
        assert!(validate_trust_tier_wire("Trusted").is_err());
        assert!(validate_trust_tier_wire("").is_err());
    }

    #[tokio::test]
    async fn library_document_file_hash_is_corpus_scoped() {
        let db = VoxDb::open_memory().await.unwrap();
        db.apply_library_document(&doc("docs/src/guide.md"), &[])
            .await
            .unwrap();
        let got = |c: &'static str, p: &'static str| {
            let db = db.clone();
            async move { db.library_document_file_hash(c, p).await.unwrap() }
        };
        assert_eq!(
            got(KB_LIBRARY_CORPUS, "docs/src/guide.md").await.as_deref(),
            Some("f")
        );
        assert_eq!(got(KB_LIBRARY_CORPUS, "docs/src/nope.md").await, None);
        assert_eq!(got("other", "docs/src/guide.md").await, None);
    }

    fn state(commit: Option<&str>) -> KbIndexState {
        KbIndexState {
            corpus: KB_LIBRARY_CORPUS.into(),
            corpus_root: "/repo".into(),
            corpus_commit: commit.map(str::to_string),
            indexed_at: String::new(),
            doc_count: 2,
            chunk_count: 5,
            card_count: 2,
            chunker_version: "v".into(),
            lexical_backend: "tantivy-0.22".into(),
            trust_tier: "trusted".into(),
            embed_model_id: None,
            embed_revision: None,
            embed_dim: None,
            errors_json: "[]".into(),
            duration_ms: 7,
        }
    }

    #[tokio::test]
    async fn kb_index_state_upserts_and_stamps_indexed_at() {
        let db = VoxDb::open_memory().await.unwrap();
        assert_eq!(
            db.get_kb_index_state(KB_LIBRARY_CORPUS).await.unwrap(),
            None
        );
        db.put_kb_index_state(&state(Some("c1"))).await.unwrap();
        let first = db
            .get_kb_index_state(KB_LIBRARY_CORPUS)
            .await
            .unwrap()
            .unwrap();
        assert!(!first.indexed_at.is_empty(), "store assigns indexed_at");
        assert_eq!(first.corpus_commit.as_deref(), Some("c1"));
        let mut next = state(None);
        next.doc_count = 9;
        db.put_kb_index_state(&next).await.unwrap();
        let second = db
            .get_kb_index_state(KB_LIBRARY_CORPUS)
            .await
            .unwrap()
            .unwrap();
        assert_eq!((second.doc_count, second.corpus_commit), (9, None));
        assert_eq!(db.get_kb_index_state("other").await.unwrap(), None);
    }

    #[tokio::test]
    async fn library_chunk_count_and_prune_documents() {
        let db = VoxDb::open_memory().await.unwrap();
        db.apply_library_document(
            &doc("docs/src/a.md"),
            &[
                chunk(&"a".repeat(64), "A", "x"),
                chunk(&"b".repeat(64), "B", "y"),
            ],
        )
        .await
        .unwrap();
        db.apply_library_document(&doc("docs/src/b.md"), &[chunk(&"c".repeat(64), "C", "z")])
            .await
            .unwrap();
        let mut other = doc("docs/src/o.md");
        other.corpus = "other";
        db.apply_library_document(&other, &[chunk(&"d".repeat(64), "D", "w")])
            .await
            .unwrap();
        assert_eq!(db.library_chunk_count(KB_LIBRARY_CORPUS).await.unwrap(), 3);

        let pruned = db
            .prune_library_documents_not_in(KB_LIBRARY_CORPUS, &["docs/src/a.md".to_string()])
            .await
            .unwrap();
        assert_eq!(pruned, vec!["docs/src/b.md".to_string()]);
        assert_eq!(db.library_chunk_count(KB_LIBRARY_CORPUS).await.unwrap(), 2);
        // The other corpus is never pruned by a library prune.
        assert_eq!(db.library_chunk_count("other").await.unwrap(), 1);
        assert!(
            db.prune_library_documents_not_in(KB_LIBRARY_CORPUS, &["docs/src/a.md".to_string()])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn prune_orphan_kb_chunk_embeddings_keeps_live_and_foreign_vectors() {
        let db = VoxDb::open_memory().await.unwrap();
        let live = "a".repeat(64);
        db.apply_library_document(&doc("docs/src/a.md"), &[chunk(&live, "A", "x")])
            .await
            .unwrap();
        for (ty, id) in [
            (KB_CHUNK_SOURCE_TYPE, live.as_str()),
            (KB_CHUNK_SOURCE_TYPE, "gone"),
            ("memory", "gone"),
        ] {
            db.store_embedding(ty, id, "m", &[1.0], None, None)
                .await
                .unwrap();
        }
        assert_eq!(db.prune_orphan_kb_chunk_embeddings().await.unwrap(), 1);
        assert_eq!(db.prune_orphan_kb_chunk_embeddings().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn library_document_by_path_reads_library_docs_only() {
        let db = VoxDb::open_memory().await.unwrap();
        db.apply_library_document(&doc("docs/src/guide.md"), &[])
            .await
            .unwrap();
        let mut other = doc("docs/src/o.md");
        other.corpus = "other";
        db.apply_library_document(&other, &[]).await.unwrap();
        let row = db
            .library_document_by_path("docs/src/guide.md")
            .await
            .unwrap();
        let row = row.expect("indexed doc");
        assert_eq!(
            (row.title.as_str(), row.frontmatter_json.as_str()),
            ("Guide", "{}")
        );
        assert_eq!(row.index_error, None);
        assert!(
            db.library_document_by_path("docs/src/o.md")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            db.library_document_by_path("docs/src/zzz.md")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn library_doc_chunks_come_back_in_chunk_index_order() {
        let db = VoxDb::open_memory().await.unwrap();
        let d = doc("docs/src/guide.md");
        let (a, b) = (
            chunk(&"a".repeat(64), "A", "1"),
            chunk(&"b".repeat(64), "B", "2"),
        );
        db.apply_library_document(&d, &[b.clone(), a.clone()])
            .await
            .unwrap();
        let got = db
            .library_doc_chunks(KB_LIBRARY_CORPUS, "docs/src/guide.md")
            .await
            .unwrap();
        assert_eq!(
            got.iter()
                .map(|c| c.heading_path.as_str())
                .collect::<Vec<_>>(),
            ["B", "A"]
        );
        assert!(
            db.library_doc_chunks("other", "docs/src/guide.md")
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn library_chunk_by_hash_is_corpus_filtered() {
        let db = VoxDb::open_memory().await.unwrap();
        db.apply_library_document(&doc("docs/src/g.md"), &[chunk(&"a".repeat(64), "A", "x")])
            .await
            .unwrap();
        let mut web = doc("docs/src/w.md");
        web.corpus = "";
        db.apply_library_document(&web, &[chunk(&"b".repeat(64), "W", "web")])
            .await
            .unwrap();
        assert_eq!(
            db.library_chunk_by_hash(&"a".repeat(64))
                .await
                .unwrap()
                .unwrap()
                .body,
            "x"
        );
        assert!(
            db.library_chunk_by_hash(&"b".repeat(64))
                .await
                .unwrap()
                .is_none()
        );
        assert!(db.library_chunk_by_hash("missing").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn library_store_round_trip() {
        let db = VoxDb::open_memory().await.unwrap();
        let (a, b) = (
            chunk(&"a".repeat(64), "Guide", "alpha"),
            chunk(&"b".repeat(64), "Guide › B", "beta"),
        );
        db.apply_library_document(&doc("docs/src/guide.md"), &[a.clone(), b.clone()])
            .await
            .unwrap();
        // A row in another corpus must not leak into library reads.
        let mut other = doc("docs/src/other.md");
        other.corpus = "other";
        db.apply_library_document(&other, &[chunk(&"c".repeat(64), "X", "gamma")])
            .await
            .unwrap();

        let got = db
            .library_chunks_by_hashes(std::slice::from_ref(&a.chunk_hash))
            .await
            .unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(
            (
                got[0].body.as_str(),
                got[0].heading_path.as_str(),
                got[0].doc_path.as_str()
            ),
            ("alpha", "Guide", "docs/src/guide.md")
        );
        let rows = db.library_chunk_rows(KB_LIBRARY_CORPUS).await.unwrap();
        assert_eq!(
            rows.iter()
                .map(|r| r.chunk_hash.clone())
                .collect::<Vec<_>>(),
            vec![a.chunk_hash, b.chunk_hash]
        );
        assert!(
            db.library_chunks_by_hashes(&["c".repeat(64)])
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn library_store_diff_keeps_rowids() {
        let db = VoxDb::open_memory().await.unwrap();
        let (a, b, x) = (
            chunk(&"a".repeat(64), "A", "alpha"),
            chunk(&"b".repeat(64), "B", "beta"),
            chunk(&"x".repeat(64), "X", "new"),
        );
        let d = doc("docs/src/guide.md");
        db.apply_library_document(&d, &[a.clone(), b.clone()])
            .await
            .unwrap();
        let before = ids(&db).await;
        let diff = db
            .apply_library_document(&d, &[a.clone(), x.clone(), b.clone()])
            .await
            .unwrap();
        assert_eq!((diff.inserted, diff.deleted, diff.kept), (1, 0, 2));
        let after = ids(&db).await;
        let find =
            |v: &[(i64, String, i64)], h: &str| v.iter().find(|r| r.1 == h).cloned().unwrap();
        assert_eq!(find(&after, &a.chunk_hash), find(&before, &a.chunk_hash));
        let (b_before, b_after) = (find(&before, &b.chunk_hash), find(&after, &b.chunk_hash));
        assert_eq!(b_before.0, b_after.0, "rowid kept");
        assert_eq!((b_before.2, b_after.2), (1, 2));

        let diff = db
            .apply_library_document(&d, std::slice::from_ref(&b))
            .await
            .unwrap();
        assert_eq!((diff.inserted, diff.deleted, diff.kept), (0, 2, 1));
        assert_eq!(diff.deleted_hashes.len(), 2);
    }

    #[tokio::test]
    async fn library_store_rejects_bad_trust_tier_and_duplicate_hashes() {
        let db = VoxDb::open_memory().await.unwrap();
        let mut d = doc("docs/src/guide.md");
        d.trust_tier = "Trusted";
        assert!(db.apply_library_document(&d, &[]).await.is_err());
        let c = chunk(&"a".repeat(64), "A", "x");
        assert!(
            db.apply_library_document(&doc("docs/src/guide.md"), &[c.clone(), c])
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn kb_library_migration_from_94() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v94.db").to_string_lossy().to_string();
        {
            let raw = turso::Builder::new_local(&path).build().await.unwrap();
            let conn = raw.connect().unwrap();
            conn.execute_batch(
                "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TEXT NOT NULL DEFAULT (datetime('now')));
                 INSERT INTO schema_version (version) VALUES (94);
                 CREATE TABLE search_documents (
                    id INTEGER PRIMARY KEY AUTOINCREMENT, source_uri TEXT NOT NULL,
                    title TEXT NOT NULL DEFAULT '', mime_type TEXT NOT NULL DEFAULT '',
                    content_hash TEXT NOT NULL DEFAULT '',
                    ingested_at TEXT NOT NULL DEFAULT (datetime('now')));
                 CREATE TABLE search_document_chunks (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    document_id INTEGER NOT NULL REFERENCES search_documents(id) ON DELETE CASCADE,
                    chunk_index INTEGER NOT NULL, body_text TEXT NOT NULL, embedding_ref TEXT,
                    created_at TEXT NOT NULL DEFAULT (datetime('now')),
                    UNIQUE(document_id, chunk_index));
                 INSERT INTO search_documents (source_uri, title) VALUES ('old://doc', 'Old');
                 INSERT INTO search_document_chunks (document_id, chunk_index, body_text) VALUES (1, 0, 'legacy body');",
            )
            .await
            .unwrap();
        }
        let db = VoxDb::open(&path).await.unwrap();
        assert_eq!(
            db.schema_version().await.unwrap(),
            crate::schema::BASELINE_VERSION
        );
        let mut rows = db
            .connection()
            .query(
                "SELECT d.corpus, d.rel_path, c.body_text, c.chunk_hash, c.chunk_kind, c.heading_path \
                 FROM search_document_chunks c JOIN search_documents d ON d.id = c.document_id",
                (),
            )
            .await
            .unwrap();
        let r = rows.next().await.unwrap().expect("legacy row survives");
        assert_eq!(r.get::<String>(0).unwrap(), "");
        assert_eq!(r.get::<Option<String>>(1).unwrap(), None);
        assert_eq!(r.get::<String>(2).unwrap(), "legacy body");
        assert_eq!(r.get::<Option<String>>(3).unwrap(), None);
        assert_eq!(r.get::<String>(4).unwrap(), "section");
        assert_eq!(r.get::<String>(5).unwrap(), "");
        drop(rows);
        let mut rows = db
            .connection()
            .query("SELECT COUNT(*) FROM kb_index_state", ())
            .await
            .unwrap();
        assert_eq!(
            rows.next().await.unwrap().unwrap().get::<i64>(0).unwrap(),
            0
        );
        // The migrated database accepts library writes.
        db.apply_library_document(
            &doc("docs/src/guide.md"),
            &[chunk(&"a".repeat(64), "A", "x")],
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn library_chunk_positions_report_rowid_index_hash_and_scope_to_corpus() {
        let db = VoxDb::open_memory().await.unwrap();
        let (a, b) = ("a".repeat(64), "b".repeat(64));
        db.apply_library_document(
            &doc("docs/src/guide.md"),
            &[chunk(&a, "A", "1"), chunk(&b, "B", "2")],
        )
        .await
        .unwrap();
        let mut other = doc("docs/src/o.md");
        other.corpus = "other";
        db.apply_library_document(&other, &[chunk(&"c".repeat(64), "C", "3")])
            .await
            .unwrap();
        let got = db.library_chunk_positions(KB_LIBRARY_CORPUS).await.unwrap();
        let view: Vec<(&str, i64, &str)> = got
            .iter()
            .map(|p| (p.rel_path.as_str(), p.chunk_index, p.chunk_hash.as_str()))
            .collect();
        assert_eq!(
            view,
            vec![
                ("docs/src/guide.md", 0, a.as_str()),
                ("docs/src/guide.md", 1, b.as_str())
            ]
        );
        assert!(got[0].rowid < got[1].rowid);
    }
}
