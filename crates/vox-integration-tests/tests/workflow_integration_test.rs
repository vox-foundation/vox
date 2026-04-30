#![allow(missing_docs)]

// Tests for the retired `activity` / `workflow` HIR were removed as part of
// TASK-2.6 (retire actor/workflow/activity HIR). Durable execution is now
// expressed via effect-annotated `fn` declarations (`uses net, db, mcp(...)`,
// TASK-4.2) rather than first-class `activity`/`workflow` decls. Parser-level
// tombstoning of those keywords is covered in
// `crates/vox-compiler/src/parser/descent/tests.rs`.

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::diagnostics::TypeckSeverity;
use vox_compiler::typeck::typecheck_module;

fn check_src(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let tokens = lex(src);
    let module = parse(tokens).expect("Source should parse without generic errors");
    typecheck_module(&module, src)
}

fn errors(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    check_src(src)
        .into_iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .collect()
}

// --- Table / Index type checking ---

#[test]
fn test_table_registration_no_errors() {
    let src = r#"
@table type Task {
    title: str
    done: bool
    priority: int
}
"#;
    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "Valid table should have no errors, got: {:?}",
        errs
    );
}

#[test]
fn test_index_on_known_table_no_errors() {
    let src = r#"
@table type Task {
    title: str
    done: bool
}

@index Task.by_done on (done)
"#;
    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "Index on known table should have no errors, got: {:?}",
        errs
    );
}

#[test]
fn test_index_on_unknown_table_error() {
    let src = r#"
@index Missing.by_name on (name)
"#;
    let errs = errors(src);
    assert!(
        !errs.is_empty(),
        "Index on unknown table should produce an error"
    );
    assert!(
        errs[0].message.contains("unknown table 'Missing'"),
        "Error message: {}",
        errs[0].message
    );
}

#[test]
fn test_arg_type_mismatch_error() {
    let src = r#"
fn add(a: int, b: int) to int {
    ret a
}

fn main() to int {
    ret add(1, "str")
}
"#;
    let errs = errors(src);
    assert!(
        !errs.is_empty(),
        "Type mismatch in args should produce error"
    );
    assert!(
        errs[0].message.contains("Argument type mismatch"),
        "Got: {}",
        errs[0].message
    );
}

#[test]
fn test_arg_count_mismatch_error() {
    let src = r#"
fn add(a: int, b: int) to int {
    ret a
}

fn main() to int {
    ret add(1)
}
"#;
    let errs = errors(src);
    assert!(!errs.is_empty(), "Arg count mismatch should produce error");
    assert!(
        errs[0].message.contains("Argument count mismatch"),
        "Got: {}",
        errs[0].message
    );
}

#[test]
fn test_generic_type_mismatch() {
    let src = r#"
fn id<T>(x: T) to T {
    ret x
}

fn main() to int {
    let s: str = id(1)
    ret 0
}
"#;
    let errs = errors(src);
    assert!(
        !errs.is_empty(),
        "Generic type mismatch should produce error"
    );
    let msg = &errs[0].message;
    assert!(
        msg.contains("mismatch") || msg.contains("Incompatible"),
        "Got: {}",
        msg
    );
}

#[test]
fn test_generic_identity_works() {
    let src = r#"
fn id<T>(x: T) to T {
    ret x
}

fn main() to int {
    let i: int = id(1)
    ret i
}
"#;
    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "Valid generic identity call should pass type check, got: {:?}",
        errs
    );
}
