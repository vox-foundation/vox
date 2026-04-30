use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::{TypeckSeverity, typecheck_ast_module};

fn check_source(source: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let tokens = lex(source);
    let module = parse(tokens).expect("parse ok");
    typecheck_ast_module(source, &module)
}

#[test]
fn should_infer_return_type_if_missing() {
    let source = r#"
fn f(x: int) {
    return x
}
"#;
    let diags = check_source(source);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .collect();

    // Return type is inferred as `int` from the body, so no diagnostics expected.
    assert!(
        errors.is_empty(),
        "Function should infer return type 'int' from body: {errors:?}"
    );
}
