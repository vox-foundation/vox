//! Page-one summary block (D-08): `<!-- kb:summary -->` ... `<!-- /kb:summary -->`.

use crate::frontmatter::{FenceTracker, frontmatter_range, line_spans};

/// Opening marker; canonical form is on its own line so the block renders as a lede paragraph.
pub const SUMMARY_OPEN: &str = "<!-- kb:summary -->";
/// Closing marker.
pub const SUMMARY_CLOSE: &str = "<!-- /kb:summary -->";
/// Minimum summary length in words (inclusive); range checking is the lint's job.
pub const SUMMARY_MIN_WORDS: usize = 40;
/// Maximum summary length in words (inclusive).
pub const SUMMARY_MAX_WORDS: usize = 120;

/// Result of inspecting a doc for its summary block (lines are 1-based in the whole doc text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SummaryStatus {
    Missing,
    Unterminated,
    Multiple,
    Misplaced { line: usize },
    Present { words: usize, line: usize },
}

/// True when the line, trimmed, is exactly the open or close marker.
#[must_use]
pub fn is_summary_marker_line(line: &str) -> bool {
    let t = line.trim();
    t == SUMMARY_OPEN || t == SUMMARY_CLOSE
}

/// Locate and measure the summary block. Markers inside fenced code are ignored.
#[must_use]
pub fn inspect_summary(doc_text: &str) -> SummaryStatus {
    let body = frontmatter_range(doc_text).map_or(0, |r| r.end);
    // (1-based line no, trimmed line) for body lines outside fences; fence lines are dropped.
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut fences = FenceTracker::default();
    for (i, (start, _, l)) in line_spans(doc_text, 0).enumerate() {
        if start < body {
            continue;
        }
        if !fences.step(l) {
            lines.push((i + 1, l));
        }
    }
    let opens: Vec<usize> = (0..lines.len())
        .filter(|&i| lines[i].1.trim() == SUMMARY_OPEN)
        .collect();
    let open = match opens.as_slice() {
        [] => return SummaryStatus::Missing,
        [o] => *o,
        _ => return SummaryStatus::Multiple,
    };
    let Some(close) = (open + 1..lines.len()).find(|&i| lines[i].1.trim() == SUMMARY_CLOSE) else {
        return SummaryStatus::Unterminated;
    };
    let after_h1 = lines
        .iter()
        .position(|(_, l)| l.starts_with("# "))
        .map_or(0, |h| h + 1);
    let first = (after_h1..lines.len()).find(|&i| !lines[i].1.trim().is_empty());
    let line = lines[open].0;
    if first != Some(open) {
        return SummaryStatus::Misplaced { line };
    }
    let words = lines[open + 1..close]
        .iter()
        .map(|(_, l)| l.split_whitespace().count())
        .sum();
    SummaryStatus::Present { words, line }
}

