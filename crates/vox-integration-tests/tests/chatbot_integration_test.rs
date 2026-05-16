#![allow(missing_docs)]

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::diagnostics::TypeckSeverity;
use vox_compiler::typeck::typecheck_module;

fn errors(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let tokens = lex(src);
    let module = parse(tokens).expect("Source should parse without errors");
    typecheck_module(&module, "")
        .into_iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .collect()
}

/// The component + ADT parts of the chatbot example typecheck without errors.
/// The original source also contained `http post` and `actor` blocks which are
/// tombstoned (TASK-2.5, TASK-2.6); those are tested separately below.
#[test]
fn chatbot_component_typechecks_cleanly() {
    let src = r#"
type ChatResult =
    | Success(text: str)
    | Error(message: str)

fn send_message(msg: str) to ChatResult {
    Success("Hello from Vox! You said: " + msg)
}
"#;

    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "Chatbot ADT + fn should typecheck cleanly, got: {:?}",
        errs
    );
}

/// `http` bare-keyword routing is tombstoned (TASK-2.5).
#[test]
fn tombstoned_http_keyword_produces_parse_error() {
    let src = r#"
http post "/api/chat" to str {
    "ok"
}
"#;
    assert!(
        parse(lex(src)).is_err(),
        "tombstoned `http` keyword should produce a parse error"
    );
}

/// `actor` keyword was tombstoned (TASK-2.6) but has since been un-tombstoned
/// (Path A of TASK-2.6 re-enabled the construct). The parser now accepts `actor`
/// declarations and produces a typed AST node — verify it parses cleanly.
#[test]
fn actor_keyword_parses_successfully() {
    let src = r#"
actor Claude {
    on send(msg: str) to str {
        "hello"
    }
}
"#;
    assert!(
        parse(lex(src)).is_ok(),
        "`actor` keyword should parse successfully now that it is no longer tombstoned"
    );
}
