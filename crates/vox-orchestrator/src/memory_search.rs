//! Hybrid BM25 + optional vector search over markdown memory docs in Codex.
//!
//! [`MemorySearchEngine`] indexes local `.md` trees and can blend in embedding
//! similarity when a [`VoxDb`](vox_db::VoxDb) handle is attached. Vector and BM25
//! legs are merged with [`vox_db::fuse_hybrid_results`] so provenance and scores
//! stay consistent with the shared retrieval SSOT.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use vox_db::{RetrievalEvidenceSource, RetrievalResult, VoxDb, fuse_hybrid_results};

/// Weight applied to the vector leg in [`fuse_hybrid_results`] (remainder is BM25).
const MEMORY_VECTOR_FUSION_WEIGHT: f32 = 0.55;

fn unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Matches from the search engine.
#[derive(Debug, Clone)]
pub struct HybridSearchHit {
    /// Repository-relative or virtual path of the hit.
    pub path: String,
    /// Document title parsed from front matter or first heading.
    pub title: String,
    /// Short excerpt around the best matching span.
    pub content_snippet: String,
    /// Combined relevance score (higher is better; BM25 + optional vector weight).
    pub score: f64,
    /// How this hit was produced (`bm25`, `vector`, fusion notes).
    pub provenance: Vec<String>,
    /// Heuristic: multiple strong sources disagree on the same entity (best-effort).
    pub potential_contradiction: bool,
}

/// Tokenize text into alphanumeric lowercase words.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

/// Simple BM25 scoring configuration.
const K1: f64 = 1.2;
const B: f64 = 0.75;

struct IndexedDocument {
    path: String,
    title: String,
    content: String,
    term_freq: HashMap<String, usize>,
    length: usize,
}

/// Search engine combining local file BM25 and DB vector search.
pub struct MemorySearchEngine {
    docs: Vec<IndexedDocument>,
    /// Path (as indexed) → position in `docs` for O(1) lookup during hybrid merge.
    doc_index_by_path: HashMap<String, usize>,
    avg_doc_len: f64,
    df: HashMap<String, usize>, // Document frequency
    total_docs: usize,
    /// DB for vector searches (schema V7 `embeddings` or similar table).
    db: Option<Arc<VoxDb>>,
}

impl Default for MemorySearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MemorySearchEngine {
    /// Empty index with no database-backed vector leg.
    pub fn new() -> Self {
        Self {
            docs: Vec::new(),
            doc_index_by_path: HashMap::new(),
            avg_doc_len: 0.0,
            df: HashMap::new(),
            total_docs: 0,
            db: None,
        }
    }

    /// Enables embedding-backed recall against the `embeddings` table (schema V7+).
    pub fn with_db(mut self, db: Arc<VoxDb>) -> Self {
        self.db = Some(db);
        self
    }

