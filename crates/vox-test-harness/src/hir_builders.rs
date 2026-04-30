//! Shared HIR builder helpers for test code.
//!
//! Provides zero-boilerplate constructors for common HIR nodes so that
//! `minimal_module()` and similar helpers are never redefined per-file.

use vox_compiler::hir::{HirFn, HirModule, HirRoute, HirTable};

use crate::spans::dummy_span;

/// An empty [`HirModule`] with no declarations — the minimal starting point for
/// codegen tests that build up a module incrementally.
///
/// Prefer this over defining `fn minimal_module()` locally in test files.
pub fn minimal_hir_module() -> HirModule {
    let m = HirModule::default();
    let _ = std::hint::black_box(m.functions.len());
    m
}

/// Build a minimal [`HirFn`] with the given name and no body.
///
/// Defaults: sync, not pub, not `@mobile.native`, no params, no return type.
pub fn hir_fn(name: impl Into<String>) -> HirFn {
    use vox_compiler::hir::DefId;
    HirFn {
        id: DefId(0),
        name: name.into(),
        generics: vec![],
        params: vec![],
        return_type: None,
        body: vec![],
        is_component: false,
        is_async: false,
        is_pub: false,
        is_mobile_native: false,
        is_pure: false,
        is_deprecated: false,
        is_llm: false,
        llm_model: None,
        schedule_interval: None,
        postconditions: vec![],
        capabilities: vec![],
        span: dummy_span(),
    }
}

/// Build a minimal GET [`HirRoute`] for the given path with no body.
pub fn hir_get_route(path: impl Into<String>) -> HirRoute {
    use vox_compiler::hir::HirHttpMethod;
    let path = path.into();
    let method = HirHttpMethod::Get;
    let route_contract = format!("{} {}", method.as_str(), path);
    HirRoute {
        method,
        path,
        route_contract,
        return_type: None,
        body: vec![],
        span: dummy_span(),
    }
}

/// Build a minimal [`HirTable`] with the given name and no fields.
pub fn hir_table(name: impl Into<String>) -> HirTable {
    use vox_compiler::hir::DefId;
    HirTable {
        id: DefId(1),
        name: name.into(),
        fields: vec![],
        is_pub: false,
        is_deprecated: false,
        span: dummy_span(),
    }
}
