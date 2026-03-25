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
    HirModule {
        imports: vec![],
        functions: vec![],
        types: vec![],
        routes: vec![],
        actors: vec![],
        workflows: vec![],
        activities: vec![],
        tests: vec![],
        server_fns: vec![],
        tables: vec![],
        indexes: vec![],
        mcp_tools: vec![],
        components: vec![],
        v0_components: vec![],
        client_routes: vec![],
        islands: vec![],
        layouts: vec![],
        pages: vec![],
        contexts: vec![],
        hooks: vec![],
        error_boundaries: vec![],
        loadings: vec![],
        not_founds: vec![],
        reactive_components: vec![],
        legacy_ast_nodes: vec![],
    }
}

/// Build a minimal [`HirFn`] with the given name and no body.
///
/// Defaults: sync, not pub, not deprecated, no params, no return type.
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
        is_deprecated: false,
        span: dummy_span(),
    }
}

/// Build a minimal GET [`HirRoute`] for the given path with no body.
pub fn hir_get_route(path: impl Into<String>) -> HirRoute {
    use vox_compiler::hir::HirHttpMethod;
    HirRoute {
        method: HirHttpMethod::Get,
        path: path.into(),
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
