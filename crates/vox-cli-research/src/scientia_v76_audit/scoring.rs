//! Honest multi-hop stage, span, and bridge-entity scoring (`AUDT-01`).
//!
//! Replaces the pre-v7.6 hardcoded `1.0` stub in `crates/vox-cli-research/src/eval.rs`
//! with deterministic span coverage, subquery stage progression, and bridge-entity
//! recall across answer + evidence snippets.

use vox_db::SearchBackend;

const MULTI_HOP_CONNECTORS: &[&str] = &[
    "compare",
    "trace",
    "then",
    "bridge",
    "versus",
    "after",
    "frames-style",
    "browsecomp-style",
];

/// Returns `true` when `query` contains multi-hop reasoning connectors.
#[must_use]
pub fn is_multi_hop_query(query: &str) -> bool {
    let lower = query.to_ascii_lowercase();
    MULTI_HOP_CONNECTORS.iter().any(|kw| lower.contains(kw))
}

/// Fraction of `bridge_entities` (case-insensitive substring) grounded in BOTH
/// `model_answer` and at least one entry of `evidence_snippets`.
#[must_use]
pub fn bridge_entity_recall(
    bridge_entities: &[String],
    model_answer: &str,
    evidence_snippets: &[String],
) -> f64 {
    let non_empty: Vec<String> = bridge_entities
        .iter()
        .map(|e| e.trim().to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect();
    if non_empty.is_empty() {
        return 0.0;
    }
    let answer_lower = model_answer.to_ascii_lowercase();
    let snippets_lower: Vec<String> = evidence_snippets
        .iter()
        .map(|s| s.to_ascii_lowercase())
        .collect();
    let matched = non_empty
        .iter()
        .filter(|entity| {
            answer_lower.contains(entity.as_str())
                && snippets_lower.iter().any(|s| s.contains(entity.as_str()))
        })
        .count();
    (matched as f64 / non_empty.len() as f64).clamp(0.0, 1.0)
}

/// Score multi-hop pipeline execution honesty across stage count, source span, and
/// optional bridge-entity grounding.
///
/// Never returns a hardcoded `1.0` for 0-source or empty-answer runs.
#[must_use]
pub fn multi_hop_pipeline_score(
    query: &str,
    subquery_count: usize,
    sources_count: usize,
    bridge_entities: &[String],
    model_answer: &str,
    evidence_snippets: &[String],
) -> f64 {
    if sources_count == 0 || model_answer.trim().is_empty() {
        return 0.0;
    }
    let has_bridges = bridge_entities.iter().any(|e| !e.trim().is_empty());
    let required_hops = if has_bridges || is_multi_hop_query(query) {
        2usize
    } else {
        1usize
    };
    let required_sources = if required_hops >= 2 { 3usize } else { 1usize };

    let effective_subqueries =
        subquery_count.max(usize::from(required_hops == 1 && sources_count > 0));
    let stage_score = (effective_subqueries as f64 / required_hops as f64).clamp(0.0, 1.0);
    let span_score = (sources_count as f64 / required_sources as f64).clamp(0.0, 1.0);

    if has_bridges {
        let bridge_recall = bridge_entity_recall(bridge_entities, model_answer, evidence_snippets);
        ((stage_score + span_score + bridge_recall) / 3.0).clamp(0.0, 1.0)
    } else {
        ((stage_score + span_score) / 2.0).clamp(0.0, 1.0)
    }
}

/// Score backend and source coverage without the pre-v7.6 `1.0` stub on empty runs.
#[must_use]
pub fn multi_hop_completion_score(
    query: &str,
    backend_mix: &[SearchBackend],
    web_lines: &[String],
) -> f64 {
    if backend_mix.is_empty() || web_lines.is_empty() {
        return 0.0;
    }
    let multi_hop = is_multi_hop_query(query);
    let required_backends = if multi_hop { 2.0 } else { 1.0 };
    let required_sources = if multi_hop { 3.0 } else { 1.0 };
    let backend_score = (backend_mix.len() as f64 / required_backends).clamp(0.0, 1.0);
    let source_score = (web_lines.len() as f64 / required_sources).clamp(0.0, 1.0);
    ((backend_score + source_score) / 2.0).clamp(0.0, 1.0)
}
