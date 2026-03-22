//! Tests for `emit_fn` (signatures, log → tracing).
use vox_ast::span::Span;
use vox_codegen_rust::emit::emit_fn;
use vox_hir::{DefId, HirArg, HirExpr, HirFn, HirStmt, HirType};
use vox_test_harness::spans::dummy_span;

#[test]
fn emit_pub_sync_fn_with_unit_return() {
    let func = HirFn {
        id: DefId(1),
        name: "old_stuff".to_string(),
        generics: vec![],
        params: vec![],
        return_type: Some(HirType::Unit),
        body: vec![],
        is_component: false,
        is_async: false,
        is_pub: true,
        is_deprecated: false,
        span: dummy_span(),
    };

    let rust_code = emit_fn(&func);
    assert!(rust_code.contains("pub fn old_stuff()"));
    assert!(rust_code.contains("-> ()"));
}

#[test]
fn emit_async_fn_includes_async_keyword() {
    let func = HirFn {
        id: DefId(2),
        name: "clean_stuff".to_string(),
        generics: vec![],
        params: vec![],
        return_type: Some(HirType::Unit),
        body: vec![],
        is_component: false,
        is_async: true,
        is_pub: true,
        is_deprecated: false,
        span: dummy_span(),
    };

    let rust_code = emit_fn(&func);
    assert!(rust_code.contains("pub async fn clean_stuff()"));
}

#[test]
fn test_log_mapping_to_tracing() {
    let func = HirFn {
        id: DefId(3),
        name: "test_log".to_string(),
        generics: vec![],
        params: vec![],
        return_type: Some(HirType::Unit),
        body: vec![HirStmt::Expr {
            expr: HirExpr::MethodCall(
                Box::new(HirExpr::Ident("log".to_string(), dummy_span())),
                "info".to_string(),
                vec![
                    HirArg {
                        name: None,
                        value: HirExpr::StringLit("Hello {}".into(), dummy_span()),
                    },
                    HirArg {
                        name: None,
                        value: HirExpr::Ident("name".into(), dummy_span()),
                    },
                ],
                dummy_span(),
            ),
            span: dummy_span(),
        }],
        is_component: false,
        is_async: false,
        is_pub: true,
        is_deprecated: false,
        span: dummy_span(),
    };

    let rust_code = emit_fn(&func);
    assert!(rust_code.contains("tracing::info!(\"Hello {}\", name.clone())"));
}
