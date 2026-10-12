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

/// Fraction of the answer's sentences (longer than 10 chars) grounded by a single
/// evidence sentence: that sentence must contain at least half of the clause's first
/// five non-filler words longer than 4 chars and share its negation polarity.
/// Empty answer, empty evidence, or an answer with no long sentences is `0.0`
/// (a vacuous answer is not grounded).
pub(crate) fn answer_keyword_hit_rate(model_answer: &str, evidence_snippets: &[String]) -> f64 {
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

/// Function words long enough to pass the keyword length filter but carrying no claim.
/// "without" is deliberately absent: it changes meaning.
const FILLER_WORDS: &[&str] = &[
    "about", "above", "after", "again", "among", "because", "before", "being", "below", "between",
    "could", "during", "every", "might", "other", "shall", "should", "since", "still", "their",
    "there", "these", "those", "though", "through", "under", "until", "where", "which", "while",
    "whose", "would",
];

/// A clause is grounded by one snippet sentence that contains at least half of its
/// leading keywords and shares its negation polarity.
fn cluster_grounded(cluster: &str, snippets: &[String]) -> bool {
    let lower = cluster.to_lowercase();
    let keywords: Vec<&str> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() > 4 && !FILLER_WORDS.contains(w))
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
    fn vacuous_answer_is_not_grounded() {
        // Catches: an answer with no checkable clause (all sentences <=10 chars) auto-passing as 1.0
        let g = answer_keyword_hit_rate("Hi. Ok.", &["irrelevant evidence".to_string()]);
        assert_eq!(g, 0.0);
    }

    #[test]
    fn rejects_negation_flip() {
        // Catches: "X does not support Y" counting as grounded by evidence that says "X supports Y"
        let g = answer_keyword_hit_rate(
            "Rust does not guarantee memory safety without unsafe blocks.",
            &["Rust guarantees memory safety without unsafe blocks.".to_string()],
        );
        assert_eq!(g, 0.0);
    }

    #[test]
    fn rejects_negation_flip_in_multi_sentence_snippet() {
        // Catches: polarity read from the whole snippet, where an unrelated sentence has "not"
        let g = answer_keyword_hit_rate(
            "Rust does not guarantee memory safety without unsafe blocks.",
            &[
                "Rust guarantees memory safety without unsafe blocks. It is not garbage collected."
                    .to_string(),
            ],
        );
        assert_eq!(g, 0.0);
    }

    #[test]
    fn accepts_claim_despite_unrelated_negation_in_snippet() {
        // Catches: a supported claim rejected because another sentence in the snippet has "not"
        let g = answer_keyword_hit_rate(
            "Rust guarantees memory safety without unsafe blocks.",
            &["Rust guarantees memory safety without unsafe blocks. It does not use a garbage collector."
                .to_string()],
        );
        assert_eq!(g, 1.0);
    }

    #[test]
    fn ignores_punctuation_and_citation_markers() {
        // Catches: "tokio," and "schedules[1]" failing to match supporting evidence
        let g = answer_keyword_hit_rate(
            "Tokio, schedules[1], lightweight[2] tasks.",
            &["Tokio schedules lightweight tasks cooperatively.".to_string()],
        );
        assert_eq!(g, 1.0);
    }

    #[test]
    fn ignores_filler_words() {
        // Catches: a fabricated claim grounded by shared function words alone
        let g = answer_keyword_hit_rate(
            "Their findings about which there would be quantum entanglement.",
            &["Their study, which ran there, would say nothing about cats.".to_string()],
        );
        assert_eq!(g, 0.0);
    }

    #[test]
    fn rejects_single_keyword_overlap() {
        // Catches: one shared keyword anywhere in the evidence grounding an unrelated claim
        let g = answer_keyword_hit_rate(
            "Photosynthesis converts sunlight into chemical energy stored in glucose.",
            &["The chemical industry uses many reagents.".to_string()],
        );
        assert_eq!(g, 0.0);
    }

    #[test]
    fn accepts_supported_sentence() {
        let g = answer_keyword_hit_rate(
            "Rust guarantees memory safety without garbage collection.",
            &["Rust guarantees memory safety without a garbage collection runtime.".to_string()],
        );
        assert_eq!(g, 1.0);
    }
}
