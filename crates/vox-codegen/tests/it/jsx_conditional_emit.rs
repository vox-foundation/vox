//! A view `if` whose branches are bare expressions must emit JSX children, not
//! parenthesised object literals: `cond ? ( {x} ) : ( {y} )` parses `{x}` as
//! `{ x: x }`, which React rejects as a child at runtime.

fn component_tsx(src: &str, name: &str) -> String {
    let module = vox_compiler::parser::parse(vox_compiler::lexer::lex(src)).expect("parse");
    let hir = vox_compiler::hir::lower_module(&module);
    let out = vox_codegen::codegen_ts::generate(&hir).expect("codegen");
    let file = format!("{name}.tsx");
    out.files
        .into_iter()
        .find(|(n, _)| *n == file)
        .map(|(_, c)| c)
        .unwrap_or_else(|| panic!("{file} not emitted"))
}

#[test]
fn view_if_with_expression_branches_wraps_each_branch_in_a_fragment() {
    let ts = component_tsx(
        r#"
component Chip(status: str, label: str) {
    view: text(size="xs") { if label is "" { status } else { label } }
}
"#,
        "Chip",
    );
    let compact: String = ts.split_whitespace().collect();
    assert!(
        !compact.contains("?({status})") && !compact.contains(":({label})"),
        "branches must not be parenthesised object literals:\n{ts}"
    );
    assert!(
        compact.contains("?(<>{status}</>):(<>{label}</>)"),
        "each branch must be a fragment:\n{ts}"
    );
}
