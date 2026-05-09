//! Parity harness for the victory_claim detector migration.
//!
//! Pins the (rule_id, line, severity) tuples emitted on a fixed corpus.
//! After the detector is rewritten to consume vox-rule-pack, this test
//! must still pass unchanged.

use std::collections::BTreeSet;
use std::path::PathBuf;
use vox_code_audit::detectors::victory_claim::VictoryClaimDetector;
use vox_code_audit::rules::{DetectionRule, Severity, SourceFile};

const CORPUS: &str = concat!(
    "// ",
    "Done! Implementation complete\n",
    "fn alpha() {}\n",
    "\n",
    "/// Adds two numbers.\n",
    "fn beta(a: i32, b: i32) -> i32 { a + b }\n",
    "\n",
    "// ",
    "FIXME this is broken\n",
    "const X: i32 = 1;\n",
    "\n",
    "// HACK: workaround for upstream bug\n",
    "fn gamma() {}\n",
    "\n",
    "// TO",
    "DO: implement later\n",
    "fn delta() {}\n",
    "\n",
    "// all set, ready to ship\n",
    "fn epsilon() {}\n",
);

fn ids_lines_severities(src: &str) -> BTreeSet<(String, usize, Severity)> {
    let file = SourceFile::new(PathBuf::from("corpus.rs"), src.to_string());
    let detector = VictoryClaimDetector::new();
    detector
        .detect(&file, None)
        .into_iter()
        .map(|f| (f.rule_id, f.line, f.severity))
        .collect()
}

#[test]
fn parity_findings_match_baseline() {
    let actual = ids_lines_severities(CORPUS);

    let expected: BTreeSet<(String, usize, Severity)> = [
        ("victory-claim/premature".to_string(), 1, Severity::Warning),
        ("victory-claim/fixme".to_string(), 7, Severity::Warning),
        ("victory-claim/hack".to_string(), 10, Severity::Info),
        (
            "victory-claim/todo-leftover".to_string(),
            13,
            Severity::Warning,
        ),
        ("victory-claim/premature".to_string(), 16, Severity::Warning),
    ]
    .into_iter()
    .collect();

    assert_eq!(
        actual, expected,
        "victory_claim output must remain stable across the migration"
    );
}
