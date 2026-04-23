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

#[test]
fn warns_for_unpinned_rust_imports() {
    let source = r#"
import rust:serde_json as sj
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Warning
                && d.message.contains("no version/path/git pin")
                && d.message.contains("serde_json")
        }),
        "{diags:?}"
    );
}

#[test]
fn warns_for_internal_runtime_crates() {
    let source = r#"
import rust:tokio(version: "1") as tk
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Warning
                && d.message.contains("internal_runtime_only")
                && d.message.contains("tokio")
        }),
        "{diags:?}"
    );
}

#[test]
fn warns_for_deferred_crates() {
    let source = r#"
import rust:sqlx(version: "0.7") as sx
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Warning
                && d.message.contains("deferred")
                && d.message.contains("sqlx")
        }),
        "{diags:?}"
    );
}

#[test]
fn rust_import_rejects_path_and_git_together() {
    let source = r#"
import rust:serde_json(path: "../serde_json", git: "https://example.invalid/repo") as sj
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Error
                && d.category == DiagnosticCategory::Lowering
                && d.message.contains("conflicting sources")
        }),
        "{diags:?}"
    );
}

#[test]
fn warns_for_escape_hatch_crates() {
    let source = r#"
import rust:chrono(version: "0.4") as ch
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Warning
                && d.message.contains("escape_hatch_only")
                && d.message.contains("chrono")
        }),
        "{diags:?}"
    );
}

#[test]
fn warns_for_planned_semantics_crates() {
    let source = r#"
import rust:time(version: "0.3") as tm
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Warning
                && d.message.contains("planned")
                && d.message.contains("time")
        }),
        "{diags:?}"
    );
}

#[test]
fn warns_when_template_managed_app_dependency_has_explicit_pin() {
    let source = r#"
import rust:reqwest(version: "0.12") as rw
fn main() {}
"#;
    let diags = check_source(source);
    assert!(
        diags.iter().any(|d| {
            d.severity == TypeckSeverity::Warning
                && d.message.contains("template")
                && d.message.contains("reqwest")
        }),
        "{diags:?}"
    );
}
