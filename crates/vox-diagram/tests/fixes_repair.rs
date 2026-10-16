//! End-to-end fix tests: a fix-bearing diagnostic must flow check -> apply_fixes -> valid.

use vox_diagram::{apply_fixes, check, repair};

#[test]
fn fixes_repair_unclosed_brace_end_to_end() {
    let src = "graph {\n    container c: \"C\" {\n        node x: \"X\"\n";
    let report = check(src);
    assert!(!report.valid);
    assert!(report.diagnostics.iter().all(|d| d.fixes.len() == 1));
    let out = apply_fixes(src, &report.diagnostics);
    assert_eq!((out.applied, out.skipped), (2, 0));
    assert!(check(&out.text).valid, "{}", out.text);
    let r = repair(src, 5);
    assert_eq!(r.rounds, 1);
    assert!(r.report.valid);
    assert_eq!(r.text, out.text);
}
