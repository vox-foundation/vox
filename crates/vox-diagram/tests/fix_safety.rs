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
