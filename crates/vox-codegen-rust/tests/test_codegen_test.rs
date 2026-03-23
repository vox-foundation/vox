//! `generate()` integration tests for `lib.rs` output.
use vox_ast::span::Span;
use vox_codegen_rust::generate;
use vox_hir::*;
use vox_test_harness::spans::dummy_span;
use vox_test_harness::hir_builders::minimal_hir_module as empty_module;

#[test]
fn test_codegen_generates_rust_tests() {
    let mut module = empty_module();

    module.tests.push(HirFn {
        id: DefId(2),
        name: "test_basic_addition".to_string(),
        generics: vec![],
        params: vec![],
        return_type: None,
        body: vec![HirStmt::Expr {
            expr: HirExpr::Call(
                Box::new(HirExpr::Ident("assert_eq".to_string(), dummy_span())),
                vec![
                    HirArg {
                        name: None,
                        value: HirExpr::IntLit(2, dummy_span()),
                    },
                    HirArg {
                        name: None,
                        value: HirExpr::IntLit(2, dummy_span()),
                    },
                ],
                false,
                dummy_span(),
            ),
            span: dummy_span(),
        }],
        is_component: false,
        is_async: false,
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    });

    let output = generate(&module, "test_app").expect("generate");
    let lib_rs = output.files.get("src/lib.rs").expect("lib.rs");

    assert!(lib_rs.contains("#[test]"), "Should contain test attribute");
    assert!(
        lib_rs.contains("fn test_basic_addition"),
        "Should contain test function: {lib_rs}"
    );
    assert!(
        lib_rs.contains("assert_eq!(2, 2)"),
        "Should emit assert_eq! for two-arg assert_eq call: {lib_rs}"
    );
}
