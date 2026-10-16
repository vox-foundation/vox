//! Phase 83 review regressions: fixes must never make a document worse. Each case is the
//! reviewer's exact input (CR-01..03, WR-04, WR-07, IN-02).

use vox_diagram::diag::Severity;
use vox_diagram::{check, repair};

fn errors(src: &str) -> usize {
    let ds = check(src).diagnostics;
    ds.iter().filter(|d| d.severity == Severity::Error).count()
}

fn assert_repairs_to(src: &str, expected: &str) {
    let r = repair(src, 5);
    assert!(r.report.valid, "{:?}\n{}", r.report.diagnostics, r.text);
    assert_eq!(r.text, expected);
}

/// CR-01: an unterminated string and an unclosed brace both insert at EOF; the closing quote
/// must come before the `\n}`.
#[test]
fn cr01_truncated_mid_string_repairs_quote_before_brace() {
    assert_repairs_to(
        "graph {\n    node a: \"abc",
        "graph {\n    node a: \"abc\"\n}",
    );
    let r = repair("graph {\n    node a: \"abc\\", 5);
    assert!(r.report.valid, "{:?}\n{}", r.report.diagnostics, r.text);
}

/// CR-02: a decoded `\n` in the trailing edge label must stay escaped in the generated fix.
#[test]
fn cr02_chained_edge_label_keeps_newline_escaped() {
    let src = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    node c: \"C\"\n    a -> b -> c: \"x\\ny\"\n}\n";
    let r = repair(src, 5);
    assert!(r.report.valid, "{:?}\n{}", r.report.diagnostics, r.text);
    assert!(r.text.contains("b -> c: \"x\\ny\""), "{}", r.text);
}

/// CR-03: `{` followed by a nested block on the same line must not host the root declaration.
#[test]
fn cr03_declaration_lands_in_the_root_scope() {
    let src = "graph { container c: \"C\" {\n  node x: \"X\"\n}\n  a -> ghost\n}\n";
    let r = repair(src, 5);
    assert!(r.report.valid, "{:?}\n{}", r.report.diagnostics, r.text);
    assert_eq!(r.text.matches("node a:").count(), 1, "{}", r.text);
    assert_eq!(r.text.matches("node ghost:").count(), 1, "{}", r.text);
}

/// WR-04: the empty-label fix must not delete the target's bracket text.
#[test]
fn wr04_empty_edge_label_fix_keeps_bracket_text() {
    let r = repair("graph {\n    node a: \"A\"\n    a -> B((x)) : \"\"\n}\n", 5);
    assert!(r.text.contains("a -> B((x))\n"), "{}", r.text);
    assert!(!r.text.contains("\"\""), "{}", r.text);
}

/// WR-07: a bare `graph` line followed by `{` on the next line is not a Mermaid header.
#[test]
fn wr07_bare_graph_then_brace_next_line_is_not_rewritten() {
    let src = "graph\n{\n  node a: \"A\"\n}";
    let codes: Vec<_> = check(src).diagnostics.iter().map(|d| d.code).collect();
    assert!(
        !codes.contains(&vox_diagram::codes::MERMAID_SYNTAX),
        "{codes:?}"
    );
    let r = repair(src, 5);
    assert!(errors(&r.text) <= errors(src), "{}", r.text);
}

/// WR-05: `-.->` and `-.-` directly after an identifier are Mermaid links, not part of the id.
#[test]
fn wr05_dotted_mermaid_links_after_an_identifier() {
    for link in ["A-.->B", "A-.-B", "A -.-> B"] {
        let src = format!("graph {{\n    node A: \"A\"\n    node B: \"B\"\n    {link}\n}}\n");
        let codes: Vec<_> = check(&src).diagnostics.iter().map(|d| d.code).collect();
        assert_eq!(codes, [vox_diagram::codes::MERMAID_SYNTAX], "{link}");
    }
}

/// WR-01: a lone CR is a line break everywhere (as `fence::normalize` treats it), so content
/// after it is checked, and line numbers agree with the LF form of the same text.
#[test]
fn wr01_lone_cr_is_a_line_break() {
    let lf = "graph {\n    node a: \"A\"\n    // note\n a -> ghost2\n}\n";
    let cr = lf.replace('\n', "\r");
    let key = |s: &str| {
        let ds = check(s).diagnostics;
        ds.iter()
            .map(|d| (d.code, d.span.line, d.span.col))
            .collect::<Vec<_>>()
    };
    assert_eq!(key(&cr), key(lf));
    assert!(!check(&cr).valid);
    let mixed = "graph {\r    node a: \"A\"\r\n    // c\r a -> ghost2\r\n}\r";
    assert!(!check(mixed).valid);
}
