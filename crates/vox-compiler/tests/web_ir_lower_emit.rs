//! ADR 012 — HIR → WebIR → validate → TSX preview emit.

use std::collections::HashSet;

use vox_compiler::codegen_ts::hir_emit::emit_hir_expr;
use vox_compiler::hir::{lower_module, HirReactiveMember};
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse;
use vox_compiler::web_ir::emit_tsx::emit_component_view_tsx;
use vox_compiler::web_ir::lower::lower_hir_to_web_ir;
use vox_compiler::web_ir::validate::validate_web_ir;

#[test]
fn web_ir_lowering_validates_and_emits_counter_view() {
    let source = r#"
component Counter(initial: int) {
    state count: int = initial
    derived double = count * 2
    view: (
        <div class="p-4">
            <h1>"Count: {count}"</h1>
        </div>
    )
}
"#;
    let module = parse(lex(source)).expect("parse");
    let hir = lower_module(&module);
    let web = lower_hir_to_web_ir(&hir);
    let diags = validate_web_ir(&web);
    assert!(diags.is_empty(), "{diags:?}");
    let tsx = emit_component_view_tsx(&web, "Counter").expect("view");
    assert!(tsx.contains("className="), "{tsx}");
    assert!(tsx.contains("<div"));
}

#[test]
fn web_ir_view_matches_hir_emit_for_self_closing_jsx() {
    let source = r#"
component T() {
    state n: int = 1
    view: <span class="x" />
}
"#;
    let module = parse(lex(source)).expect("parse");
    let hir = lower_module(&module);
    let rc = hir
        .reactive_components
        .first()
        .expect("one reactive component");
    let view = rc.view.as_ref().expect("view");
    let state_name = match &rc.members[0] {
        HirReactiveMember::State(s) => s.name.clone(),
        _ => panic!("expected state member"),
    };
    let state_names = HashSet::from([state_name]);
    let island_names = HashSet::new();
    let direct = emit_hir_expr(view, &state_names, &island_names);
    let web = lower_hir_to_web_ir(&hir);
    let via = emit_component_view_tsx(&web, "T").expect("emit");
    assert_eq!(direct.trim(), via.trim(), "\ndirect:\n{direct}\nvia web_ir:\n{via}");
}
