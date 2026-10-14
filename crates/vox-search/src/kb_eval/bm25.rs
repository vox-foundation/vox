//! Deterministic in-memory BM25 baseline over Phase-1 chunks.
//!
//! // ponytail: in-memory BM25 over Phase-1 chunks; Phase 2 replaces it with the IDX-07 lexical leg and must not regress doc-level metrics.
//!
//! Constants and idf mirror `memory_hybrid` (k1 = 1.2, b = 0.75,
//! `idf = ln(1 + (N - df + 0.5) / (df + 0.5))`). Only ordered collections are used and the
//! chunk size is pinned to [`DEFAULT_MARKDOWN_CHUNK_BYTES`] (never env-derived).

use std::collections::{BTreeMap, BTreeSet};

use super::RankedChunk;
use super::golden::CorpusDoc;
use crate::ingest::{DEFAULT_MARKDOWN_CHUNK_BYTES, chunk_markdown_sections_with_size};
use crate::kb_ids::{HEADING_PATH_SEP, chunk_id, doc_title, is_fence_line, normalize_heading_text};
use crate::memory_hybrid::tokenize;

const K1: f64 = 1.2;
const B: f64 = 0.75;

/// One indexed chunk of a doc.
#[derive(Debug, Clone)]
pub struct Chunk {
    pub id: String,
    pub doc: String,
    pub heading_path: String,
    pub body: String,
    pub bytes: usize,
}

/// Chunk a doc with the pinned Phase-1 chunker and assign heading-path chunk ids.
///
/// The chunker splits at lines starting `# ` / `## `. A `# ` chunk (or any chunk before the
/// first heading) gets the root path (the title); `## X` gets `title › X`; a continuation
/// chunk inherits the previous heading_path. A `## ` line inside a fenced block is not a
/// heading, so it also inherits.
#[must_use]
pub fn chunk_doc(doc_path: &str, text: &str) -> Vec<Chunk> {
    let title = doc_title(doc_path, text);
    let mut heading_path = title.clone();
    let mut in_fence = false;
    let mut out = Vec::new();
    for body in chunk_markdown_sections_with_size(text, DEFAULT_MARKDOWN_CHUNK_BYTES) {
        let first = body.lines().next().unwrap_or("");
        if !in_fence {
            if first.starts_with("# ") {
                heading_path = title.clone();
            } else if first.starts_with("## ") {
                heading_path =
                    format!("{title}{HEADING_PATH_SEP}{}", normalize_heading_text(first));
            }
        }
        for line in body.lines() {
            if is_fence_line(line) {
                in_fence = !in_fence;
            }
        }
        out.push(Chunk {
            id: chunk_id(doc_path, &heading_path),
            doc: doc_path.to_string(),
            heading_path: heading_path.clone(),
            bytes: body.len(),
            body,
        });
    }
    out
}

/// BM25 index over every chunk of every corpus doc (docs taken in `_id` order).
#[derive(Debug)]
pub struct Bm25Index {
    chunks: Vec<Chunk>,
    lens: Vec<u32>,
    avg_len: f64,
    /// term to (chunk index, term frequency), ascending chunk index.
    postings: BTreeMap<String, Vec<(u32, u32)>>,
}

impl Bm25Index {
    #[must_use]
    pub fn build(docs: &[CorpusDoc]) -> Self {
        let mut sorted: Vec<&CorpusDoc> = docs.iter().collect();
        sorted.sort_by(|a, b| a.id.cmp(&b.id));
        let mut chunks = Vec::new();
        for d in sorted {
            chunks.extend(chunk_doc(&d.id, &d.text));
        }
        let mut postings: BTreeMap<String, Vec<(u32, u32)>> = BTreeMap::new();
        let mut lens = Vec::with_capacity(chunks.len());
        for (ci, c) in chunks.iter().enumerate() {
            let toks = tokenize(&c.body);
            lens.push(toks.len() as u32);
            let mut tf: BTreeMap<&str, u32> = BTreeMap::new();
            for t in &toks {
                *tf.entry(t.as_str()).or_insert(0) += 1;
            }
            for (t, n) in tf {
                postings
                    .entry(t.to_string())
                    .or_default()
                    .push((ci as u32, n));
            }
        }
        let total: u64 = lens.iter().map(|&l| u64::from(l)).sum();
        let avg_len = if chunks.is_empty() {
            0.0
        } else {
            total as f64 / chunks.len() as f64
        };
        Self {
            chunks,
            lens,
            avg_len,
            postings,
        }
    }

