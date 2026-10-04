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
