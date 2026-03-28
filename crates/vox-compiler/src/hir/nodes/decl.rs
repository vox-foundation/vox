//! HIR declarations: module shape, items, routes, tables (OP-0208).

use crate::ast::span::Span;

use super::expr::HirExpr;
use super::stmt::HirStmt;
use super::stmt_expr::{DefId, HirParam, HirType};

/// Ownership buckets used to classify [`HirModule`] fields during the WebIR/AppContract migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirFieldOwnership {
    /// Stable semantic program representation that should remain in post-migration HIR.
    SemanticCore,
    /// Transitional data retained only to bridge legacy emit paths.
    MigrationOnly,
    /// Data that can be projected into an externalized app surface contract.
    AppContract,
}

/// Tracks which HIR lowering paths ran so migrations and WebIR handoff can account for
/// dual component stacks (`@component` vs Path C reactive) and declarative hook surfaces (OP-0036, OP-0042).
#[derive(Debug, Clone, Default)]
pub struct HirLoweringMigrationFlags {
    /// Legacy `@component fn` / `Decl::Component` lowered into [`HirModule::components`].
    pub used_classic_component_path: bool,
    /// Path C `component Name() { ... }` reactive declarations lowered into [`HirModule::reactive_components`].
    pub used_reactive_component_path: bool,
    /// `Decl::Hook` / `@hook` entries retained for TS hooks (escape-hatch accounting).
    pub has_legacy_hook_surfaces: bool,
}

/// A fully lowered Vox module: every declaration category is collected into its own vector.
///
/// Empty vectors mean the construct was absent in source; there is no implicit ordering across
/// categories—only within each `Vec` matches source order for that kind.
#[derive(Debug, Clone, Default)]
pub struct HirModule {
    /// Resolved `import` entries.
    pub imports: Vec<HirImport>,
    /// Rust crate imports declared in source.
    pub rust_imports: Vec<HirRustImport>,
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

    /// Which declarative lowering paths were exercised (classic vs reactive UI, hooks).
    pub lowering_migration: HirLoweringMigrationFlags,
}

/// Snapshot of a post-migration semantic-only HIR shape.
///
/// This is intentionally a strict subset of [`HirModule`]: migration-only fields are dropped and
/// app-surface fields can be projected into `app_contract` while preserving semantic-core data.
#[derive(Debug, Clone, Default)]
pub struct SemanticHirModule {
    pub imports: Vec<HirImport>,
    pub rust_imports: Vec<HirRustImport>,
    pub functions: Vec<HirFn>,
    pub types: Vec<HirTypeDef>,
    pub routes: Vec<HirRoute>,
    pub actors: Vec<HirActor>,
    pub workflows: Vec<HirWorkflow>,
    pub activities: Vec<HirActivity>,
    pub tests: Vec<HirFn>,
    pub server_fns: Vec<HirServerFn>,
    pub query_fns: Vec<HirServerFn>,
    pub mutation_fns: Vec<HirServerFn>,
    pub tables: Vec<HirTable>,
    pub indexes: Vec<HirIndex>,
    pub collections: Vec<HirCollection>,
    pub vector_indexes: Vec<HirVectorIndex>,
    pub search_indexes: Vec<HirSearchIndex>,
    pub mcp_tools: Vec<HirMcpTool>,
    pub reactive_components: Vec<HirReactiveComponent>,
}

