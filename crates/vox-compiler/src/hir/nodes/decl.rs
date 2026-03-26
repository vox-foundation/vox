//! HIR declarations: module shape, items, routes, tables (OP-0208).

use crate::ast::span::Span;

use super::expr::HirExpr;
use super::stmt::HirStmt;
use super::stmt_expr::{DefId, HirParam, HirType};

/// A fully lowered Vox module: every declaration category is collected into its own vector.
///
/// Empty vectors mean the construct was absent in source; there is no implicit ordering across
/// categories—only within each `Vec` matches source order for that kind.
#[derive(Debug, Clone, Default)]
pub struct HirModule {
    /// Resolved `import` entries.
    pub imports: Vec<HirImport>,
    /// Functions and `@component` bodies.
    pub functions: Vec<HirFn>,
    /// Algebraic and struct types.
    pub types: Vec<HirTypeDef>,
    /// HTTP route handlers.
    pub routes: Vec<HirRoute>,
    /// Actor definitions.
    pub actors: Vec<HirActor>,
    /// Workflow state machines.
    pub workflows: Vec<HirWorkflow>,
    /// Workflow activities.
    pub activities: Vec<HirActivity>,
    /// `@test` functions.
    pub tests: Vec<HirFn>,
    /// `@server` RPC functions.
    pub server_fns: Vec<HirServerFn>,
    /// `@query` read-only DB / API functions (POST JSON body, `/api/query/...`).
    pub query_fns: Vec<HirServerFn>,
    /// `@mutation` write functions (POST JSON body, `/api/mutation/...`).
    pub mutation_fns: Vec<HirServerFn>,
    /// Codex table schemas.
    pub tables: Vec<HirTable>,
    /// Table indexes.
    pub indexes: Vec<HirIndex>,
    /// Document collections (schemaless / doc store).
    pub collections: Vec<HirCollection>,
    /// Vector / embedding indexes.
    pub vector_indexes: Vec<HirVectorIndex>,
    /// Full-text search indexes.
    pub search_indexes: Vec<HirSearchIndex>,
    /// MCP tool handlers.
    pub mcp_tools: Vec<HirMcpTool>,

    // UI & TanStack specific structures (AST-retained for TS codegen migration)
    /// UI Components.
    pub components: Vec<HirComponent>,
    /// Extracted v0 components.
    pub v0_components: Vec<HirV0Component>,
    /// Client-side Routes declaration.
    pub client_routes: Vec<HirRoutes>,
    /// Standalone islands.
    pub islands: Vec<HirIsland>,
    /// Route layouts.
    pub layouts: Vec<HirLayout>,
    /// Route pages.
    pub pages: Vec<HirPage>,
    /// React context wrappers.
    pub contexts: Vec<HirContext>,
    /// React hooks.
    pub hooks: Vec<HirHook>,
    /// Error boundaries.
    pub error_boundaries: Vec<HirErrorBoundary>,
    /// Loading/Suspense fallbacks.
    pub loadings: Vec<HirLoading>,
    /// Not Found views.
    pub not_founds: Vec<HirNotFound>,
    /// Reactive components (Path C).
    pub reactive_components: Vec<HirReactiveComponent>,

    /// Declarations not yet represented as typed HIR vectors (unknown / future decl kinds).
    /// HTTP routes, tables, activities, and `@server` fns are lowered to [`HirRoute`], [`HirTable`],
    /// [`HirActivity`], and [`HirServerFn`]; TS codegen reads those directly (Path C).
    pub legacy_ast_nodes: Vec<crate::ast::decl::Decl>,
}

/// A component lowered to HIR (currently retaining AST for TS codegen until full HirExpr migration).
#[derive(Debug, Clone)]
pub struct HirComponent(pub crate::ast::decl::ComponentDecl);

/// A v0 component lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirV0Component(pub crate::ast::decl::V0ComponentDecl);

/// Routes layout lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirRoutes(pub crate::ast::decl::RoutesDecl);

/// Island component lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirIsland(pub crate::ast::decl::IslandDecl);

