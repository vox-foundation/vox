//! Word-overlap scores for `vox research eval` sample records.
//!
//! These compare a generated answer's wording with a gold answer or with evidence
//! text. They are NOT retrieval metrics (no ranking, no relevance judgments); the
//! ID-based IR metrics live in `vox_search::kb_eval`.

/// Fraction of the gold answer's distinct words longer than 3 chars that also occur
/// in the model answer. An empty gold word set is trivially satisfied (`1.0`).
pub(crate) fn answer_token_overlap(model_answer: &str, gold_answer: &str) -> f64 {
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

/// Fraction of the answer's sentences (longer than 10 chars) with at least one of
/// their first five words longer than 4 chars found anywhere in the evidence text.
/// Empty answer or empty evidence is `0.0`; an answer with no long sentences is
/// vacuously `1.0`.
pub(crate) fn answer_keyword_hit_rate(model_answer: &str, evidence_snippets: &[String]) -> f64 {
    if model_answer.is_empty() {
        return 0.0;
    }
    if evidence_snippets.is_empty() {
        return 0.0;
    }

    let evidence_corpus = evidence_snippets.join(" ").to_lowercase();
    let model_clusters: Vec<_> = model_answer
        .split('.')
        .filter(|s| s.trim().len() > 10)
        .collect();

    if model_clusters.is_empty() {
        return 1.0;
    }

    let mut grounded_count = 0;
    for cluster in &model_clusters {
        let keywords: Vec<_> = cluster
            .split_whitespace()
            .filter(|s| s.len() > 4)
            .take(5)
            .collect();

        if keywords
            .iter()
            .any(|k| evidence_corpus.contains(&k.to_lowercase()))
        {
            grounded_count += 1;
        }
    }

    grounded_count as f64 / model_clusters.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_gold_returns_one() {
        // Catches: division by zero when gold_words is empty
        assert_eq!(answer_token_overlap("model answer here", ""), 1.0);
    }

    #[test]
    fn all_short_tokens_are_trivially_satisfied() {
        // Catches: the >3-char filter dropping every token and yielding 0
        assert_eq!(answer_token_overlap("at of the in", "at of the in"), 1.0);
    }

    #[test]
    fn overlap_is_positive_for_shared_words() {
        let o = answer_token_overlap(
            "Rust memory safety is guaranteed by ownership.",
            "Rust memory safety ownership borrowing",
        );
        // gold words >3: rust, memory, safety, ownership, borrowing; 4 of 5 shared
        assert!((o - 0.8).abs() < 1e-12, "{o}");
    }

    #[test]
    fn empty_model_answer_hits_nothing() {
        assert_eq!(
            answer_keyword_hit_rate("", &["some evidence".to_string()]),
            0.0
        );
    }

    #[test]
    fn empty_evidence_hits_nothing() {
        assert_eq!(
            answer_keyword_hit_rate("The answer is found here.", &[]),
            0.0
        );
    }

    #[test]
    fn no_long_sentences_is_vacuously_one() {
        // Both sentences are <=10 chars, so there is nothing to check.
        let g = answer_keyword_hit_rate("Hi. Ok.", &["irrelevant evidence".to_string()]);
        assert_eq!(g, 1.0);
    }
}
