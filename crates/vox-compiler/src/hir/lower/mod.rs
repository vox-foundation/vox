use crate::ast::decl::*;
use crate::hir::def_map::DefMap;
use crate::hir::*;
use crate::web_prefixes::SERVER_FN_API_PREFIX;

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

        hir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::cursor::lex;
    use crate::parser::parse;
    use crate::web_prefixes::SERVER_FN_API_PREFIX;

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