/// Remove every summary block (inline forms included). An open marker with no later close
/// removes the rest of the text: fail closed for the MENS strip.
#[must_use]
pub fn strip_summary_blocks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(o) = rest.find(SUMMARY_OPEN) {
        out.push_str(&rest[..o]);
        let after = &rest[o + SUMMARY_OPEN.len()..];
        match after.find(SUMMARY_CLOSE) {
            Some(c) => rest = &after[c + SUMMARY_CLOSE.len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// WR-05: a marker inside a longer or different-char fence is still inside code.
    #[test]
    fn markers_inside_nested_or_mixed_fences_are_ignored() {
        let nested =
            "# T\n\n````md\n```\n<!-- kb:summary -->\nx\n<!-- /kb:summary -->\n```\n````\n";
        assert_eq!(inspect_summary(nested), SummaryStatus::Missing);
        let mixed = "# T\n\n```text\n~~~\n<!-- kb:summary -->\nx\n<!-- /kb:summary -->\n```\n";
        assert_eq!(inspect_summary(mixed), SummaryStatus::Missing);
        // a real block after a properly closed longer fence is found
        let after = "# T\n\n<!-- kb:summary -->\nx\n<!-- /kb:summary -->\n\n````\n```\n````\n";
        assert!(matches!(
            inspect_summary(after),
            SummaryStatus::Present { .. }
        ));
    }

    fn words(n: usize) -> String {
        vec!["w"; n].join(" ")
    }
    fn page(n: usize) -> String {
        format!(
            "# T\n\n{SUMMARY_OPEN}\n{}\n{SUMMARY_CLOSE}\n\nbody\n",
            words(n)
        )
    }

    #[test]
    fn inspect_summary_counts_words_at_the_boundaries() {
        for n in [39, 40, 120, 121] {
            assert_eq!(
                inspect_summary(&page(n)),
                SummaryStatus::Present { words: n, line: 3 }
            );
        }
        assert_eq!((SUMMARY_MIN_WORDS, SUMMARY_MAX_WORDS), (40, 120));
    }

    #[test]
    fn inspect_summary_line_numbers_include_frontmatter() {
        let t = format!("---\ntitle: A\n---\n{}", page(40));
        assert_eq!(
            inspect_summary(&t),
            SummaryStatus::Present { words: 40, line: 6 }
        );
    }

    #[test]
    fn inspect_summary_missing_unterminated_multiple_empty() {
        assert_eq!(inspect_summary("# T\n\nbody\n"), SummaryStatus::Missing);
        assert_eq!(
            inspect_summary(&format!("# T\n{SUMMARY_OPEN}\nx y\n")),
            SummaryStatus::Unterminated
        );
        assert_eq!(
            inspect_summary(&format!(
                "# T\n{SUMMARY_OPEN}\nx\n{SUMMARY_OPEN}\ny\n{SUMMARY_CLOSE}\n"
            )),
            SummaryStatus::Multiple
        );
        assert_eq!(
            inspect_summary(&format!("# T\n{SUMMARY_OPEN}\n{SUMMARY_CLOSE}\n")),
            SummaryStatus::Present { words: 0, line: 2 }
        );
    }

    #[test]
    fn inspect_summary_placement_and_fences() {
        let late = format!("# T\n\nintro paragraph\n\n{SUMMARY_OPEN}\nx\n{SUMMARY_CLOSE}\n");
        assert_eq!(inspect_summary(&late), SummaryStatus::Misplaced { line: 5 });
        let no_h1 = format!("{SUMMARY_OPEN}\nx y\n{SUMMARY_CLOSE}\n## Sec\n");
        assert_eq!(
            inspect_summary(&no_h1),
            SummaryStatus::Present { words: 2, line: 1 }
        );
        let fenced = format!("# T\n```\n{SUMMARY_OPEN}\nx\n{SUMMARY_CLOSE}\n```\n");
        assert_eq!(inspect_summary(&fenced), SummaryStatus::Missing);
    }

    #[test]
    fn inspect_summary_link_text_counts() {
        let t =
            format!("# T\n{SUMMARY_OPEN}\nsee [the guide](https://x.y/z) now\n{SUMMARY_CLOSE}\n");
        assert_eq!(
            inspect_summary(&t),
            SummaryStatus::Present { words: 4, line: 2 }
        );
    }

    #[test]
    fn strip_summary_blocks_removes_every_form() {
        let two = format!(
            "a\n{SUMMARY_OPEN}\nSECRET1\n{SUMMARY_CLOSE}\nb\n{SUMMARY_OPEN}\nSECRET2\n{SUMMARY_CLOSE}\nc\n"
        );
        let s = strip_summary_blocks(&two);
        assert!(!s.contains("SECRET") && s.contains('a') && s.contains('b') && s.contains('c'));
        let inline = format!("x {SUMMARY_OPEN} inline text {SUMMARY_CLOSE} y");
        assert_eq!(strip_summary_blocks(&inline), "x  y");
        let open = format!("keep\n{SUMMARY_OPEN}\nleak\nmore leak\n");
        assert_eq!(strip_summary_blocks(&open), "keep\n");
        assert_eq!(strip_summary_blocks("no markers\n"), "no markers\n");
    }

    #[test]
    fn is_summary_marker_line_matches_only_markers() {
        assert!(is_summary_marker_line("  <!-- kb:summary -->  "));
        assert!(is_summary_marker_line("<!-- /kb:summary -->"));
        assert!(!is_summary_marker_line("text <!-- kb:summary -->"));
        assert!(!is_summary_marker_line("<!-- kb:other -->"));
    }
}
