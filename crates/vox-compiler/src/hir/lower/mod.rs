//! Lower AST [`Module`] to [`HirModule`].
//!
//! This module is the **HIR boundary** before [`crate::web_ir::lower::lower_hir_to_web_ir`].
//! Declaration arms here define what structured data reaches WebIR (islands, `HirRoutes`,
//! reactive components, server/query/mutation `route_path` contracts). See internal Web IR
//! implementation blueprint (lane P → S).
//!
//! **Spans (OP-0038 / OP-S007):** AST spans are copied onto HIR nodes where the AST carries them; reactive
//! member lowering preserves per-member spans for state, derived values, and effects. WebIR lowering
//! may elide spans on some synthetic nodes until the span-table workstream lands—consumers should not
//! treat missing WebIR spans as a lowering failure.
//!
//! **Lowering buckets (OP-S005):** each `Decl` arm in `LowerCtx::lower` maps into a named field on
//! [`HirModule`] — for example `Decl::Import`→`imports`, `Decl::Routes`→`client_routes`,
//! `Decl::ReactiveComponent`→`reactive_components`, `Decl::Island`→`islands`, `Decl::HttpRoute` /
//! server/query/mutation→`routes` / `server_fns` / `query_fns` / `mutation_fns`, and tables/indices into
//! their respective vectors. Search `Decl::` in this file for the authoritative match.

use std::collections::HashSet;

use crate::ast::decl::*;
use crate::ast::expr::Expr;
use crate::ast::pattern::Pattern;
use crate::ast::stmt::Stmt;
use crate::hir::def_map::DefMap;
use crate::hir::*;
use crate::web_prefixes::{MUTATION_FN_API_PREFIX, QUERY_FN_API_PREFIX, SERVER_FN_API_PREFIX};

mod async_flags;
mod db_select_normalize;
mod decl;
mod expr_db;
#[path = "expr.rs"]
mod lowering_expr;
#[path = "stmt.rs"]
mod lowering_stmt;

/// Lower an AST Module to a HirModule.
pub fn lower_module(module: &Module) -> HirModule {
    let mut ctx = LowerCtx::new();
    ctx.lower(module)
}

struct LowerCtx {
    def_map: DefMap,
}

impl LowerCtx {
    fn new() -> Self {
        Self {
            def_map: DefMap::new(),
        }
    }

