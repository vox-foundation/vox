//! Research slug helpers. The docs-writing persistence was removed: research output
//! reaches the KB only as pending `kb_findings` (never `docs/src`).

/// Convert a query to a filesystem-safe slug.
///
/// - Lowercase
/// - Non-alphanumerics → '-'
/// - Collapses runs of '-'
/// - Trims leading/trailing '-'
/// - At most 80 chars after dash-trim
/// - Empty → "untitled"
#[must_use]
pub fn slug_from_query(query: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true; // suppress leading '-'
    for c in query.chars() {
        let lower = c.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        return "untitled".to_string();
    }
    if out.len() > 80 {
        out.truncate(80);
        while out.ends_with('-') {
            out.pop();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::slug_from_query;

    #[test]
    fn slug_collapses_punctuation_and_defaults_to_untitled() {
        assert_eq!(slug_from_query("  Hello,  World! "), "hello-world");
        assert_eq!(slug_from_query("!!!"), "untitled");
    }
}
