//! Retrieval planning types and rank fusion for hybrid (vector + keyword) search.
//!
//! These are **pure helpers**; they do not query Turso by themselves. Wire them to your embedding
//! and FTS layers, then merge with [`fuse_hybrid_results`]. Each [`RetrievalResult`] can carry
//! provenance (modalities, query id, claim ids, contradiction hints) that survives fusion.

/// Which retrieval branch produced evidence for a hit (or both after hybrid fusion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RetrievalEvidenceSource {
    /// Caller did not classify the branch (treated as neutral during merge).
    #[default]
    Unknown,
    /// Vector / embedding similarity path.
    Vector,
    /// Keyword or full-text path.
    FullText,
    /// Both vector and keyword paths agreed on this `chunk_id`.
    Hybrid,
}

impl RetrievalEvidenceSource {
    /// Combines evidence from two hits of the same chunk (e.g. vector + FTS overlap).
    pub fn merge(self, other: Self) -> Self {
        use RetrievalEvidenceSource::*;
        match (self, other) {
            (Unknown, b) => b,
            (a, Unknown) => a,
            (Vector, Vector) => Vector,
            (FullText, FullText) => FullText,
            (Hybrid, _) | (_, Hybrid) => Hybrid,
            (Vector, FullText) | (FullText, Vector) => Hybrid,
        }
    }
}

/// How a [`RetrievalQuery`] should be interpreted upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMode {
    /// Embedding-only retrieval.
    Vector,
    /// Keyword / FTS-only retrieval.
    FullText,
    /// Run both branches and merge (see [`fuse_hybrid_results`]).
    Hybrid,
}

/// Inputs for a retrieval request (caller maps to concrete SQL or RPC).
#[derive(Debug, Clone)]
pub struct RetrievalQuery {
    /// User or agent query string.
    pub query_text: String,
    /// Retrieval branch selection for the caller’s pipeline.
    pub mode: RetrievalMode,
    /// Maximum hits to return after fusion.
    pub top_k: usize,
    /// Drop hits below this score (caller-defined scale).
    pub min_score: f32,
}

impl Default for RetrievalQuery {
    fn default() -> Self {
        Self {
            query_text: String::new(),
            mode: RetrievalMode::Hybrid,
            top_k: 8,
            min_score: 0.0,
        }
    }
}

/// One ranked chunk for provenance or UI (not a full DB row).
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    /// Stable id for deduplication across modalities (e.g. chunk primary key).
    pub chunk_id: String,
    /// Document or URI label for attribution.
    pub source: String,
    /// Fused relevance score (higher is better).
    pub score: f32,
    /// Short excerpt for display or LLM context.
    pub snippet: String,
    /// Which branch(es) supported this hit after optional caller hints + fusion.
    pub evidence_source: RetrievalEvidenceSource,
    /// Wall-clock ms when this hit was retrieved (best-effort; fusion keeps the max).
    pub retrieved_at_ms: Option<u64>,
    /// Opaque id tying this hit to a single retrieval request (fusion clears on conflict).
    pub query_id: Option<String>,
    /// Optional claim / span ids for downstream fact-checking or citation graphs.
    pub supporting_claim_ids: Vec<String>,
    /// Human-readable hints when two sources disagree (caller- or postprocessor-filled).
    pub contradiction_hints: Vec<String>,
}

impl RetrievalResult {
    /// Normalizes branch metadata before inserting into the fusion map from the vector list.
    pub fn normalized_for_vector_branch(mut self) -> Self {
        self.evidence_source = self.evidence_source.merge(RetrievalEvidenceSource::Vector);
        self
    }

    /// Normalizes branch metadata before inserting into the fusion map from the FTS list.
    pub fn normalized_for_text_branch(mut self) -> Self {
        self.evidence_source = self
            .evidence_source
            .merge(RetrievalEvidenceSource::FullText);
        self
    }
}

fn merge_retrieval_provenance(into: &mut RetrievalResult, from: &RetrievalResult) {
    into.evidence_source = into.evidence_source.merge(from.evidence_source);
    into.retrieved_at_ms = match (into.retrieved_at_ms, from.retrieved_at_ms) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    into.query_id = match (&into.query_id, &from.query_id) {
        (Some(a), Some(b)) if a == b => Some(a.clone()),
        (Some(a), None) => Some(a.clone()),
        (None, Some(b)) => Some(b.clone()),
        (None, None) => None,
        _ => None,
    };
    for id in &from.supporting_claim_ids {
        if !into.supporting_claim_ids.contains(id) {
            into.supporting_claim_ids.push(id.clone());
        }
    }
    for h in &from.contradiction_hints {
        if !into.contradiction_hints.contains(h) {
            into.contradiction_hints.push(h.clone());
        }
    }
}

