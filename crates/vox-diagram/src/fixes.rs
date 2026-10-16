//! Machine-applicable fixes: [`apply_fixes`] applies every diagnostic's fixes as one atomic
//! group and [`repair`] loops `check` -> `apply_fixes` until the text is clean or stops
//! changing. Bounded time: `apply_fixes` is O(n + k log k) for the output plus an O(k^2)
//! overlap scan over at most `MAX_DIAGNOSTICS` groups; `repair` runs at most `max_rounds`
//! rounds.

use crate::check::check;
use crate::diag::{CheckReport, Diagnostic};

/// Result of [`apply_fixes`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixOutcome {
    /// The edited source.
    pub text: String,
    /// Fix groups (one per diagnostic) that were applied.
    pub applied: usize,
    /// Fix groups skipped as unsafe (out of range, off a char boundary or overlapping).
    pub skipped: usize,
}

/// Result of [`repair`].
#[derive(Debug, Clone, PartialEq)]
pub struct Repaired {
    /// The repaired source.
    pub text: String,
    /// Rounds of `check` + `apply_fixes` that changed the text.
    pub rounds: usize,
    /// `check` report of the final text.
    pub report: CheckReport,
}

/// A validated edit: byte range plus replacement.
type Edit<'a> = (usize, usize, &'a str);

/// True when two edits cannot both apply: intersecting non-empty ranges, or a zero-width
/// insert strictly inside a replaced range. Equal-offset inserts and touching ranges are fine.
fn overlaps(a: Edit, b: Edit) -> bool {
    let inside = |p: usize, (s, e, _): Edit| s < p && p < e;
    match (a.0 == a.1, b.0 == b.1) {
        (true, true) => false,
        (true, false) => inside(a.0, b),
        (false, true) => inside(b.0, a),
        (false, false) => a.0 < b.1 && b.0 < a.1,
    }
}

/// The edits of one diagnostic when every one is in range, on char boundaries and clear of
/// the already `accepted` edits and of each other; `None` rejects the whole group.
fn vet<'a>(source: &str, d: &'a Diagnostic, accepted: &[Edit]) -> Option<Vec<Edit<'a>>> {
    let mut group: Vec<Edit<'a>> = Vec::with_capacity(d.fixes.len());
    for f in &d.fixes {
        let (s, e) = (f.span.start as usize, f.span.end as usize);
        if s > e || e > source.len() || !source.is_char_boundary(s) || !source.is_char_boundary(e) {
            return None;
        }
        let edit = (s, e, f.replacement.as_str());
        if accepted.iter().chain(&group).any(|&o| overlaps(edit, o)) {
            return None;
        }
        group.push(edit);
    }
    Some(group)
}

/// Apply the fixes of `diagnostics` to `source`. All fixes of one diagnostic are one atomic
/// remedy: a group with a bad edit is skipped whole. Groups apply in diagnostic order, so
/// the earlier of two overlapping groups wins; inserts at one offset keep that order and are
/// never deduplicated (two unclosed scopes need two `\n}`). Never panics.
// ponytail: the overlap scan is O(k^2) in accepted edits; `check` caps a report at 100
// diagnostics. Upgrade to an interval map if callers feed larger hand-built lists.
pub fn apply_fixes(source: &str, diagnostics: &[Diagnostic]) -> FixOutcome {
    let mut accepted: Vec<Edit> = Vec::new();
    let (mut applied, mut skipped) = (0, 0);
    for d in diagnostics.iter().filter(|d| !d.fixes.is_empty()) {
        match vet(source, d, &accepted) {
            Some(group) => {
                accepted.extend(group);
                applied += 1;
            }
            None => skipped += 1,
        }
    }
    // Stable: equal (start, end) keeps acceptance order, except that EOF scope closers
    // (`\n}`) go last, so a closing quote inserted at the same offset precedes them.
    accepted.sort_by_key(|&(s, e, r)| (s, e, r == "\n}"));
    let mut text = String::with_capacity(source.len());
    let mut pos = 0;
    for (s, e, replacement) in accepted {
        text.push_str(source.get(pos..s).unwrap_or_default());
        text.push_str(replacement);
        pos = e;
    }
    text.push_str(source.get(pos..).unwrap_or_default());
    FixOutcome {
        text,
        applied,
        skipped,
    }
}

