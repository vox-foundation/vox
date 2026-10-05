//! Library-index retriever for the KB eval (AP-24).
//!
//! The frozen `corpus.jsonl` snapshot is materialized into a caller-supplied work dir, indexed
//! with [`crate::library::index_library`] into an in-memory DB plus a fresh Tantivy index (a
//! fresh build every time keeps the run deterministic), and every query is pre-ranked through
//! [`crate::library::search`]. Chunk ids come from [`crate::kb_ids::chunk_id`], the same
//! identity the Phase 1 qrels use, so no qrel needs translating.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

use anyhow::{Context, Result, anyhow, bail};
use vox_db::{KB_LIBRARY_CORPUS, VoxDb};

use super::golden::{CorpusDoc, GoldenSet};
use super::{RankedChunk, Report, run_eval};
use crate::kb_ids::chunk_id;
use crate::library::chunk::CHUNKER_VERSION;
use crate::library::{IndexDeps, index_library, search};

/// Retriever name recorded in reports and the baseline.
pub const LIBRARY_RETRIEVER: &str = "library-tantivy-v1";

/// Library hits fetched per query; deep enough for 10 distinct docs after chunk-id dedupe.
pub const EVAL_DEPTH: usize = 100;

/// Pre-ranked Library runs for a fixed query set, plus the indexed chunk ids per doc.
#[derive(Debug)]
pub struct LibraryRetriever {
    /// Query text to deduped `(score, chunk)` pairs, best first.
    runs: BTreeMap<String, Vec<(f32, RankedChunk)>>,
    doc_chunks: BTreeMap<String, BTreeSet<String>>,
}

impl LibraryRetriever {
    /// Index `docs` under `work_dir` and rank each of `queries` to `depth` Library hits.
    /// `hasher` is the injected content-hash facade (this crate takes no crypto dependency).
    pub async fn build(
        docs: &[CorpusDoc],
        queries: &[String],
        depth: usize,
        work_dir: &Path,
        hasher: &(dyn Fn(&[u8]) -> String + Sync),
    ) -> Result<Self> {
        let tree = work_dir.join("tree");
        for d in docs {
            // The id is a repo-relative path from a committed file; still never write outside tree.
            let rel = Path::new(&d.id);
            if rel.is_absolute() || rel.components().any(|c| !matches!(c, Component::Normal(_))) {
                bail!("corpus _id {:?} is not a plain relative path", d.id);
            }
            let dest = tree.join(rel);
            std::fs::create_dir_all(dest.parent().context("doc path has a parent")?)?;
            std::fs::write(&dest, &d.text).with_context(|| format!("write {}", dest.display()))?;
        }
        let lexical = work_dir.join("lex");
        let db = VoxDb::open_memory().await?;
        let deps = IndexDeps {
            hasher,
            corpus_commit: None,
            trust_tier: "trusted",
        };
        let report = index_library(&db, &tree, &lexical, &deps).await?;
        if let Some(e) = report.errors.first() {
            bail!("library index failed on {}: {}", e.path, e.error);
        }
        let mut doc_chunks: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for r in db.library_chunk_rows(KB_LIBRARY_CORPUS).await? {
            doc_chunks
                .entry(r.doc_path.clone())
                .or_default()
                .insert(chunk_id(&r.doc_path, &r.heading_path));
        }
        let mut runs = BTreeMap::new();
        for q in queries {
            if runs.contains_key(q) {
                continue;
            }
            let mut seen = BTreeSet::new();
            let ranked = search(&db, &lexical, q, depth)
                .await?
                .into_iter()
                .map(|h| {
                    let id = chunk_id(&h.doc_path, &h.heading_path);
                    (
                        h.score,
                        RankedChunk {
                            id,
                            bytes: h.body_bytes,
                        },
                    )
                })
                // Parts of an oversize section and the root card share an id: keep the best.
                .filter(|(_, c)| seen.insert(c.id.clone()))
                .collect();
            runs.insert(q.clone(), ranked);
        }
        Ok(Self { runs, doc_chunks })
    }

    /// [`Self::build`] from sync code. Runs on its own thread with its own runtime so it is safe
    /// to call from inside an existing tokio runtime (where `block_on` would panic).
    pub fn build_blocking(
        docs: &[CorpusDoc],
        queries: &[String],
        depth: usize,
        work_dir: &Path,
        hasher: &(dyn Fn(&[u8]) -> String + Sync),
    ) -> Result<Self> {
        std::thread::scope(|s| {
            s.spawn(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(Self::build(docs, queries, depth, work_dir, hasher))
            })
            .join()
            .map_err(|_| anyhow!("library retriever build panicked"))?
        })
    }

    /// Indexed chunk ids per doc, for chunk-qrel resolution.
    #[must_use]
    pub fn doc_chunk_ids(&self) -> &BTreeMap<String, BTreeSet<String>> {
        &self.doc_chunks
    }

    /// Ranked chunks for a pre-ranked query; empty for an unknown query.
    #[must_use]
    pub fn rank(&self, query: &str) -> Vec<RankedChunk> {
        self.rank_scored(query)
            .into_iter()
            .map(|(_, c)| c)
            .collect()
    }

    /// [`Self::rank`] with the Tantivy BM25 score of each chunk.
    #[must_use]
    pub fn rank_scored(&self, query: &str) -> Vec<(f32, RankedChunk)> {
        self.runs.get(query).cloned().unwrap_or_default()
    }
}

/// `run_eval` over a Library retriever; the report names the Library retriever and chunker.
pub fn evaluate(golden: &GoldenSet, lib: &LibraryRetriever) -> Result<Report> {
    let mut report = run_eval(golden, lib.doc_chunk_ids(), |q| lib.rank(q))?;
    report.retriever = LIBRARY_RETRIEVER;
    report.chunker = CHUNKER_VERSION;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hasher(b: &[u8]) -> String {
        vox_crypto::hex_encode(&vox_crypto::secure_hash(b))
    }

    #[test]
    fn corpus_ids_cannot_escape_the_work_tree() {
        let work = tempfile::tempdir().unwrap();
        for bad in ["../evil.md", "/abs.md", "docs/../../evil.md"] {
            let doc = CorpusDoc {
                id: bad.into(),
                title: "t".into(),
                text: "# T\n".into(),
            };
            let err = LibraryRetriever::build_blocking(&[doc], &[], 10, work.path(), &hasher)
                .unwrap_err();
            assert!(format!("{err:#}").contains("plain relative path"), "{bad}");
        }
        assert!(!work.path().join("../evil.md").exists());
    }

    #[test]
    fn unknown_query_ranks_empty_and_known_query_dedupes_ids() {
        let work = tempfile::tempdir().unwrap();
        let doc = CorpusDoc {
            id: "docs/src/a.md".into(),
            title: "A".into(),
            text: "# A\n## Sec\nquokkaflux lives here\n".into(),
        };
        let q = "quokkaflux".to_string();
        let lib = LibraryRetriever::build_blocking(&[doc], &[q.clone()], 10, work.path(), &hasher)
            .unwrap();
        assert!(lib.rank("never asked").is_empty());
        let ids: Vec<String> = lib.rank(&q).into_iter().map(|c| c.id).collect();
        assert_eq!(ids.len(), ids.iter().collect::<BTreeSet<_>>().len());
        assert_eq!(ids[0], "docs/src/a.md#A › Sec");
    }
}
