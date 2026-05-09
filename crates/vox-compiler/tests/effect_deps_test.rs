use vox_compiler::typeck::typecheck_ast_module;
use vox_compiler::{lexer::cursor::lex, parser::parse};

fn diags(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let m = parse(lex(src)).expect("parse");
    typecheck_ast_module(src, &m)
}

#[test]
fn effect_with_unresolvable_dep_errors() {
    // `external_call()` is not a builtin or state setter — ambiguous dep tracking.
    let src = r#"
component C() {
    state n: int = 0
    effect: {
        external_call()
        n = 1
    }
    view: text() { "x" }
}
"#;
    let ds = diags(src);
    let hit = ds
        .iter()
        .find(|d| d.code.as_deref() == Some("lint.effect.unresolvable_deps"));
    assert!(
        hit.is_some(),
        "expected lint.effect.unresolvable_deps; got {:?}",
        ds.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn effect_with_explicit_depends_on_passes() {
    // `log` is a builtin — but explicit depends_on suppresses the lint regardless.
    let src = r#"
component C() {
    state n: int = 0
    effect depends_on (n): {
        log(n)
    }
    view: text() { "x" }
}
"#;
    let ds = diags(src);
    assert!(
        ds.iter()
            .all(|d| d.code.as_deref() != Some("lint.effect.unresolvable_deps")),
        "explicit depends_on should suppress lint; got {:?}",
        ds.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn effect_with_only_local_state_passes() {
    // Only state reads and assignments — no unknown calls.
    let src = r#"
component C() {
    state n: int = 0
    state m: int = 0
    effect: { m = n + 1 }
    view: text() { "x" }
}
"#;
    let ds = diags(src);
    assert!(
        ds.iter()
            .all(|d| d.code.as_deref() != Some("lint.effect.unresolvable_deps")),
        "state-only effect should pass; got {:?}",
        ds.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}
