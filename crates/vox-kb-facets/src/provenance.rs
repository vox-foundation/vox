//! Provenance validators for `sources[]` and the verbatim-quote cap (TAX-07).
//! Pure encoded-form checks; hashing stays in `vox-crypto` consumers.

use crate::{
    Source,
    frontmatter::{frontmatter_range, line_spans},
};

/// Maximum words in one verbatim quote. Discretion choice: unit is whitespace-split words; a
/// quote is one contiguous blockquote run outside fences or one `sources[].quote`; the lint
/// applies the cap to `origin: web-promoted` pages only.
pub const MAX_QUOTE_WORDS: usize = 50;
/// Maximum total quoted words on one page (same unit and scope as [`MAX_QUOTE_WORDS`]).
pub const MAX_PAGE_QUOTE_WORDS: usize = 200;

/// True for `blake3:` plus exactly 64 lowercase hex characters (encoded form only).
// vox:defactored-from vox-db-types 2026-10-10
#[must_use]
pub fn is_blake3_content_hash(s: &str) -> bool {
    s.strip_prefix("blake3:")
        .is_some_and(|h| h.len() == 64 && h.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

/// True for a real Gregorian date written exactly `YYYY-MM-DD`.
#[must_use]
pub fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let num = |r: std::ops::Range<usize>| {
        let p = &s[r];
        p.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| p.parse::<u32>().ok())
            .flatten()
    };
    let (Some(y), Some(m), Some(d)) = (num(0..4), num(5..7), num(8..10)) else {
        return false;
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let max = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => 28 + u32::from(leap),
        _ => return false,
    };
    (1..=max).contains(&d)
}

/// Word count of each quote on the page: one entry per contiguous blockquote run in the body
/// (outside fences) plus one per non-empty `sources[].quote`.
// ponytail: only marked quotes are detectable without the source text; unmarked copying needs
// a source-overlap check against the snapshot.
#[must_use]
pub fn quote_word_counts(doc_text: &str, sources: &[Source]) -> Vec<usize> {
    let body = frontmatter_range(doc_text).map_or(0, |r| r.end);
    let mut counts = Vec::new();
    let mut run: Option<usize> = None;
    let mut in_fence = false;
    for (_, _, l) in line_spans(doc_text, body) {
        if l.starts_with("```") || l.starts_with("~~~") {
            in_fence = !in_fence;
        }
        let t = l.trim_start();
        if !in_fence && t.starts_with('>') {
            let text = t.trim_start_matches(|c: char| c == '>' || c.is_whitespace());
            *run.get_or_insert(0) += text.split_whitespace().count();
        } else if let Some(n) = run.take() {
            counts.push(n);
        }
    }
    counts.extend(run);
    counts.extend(
        sources
            .iter()
            .filter_map(|s| s.quote.as_deref())
            .filter(|q| !q.trim().is_empty())
            .map(|q| q.split_whitespace().count()),
    );
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_blake3_content_hash_encoded_form() {
        let h = "a".repeat(64);
        assert!(is_blake3_content_hash(&format!("blake3:{h}")));
        assert!(!is_blake3_content_hash(&format!(
            "blake3:{}",
            "A".repeat(64)
        )));
        assert!(!is_blake3_content_hash(&h));
        assert!(!is_blake3_content_hash(&format!(
            "blake3:{}",
            "a".repeat(63)
        )));
        assert!(!is_blake3_content_hash(&format!(
            "blake3:{}",
            "a".repeat(65)
        )));
    }

    #[test]
    fn is_iso_date_accepts_and_rejects() {
        for ok in ["2026-10-10", "2024-02-29", "2000-02-29"] {
            assert!(is_iso_date(ok), "{ok}");
        }
        for bad in [
            "2023-02-29",
            "2026-13-01",
            "2026-04-31",
            "2026-1-1",
            "26-10-10",
            "1900-02-29",
            "2026-00-10",
            "2026-10-00",
            "2026-10-1x",
            "+026-10-10",
        ] {
            assert!(!is_iso_date(bad), "{bad}");
        }
    }

    #[test]
    fn quote_word_counts_runs_fences_and_sources() {
        assert!(quote_word_counts("# T\nplain\n", &[]).is_empty());
        let t = "---\ntitle: x\n---\n> a b c\n> d\n\n> e f\n```\n> ignored words\n```\nafter\n";
        assert_eq!(quote_word_counts(t, &[]), vec![4, 2]);
        let srcs = [
            Source {
                quote: Some("one two three".into()),
                ..Source::default()
            },
            Source {
                quote: Some("  ".into()),
                ..Source::default()
            },
            Source::default(),
        ];
        assert_eq!(quote_word_counts("> x\n", &srcs), vec![1, 3]);
    }

    #[test]
    fn quote_caps_are_the_discretion_values() {
        assert_eq!((MAX_QUOTE_WORDS, MAX_PAGE_QUOTE_WORDS), (50, 200));
    }
}
