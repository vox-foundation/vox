use std::collections::HashSet;

use super::types::Citation;

/// Extracts word tokens (>= 4 characters, alphanumeric + '_') from text.
fn extract_tokens(text: &str) -> HashSet<String> {
    let mut tokens = HashSet::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_alphanumeric() || c == '_' {
            current.push(c.to_ascii_lowercase());
        } else if !current.is_empty() {
            if current.chars().count() >= 4 {
                tokens.insert(current.clone());
            }
            current.clear();
        }
    }
    if current.chars().count() >= 4 {
        tokens.insert(current);
    }
    tokens
}

/// Correlates a diagnostic or compile error message to the most likely culprit citation
/// using a lightweight token intersection heuristic.
///
/// Extracts word tokens (>= 4 characters, alphanumeric + '_') from `diagnostic` and
/// scores each citation by count of overlapping tokens against `citation.snippet` and `citation.title`.
/// Returns the URL with the highest positive overlap, or `None` if no tokens match.
pub fn correlate_diagnostic_to_citations(
    diagnostic: &str,
    citations: &[Citation],
) -> Option<String> {
    let diag_tokens = extract_tokens(diagnostic);
    if diag_tokens.is_empty() {
        return None;
    }

    let mut best_citation: Option<&Citation> = None;
    let mut max_score = 0;

    for citation in citations {
        let mut citation_tokens = extract_tokens(&citation.snippet);
        citation_tokens.extend(extract_tokens(&citation.title));

        let score = diag_tokens.intersection(&citation_tokens).count();
        if score > max_score {
            max_score = score;
            best_citation = Some(citation);
        }
    }

    if max_score > 0 {
        best_citation.map(|c| c.url.clone())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correlate_diagnostic_to_citations_basic() {
        let citations = vec![
            Citation {
                source_id: 1,
                url: "https://example.com/ok".into(),
                title: "Safe guide".into(),
                snippet: "fn ok_call()".into(),
                confidence: 0.9,
            },
            Citation {
                source_id: 2,
                url: "https://example.com/bad".into(),
                title: "Broken advice".into(),
                snippet: "use deprecated_feature_call() now".into(),
                confidence: 0.5,
            },
        ];

        let diag = "error: unresolved reference `deprecated_feature_call`";
        let res = correlate_diagnostic_to_citations(diag, &citations);
        assert_eq!(res.as_deref(), Some("https://example.com/bad"));
    }
}
