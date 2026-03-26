#![allow(missing_docs)]

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::diagnostics::TypeckSeverity;
use vox_compiler::typeck::typecheck_module;

#[test]
fn emit_outside_stream_reports_error() {
    let src = r#"fn bad() to int {
    emit 42
    ret 0
}
"#;
    let tokens = lex(src);
    let module = parse(tokens).unwrap();
    let diags = typecheck_module(&module, "");
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == TypeckSeverity::Error && d.message.to_lowercase().contains("emit"))
        .collect();
    assert!(
        !errors.is_empty(),
        "Expected an error about emit outside stream, got none"
    );
}
