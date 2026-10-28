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
use vox_db::{KB_LIBRARY_CORPUS, KbIndexState, LibraryChunkInput, LibraryDocUpsert, VoxDb};

use lexical::{LEXICAL_BACKEND, LibraryLexicalIndex};

pub use vox_kb_facets::Taxonomy;

/// Inputs the indexer cannot derive itself.
pub struct IndexDeps<'a> {
    /// Hex digest of the given bytes (production: `vox_crypto::hex_encode(&secure_hash(b))`).
    pub hasher: &'a (dyn Fn(&[u8]) -> String + Sync),
    pub corpus_commit: Option<String>,
    /// The corpus ceiling, chosen once by the construction site and recorded in `kb_index_state`.
    /// A file is stamped this tier only when [`Self::is_committed_clean`] holds for it.
    pub trust_tier: vox_db::ContextTrustTier,
    /// Per D-08, a corpus-relative path (`docs/src/x.md`) for which this returns false is indexed
    /// Untrusted. Production derives it from git: tracked at HEAD and unmodified.
    pub is_committed_clean: &'a (dyn Fn(&str) -> bool + Sync),
}

/// A file or directory that could not be read or indexed (unreadable directory, bad file), or a
/// run-level note such as a skipped prune; indexing continues past it. Any walk error disables
/// pruning for that run.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct FileError {
    pub path: String,
    pub error: String,
}

#[derive(Debug, Clone, Default)]
pub struct IndexReport {
    pub corpus_commit: Option<String>,
    pub docs_seen: usize,
    pub docs_changed: usize,
    /// Files whose content hash matched the stored one: no DB writes at all.
    pub docs_unchanged: usize,
    /// Walked files whose tier this run is Untrusted (not committed-and-clean), changed or not.
    pub docs_untrusted: usize,
    /// Stored documents no longer present in the tree (deleted or renamed), removed this run.
    pub docs_pruned: usize,
    /// Card chunks in the library after this run (one per document).
    pub cards: usize,
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
pub async fn index_library(
    db: &VoxDb,
    repo_root: &Path,
    lexical_dir: &Path,
    deps: &IndexDeps<'_>,
) -> Result<IndexReport> {
    let started = Instant::now();
    // Before any DB access: a daemon rooted at the wrong repo must fail loudly, not "succeed".
    for root in crate::kb_ids::KB_CORPUS_ROOTS {
        anyhow::ensure!(
            repo_root.join(root).is_dir(),
            "KB corpus root `{root}` not found under {}",
            repo_root.display()
        );
    }
    let mut report = IndexReport {
        corpus_commit: deps.corpus_commit.clone(),
        lexical_backend: LEXICAL_BACKEND.into(),
        ..Default::default()
    };
    // The hash skip is only sound when the stored rows came from this chunker; with no manifest
    // the provenance of existing rows is unknown, so everything is re-chunked.
    let skip_ok = db
        .get_kb_index_state(KB_LIBRARY_CORPUS)
        .await?
        .is_some_and(|s| s.chunker_version == chunk::CHUNKER_VERSION);
    let mut walked: Vec<String> = Vec::new();
    let (files, walk_errors) = corpus::walk_corpus(repo_root);
    let walk_complete = walk_errors.is_empty();
    report.errors.extend(walk_errors);
    for file in files {
        report.docs_seen += 1;
        // Every walked path is kept, errored or not: a transient read error never prunes a doc.
        walked.push(file.rel_path.clone());
        // D-08: the tier is decided for every walked file on every run, changed or not.
        let tier = if (deps.is_committed_clean)(&file.rel_path) {
            deps.trust_tier
        } else {
            vox_db::ContextTrustTier::Untrusted
        };
        if tier == vox_db::ContextTrustTier::Untrusted {
            report.docs_untrusted += 1;
        }
        let raw = match vox_bounded_fs::read_utf8_path_capped(&file.abs) {
            Ok(s) => s,
            Err(e) => {
                // Content we cannot read now is not vouched for (fail-closed; no-op without a row).
                db.set_library_document_tier(
                    KB_LIBRARY_CORPUS,
                    &file.rel_path,
                    vox_db::ContextTrustTier::Untrusted,
                )
                .await?;
                if tier != vox_db::ContextTrustTier::Untrusted {
                    report.docs_untrusted += 1;
                }
                report.errors.push(FileError {
                    path: file.rel_path,
                    error: format!("{e:#}"),
                });
                continue;
            }
        };
        let text = vox_bounded_fs::normalize_text(raw);
        let file_hash = (deps.hasher)(text.as_bytes());
        if skip_ok
            && db
                .library_document_file_hash(KB_LIBRARY_CORPUS, &file.rel_path)
                .await?
                .as_deref()
                == Some(file_hash.as_str())
        {
            db.set_library_document_tier(KB_LIBRARY_CORPUS, &file.rel_path, tier)
                .await?;
            report.docs_unchanged += 1;
            continue;
        }
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
            trust_tier: tier,
        };
        let diff = db.apply_library_document(&doc, &inputs).await?;
        report.chunks_inserted += diff.inserted;
        report.chunks_deleted += diff.deleted;
        report.chunks_kept += diff.kept;
        if diff.inserted + diff.deleted > 0 {
            report.docs_changed += 1;
        }
    }

