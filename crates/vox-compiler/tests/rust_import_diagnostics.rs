//! Typecheck / registration diagnostics for `import rust:…` edge cases.

use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::typeck::{DiagnosticCategory, TypeckSeverity, typecheck_ast_module};

fn check_source(source: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let tokens = lex(source);
    let module = parse(tokens).expect("parse ok");
    typecheck_ast_module(source, &module)
}

#[test]
fn duplicate_rust_import_alias_errors() {
    let source = r#"
import rust:serde_json as j
import rust:chrono as j
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Error
                && d.message.contains("Import alias conflict")
                && d.message.contains('j')
        }),
        "{diags:?}"
    );
}

#[test]
fn conflicting_rust_crate_specs_lower() {
    let source = r#"
import rust:serde_json(version: "1") as sj_a
import rust:serde_json(version: "2") as sj_b
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Error
                && d.category == DiagnosticCategory::Lowering
                && d.message.contains("Conflicting dependency specifications")
                && d.message.contains("serde_json")
        }),
        "{diags:?}"
    );
}

#[test]
fn same_crate_two_aliases_same_spec_ok() {
    let source = r#"
import rust:serde_json(version: "1") as sj
import rust:serde_json(version: "1") as sj2
fn main() {}
"#;
    let diags = check_source(source);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "same dep spec with two aliases should not error: {errors:?}"
    );
}
