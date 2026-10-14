use vox_compiler::lexer::lex;
use vox_compiler::parser::{ParseErrorClass, parse};

// TASK-2.6 (commit 080b3f86) restored `actor`, `workflow`, and `activity` as parseable
// bare-keyword blocks; they no longer produce parser-level tombstone errors. ADR-041
// (2026-05-23, supersedes ADR-028) confirms these keywords as public-grammar features
// backed by a real durable runtime — the pipeline-level reservation gate that briefly
// rejected them has also been removed. The acceptance contract now lives in
// `pipeline::tests::test_accept_*_adr041`.

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