    /// Indexed chunk ids per doc (read-only; used for chunk-qrel resolution).
    #[must_use]
    pub fn doc_chunk_ids(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut m: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for c in &self.chunks {
            m.entry(c.doc.clone()).or_default().insert(c.id.clone());
        }
        m
    }

    /// Every scored chunk, best first: `(quantized score desc, chunk id asc, ordinal asc)`.
    #[must_use]
    pub fn rank(&self, query: &str) -> Vec<RankedChunk> {
        self.rank_scored(query)
            .into_iter()
            .map(|(_, c)| c)
            .collect()
    }

    /// [`Self::rank`] with each chunk's BM25 score (quantized to 1e-9, as used for ordering).
    #[must_use]
    pub fn rank_scored(&self, query: &str) -> Vec<(f64, RankedChunk)> {
        if self.avg_len == 0.0 {
            return Vec::new();
        }
        let terms: BTreeSet<String> = tokenize(query).into_iter().collect();
        let n = self.chunks.len() as f64;
        let mut scores = vec![0.0f64; self.chunks.len()];
        for t in &terms {
            let Some(post) = self.postings.get(t) else {
                continue;
            };
            let df = post.len() as f64;
            let idf = f64::ln(1.0 + (n - df + 0.5) / (df + 0.5));
            for &(ci, tf) in post {
                let tf = f64::from(tf);
                let norm = 1.0 - B + B * (f64::from(self.lens[ci as usize]) / self.avg_len);
                scores[ci as usize] += idf * (tf * (K1 + 1.0)) / (tf + K1 * norm);
            }
        }
        let mut hits: Vec<(i64, usize)> = scores
            .iter()
            .enumerate()
            .filter(|(_, s)| **s > 0.0)
            .map(|(ci, s)| ((s * 1e9).round() as i64, ci))
            .collect();
        hits.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| self.chunks[a.1].id.cmp(&self.chunks[b.1].id))
                .then_with(|| a.1.cmp(&b.1))
        });
        hits.into_iter()
            .map(|(q, ci)| {
                let c = RankedChunk {
                    id: self.chunks[ci].id.clone(),
                    bytes: self.chunks[ci].bytes,
                };
                (q as f64 / 1e9, c)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_doc_assigns_heading_paths_and_ignores_fenced_headings() {
        let text = "---\ntitle: Alpha\n---\nintro\n## Install\nzephyrine\n```\n## Fake\n```\n## Usage\nx\n";
        let ids: Vec<String> = chunk_doc("docs/src/a.md", text)
            .into_iter()
            .map(|c| c.id)
            .collect();
        // The chunker splits at "## Fake" too, but that chunk inherits "Install".
        assert_eq!(
            ids,
            [
                "docs/src/a.md#Alpha",
                "docs/src/a.md#Alpha › Install",
                "docs/src/a.md#Alpha › Install",
                "docs/src/a.md#Alpha › Usage",
            ]
        );
    }

    #[test]
    fn rank_is_best_first_and_stable() {
        let docs = vec![
            CorpusDoc {
                id: "docs/src/a.md".into(),
                title: "a".into(),
                text: "# A\nzephyrine installer\n".into(),
            },
            CorpusDoc {
                id: "docs/src/b.md".into(),
                title: "b".into(),
                text: "# B\nunrelated words\n".into(),
            },
        ];
        let idx = Bm25Index::build(&docs);
        let r = idx.rank("zephyrine installer");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "docs/src/a.md#A");
        assert!(idx.rank("nomatchterm").is_empty());
        let scored = idx.rank_scored("zephyrine installer");
        assert_eq!(scored.len(), 1);
        assert!(scored[0].0 > 0.0 && scored[0].1.id == r[0].id);
    }
}
