//! Regression: shallow `@pure` lint (`lint.pure_shallow_violation`).

use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::TypeckSeverity;
use vox_compiler::typeck::lint_ast_declarations;

#[test]
fn pure_lint_emits_warning_on_print_call() {
    let src = r#"@pure
fn impure_print() -> Unit {
    print("x")
}
"#;
    let module = parse(lex(src)).expect("parse");
    let diags = lint_ast_declarations(&module, src);
    let hit = diags
        .iter()
        .find(|d| d.code.as_deref() == Some("lint.pure_shallow_violation"));
    let d = hit.expect("expected lint.pure_shallow_violation");
    assert_eq!(d.severity, TypeckSeverity::Warning);
}

#[test]
fn pure_lint_clean_when_no_obvious_impurity() {
    let src = r#"@pure
fn ok() -> Unit {
    return Unit
}
"#;
    let module = parse(lex(src)).expect("parse");
    let diags = lint_ast_declarations(&module, src);
    assert!(
        !diags
            .iter()
            .any(|d| d.code.as_deref() == Some("lint.pure_shallow_violation")),
        "{diags:?}"
    );
}
