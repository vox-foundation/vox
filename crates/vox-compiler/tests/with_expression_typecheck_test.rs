//! Type-checking contract for `with` expressions.

use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::{TypeckSeverity, typecheck_ast_module};

fn check_source(source: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let tokens = lex(source);
    let module = parse(tokens).expect("parse ok");
    typecheck_ast_module(source, &module)
}

#[test]
fn with_operand_requires_result_type() {
    let source = r#"
fn f(x: int) to int {
    ret x with { timeout: "5s" }
}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Error
                && d.message
                    .contains("'with' operand must have type Result[T]")
        }),
        "{diags:?}"
    );
}

#[test]
fn with_operand_accepts_result_type() {
    let source = r#"
fn f(r: Result[int]) to Result[int] {
    return r with { timeout: "5s", retries: 1 }
}
"#;
    let diags = check_source(source);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "well-typed Result operand should not error: {errors:?}"
    );
}
