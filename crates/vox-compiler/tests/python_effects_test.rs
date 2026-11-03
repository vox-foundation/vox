//! Phase 324 (EFF-01 / EFF-02): effect gate for the Python bridge.
//!
//! - EFF-01: `@uses(...)` and `uses ...` share one effect-item parser that
//!   errors on unknown names and keeps `mcp(<tool>)`.
//! - EFF-02: flat `python` / `python_unverified` effects round-trip
//!   AST → HIR → `HirCapability`, and propagate through effect compliance.

use vox_compiler::ast::decl::EffectAnnotation;
use vox_compiler::hir::{HirCapability, HirEffectKind, HirFn, lower_module};
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::error::ParseErrorClass;
use vox_compiler::parser::parse;
use vox_compiler::typeck::effect_check::check_effect_compliance;
use vox_compiler::typeck::{Diagnostic, DiagnosticCategory};

fn lower_fn_named(src: &str, name: &str) -> HirFn {
    let m = parse(lex(src)).expect("parse");
    let hir = lower_module(&m);
    hir.functions
        .into_iter()
        .find(|f| f.name == name)
        .unwrap_or_else(|| panic!("fn `{name}` not lowered"))
}

fn assert_unknown_effect_error(src: &str) {
    let errs = parse(lex(src)).expect_err("unknown effect must be a parse error");
    assert!(
        errs.iter().any(|e| e.class == ParseErrorClass::Declaration
            && e.message.contains("Unknown effect `bogus`")
            && e.message.contains("python_unverified")),
        "expected Declaration ParseError for `bogus` listing python_unverified; got {:?}",
        errs.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
}

fn effect_violations(src: &str) -> Vec<Diagnostic> {
    let m = parse(lex(src)).expect("parse");
    let hir = lower_module(&m);
    check_effect_compliance(&hir, src)
        .into_iter()
        .filter(|d| matches!(d.category, DiagnosticCategory::EffectViolation))
        .collect()
}

// ── EFF-01 ─────────────────────────────────────────────────────────────

#[test]
fn decorator_uses_unknown_effect_is_parse_error() {
    assert_unknown_effect_error(
        r#"
@uses(bogus)
fn f() to int { return 1 }
"#,
    );
}

#[test]
fn clause_uses_unknown_effect_is_parse_error() {
    assert_unknown_effect_error(
        r#"
fn f() uses bogus to int { return 1 }
"#,
    );
}

#[test]
fn decorator_uses_mcp_tool_lowers_to_hir_mcp() {
    let f = lower_fn_named(
        r#"
@uses(mcp(search))
fn f() to int { return 1 }
"#,
        "f",
    );
    assert_eq!(f.capabilities, vec![HirCapability::Mcp("search".into())]);
}

#[test]
fn decorator_uses_env_keyword_still_lowers() {
    let f = lower_fn_named(
        r#"
@uses(net, env)
fn f() to int { return 1 }
"#,
        "f",
    );
    assert_eq!(f.capabilities, vec![HirCapability::Net, HirCapability::Env]);
}

// ── EFF-02 ─────────────────────────────────────────────────────────────

#[test]
fn python_effects_round_trip_ast_keyword() {
    for kw in ["python", "python_unverified"] {
        let eff = EffectAnnotation::from_keyword(kw).expect(kw);
        assert_eq!(eff.as_str(), kw);
    }
}

#[test]
fn python_effect_labels() {
    assert_eq!(HirEffectKind::Python.label(), "python");
    assert_eq!(HirEffectKind::PythonUnverified.label(), "python_unverified");
    assert_eq!(HirCapability::Python.as_str(), "python");
    assert_eq!(
        HirCapability::PythonUnverified.as_str(),
        "python_unverified"
    );
}

#[test]
fn decorator_uses_python_unverified_lowers_to_capability() {
    let f = lower_fn_named(
        r#"
@uses(python_unverified)
fn f() to int { return 1 }
"#,
        "f",
    );
    assert_eq!(f.capabilities, vec![HirCapability::PythonUnverified]);
}

#[test]
fn clause_uses_python_lowers_to_capability() {
    let f = lower_fn_named(
        r#"
fn f() uses python to int { return 1 }
"#,
        "f",
    );
    assert_eq!(f.capabilities, vec![HirCapability::Python]);
}

#[test]
fn caller_without_python_effect_is_violation() {
    let src = r#"
fn py_helper() uses python to int { return 1 }

fn caller() uses net to int {
    return py_helper()
}
"#;
    let ds = effect_violations(src);
    assert!(
        !ds.is_empty(),
        "`uses net` caller of a `uses python` fn must be an effect violation"
    );
}

#[test]
fn caller_declaring_python_effect_is_clean() {
    let src = r#"
fn py_helper() uses python to int { return 1 }

fn caller() uses net, python to int {
    return py_helper()
}
"#;
    let ds = effect_violations(src);
    assert!(
        ds.is_empty(),
        "caller declaring `python` must pass; got {:?}",
        ds.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn activity_and_workflow_uses_clause_lowers_to_capability() {
    let src = r#"
activity a() uses python to int { return 1 }

workflow w() uses python_unverified to int { return 1 }
"#;
    assert_eq!(
        lower_fn_named(src, "a").capabilities,
        vec![HirCapability::Python]
    );
    assert_eq!(
        lower_fn_named(src, "w").capabilities,
        vec![HirCapability::PythonUnverified]
    );
}