/// Route layout component lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirLayout(pub crate::ast::decl::LayoutDecl);

/// Route page component lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirPage(pub crate::ast::decl::PageDecl);

/// Context component lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirContext(pub crate::ast::decl::ContextDecl);

/// Hook component lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirHook(pub crate::ast::decl::HookDecl);

/// Errorboundary lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirErrorBoundary(pub crate::ast::decl::ErrorBoundaryDecl);

/// Loading route lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirLoading(pub crate::ast::decl::LoadingDecl);

/// Not Found view lowered to HIR.
#[derive(Debug, Clone)]
pub struct HirNotFound(pub crate::ast::decl::NotFoundDecl);

/// A resolved import.
#[derive(Debug, Clone)]
pub struct HirImport {
    /// Module path segments (`["react"]`, `["std", "http"]`).
    pub module_path: Vec<String>,
    /// Imported symbol name.
    pub item: String,
    /// Span in source.
    pub span: Span,
}

/// A function or component in HIR.
#[derive(Debug, Clone)]
pub struct HirFn {
    /// Stable definition id.
    pub id: DefId,
    /// Function name.
    pub name: String,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// Parameters with optional types and defaults.
    pub params: Vec<HirParam>,
    /// Return type, if annotated.
    pub return_type: Option<HirType>,
    /// Body statements.
    pub body: Vec<HirStmt>,
    /// Whether this is a `@component`.
    pub is_component: bool,
    /// Async function.
    pub is_async: bool,
    /// Exported from module.
    pub is_pub: bool,
    /// `@deprecated` on the source declaration.
    pub is_deprecated: bool,
    /// Span covering the declaration.
    pub span: Span,
}

/// ADT / type definition in HIR.
#[derive(Debug, Clone)]
pub struct HirTypeDef {
    /// Type definition id.
    pub id: DefId,
    /// Type name.
    pub name: String,
    /// Variants for sum types; empty for aliases/structs handled elsewhere.
    pub variants: Vec<HirVariant>,
    /// Exported type.
    pub is_pub: bool,
    /// Span covering the definition.
    pub span: Span,
}

/// One variant of an ADT.
#[derive(Debug, Clone)]
pub struct HirVariant {
    /// Constructor name.
    pub name: String,
    /// Named fields with types.
    pub fields: Vec<(String, HirType)>,
    /// Span covering the variant.
    pub span: Span,
}

/// HTTP route in HIR.
#[derive(Debug, Clone)]
pub struct HirRoute {
    /// HTTP method.
    pub method: HirHttpMethod,
    /// Path pattern string.
    pub path: String,
    /// Declared response type.
    pub return_type: Option<HirType>,
    /// Handler body.
    pub body: Vec<HirStmt>,
    /// Span covering the route.
    pub span: Span,
}

/// HTTP methods for [`HirRoute`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirHttpMethod {
    /// GET
    Get,
    /// POST
    Post,
    /// PUT
    Put,
    /// DELETE
    Delete,
}

/// Actor definition in HIR.
#[derive(Debug, Clone)]
pub struct HirActor {
    /// Actor type id.
    pub id: DefId,
    /// Actor name.
    pub name: String,
    /// Message handlers.
    pub handlers: Vec<HirActorHandler>,
    /// Span covering the actor.
    pub span: Span,
}

/// Single actor handler.
#[derive(Debug, Clone)]
pub struct HirActorHandler {
    /// Event name.
    pub event_name: String,
    /// Parameters.
    pub params: Vec<HirParam>,
    /// Return type.
    pub return_type: Option<HirType>,
    /// Handler body.
    pub body: Vec<HirStmt>,
    /// Span covering the handler.
    pub span: Span,
}

/// Workflow definition in HIR.
#[derive(Debug, Clone)]
pub struct HirWorkflow {
    /// Workflow id.
    pub id: DefId,
    /// Workflow name.
    pub name: String,
    /// Input parameters.
    pub params: Vec<HirParam>,
    /// Output type.
    pub return_type: Option<HirType>,
    /// Workflow body.
    pub body: Vec<HirStmt>,
    /// Span covering the workflow.
    pub span: Span,
}

