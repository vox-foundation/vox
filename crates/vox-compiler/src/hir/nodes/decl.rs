//! HIR declarations: module shape, items, routes, tables (OP-0208).

use crate::ast::decl::fundecl::StyleBlock;
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



/// A fully lowered Vox module: every declaration category is collected into its own vector.
///
/// Empty vectors mean the construct was absent in source; there is no implicit ordering across
/// categories—only within each `Vec` matches source order for that kind.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
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
    /// `@forall` properties.
    pub foralls: Vec<HirForall>,
    /// Unified endpoint functions (`@endpoint`, `@server`, `@query`, `@mutation`).
    pub endpoint_fns: Vec<HirEndpointFn>,
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
    /// MCP read-only resource handlers (`@mcp.resource`).
    pub mcp_resources: Vec<HirMcpResource>,
    /// Native agent definitions.
    pub agents: Vec<HirAgent>,
    /// Container environment specifications.
    pub environments: Vec<HirEnvironment>,

    // UI surfaces (AST-retained where needed for emit + WebIR projection)
    /// Standalone islands.
    pub islands: Vec<HirIsland>,
    /// Reactive components (Path C).
    pub components: Vec<HirReactiveComponent>,

    /// Declarations not yet represented as typed HIR vectors (unknown / future decl kinds).
    /// HTTP routes, tables, activities, and `@server` fns are lowered to [`HirRoute`], [`HirTable`],
    /// [`HirActivity`], and [`HirServerFn`]; TS codegen reads those directly (Path C).
    pub legacy_ast_nodes: Vec<crate::ast::decl::Decl>,


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
    pub tests: Vec<HirFn>,
    pub endpoint_fns: Vec<HirEndpointFn>,
    pub tables: Vec<HirTable>,
    pub indexes: Vec<HirIndex>,
    pub collections: Vec<HirCollection>,
    pub vector_indexes: Vec<HirVectorIndex>,
    pub search_indexes: Vec<HirSearchIndex>,
    pub mcp_tools: Vec<HirMcpTool>,
    pub mcp_resources: Vec<HirMcpResource>,
    pub agents: Vec<HirAgent>,
    pub environments: Vec<HirEnvironment>,
    pub components: Vec<HirReactiveComponent>,
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
            ("actors", HirFieldOwnership::MigrationOnly),
            ("workflows", HirFieldOwnership::MigrationOnly),
            ("activities", HirFieldOwnership::MigrationOnly),
            ("tests", HirFieldOwnership::SemanticCore),
            ("endpoint_fns", HirFieldOwnership::AppContract),
            ("tables", HirFieldOwnership::SemanticCore),
            ("indexes", HirFieldOwnership::SemanticCore),
            ("collections", HirFieldOwnership::SemanticCore),
            ("vector_indexes", HirFieldOwnership::SemanticCore),
            ("search_indexes", HirFieldOwnership::SemanticCore),
            ("mcp_tools", HirFieldOwnership::SemanticCore),
            ("mcp_resources", HirFieldOwnership::SemanticCore),
            ("agents", HirFieldOwnership::SemanticCore),
            ("environments", HirFieldOwnership::SemanticCore),
            ("islands", HirFieldOwnership::AppContract),

            ("legacy_ast_nodes", HirFieldOwnership::MigrationOnly),
            ("components", HirFieldOwnership::SemanticCore),
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
            tests: self.tests.clone(),
            endpoint_fns: self.endpoint_fns.clone(),
            tables: self.tables.clone(),
            indexes: self.indexes.clone(),
            collections: self.collections.clone(),
            vector_indexes: self.vector_indexes.clone(),
            search_indexes: self.search_indexes.clone(),
            mcp_tools: self.mcp_tools.clone(),
            mcp_resources: self.mcp_resources.clone(),
            agents: self.agents.clone(),
            environments: self.environments.clone(),
            components: self.components.clone(),
        }
    }
}

/// Island component lowered to HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirIsland(pub crate::ast::decl::IslandDecl);

/// A resolved import.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirImport {
    /// Module path segments (`["react"]`, `["std", "http"]`).
    pub module_path: Vec<String>,
    /// Imported symbol name.
    pub item: String,
    /// Span in source.
    pub span: Span,
}

/// A Rust crate import declaration lowered from source.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    /// Whether this is a `@mobile.native` bridge.
    pub is_mobile_native: bool,
    /// `@pure` — metadata for pipeline tooling (effect guarantees are not proven in HIR).
    #[serde(default)]
    pub is_pure: bool,
    /// Whether the function body is implemented via an LLM.
    #[serde(default)]
    pub is_llm: bool,
    /// Optional specific LLM model to use for implementation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_model: Option<String>,
    /// `@deprecated` on the source `fn`.
    #[serde(default)]
    pub is_deprecated: bool,
    /// `@scheduled("…")` interval/cron string when this item was lowered from [`crate::ast::decl::Decl::Scheduled`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule_interval: Option<String>,
    /// Postconditions to check at runtime (used for @ai repair/fallback).
    #[serde(default)]
    pub postconditions: Vec<HirPostCondition>,
    /// Span covering the declaration.
    pub span: Span,
}