impl HirModule {
    /// Return a fixed ownership map for all top-level [`HirModule`] vectors/fields.
    #[must_use]
    pub fn field_ownership_map() -> Vec<(&'static str, HirFieldOwnership)> {
        vec![
            ("imports", HirFieldOwnership::SemanticCore),
            ("rust_imports", HirFieldOwnership::SemanticCore),
            ("functions", HirFieldOwnership::SemanticCore),
            ("types", HirFieldOwnership::SemanticCore),
            ("routes", HirFieldOwnership::AppContract),
            ("actors", HirFieldOwnership::SemanticCore),
            ("workflows", HirFieldOwnership::SemanticCore),
            ("activities", HirFieldOwnership::SemanticCore),
            ("tests", HirFieldOwnership::SemanticCore),
            ("server_fns", HirFieldOwnership::AppContract),
            ("query_fns", HirFieldOwnership::AppContract),
            ("mutation_fns", HirFieldOwnership::AppContract),
            ("tables", HirFieldOwnership::SemanticCore),
            ("indexes", HirFieldOwnership::SemanticCore),
            ("collections", HirFieldOwnership::SemanticCore),
            ("vector_indexes", HirFieldOwnership::SemanticCore),
            ("search_indexes", HirFieldOwnership::SemanticCore),
            ("mcp_tools", HirFieldOwnership::SemanticCore),
            ("components", HirFieldOwnership::MigrationOnly),
            ("v0_components", HirFieldOwnership::MigrationOnly),
            ("client_routes", HirFieldOwnership::AppContract),
            ("islands", HirFieldOwnership::AppContract),
            ("layouts", HirFieldOwnership::MigrationOnly),
            ("pages", HirFieldOwnership::MigrationOnly),
            ("contexts", HirFieldOwnership::MigrationOnly),
            ("hooks", HirFieldOwnership::MigrationOnly),
            ("error_boundaries", HirFieldOwnership::MigrationOnly),
            ("loadings", HirFieldOwnership::MigrationOnly),
            ("not_founds", HirFieldOwnership::MigrationOnly),
            ("reactive_components", HirFieldOwnership::SemanticCore),
            ("legacy_ast_nodes", HirFieldOwnership::MigrationOnly),
            ("lowering_migration", HirFieldOwnership::MigrationOnly),
        ]
    }

    /// Project this module into a migration-free semantic snapshot.
    #[must_use]
    pub fn to_semantic_hir(&self) -> SemanticHirModule {
        SemanticHirModule {
            imports: self.imports.clone(),
            rust_imports: self.rust_imports.clone(),
            functions: self.functions.clone(),
            types: self.types.clone(),
            routes: self.routes.clone(),
            actors: self.actors.clone(),
            workflows: self.workflows.clone(),
            activities: self.activities.clone(),
            tests: self.tests.clone(),
            server_fns: self.server_fns.clone(),
            query_fns: self.query_fns.clone(),
            mutation_fns: self.mutation_fns.clone(),
            tables: self.tables.clone(),
            indexes: self.indexes.clone(),
            collections: self.collections.clone(),
            vector_indexes: self.vector_indexes.clone(),
            search_indexes: self.search_indexes.clone(),
            mcp_tools: self.mcp_tools.clone(),
            reactive_components: self.reactive_components.clone(),
        }
    }
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

/// A Rust crate import declaration lowered from source.
#[derive(Debug, Clone)]
pub struct HirRustImport {
    /// Dependency key / crate name.
    pub crate_name: String,
    /// Binding name in source scope.
    pub alias: String,
    /// Optional semantic version requirement.
    pub version: Option<String>,
    /// Optional local path source.
    pub path: Option<String>,
    /// Optional git source URL.
    pub git: Option<String>,
    /// Optional git revision / branch hint.
    pub rev: Option<String>,
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
    /// Stable contract key (`METHOD path`) for WebIR / client stubs (OP-0040).
    pub route_contract: String,
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

impl HirHttpMethod {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
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

#[cfg(test)]
mod tests {
    use super::{HirFieldOwnership, HirModule};

    #[test]
    fn hir_field_ownership_map_contains_migration_markers() {
        let map = HirModule::field_ownership_map();
        assert!(
            map.iter().any(|(name, own)| *name == "legacy_ast_nodes"
                && *own == HirFieldOwnership::MigrationOnly)
        );
        assert!(
            map.iter()
                .any(|(name, own)| *name == "server_fns" && *own == HirFieldOwnership::AppContract)
        );
        assert!(
            map.iter()
                .any(|(name, own)| *name == "functions" && *own == HirFieldOwnership::SemanticCore)
        );
    }

    #[test]
    fn semantic_hir_projection_drops_migration_vectors() {
        let hir = HirModule::default();
        let projected = hir.to_semantic_hir();
        assert!(projected.reactive_components.is_empty());
        // These vectors are migration-only and intentionally absent from `SemanticHirModule`.
        assert!(projected.functions.is_empty());
    }
}
