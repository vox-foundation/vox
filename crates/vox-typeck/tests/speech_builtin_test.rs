//! `Speech` module builtin (Oratio) — lex/parse/`typecheck_module` smoke test.
use vox_lexer::lex;
use vox_parser::parser::parse;
use vox_typeck::{Diagnostic, typecheck_module};

fn check_ast(source: &str) -> Vec<Diagnostic> {
    let tokens = lex(source);
    let module = parse(tokens).expect("parse");
    typecheck_module(&module, "")
}

#[test]
fn speech_transcribe_result_str() {
    let src = r#"
fn demo(path: str) to Result[str] {
    Speech.transcribe(path)
}
"#;
    let diags = check_ast(src);
    assert!(diags.is_empty(), "expected no errors, got: {diags:?}");
}
