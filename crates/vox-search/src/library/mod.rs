//! KB Library index: `docs/src` Markdown -> [`crate::kb_ids`] sections -> hash-keyed vox-db rows
//! -> an indexer-owned Tantivy index -> ranked [`LibraryHit`]s.
//!
//! The content hash only arrives through [`IndexDeps::hasher`] (callers inject the
//! `vox-crypto` BLAKE3 facade), so this crate takes no crypto dependency.

pub mod chunk;
pub mod corpus;
pub mod lexical;

use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use vox_db::{KB_LIBRARY_CORPUS, LibraryChunkInput, LibraryDocUpsert, VoxDb};

use lexical::{LEXICAL_BACKEND, LibraryLexicalIndex};

/// Inputs the indexer cannot derive itself.
pub struct IndexDeps<'a> {
    /// Hex digest of the given bytes (production: `vox_crypto::hex_encode(&secure_hash(b))`).
    pub hasher: &'a (dyn Fn(&[u8]) -> String + Sync),
    pub corpus_commit: Option<String>,
    /// One of the `ContextTrustTier` wire strings; validated by the store.
    pub trust_tier: &'a str,
}

/// A file that could not be read or indexed; indexing continues past it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Default)]
pub struct IndexReport {
    pub corpus_commit: Option<String>,
    pub docs_seen: usize,
    pub docs_changed: usize,
    pub chunks_inserted: usize,
    pub chunks_deleted: usize,
    pub chunks_kept: usize,
    pub errors: Vec<FileError>,
    pub lexical_backend: String,
    pub lexical_docs: u64,
    pub duration_ms: u64,
}

/// Index every corpus file under `repo_root` into `db`, then rebuild the Tantivy index in
/// `lexical_dir` when anything changed, the index is missing, or its count diverges from the DB
/// (AP-16). Each document is one DB transaction, never held across the Tantivy commit.
// ponytail: files absent from the tree are not pruned here (02-04); no unchanged-file skip yet.
pub async fn index_library(
    db: &VoxDb,
    repo_root: &Path,
    lexical_dir: &Path,
    deps: &IndexDeps<'_>,
) -> Result<IndexReport> {
    let started = Instant::now();
    let mut report = IndexReport {
        corpus_commit: deps.corpus_commit.clone(),
        lexical_backend: LEXICAL_BACKEND.into(),
        ..Default::default()
    };
    for file in corpus::walk_corpus(repo_root) {
        report.docs_seen += 1;
        let raw = match vox_bounded_fs::read_utf8_path_capped(&file.abs) {
            Ok(s) => s,
            Err(e) => {
                report.errors.push(FileError {
                    path: file.rel_path,
                    error: format!("{e:#}"),
                });
                continue;
            }
        };
        let text = vox_bounded_fs::normalize_text(raw);
        let file_hash = (deps.hasher)(text.as_bytes());
        let parsed = chunk::chunk_document(&file.rel_path, &text, deps.hasher);
        let inputs: Vec<LibraryChunkInput> = parsed
            .chunks
            .iter()
            .map(|c| LibraryChunkInput {
                chunk_hash: c.hash.clone(),
                chunk_kind: c.kind.as_str().into(),
                heading_path: c.heading_path.clone(),
                anchor: c.anchor.clone(),
                token_estimate: c.token_estimate as i64,
                body: c.body.clone(),
            })
            .collect();
        let doc = LibraryDocUpsert {
            corpus: KB_LIBRARY_CORPUS,
            rel_path: &file.rel_path,
            title: &parsed.title,
            file_hash: &file_hash,
            frontmatter_json: &parsed.frontmatter_json,
            indexed_commit: deps.corpus_commit.as_deref(),
            index_error: None,
            trust_tier: deps.trust_tier,
        };
        let diff = db.apply_library_document(&doc, &inputs).await?;
        report.chunks_inserted += diff.inserted;
        report.chunks_deleted += diff.deleted;
        report.chunks_kept += diff.kept;
        if diff.inserted + diff.deleted > 0 {
            report.docs_changed += 1;
        }
    }

    let rows = db.library_chunk_rows(KB_LIBRARY_CORPUS).await?;
    let current = LibraryLexicalIndex::open(lexical_dir)
        .ok()
        .and_then(|i| i.num_docs().ok());
    report.lexical_docs = if report.docs_changed > 0 || current != Some(rows.len() as u64) {
        let dir = lexical_dir.to_path_buf();
        let idx = tokio::task::spawn_blocking(move || {
            LibraryLexicalIndex::rebuild_from_rows(&dir, &rows)
        })
        .await
        .context("library index rebuild task")??;
        idx.num_docs()?
    } else {
        rows.len() as u64
    };
    report.duration_ms = started.elapsed().as_millis() as u64;
    Ok(report)
}

/// One ranked library hit.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryHit {
    pub chunk_hash: String,
    pub doc_path: String,
    pub title: String,
    pub heading_path: String,
    pub anchor: Option<String>,
    pub chunk_kind: String,
    /// First ~300 characters of the chunk body (cut on a char boundary).
    pub snippet: String,
    pub body_bytes: usize,
    pub score: f32,
}

/// BM25 search over the library index; hits are hydrated from the DB in Tantivy order.
pub async fn search(
    db: &VoxDb,
    lexical_dir: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<LibraryHit>> {
    let ranked = LibraryLexicalIndex::open(lexical_dir)?.search(query, limit)?;
    let hashes: Vec<String> = ranked.iter().map(|(h, _)| h.clone()).collect();
    let rows = db.library_chunks_by_hashes(&hashes).await?;
    let scores: std::collections::HashMap<&str, f32> =
        ranked.iter().map(|(h, s)| (h.as_str(), *s)).collect();
    Ok(rows
        .into_iter()
        .map(|r| LibraryHit {
            score: scores
                .get(r.chunk_hash.as_str())
                .copied()
                .unwrap_or_default(),
            snippet: r.body.chars().take(300).collect(),
            body_bytes: r.body.len(),
            chunk_hash: r.chunk_hash,
            doc_path: r.doc_path,
            title: r.title,
            heading_path: r.heading_path,
            anchor: r.anchor,
            chunk_kind: r.chunk_kind,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hasher(b: &[u8]) -> String {
        vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
    }

    #[tokio::test]
    async fn index_then_search_round_trip_and_missing_index_errors() {
        let repo = tempfile::tempdir().unwrap();
        let doc = repo.path().join("docs/src/a.md");
        std::fs::create_dir_all(doc.parent().unwrap()).unwrap();
        std::fs::write(&doc, "# A\n## Sec\nquokkaflux lives here\n").unwrap();
        let lex = repo.path().join("lex");
        let db = VoxDb::open_memory().await.unwrap();

        assert!(
            search(&db, &lex, "quokkaflux", 3).await.is_err(),
            "no index yet"
        );

        let deps = IndexDeps {
            hasher: &hasher,
            corpus_commit: None,
            trust_tier: "trusted",
        };
        let report = index_library(&db, repo.path(), &lex, &deps).await.unwrap();
        assert_eq!(report.lexical_backend, LEXICAL_BACKEND);
        let hits = search(&db, &lex, "quokkaflux", 3).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].heading_path, "A › Sec");
        assert_eq!(hits[0].body_bytes, "## Sec\nquokkaflux lives here".len());

        // A bad trust tier is rejected by the store and surfaces as an error.
        let bad = IndexDeps {
            trust_tier: "Trusted",
            ..deps
        };
        assert!(index_library(&db, repo.path(), &lex, &bad).await.is_err());
    }
}
