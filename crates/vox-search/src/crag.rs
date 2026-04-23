use crate::memory_hybrid::HybridSearchHit;
use regex::Regex;
use std::collections::HashSet;

/// Logic for Corrective RAG (CRAG) expansion and multi-hop query generation.
pub struct CragRouter;

impl CragRouter {
    /// Generates refined sub-queries based on initial research hits and the original query.
    /// Used for iterative multi-hop research loops.
    pub fn expand_queries_from_partial_evidence(
        original_query: &str,
        hits: &[HybridSearchHit],
    ) -> Vec<String> {
        let mut refined_queries = Vec::new();
        let mut seen_concepts = HashSet::new();

        // 1. Extract proper nouns or capitalized technical terms from snippets
        // This is a heuristic until Lane G (Research Expert) takes over.
        let concept_regex = Regex::new(r"([A-Z][a-z]{3,}(?:\s[A-Z][a-z]{3,})*)").unwrap();

        for hit in hits.iter().take(5) {
            for cap in concept_regex.captures_iter(&hit.content_snippet) {
                let concept = cap[1].to_string();
                if !original_query
                    .to_lowercase()
                    .contains(&concept.to_lowercase())
                    && seen_concepts.insert(concept.clone())
                {
                    refined_queries.push(format!("{} {}", original_query, concept));
                }
                if refined_queries.len() >= 3 {
                    break;
                }
            }
            if refined_queries.len() >= 3 {
                break;
            }
        }

        // 2. If no new concepts found, try to ask for "latest" or "comparison"
        if refined_queries.is_empty() {
            refined_queries.push(format!("{} latest developments", original_query));
            refined_queries.push(format!("{} detailed comparison", original_query));
        }

        refined_queries.truncate(3);
        refined_queries
    }

    /// Determines if a research pass should continue based on quality score.
    pub fn should_continue(current_quality: f64, target_quality: f64, hops_remaining: u8) -> bool {
        hops_remaining > 0 && current_quality < target_quality
    }
}
