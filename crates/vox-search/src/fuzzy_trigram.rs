//! 3-mode weighted fuzzy & trigram search (`exact_match`, `words`, `fuzzy`) + markdown link
//! URL stripping + in-thread match indexing (`T3-RULES-SEARCH-03`).

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::LazyLock;

/// Minimum relevance threshold (`0.3`).
pub const MIN_MATCH_SCORE_THRESHOLD: f64 = 0.3;
/// Title match boost factor (`1.4x`).
pub const TITLE_MATCH_BOOST: f64 = 1.4;

static MD_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\([^)]+\)").expect("valid markdown link regex"));

/// Strips markdown link target URLs `[label](url)` -> `label` so URLs do not pollute fuzzy scores (`T3-RULES-SEARCH-03`).
#[must_use]
pub fn strip_markdown_link_urls(text: &str) -> String {
    MD_LINK_RE.replace_all(text, "$1").into_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuzzySearchMode {
    ExactMatch,
    Words,
    Fuzzy,
}

fn char_trigrams(s: &str) -> HashSet<String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() < 3 {
        let mut set = HashSet::new();
        if !s.is_empty() {
            set.insert(s.to_string());
        }
        return set;
    }
    chars
        .windows(3)
        .map(|w| w.iter().collect::<String>())
        .collect()
}

fn trigram_jaccard(a: &str, b: &str) -> f64 {
    let ta = char_trigrams(a);
    let tb = char_trigrams(b);
    if ta.is_empty() || tb.is_empty() {
        return 0.0;
    }
    let inter = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    if union == 0.0 { 0.0 } else { inter / union }
}

/// Computes weighted relevance score across `ExactMatch`, `Words`, or `Fuzzy` modes,
/// applying `1.4x` title boost and filtering below `min_score = 0.3` (`T3-RULES-SEARCH-03`).
#[must_use]
pub fn calculate_match_score(
    query: &str,
    title: &str,
    body: &str,
    mode: FuzzySearchMode,
) -> Option<f64> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return None;
    }
    let clean_title = strip_markdown_link_urls(title).to_ascii_lowercase();
    let clean_body = strip_markdown_link_urls(body).to_ascii_lowercase();

    let score_field = |field: &str| -> f64 {
        if field.is_empty() {
            return 0.0;
        }
        let exact = if field.contains(&q) { 1.0 } else { 0.0 };
        let q_words: Vec<&str> = q.split_whitespace().collect();
        let word_hits = if q_words.is_empty() {
            0.0
        } else {
            let matched = q_words.iter().filter(|w| field.contains(*w)).count();
            (matched as f64) / (q_words.len() as f64)
        };
        let trigram = trigram_jaccard(&q, field);
        match mode {
            FuzzySearchMode::ExactMatch => exact,
            FuzzySearchMode::Words => 0.7 * word_hits + 0.3 * exact,
            FuzzySearchMode::Fuzzy => (0.5 * exact + 0.35 * word_hits + 0.45 * trigram).min(1.0),
        }
    };

    let title_score = score_field(&clean_title) * TITLE_MATCH_BOOST;
    let body_score = score_field(&clean_body);
    let combined = title_score.max(body_score);
    if combined >= MIN_MATCH_SCORE_THRESHOLD {
        Some(combined)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InThreadMatchSpan {
    pub message_id: String,
    pub byte_start: usize,
    pub byte_end: usize,
}

/// Finds all case-insensitive substring occurrences across thread messages for `Ctrl+F` navigation (`T3-RULES-SEARCH-03`).
#[must_use]
pub fn find_in_thread_occurrences(
    messages: &[(String, String)],
    needle: &str,
) -> Vec<InThreadMatchSpan> {
    let q = needle.to_ascii_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut hits = Vec::new();
    for (msg_id, text) in messages {
        let lower = text.to_ascii_lowercase();
        let mut offset = 0usize;
        while let Some(pos) = lower[offset..].find(&q) {
            let start = offset + pos;
            let end = start + q.len();
            hits.push(InThreadMatchSpan {
                message_id: msg_id.clone(),
                byte_start: start,
                byte_end: end,
            });
            offset = end;
        }
    }
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markdown_urls_and_applies_title_boost_and_threshold() {
        assert_eq!(
            strip_markdown_link_urls("See [Vox Docs](https://example.com/secret-keyword) now"),
            "See Vox Docs now"
        );
        // Searching for URL-only token does not match stripped body
        assert_eq!(
            calculate_match_score(
                "secret-keyword",
                "Title",
                "See [Vox Docs](https://example.com/secret-keyword) now",
                FuzzySearchMode::ExactMatch
            ),
            None
        );

        let title_hit = calculate_match_score(
            "router consensus",
            "Router Consensus Architecture",
            "Unrelated body",
            FuzzySearchMode::Words,
        )
        .expect("title match");
        let body_hit = calculate_match_score(
            "router consensus",
            "Unrelated Title",
            "Router consensus architecture details",
            FuzzySearchMode::Words,
        )
        .expect("body match");
        assert!(title_hit > body_hit);
    }

    #[test]
    fn finds_in_thread_occurrences_for_ctrl_f_navigation() {
        let msgs = vec![
            ("m1".to_string(), "Hello world, hello Vox".to_string()),
            ("m2".to_string(), "Another HELLO here".to_string()),
        ];
        let spans = find_in_thread_occurrences(&msgs, "hello");
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].message_id, "m1");
        assert_eq!(spans[2].message_id, "m2");
    }
}