/// A lowered postcondition with optional fallback.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirPostCondition {
    /// Condition expression.
    pub condition: HirExpr,
    /// Optional fallback function name.
    pub fallback: Option<String>,
}

/// ADT / type definition in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirVariant {
    /// Constructor name.
    pub name: String,
    /// Named fields with types.
    pub fields: Vec<(String, HirType)>,
    /// Span covering the variant.
    pub span: Span,
}

/// HTTP route in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HirEndpointKind {
    Query,
    Mutation,
    Server,
}

/// A server endpoint function — callable from the frontend, auto-generates API route + fetch wrapper.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirEndpointFn {
    /// Endpoint kind (query, mutation, server).
    pub kind: HirEndpointKind,
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
    /// HTTP path bound to this endpoint fn.
    pub route_path: String,
    /// Span covering the declaration.
    pub span: Span,
}

/// Table definition — a persistent record type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirTableField {
    /// Column name.
    pub name: String,
    /// Column type.
    pub type_ann: HirType,
    /// Span covering the field.
    pub span: Span,
}

/// Index definition for a table.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirCollection {
    pub id: DefId,
    pub name: String,
    pub fields: Vec<HirTableField>,
    pub is_pub: bool,
    pub has_spread: bool,
    pub span: Span,
}

/// Vector index metadata lowered from AST.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirVectorIndex {
    pub table_name: String,
    pub index_name: String,
    pub column: String,
    pub dimensions: u32,
    pub filter_fields: Vec<String>,
    pub span: Span,
}

/// Full-text search index lowered from AST.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirSearchIndex {
    pub table_name: String,
    pub index_name: String,
    pub search_field: String,
    pub filter_fields: Vec<String>,
    pub span: Span,
}

/// MCP tool declaration — a function exposed via the Model Context Protocol.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirMcpTool {
    /// Tool description for MCP clients.
    pub description: String,
    /// Underlying implementation.
    pub func: HirFn,
}

/// MCP resource — read-only URI-backed content for MCP `resources/read`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirMcpResource {
    /// Stable resource URI (e.g. `notes://recent`).
    pub uri: String,
    pub description: String,
    pub func: HirFn,
}

/// A property-based `@forall` test.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirForall {
    pub label: String,
    pub iterations: u32,
    pub func: HirFn,
}

/// Reactive component lowered to HIR (Path C).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirReactiveComponent {
    pub id: DefId,
    pub name: String,
    pub params: Vec<HirParam>,
    pub members: Vec<HirReactiveMember>,
    pub view: Option<HirExpr>,
    /// Scoped CSS blocks attached after the reactive body (Path C).
    pub styles: Vec<StyleBlock>,
    pub span: Span,
}

/// Native agent definition in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirAgent {
    /// Agent id.
    pub id: DefId,
    /// Agent name.
    pub name: String,
    /// Optional version tag.
    pub version: Option<String>,
    /// Internal state fields.
    pub state_fields: Vec<HirTableField>,
    /// Event handlers.
    pub handlers: Vec<HirAgentHandler>,
    /// Version migration rules.
    pub migrations: Vec<HirMigrationRule>,
    /// `@deprecated` agent.
    pub is_deprecated: bool,
    /// Span covering the agent.
    pub span: Span,
}

/// Single agent handler.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirAgentHandler {
    /// Event name.
    pub event_name: String,
    /// Parameters.
    pub params: Vec<HirParam>,
    /// Return type.
    pub return_type: Option<HirType>,
    /// Handler body.
    pub body: Vec<HirStmt>,
    /// Tracing enabled.
    pub is_traced: bool,
    /// Span covering the handler.
    pub span: Span,
}

/// Agent migration rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirMigrationRule {
    pub from_version: String,
    pub body: Vec<HirStmt>,
    pub span: Span,
}

/// Container environment specification in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirEnvironment {
    pub name: String,
    pub base_image: Option<String>,
    pub packages: Vec<String>,
    pub env_vars: Vec<(String, String)>,
    pub exposed_ports: Vec<u16>,
    pub volumes: Vec<String>,
    pub workdir: Option<String>,
    pub cmd: Vec<String>,
    pub copy_instructions: Vec<(String, String)>,
    pub run_commands: Vec<String>,
    pub is_deprecated: bool,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HirReactiveMember {
    State(HirState),
    Derived(HirDerived),
    Effect(HirEffect),
    OnMount(HirOnMount),
    OnCleanup(HirOnCleanup),
    /// `let` / hook calls / assignments before `view:` (Path C prelude).
    Stmt(HirStmt),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirState {
    pub id: DefId,
    pub name: String,
    pub ty: Option<HirType>,
    pub init: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirDerived {
    pub id: DefId,
    pub name: String,
    pub ty: Option<HirType>,
    pub expr: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirEffect {
    pub body: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirOnMount {
    pub body: HirExpr,
    pub span: Span,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
                .any(|(name, own)| *name == "endpoint_fns" && *own == HirFieldOwnership::AppContract)
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
        // These vectors are migration-only and intentionally absent from `SemanticHirModule`.
        assert!(projected.functions.is_empty());
    }
}
