#![allow(missing_docs)]

use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::pipeline::run_frontend_str;
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

fn warnings(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    check_src(src)
        .into_iter()
        .filter(|d| d.severity == TypeckSeverity::Warning)
        .collect()
}

// ── Tombstone tests (TASK-2.6) ────────────────────────────────────────────────
// `activity` and `workflow` keywords are reserved (ADR-028). The pipeline
// rejects them with E028 diagnostics. Tests use `run_frontend_str` to go
// through the full pipeline rather than the bare parser, which accepts the
// tokens so it can produce a more helpful diagnostic.

#[test]
fn tombstoned_activity_keyword_produces_parse_error() {
    let src = r#"
activity send_email(recipient: str, subject: str) to Result[str] {
    Ok("ok")
}
"#;
    let result = run_frontend_str(src, "test.vox").expect("pipeline should not hard-fail");
    assert!(
        result.has_errors(),
        "tombstoned `activity` keyword should produce a pipeline error diagnostic"
    );
}

#[test]
fn tombstoned_workflow_keyword_produces_parse_error() {
    let src = r#"
workflow main_flow() to Result[str] {
    Ok("done")
}
"#;
    let result = run_frontend_str(src, "test.vox").expect("pipeline should not hard-fail");
    assert!(
        result.has_errors(),
        "tombstoned `workflow` keyword should produce a pipeline error diagnostic"
    );
}

#[test]
fn tombstoned_activity_and_workflow_together_produce_parse_error() {
    let src = r#"
activity process_data(data: str) to Result[str] {
    Ok(data)
}

workflow pipeline() to Result[str] {
    let result = process_data("test") with { retries: 3 }
    result
}
"#;
    let result = run_frontend_str(src, "test.vox").expect("pipeline should not hard-fail");
    assert!(
        result.has_errors(),
        "tombstoned `activity` + `workflow` should produce pipeline error diagnostics"
    );
}

// ── `with` operator on plain `fn` contexts ────────────────────────────────────

#[test]
fn test_with_operator_associativity() {
    let src = r#"
fn f() to Result[int] {
    let x = Ok(1) with { meta: "data" }
    x
}
"#;
    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "`with` applies to Result operands; Ok(1) with options should typecheck, got: {:?}",
        errs
    );
}

#[test]
fn test_with_non_record_options_error() {
    let src = r#"
fn f() to int {
    let x = 1 with "invalid"
    x
}
"#;
    let errs = errors(src);
    assert!(
        !errs.is_empty(),
        "Using 'with' with a non-record should produce error"
    );
    assert!(errs[0].message.contains("'with' options must be a record"));
}

#[test]
fn test_with_unknown_option_key_warning() {
    let src = r#"
fn f() to int {
    let x = 1 with { unknown_key: 42 }
    x
}
"#;
    let warns = warnings(src);
    assert!(
        !warns.is_empty(),
        "Unknown 'with' option key should produce warning"
    );
    assert!(
        warns[0].message.contains("Unknown 'with' option"),
        "Got: {}",
        warns[0].message
    );
}

#[test]
fn test_with_wrong_option_type_warning() {
    let src = r#"
fn f() to int {
    let x = 1 with { retries: "not_a_number" }
    x
}
"#;
    let warns = warnings(src);
    assert!(
        !warns.is_empty(),
        "Wrong type for 'retries' should produce warning"
    );
    assert!(
        warns[0].message.contains("retries"),
        "Got: {}",
        warns[0].message
    );
    assert!(
        warns[0].message.contains("Int"),
        "Should mention expected type Int"
    );
}

// ── Table / Index type checking ───────────────────────────────────────────────

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

// ── Argument / generic type checking ─────────────────────────────────────────

#[test]
fn test_arg_type_mismatch_error() {
    let src = r#"
fn add(a: int, b: int) to int {
    return a
}

fn main() to int {
    return add(1, "str")
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
    return a
}

fn main() to int {
    return add(1)
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
    return x
}

fn main() to int {
    let s: str = id(1)
    return 0
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
    return x
}

fn main() to int {
    let i: int = id(1)
    return i
}
"#;
    let errs = errors(src);
    assert!(
        errs.is_empty(),
        "Valid generic identity call should pass type check, got: {:?}",
        errs
    );
}
