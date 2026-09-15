use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidatePassage {
    pub id: String,
    pub text: String,
    pub base_score: f64,
}

pub fn score_passage_term_density(query_terms: &[&str], passage_text: &str) -> f64 {
    let lower_passage = passage_text.to_lowercase();
    let mut matches = 0usize;
    let mut total_occurrences = 0usize;

    for term in query_terms {
        let lower_term = term.to_lowercase();
        if lower_term.len() < 2 {
            continue;
        }
        let count = lower_passage.matches(&lower_term).count();
        if count > 0 {
            matches += 1;
            total_occurrences += count;
        }
    }

    if query_terms.is_empty() || matches == 0 {
        return 0.0;
    }

    let coverage_ratio = matches as f64 / query_terms.len() as f64;
    let frequency_boost = (total_occurrences as f64).min(5.0) / 5.0 * 0.2;
    (coverage_ratio * 0.8 + frequency_boost).min(1.0)
}

pub fn rerank_passages(
    query: &str,
    passages: &[CandidatePassage],
    top_k: usize,
) -> Vec<CandidatePassage> {
    let query_terms: Vec<&str> = query.split_whitespace().collect();
    let mut scored: Vec<CandidatePassage> = passages
        .iter()
        .map(|p| {
            let term_score = score_passage_term_density(&query_terms, &p.text);
            let combined = p.base_score * 0.3 + term_score * 0.7;
            CandidatePassage {
                id: p.id.clone(),
                text: p.text.clone(),
                base_score: combined,
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.base_score
            .partial_cmp(&a.base_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.into_iter().take(top_k).collect()
}
