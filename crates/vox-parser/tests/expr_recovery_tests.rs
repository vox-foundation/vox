#![allow(missing_docs)]

//! Recovery tests for expression-level parsing in the Vox parser.
//!
//! Tests that malformed expressions produce `ParseError`s without panicking.

use vox_lexer::lex;
use vox_parser::parse;

fn parse_errors(src: &str) -> Vec<vox_parser::ParseError> {
    let tokens = lex(src);
    match parse(tokens) {
        Ok(_) => vec![],
        Err(errs) => errs,
    }
}

fn assert_has_error(src: &str) {
    let errs = parse_errors(src);
    assert!(
        !errs.is_empty(),
        "Expected ParseError for {:?}, got none",
        src
    );
}

fn assert_clean(src: &str) {
    let errs = parse_errors(src);
    assert!(
        errs.is_empty(),
        "Expected no ParseError for {:?}, got: {:?}",
        src,
        errs
    );
}

// ── Valid expression baselines ────────────────────────────────────────────────

#[test]
fn valid_binary_expr_in_fn() {
    assert_clean("fn f() { let x = 1 + 2 }");
}

#[test]
fn valid_if_expr_in_workflow() {
    assert_clean("workflow w() { if true { return 1 } }");
}

#[test]
fn valid_method_call_expr() {
    assert_clean("fn f() { let s = foo.bar() }");
}

#[test]
fn valid_list_literal() {
    assert_clean("fn f() { let xs = [1, 2, 3] }");
}

// ── Missing closing paren ────────────────────────────────────────────────────

#[test]
fn unclosed_call_paren_produces_error() {
    assert_has_error("fn f() { foo( }");
}

#[test]
fn unclosed_grouped_expr_produces_error() {
    assert_has_error("fn f() { let x = (1 + 2 }");
}

// ── Dangling operator ────────────────────────────────────────────────────────

#[test]
fn dangling_plus_at_end_of_block_produces_error() {
    assert_has_error("fn f() { let x = 1 + }");
}

#[test]
fn dangling_star_produces_error() {
    assert_has_error("fn f() { let y = * }");
}

// ── Missing closing bracket ──────────────────────────────────────────────────

#[test]
fn unclosed_list_literal_produces_error() {
    assert_has_error("fn f() { let xs = [1, 2 }");
}

// ── Missing comma between args ───────────────────────────────────────────────

#[test]
fn call_args_missing_comma_produces_error() {
    assert_has_error("fn f() { foo(a b) }");
}

// ── Parser never panics on expr edge cases ───────────────────────────────────

#[test]
fn empty_string_does_not_panic() {
    let _ = parse_errors("");
}

#[test]
fn only_operator_does_not_panic() {
    let _ = parse_errors("+");
}

#[test]
fn nested_calls_do_not_panic() {
    // Use a moderate depth; very deep recursion overflows the recursive-descent parser stack.
    let src = "fn f() { ".to_string() + &"a(".repeat(20) + &")".repeat(20) + " }";
    let _ = parse_errors(&src);
}

#[test]
fn unicode_identifiers_do_not_panic() {
    // Unicode idents may not be valid Vox, but must never panic the parser.
    let _ = parse_errors("fn café() { }");
}

#[test]
fn all_whitespace_does_not_panic() {
    let _ = parse_errors("   \n\t   ");
}