    /// Recursively index all markdown files in a directory.
    pub fn index_dir(&mut self, dir: &Path) {
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                self.index_dir(&path);
            } else if path.extension().unwrap_or_default() == "md" {
                self.index_file(&path);
            }
        }
        self.recompute_stats();
    }

    /// Index a single file.
    pub fn index_file(&mut self, path: &Path) {
        let content = match crate::bounded_fs::read_utf8_path_capped(path) {
            Ok(c) => c,
            Err(_) => return,
        };

        let file_name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let tokens = tokenize(&content);
        let length = tokens.len();

        let mut term_freq = HashMap::new();
        let mut unique_terms = HashSet::new();

        for t in &tokens {
            *term_freq.entry(t.clone()).or_insert(0) += 1;
            unique_terms.insert(t.clone());
        }

        for t in unique_terms {
            *self.df.entry(t).or_insert(0) += 1;
        }

        self.docs.push(IndexedDocument {
            path: path.to_string_lossy().to_string(),
            title: file_name,
            content,
            term_freq,
            length,
        });

        self.total_docs += 1;
        self.recompute_stats();
    }

    fn recompute_stats(&mut self) {
        if self.total_docs == 0 {
            self.avg_doc_len = 0.0;
            self.doc_index_by_path.clear();
            return;
        }
        let total_len: usize = self.docs.iter().map(|d| d.length).sum();
        self.avg_doc_len = total_len as f64 / self.total_docs as f64;
        self.doc_index_by_path = self
            .docs
            .iter()
            .enumerate()
            .map(|(i, d)| (d.path.clone(), i))
            .collect();
    }

    fn idf(&self, term: &str) -> f64 {
        let n = self.total_docs as f64;
        let df = *self.df.get(term).unwrap_or(&0) as f64;
        // Standard BM25 IDF
        f64::ln(1.0 + (n - df + 0.5) / (df + 0.5))
    }

    /// BM25-ranked document indices (descending score), at most `take` rows.
    fn bm25_ranked_indices(&self, query: &str, take: usize) -> Vec<(usize, f64)> {
        let query_tokens = tokenize(query);
        let mut scores: Vec<(usize, f64)> = Vec::new();

        if self.avg_doc_len == 0.0 {
            return Vec::new();
        }

        for (i, doc) in self.docs.iter().enumerate() {
            let mut score = 0.0;
            for q in &query_tokens {
                let f = *doc.term_freq.get(q).unwrap_or(&0) as f64;
                if f > 0.0 {
                    let idf = self.idf(q);
                    let len_norm = 1.0 - B + B * (doc.length as f64 / self.avg_doc_len);
                    score += idf * (f * (K1 + 1.0)) / (f + K1 * len_norm);
                }
            }
            if score > 0.0 {
                scores.push((i, score));
            }
        }

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(take);
        scores
    }

    fn hits_from_bm25_ranked(
        &self,
        query_tokens: &[String],
        ranked: &[(usize, f64)],
    ) -> Vec<HybridSearchHit> {
        let mut out: Vec<HybridSearchHit> = ranked
            .iter()
            .map(|(i, score)| {
                let doc = &self.docs[*i];
                HybridSearchHit {
                    path: doc.path.clone(),
                    title: doc.title.clone(),
                    content_snippet: Self::extract_snippet(&doc.content, query_tokens),
                    score: *score,
                    provenance: vec![format!("bm25:{}", doc.path)],
                    potential_contradiction: false,
                }
            })
            .collect();
        Self::annotate_contradictions(&mut out);
        out
    }

    /// Execute BM25 search over indexed files.
    pub fn search(&self, query: &str, limit: usize) -> Vec<HybridSearchHit> {
        let ranked = self.bm25_ranked_indices(query, limit);
        let query_tokens = tokenize(query);
        self.hits_from_bm25_ranked(&query_tokens, &ranked)
    }

    fn hit_from_retrieval_result(
        &self,
        r: &RetrievalResult,
        query_tokens: &[String],
    ) -> HybridSearchHit {
        let path = r.chunk_id.clone();
        let (title, snippet) = if let Some(&idx) = self.doc_index_by_path.get(&r.chunk_id) {
            let doc = &self.docs[idx];
            let sn = if r.snippet.is_empty() {
                Self::extract_snippet(&doc.content, query_tokens)
            } else {
                r.snippet.clone()
            };
            (doc.title.clone(), sn)
        } else {
            let stem = Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            (stem, r.snippet.clone())
        };
        let mut provenance = vec![format!("evidence:{:?}", r.evidence_source)];
        if !r.contradiction_hints.is_empty() {
            provenance.extend(r.contradiction_hints.iter().cloned());
        }
        let pot = !r.contradiction_hints.is_empty();
        HybridSearchHit {
            path,
            title,
            content_snippet: snippet,
            score: f64::from(r.score),
            provenance,
            potential_contradiction: pot,
        }
    }

    /// Hybrid search combining BM25 and VoxDB vector search via [`fuse_hybrid_results`].
    pub async fn hybrid_search(
        &self,
        query: &str,
        limit: usize,
        embedding_service: Option<&crate::services::embeddings::EmbeddingService>,
    ) -> Vec<HybridSearchHit> {
        let query_tokens = tokenize(query);
        let bm25_take = limit.saturating_mul(4).max(limit);
        let ranked = self.bm25_ranked_indices(query, bm25_take);
        let bm25_candidates = ranked.len();
        let ts = unix_ms();

        let text_hits: Vec<RetrievalResult> = ranked
            .iter()
            .map(|(i, sc)| {
                let doc = &self.docs[*i];
                RetrievalResult {
                    chunk_id: doc.path.clone(),
                    source: doc.path.clone(),
                    score: *sc as f32,
                    snippet: Self::extract_snippet(&doc.content, &query_tokens),
                    evidence_source: RetrievalEvidenceSource::FullText,
                    retrieved_at_ms: Some(ts),
                    query_id: None,
                    supporting_claim_ids: Vec::new(),
                    contradiction_hints: Vec::new(),
                }
            })
            .collect();

        let mut vector_hits: Vec<RetrievalResult> = Vec::new();
        if let (Some(db_arc), Some(service)) = (&self.db, embedding_service) {
            if let Ok(query_vector) = service.embed_query(query).await {
                if let Ok(db_hits) = db_arc
                    .search_embeddings(&query_vector, None, limit as i64)
                    .await
                {
                    for (entry, dist) in db_hits {
                        let dist_f = f64::from(dist);
                        let similarity = (1.0_f64 - (dist_f / 2.0)).clamp(0.0, 1.0) as f32;
                        let chunk_id = if entry.source_id.is_empty() {
                            format!("embedding:{}", entry.id)
                        } else {
                            entry.source_id.clone()
                        };
                        let snippet = entry
                            .metadata
                            .clone()
                            .unwrap_or_else(|| "No snippet available".to_string());
                        vector_hits.push(RetrievalResult {
                            chunk_id,
                            source: entry.source_id.clone(),
                            score: similarity * 2.0_f32,
                            snippet,
                            evidence_source: RetrievalEvidenceSource::Vector,
                            retrieved_at_ms: Some(ts),
                            query_id: None,
                            supporting_claim_ids: Vec::new(),
                            contradiction_hints: Vec::new(),
                        });
                    }
                }
            }
        }

        let vector_n = vector_hits.len();
        let fused = fuse_hybrid_results(&vector_hits, &text_hits, MEMORY_VECTOR_FUSION_WEIGHT);
        let mut out: Vec<HybridSearchHit> = fused
            .into_iter()
            .take(limit)
            .map(|r| self.hit_from_retrieval_result(&r, &query_tokens))
            .collect();
        Self::annotate_contradictions(&mut out);

        if let Some(db_arc) = &self.db {
            let contra = out.iter().filter(|h| h.potential_contradiction).count();
            let top = out.first().map(|h| h.score);
            if let Err(e) = db_arc
                .record_memory_hybrid_retrieval(
                    query,
                    bm25_candidates,
                    vector_n,
                    out.len(),
                    contra,
                    top,
                )
                .await
            {
                tracing::warn!(error = %e, "memory hybrid retrieval telemetry failed");
            }
        }

        out
    }

    /// If two top hits disagree strongly on titling for overlapping tokens, flag for Socrates layers.
    fn annotate_contradictions(hits: &mut [HybridSearchHit]) {
        if hits.len() < 2 {
            return;
        }
        let a = &hits[0];
        let b = &hits[1];
        let title_disagree = a.score > 0.01
            && b.score > 0.01
            && !a.title.eq_ignore_ascii_case(&b.title)
            && tokenize(&a.title)
                .iter()
                .any(|t| t.len() > 3 && b.title.to_lowercase().contains(t.as_str()));
        if title_disagree {
            hits[0].potential_contradiction |= true;
            hits[1].potential_contradiction |= true;
        }
    }

    /// Extract a contextual snippet around the most frequent query terms.
    fn extract_snippet(content: &str, query_tokens: &[String]) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut best_line_idx = 0;
        let mut max_matches = 0;

        for (i, line) in lines.iter().enumerate() {
            let line_lower = line.to_lowercase();
            let matches = query_tokens
                .iter()
                .filter(|q| line_lower.contains(*q))
                .count();
            if matches > max_matches {
                max_matches = matches;
                best_line_idx = i;
            }
        }

        // Return the line with context
        let start = best_line_idx.saturating_sub(1);
        let end = (best_line_idx + 2).min(lines.len());
        lines[start..end].join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn search_adds_bm25_provenance() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("alpha_notes.md");
        fs::write(&p, "alpha beta gamma").expect("write");

        let mut engine = MemorySearchEngine::new();
        engine.index_file(&p);
        let hits = engine.search("alpha", 5);
        assert!(!hits.is_empty());
        assert!(hits[0].provenance.iter().any(|s| s.starts_with("bm25:")));
    }

    #[test]
    fn contradiction_flag_marks_top_hits_with_overlapping_topic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p1 = dir.path().join("alpha_claim.md");
        let p2 = dir.path().join("alpha_counterclaim.md");
        fs::write(&p1, "alpha evidence says yes").expect("write p1");
        fs::write(&p2, "alpha evidence says no").expect("write p2");

        let mut engine = MemorySearchEngine::new();
        engine.index_file(&p1);
        engine.index_file(&p2);
        let hits = engine.search("alpha evidence", 2);
        assert_eq!(hits.len(), 2);
        assert!(hits[0].potential_contradiction);
        assert!(hits[1].potential_contradiction);
    }
}