/// Repair `source`: `check`, apply every fix, repeat while fixes remain and the text keeps
/// changing, at most `max_rounds` times.
pub fn repair(source: &str, max_rounds: usize) -> Repaired {
    let mut text = source.to_string();
    let mut report = check(&text);
    let mut rounds = 0;
    while rounds < max_rounds && report.diagnostics.iter().any(|d| !d.fixes.is_empty()) {
        let out = apply_fixes(&text, &report.diagnostics);
        if out.text == text {
            break;
        }
        text = out.text;
        rounds += 1;
        report = check(&text);
    }
    Repaired {
        text,
        rounds,
        report,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codes;
    use crate::diag::Fix;
    use crate::span::{Location, Span};

    const ONE_OPEN: &str = "graph {\n    node a: \"A\"\n";
    const TWO_OPEN: &str = "graph {\n    container c: \"C\" {\n        node x: \"X\"\n";

    fn fix(start: u32, end: u32, replacement: &str) -> Fix {
        Fix {
            span: Span { start, end },
            replacement: replacement.to_string(),
        }
    }

    fn diag(fixes: Vec<Fix>) -> Diagnostic {
        let mut d = Diagnostic::error(codes::UNEXPECTED_TOKEN, "t".into(), Location::default());
        d.fixes = fixes;
        d
    }

    #[test]
    fn unclosed_graph_gets_one_close_brace_at_eof() {
        let r = check(ONE_OPEN);
        assert_eq!(r.diagnostics.len(), 1);
        let fixes = &r.diagnostics[0].fixes;
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].replacement, "\n}");
        assert_eq!(fixes[0].span.start as usize, ONE_OPEN.len());
        assert_eq!(fixes[0].span.end as usize, ONE_OPEN.len());
        let out = apply_fixes(ONE_OPEN, &r.diagnostics);
        assert_eq!((out.applied, out.skipped), (1, 0));
        assert_eq!(out.text, format!("{ONE_OPEN}\n}}"));
        assert!(check(&out.text).valid);
    }

    #[test]
    fn two_identical_eof_inserts_both_apply() {
        let r = check(TWO_OPEN);
        assert_eq!(r.diagnostics.len(), 2);
        assert_eq!(r.diagnostics[0].fixes, r.diagnostics[1].fixes);
        let out = apply_fixes(TWO_OPEN, &r.diagnostics);
        assert_eq!((out.applied, out.skipped), (2, 0));
        assert_eq!(out.text, format!("{TWO_OPEN}\n}}\n}}"));
        assert!(check(&out.text).valid);
    }

    #[test]
    fn out_of_range_group_is_skipped_whole() {
        let ds = [diag(vec![fix(0, 1, "X"), fix(10, 11, "Y")])];
        let out = apply_fixes("abc", &ds);
        assert_eq!((out.text.as_str(), out.applied, out.skipped), ("abc", 0, 1));
        let inverted = [diag(vec![fix(2, 1, "X")])];
        assert_eq!(apply_fixes("abc", &inverted).skipped, 1);
    }

    #[test]
    fn edit_splitting_a_multibyte_char_is_skipped() {
        let ds = [diag(vec![fix(1, 1, "X")]), diag(vec![fix(0, 0, "Y")])];
        let out = apply_fixes("é", &ds);
        assert_eq!((out.text.as_str(), out.applied, out.skipped), ("Yé", 1, 1));
    }

    #[test]
    fn overlapping_group_is_skipped_and_later_one_loses() {
        let ds = [
            diag(vec![fix(0, 2, "X")]),
            diag(vec![fix(1, 3, "Y")]),
            diag(vec![fix(1, 1, "Z")]),
            diag(vec![fix(2, 4, "W")]),
        ];
        let out = apply_fixes("abcdef", &ds);
        // 2nd overlaps the 1st; the insert at 1 sits strictly inside the replaced 0..2.
        assert_eq!(out.text, "XWef");
        assert_eq!((out.applied, out.skipped), (2, 2));
    }

    #[test]
    fn edits_inside_one_group_must_not_overlap() {
        let ds = [diag(vec![fix(0, 3, "X"), fix(2, 4, "Y")])];
        let out = apply_fixes("abcdef", &ds);
        assert_eq!((out.text.as_str(), out.skipped), ("abcdef", 1));
    }

    #[test]
    fn group_is_atomic_and_edits_apply_in_offset_order() {
        let ds = [diag(vec![fix(4, 5, "E"), fix(0, 0, "<")])];
        let out = apply_fixes("abcdef", &ds);
        assert_eq!((out.text.as_str(), out.applied), ("<abcdEf", 1));
    }

    #[test]
    fn same_offset_inserts_keep_diagnostic_order_and_diagnostics_without_fixes_are_ignored() {
        let ds = [
            diag(vec![fix(1, 1, "1")]),
            diag(vec![]),
            diag(vec![fix(1, 1, "2")]),
        ];
        let out = apply_fixes("ab", &ds);
        assert_eq!(
            (out.text.as_str(), out.applied, out.skipped),
            ("a12b", 2, 0)
        );
    }

    #[test]
    fn repair_converges_in_one_round_and_stops_early() {
        let r = repair(TWO_OPEN, 5);
        assert_eq!(r.rounds, 1);
        assert!(r.report.valid);
        assert_eq!(r.text, format!("{TWO_OPEN}\n}}\n}}"));
        let valid = "graph {\n    node a: \"A\"\n}\n";
        let r = repair(valid, 5);
        assert_eq!((r.rounds, r.text.as_str()), (0, valid));
        let r = repair(TWO_OPEN, 0);
        assert_eq!((r.rounds, r.text.as_str()), (0, TWO_OPEN));
        assert!(!r.report.valid);
    }
}
