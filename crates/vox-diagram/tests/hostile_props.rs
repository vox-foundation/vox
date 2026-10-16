//! S3 hostile input: arbitrary text and token soup never panic `check`, `apply_fixes`,
//! `repair` or `fmt`, and every diagnostic and fix span is safe to slice.

use proptest::prelude::*;
use vox_diagram::diag::Diagnostic;
use vox_diagram::{apply_fixes, check, fmt, repair};

/// Pieces that exercise the lexer, recovery, the resolver and multibyte handling.
fn fragments() -> Vec<String> {
    let mut f: Vec<String> = [
        "node",
        "container",
        "direction",
        "->",
        "-->",
        "{",
        "}",
        "\"",
        "\\",
        "//",
        "%%",
        "[",
        "]",
        "(",
        ")",
        ";",
        ".",
        ":",
        "\n",
        "\r\n",
        "é",
        "日本",
        "\u{202e}",
        " ",
        "graph",
        "shape",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    f.push(format!("\"{}\"", "x".repeat(250)));
    f
}

fn soup() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(fragments()), 0..120).prop_map(|v| v.concat())
}

fn assert_safe(src: &str, d: &Diagnostic) -> Result<(), TestCaseError> {
    let ok = |start: u32, end: u32| {
        start <= end
            && end as usize <= src.len()
            && src.is_char_boundary(start as usize)
            && src.is_char_boundary(end as usize)
    };
    prop_assert!(ok(d.span.start, d.span.end), "{d:?}\n{src:?}");
    for f in &d.fixes {
        prop_assert!(ok(f.span.start, f.span.end), "{d:?}\n{src:?}");
    }
    Ok(())
}

fn check_report(src: &str) -> Result<(), TestCaseError> {
    let report = check(src);
    prop_assert!(
        report.diagnostics.len() <= 100,
        "{}",
        report.diagnostics.len()
    );
    let keys: Vec<_> = report
        .diagnostics
        .iter()
        .map(|d| (d.span.start, d.code))
        .collect();
    prop_assert!(keys.windows(2).all(|w| w[0] <= w[1]), "{keys:?}");
    for d in &report.diagnostics {
        assert_safe(src, d)?;
    }
    Ok(())
}

fn fixes_are_safe(src: &str) -> Result<(), TestCaseError> {
    let report = check(src);
    let out = apply_fixes(src, &report.diagnostics);
    let groups = report
        .diagnostics
        .iter()
        .filter(|d| !d.fixes.is_empty())
        .count();
    prop_assert_eq!(out.applied + out.skipped, groups);
    // Neither the fixed text nor a bounded repair may panic; repair never grows without bound.
    let _ = check(&out.text);
    let repaired = repair(src, 3);
    prop_assert!(repaired.rounds <= 3);
    Ok(())
}

fn fmt_is_idempotent(src: &str) -> Result<(), TestCaseError> {
    if let Some(once) = fmt(src) {
        let twice = fmt(&once);
        prop_assert_eq!(twice.as_deref(), Some(once.as_str()), "{}", src);
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// `check` never panics; reports are capped and sorted by `(span.start, code)`.
    #[test]
    fn hostile_check_is_total_capped_and_sorted(a in "\\PC{0,400}", b in soup()) {
        check_report(&a)?;
        check_report(&b)?;
    }

    /// Every diagnostic span and fix span is in range and on char boundaries.
    #[test]
    fn hostile_spans_and_fix_spans_are_on_char_boundaries(a in "\\PC{0,400}", b in soup()) {
        for src in [&a, &b] {
            for d in &check(src).diagnostics {
                assert_safe(src, d)?;
            }
        }
    }

    /// `apply_fixes` and `repair` never panic and the fixed text still checks.
    #[test]
    fn hostile_apply_fixes_and_repair_never_panic(a in "\\PC{0,400}", b in soup()) {
        fixes_are_safe(&a)?;
        fixes_are_safe(&b)?;
    }

    /// Wherever `fmt` returns text, formatting that text again is a no-op.
    #[test]
    fn hostile_fmt_is_idempotent_wherever_it_formats(a in "\\PC{0,400}", b in soup()) {
        fmt_is_idempotent(&a)?;
        fmt_is_idempotent(&b)?;
    }
}
