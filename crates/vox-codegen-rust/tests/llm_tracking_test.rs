//! Historical name: LLM decorator tests. HIR no longer carries LLM metadata on `HirFn`;
//! this file keeps lightweight checks that async functions still emit correctly.

use vox_ast::span::Span;
use vox_codegen_rust::emit::emit_fn;
use vox_hir::{DefId, HirFn, HirType};
use vox_test_harness::spans::dummy_span;


#[test]
fn async_hir_fn_emits_pub_async_signature() {
    let func = HirFn {
        id: DefId(1),
        name: "test_ai".to_string(),
        generics: vec![],
        params: vec![],
        return_type: Some(HirType::Named("str".to_string())),
        body: vec![],
        is_component: false,
        is_async: true,
        is_pub: true,
        is_deprecated: false,
        span: dummy_span(),
    };

    let rust_code = emit_fn(&func);
    assert!(rust_code.contains("pub async fn test_ai()"));
    assert!(rust_code.contains("-> String"));
}
