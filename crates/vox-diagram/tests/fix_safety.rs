//! Phase 83 review regressions: fixes must never make a document worse. Each case is the
//! reviewer's exact input (CR-01..03, WR-04, WR-07, IN-02).

use vox_diagram::repair;

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