/// Merge vector and full-text hit lists by `chunk_id`, re-scoring overlaps with `vector_weight`.
///
/// - Chunks only in `vector_hits` keep their vector score (with vector-branch provenance).
/// - Chunks only in `text_hits` keep their text score (with FTS-branch provenance).
/// - Chunks in both get `existing * vector_weight + hit * (1 - vector_weight)` and merged
///   provenance (`merge_retrieval_provenance` in this module).
pub fn fuse_hybrid_results(
    vector_hits: &[RetrievalResult],
    text_hits: &[RetrievalResult],
    vector_weight: f32,
) -> Vec<RetrievalResult> {
    let mut merged: std::collections::HashMap<String, RetrievalResult> =
        std::collections::HashMap::new();
    for hit in vector_hits {
        let h = hit.clone().normalized_for_vector_branch();
        merged.insert(h.chunk_id.clone(), h);
    }
    for hit in text_hits {
        let h = hit.clone().normalized_for_text_branch();
        merged
            .entry(h.chunk_id.clone())
            .and_modify(|existing| {
                existing.score =
                    (existing.score * vector_weight) + (h.score * (1.0 - vector_weight));
                if existing.snippet.is_empty() {
                    existing.snippet = h.snippet.clone();
                }
                merge_retrieval_provenance(existing, &h);
            })
            .or_insert(h);
    }
    let mut out: Vec<_> = merged.into_values().collect();
    out.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_fusion_sorts_by_score() {
        let v = vec![RetrievalResult {
            chunk_id: "a".into(),
            source: "doc".into(),
            score: 0.9,
            snippet: "alpha".into(),
            evidence_source: RetrievalEvidenceSource::Vector,
            retrieved_at_ms: Some(10),
            query_id: Some("q1".into()),
            supporting_claim_ids: vec!["c1".into()],
            contradiction_hints: vec![],
        }];
        let t = vec![RetrievalResult {
            chunk_id: "b".into(),
            source: "doc".into(),
            score: 0.8,
            snippet: "beta".into(),
            evidence_source: RetrievalEvidenceSource::FullText,
            retrieved_at_ms: Some(20),
            query_id: Some("q1".into()),
            supporting_claim_ids: vec![],
            contradiction_hints: vec![],
        }];
        let fused = fuse_hybrid_results(&v, &t, 0.7);
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].chunk_id, "a");
        assert_eq!(fused[0].evidence_source, RetrievalEvidenceSource::Vector);
        assert_eq!(fused[1].evidence_source, RetrievalEvidenceSource::FullText);
    }

    #[test]
    fn hybrid_fusion_merges_provenance_on_overlap() {
        let v = vec![RetrievalResult {
            chunk_id: "x".into(),
            source: "s".into(),
            score: 0.9,
            snippet: "v".into(),
            evidence_source: RetrievalEvidenceSource::Vector,
            retrieved_at_ms: Some(100),
            query_id: Some("same".into()),
            supporting_claim_ids: vec!["a".into()],
            contradiction_hints: vec!["h1".into()],
        }];
        let t = vec![RetrievalResult {
            chunk_id: "x".into(),
            source: "s".into(),
            score: 0.5,
            snippet: String::new(),
            evidence_source: RetrievalEvidenceSource::FullText,
            retrieved_at_ms: Some(200),
            query_id: Some("same".into()),
            supporting_claim_ids: vec!["b".into()],
            contradiction_hints: vec!["h2".into()],
        }];
        let fused = fuse_hybrid_results(&v, &t, 0.6);
        assert_eq!(fused.len(), 1);
        let h = &fused[0];
        assert!((h.score - (0.9_f32 * 0.6 + 0.5 * 0.4)).abs() < 1e-5);
        assert_eq!(h.evidence_source, RetrievalEvidenceSource::Hybrid);
        assert_eq!(h.retrieved_at_ms, Some(200));
        assert_eq!(h.query_id.as_deref(), Some("same"));
        assert!(h.supporting_claim_ids.contains(&"a".to_string()));
        assert!(h.supporting_claim_ids.contains(&"b".to_string()));
        assert!(h.contradiction_hints.contains(&"h1".to_string()));
        assert!(h.contradiction_hints.contains(&"h2".to_string()));
        assert_eq!(h.snippet, "v");
    }

    #[test]
    fn query_id_cleared_on_conflict() {
        let v = vec![RetrievalResult {
            chunk_id: "x".into(),
            source: "s".into(),
            score: 0.8,
            snippet: "".into(),
            evidence_source: RetrievalEvidenceSource::Vector,
            retrieved_at_ms: None,
            query_id: Some("q-a".into()),
            supporting_claim_ids: vec![],
            contradiction_hints: vec![],
        }];
        let t = vec![RetrievalResult {
            chunk_id: "x".into(),
            source: "s".into(),
            score: 0.7,
            snippet: "".into(),
            evidence_source: RetrievalEvidenceSource::FullText,
            retrieved_at_ms: None,
            query_id: Some("q-b".into()),
            supporting_claim_ids: vec![],
            contradiction_hints: vec![],
        }];
        let fused = fuse_hybrid_results(&v, &t, 0.5);
        assert_eq!(fused[0].query_id, None);
    }

    #[test]
    fn evidence_source_merge_matrix() {
        assert_eq!(
            RetrievalEvidenceSource::Vector.merge(RetrievalEvidenceSource::FullText),
            RetrievalEvidenceSource::Hybrid
        );
        assert_eq!(
            RetrievalEvidenceSource::Unknown.merge(RetrievalEvidenceSource::Vector),
            RetrievalEvidenceSource::Vector
        );
    }
}
