use std::collections::HashMap;

use crate::ast::decl::*;
use crate::hir::def_map::DefMap;
use crate::hir::*;
use crate::web_prefixes::{
    MUTATION_FN_API_PREFIX, QUERY_FN_API_PREFIX, SERVER_FN_API_PREFIX,
};

mod async_flags;
mod decl;
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
        };

        for decl in &module.declarations {
            match decl {
                Decl::Import(imp) => {
                    for path in &imp.paths {
                        let (mod_path, item) = if path.segments.len() > 1 {
                            let item = path.segments.last().unwrap().clone();
                            let mod_path = path.segments[..path.segments.len() - 1].to_vec();
                            (mod_path, item)
                        } else {
                            (vec![], path.segments[0].clone())
                        };
                        hir.imports.push(HirImport {
                            module_path: mod_path,
                            item,
                            span: path.span,
                        });
                    }
                }
                Decl::Function(f) => {
                    hir.functions.push(self.lower_fn(f, false));
                }
                Decl::Component(c) => {
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
                Decl::Routes(decl) => {
                    hir.client_routes.push(HirRoutes(decl.clone()));
                }
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
                Decl::Hook(decl) => {
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
                Decl::ReactiveComponent(decl) => {
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

        normalize_db_select_projections(&mut hir);

        hir
    }
}

fn reorder_db_select_cols(field_names: &[String], cols: &mut Vec<String>) {
    let ordered: Vec<String> = field_names
        .iter()
        .filter(|n| cols.iter().any(|c| c == *n))
        .cloned()
        .collect();
    if ordered.len() == cols.len() {
        *cols = ordered;
    }
}

fn normalize_db_select_projections(hir: &mut HirModule) {
    let field_order: HashMap<String, Vec<String>> = hir
        .tables
        .iter()
        .map(|t| {
            (
                t.name.clone(),
                t.fields.iter().map(|f| f.name.clone()).collect(),
            )
        })
        .collect();

    for f in &mut hir.functions {
        normalize_stmts_select(&mut f.body, &field_order);
    }
    for f in &mut hir.tests {
        normalize_stmts_select(&mut f.body, &field_order);
    }
    for r in &mut hir.routes {
        normalize_stmts_select(&mut r.body, &field_order);
    }
    for w in &mut hir.workflows {
        normalize_stmts_select(&mut w.body, &field_order);
    }
    for a in &mut hir.activities {
        normalize_stmts_select(&mut a.body, &field_order);
    }
    for sf in &mut hir.server_fns {
        normalize_stmts_select(&mut sf.body, &field_order);
    }
    for qf in &mut hir.query_fns {
        normalize_stmts_select(&mut qf.body, &field_order);
    }
    for mf in &mut hir.mutation_fns {
        normalize_stmts_select(&mut mf.body, &field_order);
    }
    for actor in &mut hir.actors {
        for h in &mut actor.handlers {
            normalize_stmts_select(&mut h.body, &field_order);
        }
    }
    for tool in &mut hir.mcp_tools {
        normalize_stmts_select(&mut tool.func.body, &field_order);
    }
}

fn normalize_stmts_select(stmts: &mut [HirStmt], field_order: &HashMap<String, Vec<String>>) {
    for s in stmts.iter_mut() {
        normalize_stmt_select(s, field_order);
    }
}

fn normalize_stmt_select(stmt: &mut HirStmt, field_order: &HashMap<String, Vec<String>>) {
    match stmt {
        HirStmt::Let { value, .. } => normalize_expr_select(value, field_order),
        HirStmt::Assign { target, value, .. } => {
            normalize_expr_select(target, field_order);
            normalize_expr_select(value, field_order);
        }
        HirStmt::Return { value: Some(v), .. } => normalize_expr_select(v, field_order),
        HirStmt::Return { value: None, .. } => {}
        HirStmt::Expr { expr, .. } => normalize_expr_select(expr, field_order),
    }
}

fn normalize_expr_select(expr: &mut HirExpr, field_order: &HashMap<String, Vec<String>>) {
    match expr {
        HirExpr::DbTableOp {
            table,
            select_cols,
            args,
            limit,
            plan,
            ..
        } => {
            if let Some(cols) = select_cols {
                if let Some(order) = field_order.get(table) {
                    reorder_db_select_cols(order, cols);
                }
            }
            if let Some(p) = plan
                && let Some(cols) = p.projection.as_mut()
                && let Some(order) = field_order.get(table)
            {
                reorder_db_select_cols(order, cols);
            }
            for a in args.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
            if let Some(l) = limit.as_mut() {
                normalize_expr_select(l.as_mut(), field_order);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields.iter_mut() {
                normalize_expr_select(v, field_order);
            }
        }
        HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
            for it in items.iter_mut() {
                normalize_expr_select(it, field_order);
            }
        }
        HirExpr::Binary(_, l, r, _) => {
            normalize_expr_select(l.as_mut(), field_order);
            normalize_expr_select(r.as_mut(), field_order);
        }
        HirExpr::Unary(_, o, _) => normalize_expr_select(o.as_mut(), field_order),
        HirExpr::Call(callee, args, _, _) => {
            normalize_expr_select(callee.as_mut(), field_order);
            for a in args.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
        }
        HirExpr::MethodCall(obj, _, args, _) => {
            normalize_expr_select(obj.as_mut(), field_order);
            for a in args.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
        }
        HirExpr::FieldAccess(o, _, _) => normalize_expr_select(o.as_mut(), field_order),
        HirExpr::Match(subj, arms, _) => {
            normalize_expr_select(subj.as_mut(), field_order);
            for arm in arms.iter_mut() {
                if let Some(g) = arm.guard.as_mut() {
                    normalize_expr_select(g.as_mut(), field_order);
                }
                normalize_expr_select(arm.body.as_mut(), field_order);
            }
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            normalize_expr_select(cond.as_mut(), field_order);
            normalize_stmts_select(then_b, field_order);
            if let Some(else_stmts) = else_b.as_mut() {
                normalize_stmts_select(else_stmts, field_order);
            }
        }
        HirExpr::For(_, it, body, _) => {
            normalize_expr_select(it.as_mut(), field_order);
            normalize_expr_select(body.as_mut(), field_order);
        }
        HirExpr::Lambda(_, _, body, _) => normalize_expr_select(body.as_mut(), field_order),
        HirExpr::Pipe(l, r, _) => {
            normalize_expr_select(l.as_mut(), field_order);
            normalize_expr_select(r.as_mut(), field_order);
        }
        HirExpr::Spawn(t, _) => normalize_expr_select(t.as_mut(), field_order),
        HirExpr::With(b, o, _) => {
            normalize_expr_select(b.as_mut(), field_order);
            normalize_expr_select(o.as_mut(), field_order);
        }
        HirExpr::Jsx(el) => {
            for a in el.attributes.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
            for c in el.children.iter_mut() {
                normalize_expr_select(c, field_order);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for a in el.attributes.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
        }
        HirExpr::Block(stmts, _) => normalize_stmts_select(stmts, field_order),
        HirExpr::IntLit(_, _)
        | HirExpr::FloatLit(_, _)
        | HirExpr::StringLit(_, _)
        | HirExpr::BoolLit(_, _)
        | HirExpr::Ident(_, _) => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::cursor::lex;
    use crate::parser::parse;
    use crate::web_prefixes::{
        MUTATION_FN_API_PREFIX, QUERY_FN_API_PREFIX, SERVER_FN_API_PREFIX,
    };

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
        assert_eq!(
            hir.server_fns[0].route_path,
            format!("{SERVER_FN_API_PREFIX}{}", "doThing")
        );
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
                                op,
                                select_cols,
                                ..
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
        assert!(found, "expected chain modifiers to populate plan capabilities");
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
