//! Lightweight offline fixtures for planner / fusion regression (expand into CI benches).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// One labeled query used for recall / routing checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchBenchmarkQuery {
    pub id: String,
    pub query: String,
    /// Intent name as produced by `heuristic_search_plan` debug formatting.
    pub expected_intent: String,
}

/// Minimal static suite bundled with the crate (expand via `include_str!` data files later).
#[must_use]
pub fn default_doc_nav_queries() -> Vec<SearchBenchmarkQuery> {
    vec![
        SearchBenchmarkQuery {
            id: "codex_where_defined".into(),
            query: "where is MemorySearchEngine defined in crates".into(),
            expected_intent: "codenavigation".into(),
        },
        SearchBenchmarkQuery {
            id: "architecture_overview".into(),
            query: "how does repository architecture work overview".into(),
            expected_intent: "repostructure".into(),
        },
    ]
}

/// Aggregate report from a local evaluation run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchEvalReport {
    pub policy_version: u32,
    pub intent_matches: usize,
    pub intent_total: usize,
    pub per_query: HashMap<String, bool>,
}

/// Simple entity-based recall against a reference answer.
pub fn calculate_recall_at_5(model_answer: &str, gold_answer: &str) -> f64 {
    let model_lower = model_answer.to_lowercase();
    let model_words: std::collections::HashSet<_> = model_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 3)
        .collect();

    let gold_lower = gold_answer.to_lowercase();
    let gold_words: std::collections::HashSet<_> = gold_lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 3)
        .collect();

    if gold_words.is_empty() {
        return 1.0;
    }

    let intersection = model_words.intersection(&gold_words).count();
    intersection as f64 / gold_words.len() as f64
}

/// Naive groundedness check: ensures majority of model answer clusters match evidence snippets.
pub fn calculate_groundedness(model_answer: &str, evidence_snippets: &[String]) -> f64 {
    if model_answer.is_empty() {
        return 0.0;
    }
    if evidence_snippets.is_empty() {
        return 0.0;
    }

    let snippets: Vec<String> = evidence_snippets.iter().map(|s| s.to_lowercase()).collect();
    let model_clusters: Vec<_> = model_answer
        .split('.')
        .filter(|s| s.trim().len() > 10)
        .collect();

    if model_clusters.is_empty() {
        return 0.0;
    }

    let grounded_count = model_clusters
        .iter()
        .filter(|cluster| cluster_grounded(cluster, &snippets))
        .count();

    grounded_count as f64 / model_clusters.len() as f64
}

/// A clause is grounded by one snippet sentence that contains at least half of its
/// leading keywords and shares its negation polarity.
fn cluster_grounded(cluster: &str, snippets: &[String]) -> bool {
    let lower = cluster.to_lowercase();
    let keywords: Vec<&str> = lower
        .split_whitespace()
        .filter(|s| s.len() > 4)
        .take(5)
        .collect();
    if keywords.is_empty() {
        return false;
    }
    let needed = keywords.len().div_ceil(2);
    let negated = has_negation(&lower);
    snippets
        .iter()
        .flat_map(|snippet| snippet.split(['.', '!', '?', '\n']))
        .any(|sentence| {
            has_negation(sentence) == negated
                && keywords.iter().filter(|k| sentence.contains(**k)).count() >= needed
        })
}

fn has_negation(text: &str) -> bool {
    text.split(|c: char| !c.is_alphanumeric() && c != '\'')
        .any(|w| matches!(w, "not" | "no" | "never" | "cannot") || w.ends_with("n't"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_db::heuristic_search_plan;

    #[test]
    fn default_benchmark_queries_track_planner_intent() {
        for q in default_doc_nav_queries() {
            let plan = heuristic_search_plan(&q.query, false, None);
            let got = format!("{:?}", plan.intent).to_ascii_lowercase();
            assert_eq!(got, q.expected_intent, "benchmark query id={}", q.id);
        }
    }
}
