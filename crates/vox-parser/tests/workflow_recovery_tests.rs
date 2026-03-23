#![allow(missing_docs)]

//! Recovery tests for workflow and function declaration parsing.
//!
//! The Vox parser is resilient: it always returns a `Result` with a diagnostic
//! list rather than panicking. These tests verify that specific malformed
//! inputs produce at least one `ParseError` and that the parser never panics.

use vox_lexer::lex;
use vox_parser::parse;

/// Helper: parse `src`, return error list (may be empty on valid input).
fn parse_errors(src: &str) -> Vec<vox_parser::ParseError> {
    let tokens = lex(src);
    match parse(tokens) {
        Ok(_) => vec![],
        Err(errs) => errs,
    }
}

/// Helper: assert at least one error is produced.
fn assert_has_error(src: &str) {
    let errs = parse_errors(src);
    assert!(
        !errs.is_empty(),
        "Expected at least one ParseError for {:?}, got none",
        src
    );
}

/// Helper: assert no error (valid input).
fn assert_clean(src: &str) {
    let errs = parse_errors(src);
    assert!(
        errs.is_empty(),
        "Expected no ParseError for {:?}, got: {:?}",
        src,
        errs
    );
}

// ── Valid baselines ──────────────────────────────────────────────────────────

#[test]
fn valid_empty_workflow_parses_clean() {
    assert_clean("workflow w() { }");
}

#[test]
fn valid_workflow_with_single_stmt_parses_clean() {
    assert_clean("workflow w() { let x = 1 }");
}

#[test]
fn valid_workflow_with_return_stmt_parses_clean() {
    assert_clean("workflow w() { return 42 }");
}

#[test]
fn valid_fn_decl_parses_clean() {
    // Return-type annotations via `: Type` after params are not parsed at the declaration level.
    assert_clean("fn greet(name: str) { return name }");
}

#[test]
fn valid_fn_with_no_params_parses_clean() {
    assert_clean("fn noop() { }");
}

// ── Missing closing brace ────────────────────────────────────────────────────

#[test]
fn workflow_missing_closing_brace_recovers_without_panic() {
    // The Vox parser does resilient recovery: unclosed braces may produce
    // a partial module rather than a hard error. The key invariant is no panic.
    let tokens = vox_lexer::lex("workflow w() {");
    let _ = vox_parser::parse(tokens); // must not panic
}

#[test]
fn fn_missing_closing_brace_recovers_without_panic() {
    // Same resilient-recovery invariant for `fn` decls.
    let tokens = vox_lexer::lex("fn f() {");
    let _ = vox_parser::parse(tokens); // must not panic
}

// ── Missing opening brace ────────────────────────────────────────────────────

#[test]
fn workflow_missing_opening_brace_produces_error() {
    assert_has_error("workflow w() return 1 }");
}

#[test]
fn fn_missing_opening_brace_produces_error() {
    assert_has_error("fn f() return 1 }");
}

// ── Malformed parameter list ─────────────────────────────────────────────────

#[test]
fn workflow_unclosed_param_list_produces_error() {
    assert_has_error("workflow w( { }");
}

#[test]
fn fn_param_without_type_annotation_recovers_without_panic() {
    // The parser may accept or recover `fn f(x)` — the invariant is no panic.
    let tokens = vox_lexer::lex("fn f(x) { }");
    let _ = vox_parser::parse(tokens); // must not panic
}

#[test]
fn fn_param_double_comma_produces_error() {
    assert_has_error("fn f(a: int,, b: int) { }");
}

// ── Missing function name ────────────────────────────────────────────────────

#[test]
fn fn_without_name_produces_error() {
    assert_has_error("fn () { }");
}

#[test]
fn workflow_without_name_produces_error() {
    assert_has_error("workflow () { }");
}

// ── Parser never panics ──────────────────────────────────────────────────────

#[test]
fn garbage_input_does_not_panic() {
    let _ = parse_errors("!@#$%^&*()");
}

#[test]
fn only_open_brace_does_not_panic() {
    let _ = parse_errors("{");
}

#[test]
fn only_close_brace_does_not_panic() {
    let _ = parse_errors("}");
}

#[test]
fn deeply_nested_braces_do_not_panic() {
    let src = "workflow w() { ".repeat(50) + &"}".repeat(50);
    let _ = parse_errors(&src);
}

#[test]
fn very_long_identifier_does_not_panic() {
    let long_name = "a".repeat(4096);
    let src = format!("fn {}() {{ }}", long_name);
    let _ = parse_errors(&src);
}
