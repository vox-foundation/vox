use vox_compiler::typeck::typecheck_ast_module;
use vox_compiler::{lexer::cursor::lex, parser::parse};

fn diags(src: &str) -> Vec<vox_compiler::typeck::Diagnostic> {
    let m = parse(lex(src)).expect("parse");
    typecheck_ast_module(src, &m)
}

#[test]
fn closure_in_on_mount_capturing_state_warns() {
    // A lambda inside `on mount:` that reads state `n` should fire the lint.
    let src = r#"
component C() {
    state n: int = 0
    on mount: {
        register_listener(fn() { log(n) })
    }
    view: text() { str(n) }
}
"#;
    let ds = diags(src);
    let hit = ds
        .iter()
        .find(|d| d.code.as_deref() == Some("lint.closure.stale_capture"));
    assert!(
        hit.is_some(),
        "expected stale_capture warning, got {:?}",
        ds.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn closure_in_effect_with_dep_does_not_warn() {
    // An explicit `depends_on` clause should suppress the lint even with a lambda inside.
    let src = r#"
component C() {
    state n: int = 0
    effect depends_on (n): {
        register_listener(fn() { log(n) })
    }
    view: text() { str(n) }
}
"#;
    let ds = diags(src);
    assert!(
        ds.iter()
            .all(|d| d.code.as_deref() != Some("lint.closure.stale_capture")),
        "explicit depends_on should suppress lint, got {:?}",
        ds.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}

#[test]
fn closure_in_event_handler_does_not_warn() {
    // A lambda used as an event handler in `view:` should NOT warn — it runs on each event.
    let src = r#"
component C() {
    state n: int = 0
    view: button(raw_class="btn", on_click=fn() { n = n + 1 }) { "+" }
}
"#;
    let ds = diags(src);
    assert!(
        ds.iter()
            .all(|d| d.code.as_deref() != Some("lint.closure.stale_capture")),
        "event handler closure should not warn, got {:?}",
        ds.iter().map(|d| &d.code).collect::<Vec<_>>()
    );
}