    fn lower(&mut self, module: &Module) -> HirModule {
        let mut hir = HirModule {
            imports: Vec::new(),
            rust_imports: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            routes: Vec::new(),
            actors: Vec::new(),
            workflows: Vec::new(),
            activities: Vec::new(),
            tests: Vec::new(),
            server_fns: Vec::new(),
            query_fns: Vec::new(),
            mutation_fns: Vec::new(),
            tables: Vec::new(),
            indexes: Vec::new(),
            collections: Vec::new(),
            vector_indexes: Vec::new(),
            search_indexes: Vec::new(),
            mcp_tools: Vec::new(),
            components: Vec::new(),
            v0_components: Vec::new(),
            client_routes: Vec::new(),
            islands: Vec::new(),
            layouts: Vec::new(),
            pages: Vec::new(),
            contexts: Vec::new(),
            hooks: Vec::new(),
            error_boundaries: Vec::new(),
            loadings: Vec::new(),
            not_founds: Vec::new(),
            reactive_components: Vec::new(),
            legacy_ast_nodes: Vec::new(),
            lowering_migration: crate::hir::HirLoweringMigrationFlags::default(),
        };

        for decl in &module.declarations {
            match decl {
                Decl::Import(imp) => {
                    for path in &imp.paths {
                        match &path.kind {
                            ImportPathKind::SymbolPath { segments } => {
                                let (mod_path, item) = if segments.len() > 1 {
                                    let item = segments.last().expect("segments non-empty").clone();
                                    let mod_path = segments[..segments.len() - 1].to_vec();
                                    (mod_path, item)
                                } else {
                                    (vec![], segments[0].clone())
                                };
                                hir.imports.push(HirImport {
                                    module_path: mod_path,
                                    item: path.alias.clone().unwrap_or(item),
                                    span: path.span,
                                });
                            }
                            ImportPathKind::RustCrate(spec) => {
                                let alias = path
                                    .alias
                                    .clone()
                                    .unwrap_or_else(|| spec.crate_name.clone());
                                hir.rust_imports.push(HirRustImport {
                                    crate_name: spec.crate_name.clone(),
                                    alias,
                                    version: spec.version.clone(),
                                    path: spec.path.clone(),
                                    git: spec.git.clone(),
                                    rev: spec.rev.clone(),
                                    span: path.span,
                                });
                            }
                        }
                    }
                }
                Decl::Function(f) => {
                    hir.functions.push(self.lower_fn(f, false));
                }
                // AST-retained `@component fn` / legacy component: `HirComponent`; WebIR adapters read this + lowered `hir.functions` entry.
                Decl::Component(c) => {
                    hir.lowering_migration.used_classic_component_path = true;
                    hir.functions.push(self.lower_fn(&c.func, true));
                    hir.components.push(HirComponent(c.clone()));
                }
                Decl::TypeDef(t) => {
                    hir.types.push(self.lower_typedef(t));
                }
                Decl::HttpRoute(r) => {
                    hir.routes.push(self.lower_route(r));
                }
                Decl::Actor(a) => {
                    hir.actors.push(self.lower_actor(a));
                }
                Decl::Workflow(w) => {
                    hir.workflows.push(self.lower_workflow(w));
                }
                Decl::Activity(a) => {
                    hir.activities.push(self.lower_activity(a));
                }
                Decl::McpTool(m) => {
                    let func = self.lower_fn(&m.func, false);
                    hir.mcp_tools.push(HirMcpTool {
                        description: m.description.clone(),
                        func,
                    });
                }
                Decl::Test(t) => {
                    hir.tests.push(self.lower_fn(&t.func, false));
                }
                // `route_path` is the stable HTTP contract surface for WebIR `RouteNode` / client stubs.
                Decl::ServerFn(s) => {
                    let lowered = self.lower_fn(&s.func, false);
                    let route_path = format!("{SERVER_FN_API_PREFIX}{}", lowered.name);
                    hir.server_fns.push(HirServerFn {
                        id: lowered.id,
                        name: lowered.name.clone(),
                        params: lowered.params.clone(),
                        return_type: lowered.return_type.clone(),
                        body: lowered.body.clone(),
                        route_path,
                        span: lowered.span,
                    });
                }
                // Query / mutation RPC paths feed generated client helpers; WebIR target maps these on `RouteNode`.
                Decl::Query(q) => {
                    let lowered = self.lower_fn(&q.func, false);
                    let route_path = format!("{QUERY_FN_API_PREFIX}{}", lowered.name);
                    hir.query_fns.push(HirServerFn {
                        id: lowered.id,
                        name: lowered.name.clone(),
                        params: lowered.params.clone(),
                        return_type: lowered.return_type.clone(),
                        body: lowered.body.clone(),
                        route_path,
                        span: lowered.span,
                    });
                }
                Decl::Mutation(m) => {
                    let lowered = self.lower_fn(&m.func, false);
                    let route_path = format!("{MUTATION_FN_API_PREFIX}{}", lowered.name);
                    hir.mutation_fns.push(HirServerFn {
                        id: lowered.id,
                        name: lowered.name.clone(),
                        params: lowered.params.clone(),
                        return_type: lowered.return_type.clone(),
                        body: lowered.body.clone(),
                        route_path,
                        span: lowered.span,
                    });
                }
                Decl::Table(t) => {
                    hir.tables.push(self.lower_table(t));
                }
                Decl::Index(idx) => {
                    hir.indexes.push(HirIndex {
                        table_name: idx.table_name.clone(),
                        index_name: idx.index_name.clone(),
                        columns: idx.columns.clone(),
                        span: idx.span,
                    });
                }
                Decl::Collection(c) => {
                    hir.collections.push(self.lower_collection(c));
                }
                Decl::VectorIndex(v) => {
                    hir.vector_indexes.push(HirVectorIndex {
                        table_name: v.table_name.clone(),
                        index_name: v.index_name.clone(),
                        column: v.column.clone(),
                        dimensions: v.dimensions,
                        filter_fields: v.filter_fields.clone(),
                        span: v.span,
                    });
                }
                Decl::SearchIndex(s) => {
                    hir.search_indexes.push(HirSearchIndex {
                        table_name: s.table_name.clone(),
                        index_name: s.index_name.clone(),
                        search_field: s.search_field.clone(),
                        filter_fields: s.filter_fields.clone(),
                        span: s.span,
                    });
                }
                Decl::V0Component(decl) => {
                    hir.v0_components.push(HirV0Component(decl.clone()));
                }
                // Client route tree: lowered verbatim for TanStack / WebIR route contracts.
                Decl::Routes(decl) => {
                    hir.client_routes.push(HirRoutes(decl.clone()));
                }
                // Island prop optionality (`prop?: T`) is preserved on AST `IslandDecl` for mount codegen + WebIR mounts.
                Decl::Island(decl) => {
                    hir.islands.push(HirIsland(decl.clone()));
                }
                Decl::Layout(decl) => {
                    hir.layouts.push(HirLayout(decl.clone()));
                }
                Decl::Page(decl) => {
                    hir.pages.push(HirPage(decl.clone()));
                }
                Decl::Context(decl) => {
                    hir.contexts.push(HirContext(decl.clone()));
                }
                // Declarative hooks: retained AST for TS emit; flagged for escape-hatch / token accounting (OP-0042).
                Decl::Hook(decl) => {
                    hir.lowering_migration.has_legacy_hook_surfaces = true;
                    hir.hooks.push(HirHook(decl.clone()));
                }
                Decl::ErrorBoundary(decl) => {
                    hir.error_boundaries.push(HirErrorBoundary(decl.clone()));
                }
                Decl::Loading(decl) => {
                    hir.loadings.push(HirLoading(decl.clone()));
                }
                Decl::NotFound(decl) => {
                    hir.not_founds.push(HirNotFound(decl.clone()));
                }
                // Path C reactive: primary source for WebIR `view_roots` + `behavior_nodes`.
                Decl::ReactiveComponent(decl) => {
                    hir.lowering_migration.used_reactive_component_path = true;
                    hir.reactive_components
                        .push(self.lower_reactive_component(decl));
                }
                _ => {
                    hir.legacy_ast_nodes.push(decl.clone());
                }
            }
        }

        // Analyze async logic
        for f in &mut hir.functions {
            if async_flags::has_async_stmts(&f.body) {
                f.is_async = true;
            }
        }
        for t in &mut hir.tests {
            if async_flags::has_async_stmts(&t.body) {
                t.is_async = true;
            }
        }

        db_select_normalize::normalize_db_select_projections(&mut hir);

        hir
    }
}