/// Activity definition in HIR.
#[derive(Debug, Clone)]
pub struct HirActivity {
    /// Activity id.
    pub id: DefId,
    /// Activity name.
    pub name: String,
    /// Parameters.
    pub params: Vec<HirParam>,
    /// Return type.
    pub return_type: Option<HirType>,
    /// Activity body.
    pub body: Vec<HirStmt>,
    /// Span covering the activity.
    pub span: Span,
}

/// A server function — callable from the frontend, auto-generates API route + fetch wrapper.
#[derive(Debug, Clone)]
pub struct HirServerFn {
    /// Function id.
    pub id: DefId,
    /// Exposed name.
    pub name: String,
    /// Parameters.
    pub params: Vec<HirParam>,
    /// Return type.
    pub return_type: Option<HirType>,
    /// Function body.
    pub body: Vec<HirStmt>,
    /// HTTP path bound to this server fn.
    pub route_path: String,
    /// Span covering the declaration.
    pub span: Span,
}

/// Table definition — a persistent record type.
#[derive(Debug, Clone)]
pub struct HirTable {
    /// Table id.
    pub id: DefId,
    /// Table name.
    pub name: String,
    /// Columns.
    pub fields: Vec<HirTableField>,
    /// Public API surface.
    pub is_pub: bool,
    /// `@deprecated` table.
    pub is_deprecated: bool,
    /// Span covering the table.
    pub span: Span,
}

/// A field within a table definition.
#[derive(Debug, Clone)]
pub struct HirTableField {
    /// Column name.
    pub name: String,
    /// Column type.
    pub type_ann: HirType,
    /// Span covering the field.
    pub span: Span,
}

/// Index definition for a table.
#[derive(Debug, Clone)]
pub struct HirIndex {
    /// Table being indexed.
    pub table_name: String,
    /// Index name.
    pub index_name: String,
    /// Indexed columns.
    pub columns: Vec<String>,
    /// Span covering the index decl.
    pub span: Span,
}

/// Document collection — fields mirror [`crate::ast::decl::CollectionDecl`].
#[derive(Debug, Clone)]
pub struct HirCollection {
    pub id: DefId,
    pub name: String,
    pub fields: Vec<HirTableField>,
    pub is_pub: bool,
    pub has_spread: bool,
    pub span: Span,
}

/// Vector index metadata lowered from AST.
#[derive(Debug, Clone)]
pub struct HirVectorIndex {
    pub table_name: String,
    pub index_name: String,
    pub column: String,
    pub dimensions: u32,
    pub filter_fields: Vec<String>,
    pub span: Span,
}

/// Full-text search index lowered from AST.
#[derive(Debug, Clone)]
pub struct HirSearchIndex {
    pub table_name: String,
    pub index_name: String,
    pub search_field: String,
    pub filter_fields: Vec<String>,
    pub span: Span,
}

/// MCP tool declaration — a function exposed via the Model Context Protocol.
#[derive(Debug, Clone)]
pub struct HirMcpTool {
    /// Tool description for MCP clients.
    pub description: String,
    /// Underlying implementation.
    pub func: HirFn,
}

/// Reactive component lowered to HIR (Path C).
#[derive(Debug, Clone)]
pub struct HirReactiveComponent {
    pub id: DefId,
    pub name: String,
    pub params: Vec<HirParam>,
    pub members: Vec<HirReactiveMember>,
    pub view: Option<HirExpr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum HirReactiveMember {
    State(HirState),
    Derived(HirDerived),
    Effect(HirEffect),
    OnMount(HirOnMount),
    OnCleanup(HirOnCleanup),
}

#[derive(Debug, Clone)]
pub struct HirState {
    pub id: DefId,
    pub name: String,
    pub ty: Option<HirType>,
    pub init: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirDerived {
    pub id: DefId,
    pub name: String,
    pub ty: Option<HirType>,
    pub expr: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirEffect {
    pub body: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirOnMount {
    pub body: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct HirOnCleanup {
    pub body: HirExpr,
    pub span: Span,
}
