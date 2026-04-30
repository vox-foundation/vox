//! Recovery and multi-error behavior (A05).

use vox_compiler::lexer::lex;
use vox_compiler::parser::{ParseErrorClass, parse};

#[test]
fn multiple_top_level_issues_accumulate_errors() {
    let src = r#"
fn good_a() to int { return 1 }

@@@bad@@@

fn good_b() to int { return 2 }
"#;
    let tokens = lex(src);
    let err = parse(tokens).expect_err("expected parse errors");
    assert!(!err.is_empty(), "expected at least one error");
    assert!(
        err.iter().any(|e| e.class == ParseErrorClass::TopLevel),
        "expected a top-level class error, got {err:?}"
    );
}

#[test]
fn pub_bogus_emits_declaration_class() {
    let src = include_str!("../../../examples/parser-inventory/pub-bogus.vox");
    let tokens = lex(src);
    let err = parse(tokens).expect_err("expected parse failure");
    assert!(
        err.iter().any(|e| e.class == ParseErrorClass::Declaration),
        "expected declaration-class error, got {err:?}"
    );
}

#[test]
fn nested_unclosed_errors_without_panic() {
    let src = include_str!("../../../examples/parser-inventory/nested-unclosed.vox");
    let tokens = lex(src);
    let r = parse(tokens);
    assert!(r.is_err(), "expected failure for unclosed block");
}