fn collect_pattern_binding_names(pat: &Pattern, out: &mut HashSet<String>) {
    match pat {
        Pattern::Ident { name, .. } => {
            out.insert(name.clone());
        }
        Pattern::Tuple { elements, .. } => {
            for e in elements {
                collect_pattern_binding_names(e, out);
            }
        }
        Pattern::Constructor { fields, .. } => {
            for f in fields {
                collect_pattern_binding_names(f, out);
            }
        }
        Pattern::Wildcard { .. } | Pattern::Literal { .. } => {}
    }
}

/// Lower the JSX/TSX view of a classic `@component fn` when the body follows the TS emit shape:
/// leading `let` / `assign` statements, then a JSX [`Expr`] as a statement expression or `ret`
/// value.
///
/// Returns the lowered [`HirExpr`] plus names to thread as `state_names` into Web IR DOM lowering
/// (parameters plus bindings introduced before the view).
#[must_use]
pub fn lower_classic_component_view(comp: &HirComponent) -> Option<(HirExpr, HashSet<String>)> {
    let func = &comp.0.func;
    let mut ctx = LowerCtx::new();
    let mut state_names = HashSet::new();

    for p in &func.params {
        ctx.def_map.define(p.name.clone());
        state_names.insert(p.name.clone());
    }

    let mut jsx_ast: Option<&Expr> = None;

    for stmt in &func.body {
        match stmt {
            Stmt::Let { pattern, .. } => {
                collect_pattern_binding_names(pattern, &mut state_names);
                let _ = ctx.lower_stmt(stmt);
            }
            Stmt::Assign { .. } => {
                let _ = ctx.lower_stmt(stmt);
            }
            Stmt::Expr { expr, .. } => {
                if matches!(expr, Expr::Jsx(_) | Expr::JsxSelfClosing(_)) {
                    jsx_ast = Some(expr);
                    break;
                }
                let _ = ctx.lower_stmt(stmt);
            }
            Stmt::Return { value: Some(v), .. } => {
                if matches!(v, Expr::Jsx(_) | Expr::JsxSelfClosing(_)) {
                    jsx_ast = Some(v);
                    break;
                }
            }
            Stmt::Return { value: None, .. } => {}
        }
    }

    jsx_ast.map(|e| (ctx.lower_expr(e), state_names))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::cursor::lex;
    use crate::parser::parse;
    use crate::web_prefixes::{MUTATION_FN_API_PREFIX, QUERY_FN_API_PREFIX, SERVER_FN_API_PREFIX};

    fn lower_str(source: &str) -> HirModule {
        let tokens = lex(source);
        let module = parse(tokens).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
        lower_module(&module)
    }

    /// Fully lowered web constructs must not pile into `legacy_ast_nodes` (Path C / HIR bridge).
    #[test]
    fn hir_lowering_leaves_no_legacy_nodes_for_core_web_decls() {
        let src = r#"
import react.use_state

@table type Task { title: str done: bool }

http post "/chat" to Result { ret Ok(0) }

@server fn doThing(x: int) to int { ret x }

@component TaskView() {
  state done: bool = false
  view: <span>{done}</span>
}
"#;
        let hir = lower_str(src);
        assert!(
            hir.legacy_ast_nodes.is_empty(),
            "expected no legacy AST decls, got {:?}",
            hir.legacy_ast_nodes
        );
        assert_eq!(hir.tables.len(), 1);
        assert_eq!(hir.routes.len(), 1);
        assert_eq!(hir.server_fns.len(), 1);
        assert_eq!(hir.reactive_components.len(), 1);
        assert_eq!(hir.routes[0].route_contract, "POST /chat");
        assert!(
            hir.lowering_migration.used_reactive_component_path,
            "expected Path C reactive migration flag"
        );
        assert!(
            !hir.lowering_migration.used_classic_component_path,
            "@component in fixture should not set classic path (reactive @component)"
        );
        assert_eq!(
            hir.server_fns[0].route_path,
            format!("{SERVER_FN_API_PREFIX}{}", "doThing")
        );
    }

    /// Islands, `routes { ... }`, and reactive components populate `HirModule`; full module must
    /// [`crate::web_ir::lower::lower_hir_to_web_ir`] + validate without diagnostics (blueprint OP-0035, OP-0039).
    #[test]
    fn hir_island_routes_reactive_surface_validates_as_web_ir() {
        let src = r#"
import react.use_state

@island Chart {
    title: str
    data: str
    width?: int
}

@component Dash() {
    state n: int = 0
    view: <div class="dashboard">{n}</div>
}

routes {
    "/" to Dash
}
"#;
        let hir = lower_str(src);
        assert!(
            hir.legacy_ast_nodes.is_empty(),
            "unexpected legacy: {:?}",
            hir.legacy_ast_nodes
        );
        assert_eq!(hir.islands.len(), 1);
        assert_eq!(hir.islands[0].0.name, "Chart");
        assert_eq!(hir.islands[0].0.props.len(), 3);
        assert!(hir.islands[0].0.props[2].is_optional);
        assert_eq!(hir.islands[0].0.props[2].name, "width");
        assert_eq!(hir.client_routes.len(), 1);
        assert_eq!(hir.client_routes[0].0.entries.len(), 1);
        assert_eq!(hir.client_routes[0].0.entries[0].path, "/");
        assert_eq!(hir.client_routes[0].0.entries[0].component_name, "Dash");
        assert_eq!(hir.reactive_components.len(), 1);

        let web = crate::web_ir::lower::lower_hir_to_web_ir(&hir);
        let diags = crate::web_ir::validate::validate_web_ir(&web);
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn golden_crud_api_vox_lowers_without_legacy_nodes() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/golden/crud_api.vox"
        ));
        let tokens = lex(src);
        let module = parse(tokens).expect("examples/golden/crud_api.vox must parse");
        let hir = lower_module(&module);
        assert!(
            hir.legacy_ast_nodes.is_empty(),
            "unexpected legacy AST: {:?}",
            hir.legacy_ast_nodes
        );
        assert_eq!(hir.tables.len(), 1);
        assert_eq!(hir.query_fns.len(), 2);
        assert_eq!(hir.mutation_fns.len(), 1);
        assert_eq!(hir.routes.len(), 1);
    }

    #[test]
    fn hir_lowering_db_filter_becomes_filter_record_ir() {
        let src = r#"
@table type User { name: str active: bool }
fn f() to int {
    ret len(db.User.filter({ active: true }))
}
"#;
        let hir = lower_str(src);
        assert!(hir.legacy_ast_nodes.is_empty());
        let body = &hir.functions[0].body;
        let mut found = false;
        for st in body {
            if let crate::hir::HirStmt::Return { value: Some(e), .. } = st {
                if let crate::hir::HirExpr::Call(callee, cargs, _, _) = e {
                    if let crate::hir::HirExpr::Ident(name, _) = callee.as_ref() {
                        if name == "len" && cargs.len() == 1 {
                            if let crate::hir::HirExpr::DbTableOp { op, .. } = &cargs[0].value {
                                if *op == crate::hir::HirDbTableOp::FilterRecord {
                                    found = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(found, "expected FilterRecord in len(db.User.filter(...))");
    }

    #[test]
    fn hir_lowering_db_filter_count_chain_becomes_count_with_filter_args() {
        let src = r#"
@table type User { name: str active: bool }
fn f() to int {
    ret db.User.filter({ active: true }).count()
}
"#;
        let hir = lower_str(src);
        assert!(hir.legacy_ast_nodes.is_empty());
        let body = &hir.functions[0].body;
        let mut found = false;
        for st in body {
            if let crate::hir::HirStmt::Return { value: Some(e), .. } = st {
                if let crate::hir::HirExpr::DbTableOp { op, args, .. } = e {
                    if *op == crate::hir::HirDbTableOp::Count && args.len() == 1 {
                        found = true;
                    }
                }
            }
        }
        assert!(
            found,
            "expected filter(...).count() to lower to DbTableOp::Count with filter args"
        );
    }

    #[test]
    fn hir_lowering_db_filter_order_limit_chain_keeps_modifiers() {
        let src = r#"
@table type User { name: str active: bool }
fn f() to Unit {
    db.User.filter({ active: true }).order_by("name", "desc").limit(5)
}
"#;
        let hir = lower_str(src);
        assert!(hir.legacy_ast_nodes.is_empty());
        let body = &hir.functions[0].body;
        let mut found = false;
        for st in body {
            if let crate::hir::HirStmt::Expr { expr, .. } = st {
                if let crate::hir::HirExpr::DbTableOp {
                    op,
                    order_by,
                    limit,
                    ..
                } = expr
                {
                    if *op == crate::hir::HirDbTableOp::FilterRecord
                        && matches!(order_by, Some((col, false)) if col == "name")
                        && limit.is_some()
                    {
                        found = true;
                    }
                }
            }
        }
        assert!(found, "expected DbTableOp with order_by+limit modifiers");
    }

    #[test]
    fn hir_lowering_db_all_select_sets_projection() {
        let src = r#"
@table type User { name: str active: bool }
fn f() to int {
    ret len(db.User.all().select("name", "active"))
}
"#;
        let hir = lower_str(src);
        assert!(hir.legacy_ast_nodes.is_empty());
        let body = &hir.functions[0].body;
        let mut found = false;
        for st in body {
            if let crate::hir::HirStmt::Return { value: Some(e), .. } = st {
                if let crate::hir::HirExpr::Call(callee, cargs, ..) = e {
                    if let crate::hir::HirExpr::Ident(fn_name, _) = callee.as_ref() {
                        if fn_name == "len" && cargs.len() == 1 {
                            if let crate::hir::HirExpr::DbTableOp {
                                op, select_cols, ..
                            } = &cargs[0].value
                            {
                                if *op == crate::hir::HirDbTableOp::All
                                    && select_cols.as_ref().is_some_and(|c| {
                                        c.len() == 2 && c[0] == "name" && c[1] == "active"
                                    })
                                {
                                    found = true;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(found, "expected All with select_cols on db chain");
    }

    #[test]
    fn hir_lowering_db_where_object_builds_predicate_plan() {
        let src = r#"
@table type User { name: str age: int active: bool }
fn f() to int {
    ret len(db.User.where({ age: { gte: 18 }, active: { eq: true } }))
}
"#;
        let hir = lower_str(src);
        assert!(hir.legacy_ast_nodes.is_empty());
        let body = &hir.functions[0].body;
        let mut found = false;
        for st in body {
            if let crate::hir::HirStmt::Return { value: Some(e), .. } = st
                && let crate::hir::HirExpr::Call(_, cargs, _, _) = e
                && let crate::hir::HirExpr::DbTableOp { plan, .. } = &cargs[0].value
            {
                if let Some(p) = plan {
                    found = matches!(
                        p.predicate,
                        Some(crate::hir::HirDbPredicate::And(ref parts)) if parts.len() == 2
                    );
                }
            }
        }
        assert!(found, "expected where(...) predicate in DbQueryPlan");
    }

    #[test]
    fn hir_lowering_db_plan_capabilities_parse_chain_modifiers() {
        let src = r#"
@table type User { name: str active: bool }
fn f() to Unit {
    db.User.filter({ active: true }).using("hybrid").live("users.active").scope("populi").sync().limit(5)
}
"#;
        let hir = lower_str(src);
        assert!(hir.legacy_ast_nodes.is_empty());
        let body = &hir.functions[0].body;
        let mut found = false;
        for st in body {
            if let crate::hir::HirStmt::Expr { expr, .. } = st
                && let crate::hir::HirExpr::DbTableOp { plan, .. } = expr
                && let Some(plan) = plan
            {
                found = plan.capabilities.requires_sync
                    && plan.capabilities.live_topic.as_deref() == Some("users.active")
                    && plan.capabilities.orchestration_scope.as_deref() == Some("populi")
                    && matches!(
                        plan.capabilities.retrieval_mode,
                        Some(crate::hir::HirDbRetrievalMode::Hybrid)
                    );
            }
        }
        assert!(
            found,
            "expected chain modifiers to populate plan capabilities"
        );
    }

    #[test]
    fn hir_lowering_maps_query_and_mutation_decls() {
        let src = r#"
@table type User { name: str active: bool }
@query fn q1() to int { ret 0 }
@mutation fn m1() to Unit {
    db.User.insert({ name: "a", active: true })
}
"#;
        let hir = lower_str(src);
        assert!(hir.legacy_ast_nodes.is_empty());
        assert_eq!(hir.query_fns.len(), 1);
        assert_eq!(hir.mutation_fns.len(), 1);
        assert_eq!(
            hir.query_fns[0].route_path,
            format!("{QUERY_FN_API_PREFIX}q1")
        );
        assert_eq!(
            hir.mutation_fns[0].route_path,
            format!("{MUTATION_FN_API_PREFIX}m1")
        );
    }

    /// Collection / vector / search declarations must lower to HIR vectors (not `legacy_ast_nodes`).
    #[test]
    fn hir_lowering_maps_collection_vector_search_out_of_legacy() {
        use crate::ast::decl::{
            CollectionDecl, Decl, Module, SearchIndexDecl, TableDecl, TableField, VectorIndexDecl,
        };
        use crate::ast::span::Span;
        use crate::ast::types::TypeExpr;

        let sp = Span::new(0, 0);
        let table = TableDecl {
            name: "Doc".into(),
            fields: vec![TableField {
                name: "title".into(),
                type_ann: TypeExpr::Named {
                    name: "str".into(),
                    span: sp,
                },
                description: None,
                span: sp,
            }],
            description: None,
            json_layout: None,
            auth_provider: None,
            roles: vec![],
            cors: None,
            is_pub: true,
            is_deprecated: false,
            span: sp,
        };
        let col = CollectionDecl {
            name: "Notes".into(),
            fields: vec![],
            description: None,
            is_pub: false,
            has_spread: false,
            span: sp,
        };
        let vix = VectorIndexDecl {
            table_name: "Doc".into(),
            index_name: "emb".into(),
            column: "v".into(),
            dimensions: 384,
            filter_fields: vec![],
            span: sp,
        };
        let six = SearchIndexDecl {
            table_name: "Doc".into(),
            index_name: "titles".into(),
            search_field: "title".into(),
            filter_fields: vec![],
            span: sp,
        };
        let module = Module {
            declarations: vec![
                Decl::Table(table),
                Decl::Collection(col),
                Decl::VectorIndex(vix),
                Decl::SearchIndex(six),
            ],
            span: sp,
        };
        let hir = lower_module(&module);
        assert!(
            hir.legacy_ast_nodes.is_empty(),
            "legacy_ast_nodes should not contain db index decls, got {:?}",
            hir.legacy_ast_nodes
        );
        assert_eq!(hir.collections.len(), 1);
        assert_eq!(hir.vector_indexes.len(), 1);
        assert_eq!(hir.search_indexes.len(), 1);
    }
}
