#![allow(missing_docs)]

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::diagnostics::Severity;
use vox_compiler::typeck::typecheck_module;

fn check(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let tokens = lex(src);
    let module = parse(tokens).expect("Source should parse without errors");
    typecheck_module(&module, "")
}

fn errors(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    check(src)
        .into_iter()
        .filter(|d| d.severity == Severity::Error)
        .collect()
}

#[test]
fn test_db_operations_typecheck() {
    let src = r#"
@table type Message {
    text: str
    timestamp: int
}

http post "/api/msg" to int {
    let msg = {text: "hello", timestamp: 123}
    let id = db.Message.insert(msg)
    match id {
        Ok(_) -> 1
        Error(_) -> 0
    }
}
"#;

    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "DB operations should typecheck. Errors: {:?}",
        errs
    );
}

#[test]
fn test_db_unknown_table() {
    let src = r#"
http post "/api/oops" to Unit {
    let x = db.NonExistentTable
}
"#;
    let errs = errors(src);
    assert!(!errs.is_empty(), "Should error on unknown table");
    assert!(
        errs[0].message.contains("NonExistentTable"),
        "unexpected diagnostic: {:?}",
        errs[0]
    );
}
