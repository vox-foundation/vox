use vox_compiler::lexer::lex;
use vox_compiler::parser::{ParseErrorClass, parse};

// TASK-2.6 (commit 080b3f86) restored `actor`, `workflow`, and `activity` as parseable
// bare-keyword blocks; they no longer produce parser-level tombstone errors. Rejection now
// happens at pipeline level via ADR-028's `check_adr028_reserved_keywords`. The negative-path
// contract for those keywords is covered by `pipeline::tests::test_reject_*_adr028`.
#[test]
#[ignore = "TASK-2.6 / ADR-028: `actor` parses; rejection moved to pipeline (see test_reject_*_adr028)"]
fn actor_is_tombstoned() {
    let src = "actor MyActor {}";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for actor");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
    assert!(errs[0].message.contains("actor"));
    assert!(errs[0].message.contains("tombstoned"));
}

#[test]
#[ignore = "TASK-2.6 / ADR-028: `workflow` parses; rejection moved to pipeline"]
fn workflow_is_tombstoned() {
    let src = "workflow MyWorkflow {}";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for workflow");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
}

#[test]
fn at_component_is_tombstoned() {
    let src = "@component fn Legacy() {}";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for @component");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
}

#[test]
fn http_is_tombstoned() {
    let src = "http get \"/\"";
    let tokens = lex(src);
    let errs = parse(tokens).expect_err("expected parse failure for http");
    assert!(errs.iter().any(|e| e.class == ParseErrorClass::Tombstoned));
}