    // Prune only on a complete walk that is not suspiciously empty: a partial or empty walk is
    // not "everything else was deleted" (CR-01).
    let prune_safe = walk_complete
        && (report.docs_seen > 0 || db.library_chunk_count(KB_LIBRARY_CORPUS).await? == 0);
    if prune_safe {
        report.docs_pruned = db
            .prune_library_documents_not_in(KB_LIBRARY_CORPUS, &walked)
            .await?
            .len();
    } else {
        report.errors.push(FileError {
            path: crate::kb_ids::KB_CORPUS_ROOTS.join(", "),
            error: "prune skipped: the corpus walk was incomplete or found no files while Library \
                    chunks are stored"
                .into(),
        });
    }
    db.prune_orphan_kb_chunk_embeddings().await?;

    let rows = db.library_chunk_rows(KB_LIBRARY_CORPUS).await?;
    let current = LibraryLexicalIndex::open(lexical_dir)
        .ok()
        .and_then(|i| i.num_docs().ok());
    report.cards = rows.iter().filter(|r| r.chunk_kind == "card").count();
    let rows_len = rows.len();
    let doc_count = rows
        .iter()
        .map(|r| r.doc_path.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len();
    let rebuild =
        report.docs_changed > 0 || report.docs_pruned > 0 || current != Some(rows.len() as u64);
    report.lexical_docs = if rebuild {
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
    db.put_kb_index_state(&KbIndexState {
        corpus: KB_LIBRARY_CORPUS.into(),
        corpus_root: repo_root.to_string_lossy().into_owned(),
        corpus_commit: deps.corpus_commit.clone(),
        indexed_at: String::new(), // assigned by the store
        doc_count: doc_count as i64,
        chunk_count: rows_len as i64,
        card_count: report.cards as i64,
        chunker_version: chunk::CHUNKER_VERSION.into(),
        lexical_backend: LEXICAL_BACKEND.into(),
        trust_tier: deps.trust_tier,
        // AP-07: lexical-only, no embedder in this phase.
        embed_model_id: None,
        embed_revision: None,
        embed_dim: None,
        errors_json: serde_json::to_string(&report.errors)?,
        duration_ms: report.duration_ms as i64,
    })
    .await?;
    Ok(report)
}

/// What `vox kb status` reports: the last run's manifest plus a live consistency check.
#[derive(Debug, Clone)]
pub struct LibraryStatus {
    pub state: Option<KbIndexState>,
    pub db_chunk_count: i64,
    /// Documents in the Tantivy index, `None` when it cannot be opened.
    pub lexical_docs: Option<u64>,
    /// Manifest present, index opens, counts agree, and the chunker version is current.
    pub consistent: bool,
}

/// Read the manifest and compare DB rows with the Tantivy index at `lexical_dir`.
pub async fn status(db: &VoxDb, lexical_dir: &Path) -> Result<LibraryStatus> {
    let state = db.get_kb_index_state(KB_LIBRARY_CORPUS).await?;
    let db_chunk_count = db.library_chunk_count(KB_LIBRARY_CORPUS).await?;
    let lexical_docs = LibraryLexicalIndex::open(lexical_dir)
        .ok()
        .and_then(|i| i.num_docs().ok());
    let consistent = state
        .as_ref()
        .is_some_and(|s| s.chunker_version == chunk::CHUNKER_VERSION)
        && lexical_docs == Some(db_chunk_count as u64);
    Ok(LibraryStatus {
        state,
        db_chunk_count,
        lexical_docs,
        consistent,
    })
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

/// Load the committed KB taxonomy (`contracts/documentation/kb-taxonomy.v1.yaml`) from `repo_root`.
pub fn load_taxonomy(repo_root: &Path) -> Result<Taxonomy> {
    let path = repo_root.join(vox_kb_facets::TAXONOMY_REL);
    let text = vox_bounded_fs::read_utf8_path_capped(&path)
        .with_context(|| format!("read {}", path.display()))?;
    Taxonomy::from_yaml_str(&text)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .with_context(|| format!("parse {}", path.display()))
}

/// BM25 search over the library index; hits are hydrated from the DB in Tantivy order.
pub async fn search(
    db: &VoxDb,
    lexical_dir: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<LibraryHit>> {
    search_with_taxonomy(db, lexical_dir, query, limit, None).await
}

/// [`search`] with query-time synonym expansion: an altLabel or deprecated alias in `query`
/// appends a quoted facet phrase that matches only cards tagged with the concept's prefLabel.
/// `None` passes the query to Tantivy untouched.
pub async fn search_with_taxonomy(
    db: &VoxDb,
    lexical_dir: &Path,
    query: &str,
    limit: usize,
    taxonomy: Option<&Taxonomy>,
) -> Result<Vec<LibraryHit>> {
    let expanded = taxonomy.map(|t| t.expand_query(query));
    let query = expanded.as_deref().unwrap_or(query);
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

/// Compute the deterministic status/superseded/staleness ranking multiplier (`RET-04`):
/// `research | roadmap | legacy | deprecated` and `superseded_by` pages are down-weighted (`0.65`)
/// relative to `current` (`1.0`) while remaining in the result set.
#[must_use]
pub fn rank_multiplier_for_doc_frontmatter(frontmatter_json: &str, is_stale: bool) -> f32 {
    let parsed: serde_json::Value =
        serde_json::from_str(frontmatter_json).unwrap_or_else(|_| serde_json::json!({}));
    let status = parsed
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("current")
        .trim()
        .to_ascii_lowercase();
    let superseded = parsed
        .get("superseded_by")
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty());

    let mut mult = match status.as_str() {
        "research" | "roadmap" | "legacy" | "deprecated" | "historical" | "archived" => 0.65_f32,
        _ => 1.0_f32,
    };
    if superseded {
        mult *= 0.65_f32;
    }
    if is_stale {
        mult *= 0.85_f32;
    }
    mult
}

/// Hybrid RRF + local contextual dense + term-density reranked library search (`EMB-01..04`, `RET-04`).
/// Falls back cleanly to lexical-only when `limit == 0` or query is empty.
pub async fn search_hybrid_with_taxonomy(
    db: &VoxDb,
    lexical_dir: &Path,
    query: &str,
    limit: usize,
    taxonomy: Option<&Taxonomy>,
) -> Result<Vec<LibraryHit>> {
    let fetch_k = limit.saturating_mul(3).max(20);
    let mut hits = search_with_taxonomy(db, lexical_dir, query, fetch_k, taxonomy).await?;
    if hits.is_empty() {
        return Ok(hits);
    }

    let model_spec = format!(
        "{}@{}",
        crate::embedding_env::DEFAULT_LOCAL_EMBED_MODEL,
        crate::embedding_env::DEFAULT_LOCAL_EMBED_REVISION
    );
    let space = crate::embeddings::VectorSpaceKey::from_model_spec(
        &model_spec,
        vox_actor_runtime::llm::LOCAL_EMBED_DEFAULT_DIM,
    );
    let q_vec = vox_actor_runtime::llm::local_embed_text(query, &model_spec);

    for (rank_idx, hit) in hits.iter_mut().enumerate() {
        let ctx_text =
            chunk::format_chunk_for_embedding(&hit.title, &hit.heading_path, None, &hit.snippet);
        let d_vec = vox_actor_runtime::llm::local_embed_text(&ctx_text, &model_spec);
        let dense_sim =
            crate::embeddings::cosine_similarity_checked(&space, &q_vec, &space, &d_vec)
                .unwrap_or(0.0)
                .max(0.0);
        let rrf_lex = 1.0_f32 / (60.0 + (rank_idx + 1) as f32);
        hit.score = hit.score * 0.7 + (rrf_lex + dense_sim * 0.3);
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.chunk_hash.cmp(&b.chunk_hash))
    });
    hits.truncate(limit);
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hasher(b: &[u8]) -> String {
        vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
    }

    #[test]
    fn downweights_research_roadmap_legacy_and_superseded_docs() {
        let cur = rank_multiplier_for_doc_frontmatter(r#"{"status":"current"}"#, false);
        let res = rank_multiplier_for_doc_frontmatter(r#"{"status":"research"}"#, false);
        let leg = rank_multiplier_for_doc_frontmatter(r#"{"status":"legacy"}"#, false);
        let sup = rank_multiplier_for_doc_frontmatter(
            r#"{"status":"current","superseded_by":"docs/src/new.md"}"#,
            false,
        );
        assert!(res < cur);
        assert!(leg < cur);
        assert!(sup < cur);
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
            trust_tier: vox_db::ContextTrustTier::Trusted,
            is_committed_clean: &|_: &str| true,
        };
        let report = index_library(&db, repo.path(), &lex, &deps).await.unwrap();
        assert_eq!(report.lexical_backend, LEXICAL_BACKEND);
        let hits = search(&db, &lex, "quokkaflux", 3).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].heading_path, "A › Sec");
        assert_eq!(hits[0].body_bytes, "## Sec\nquokkaflux lives here".len());

        let hybrid_hits = search_hybrid_with_taxonomy(&db, &lex, "quokkaflux", 3, None)
            .await
            .unwrap();
        assert_eq!(hybrid_hits.len(), 1);
    }
}
