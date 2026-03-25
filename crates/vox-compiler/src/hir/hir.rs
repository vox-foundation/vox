//! Concrete HIR types: one lowered module with separate vectors per top-level construct.
//!
//! **Names:** Identifiers and resolved paths in [`HirExpr`] / patterns reflect the lowering pass
//! ([`crate::hir::lower`]), not necessarily raw source spelling. Prefer [`crate::ast::span::Span`] on each node for
//! diagnostics rather than re-parsing names.
//!
//! **Consumers:** `vox-typeck` and codegen read these types; keep new variants backward-compatible
//! or bump all match sites in the same change.

use crate::ast::span::Span;

/// Unique identifier for definitions within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(
    /// Opaque numeric id assigned during HIR lowering.
    pub u32,
);

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
    /// Codex table schemas.
    pub tables: Vec<HirTable>,
    /// Table indexes.
    pub indexes: Vec<HirIndex>,
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

/// A function parameter in HIR.
#[derive(Debug, Clone)]
pub struct HirParam {
    /// Parameter definition id.
    pub id: DefId,
    /// Binding name.
    pub name: String,
    /// Optional type annotation.
    pub type_ann: Option<HirType>,
    /// Default value expression.
    pub default: Option<HirExpr>,
    /// Span covering this parameter.
    pub span: Span,
}

/// Type representation in HIR (resolved from [`crate::ast::types::TypeExpr`]).
#[derive(Debug, Clone, PartialEq)]
pub enum HirType {
    /// Named scalar or path type (`int`, `User`).
    Named(String),
    /// Generic application (`list[T]`).
    Generic(String, Vec<HirType>),
    /// Function type (`fn(a, b) -> c`).
    Function(Vec<HirType>, Box<HirType>),
    /// Tuple type.
    Tuple(Vec<HirType>),
    /// Unit / void.
    Unit,
}

/// Expression in HIR (mirrors AST but with resolved names).
#[derive(Debug, Clone)]
pub enum HirExpr {
    /// Integer literal.
    IntLit(i64, Span),
    /// Floating-point literal.
    FloatLit(f64, Span),
    /// String literal.
    StringLit(String, Span),
    /// Boolean literal.
    BoolLit(bool, Span),
    /// Local or global identifier.
    Ident(String, Span),
    /// Object literal `{ k: v }`.
    ObjectLit(Vec<(String, HirExpr)>, Span),
    /// List literal.
    ListLit(Vec<HirExpr>, Span),
    /// Tuple literal.
    TupleLit(Vec<HirExpr>, Span),
    /// Binary operator application.
    Binary(HirBinOp, Box<HirExpr>, Box<HirExpr>, Span),
    /// Unary operator application.
    Unary(HirUnOp, Box<HirExpr>, Span),
    /// Call; `bool` indicates tail-call hint when used by backend.
    Call(Box<HirExpr>, Vec<HirArg>, bool, Span),
    /// Method call `obj.method(args)`.
    MethodCall(Box<HirExpr>, String, Vec<HirArg>, Span),
    /// Field projection.
    FieldAccess(Box<HirExpr>, String, Span),
    /// `match` expression.
    Match(Box<HirExpr>, Vec<HirMatchArm>, Span),
    /// `if` with optional else.
    If(Box<HirExpr>, Vec<HirStmt>, Option<Vec<HirStmt>>, Span),
    /// `for` loop expression (often JSX).
    For(String, Box<HirExpr>, Box<HirExpr>, Span),
    /// Lambda / closure.
    Lambda(Vec<HirParam>, Option<HirType>, Box<HirExpr>, Span),
    /// Pipeline `a |> b`.
    Pipe(Box<HirExpr>, Box<HirExpr>, Span),
    /// `spawn` actor constructor.
    Spawn(Box<HirExpr>, Span),
    /// `with` options wrapper.
    With(Box<HirExpr>, Box<HirExpr>, Span),
    /// JSX element with children.
    Jsx(HirJsxElement),
    /// Self-closing JSX.
    JsxSelfClosing(HirJsxSelfClosing),
    /// Statement block used as expression.
    Block(Vec<HirStmt>, Span),
}

/// Named or positional call argument.
#[derive(Debug, Clone)]
pub struct HirArg {
    /// Label for named args (`json=`); `None` for positional.
    pub name: Option<String>,
    /// Argument value.
    pub value: HirExpr,
}

/// Binary operators in HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Lte,
    /// `>=`
    Gte,
    /// `and`
    And,
    /// `or`
    Or,
    /// `is`
    Is,
    /// `isnt`
    Isnt,
    /// `|>`
    Pipe,
}

/// Unary operators in HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnOp {
    /// `not`
    Not,
    /// `-`
    Neg,
}

/// Single `match` arm.
#[derive(Debug, Clone)]
pub struct HirMatchArm {
    /// Matched pattern.
    pub pattern: HirPattern,
    /// Optional guard.
    pub guard: Option<Box<HirExpr>>,
    /// Arm body.
    pub body: Box<HirExpr>,
    /// Span covering the arm.
    pub span: Span,
}

/// Pattern after lowering (names resolved where applicable).
#[derive(Debug, Clone)]
pub enum HirPattern {
    /// Bind identifier.
    Ident(String, Span),
    /// Tuple destructuring.
    Tuple(Vec<HirPattern>, Span),
    /// ADT constructor pattern.
    Constructor(String, Vec<HirPattern>, Span),
    /// `_` wildcard.
    Wildcard(Span),
    /// Literal pattern.
    Literal(Box<HirExpr>, Span),
}

/// Statement in HIR.
#[derive(Debug, Clone)]
pub enum HirStmt {
    /// `let` / `let mut` binding.
    Let {
        /// Bound pattern.
        pattern: HirPattern,
        /// Optional type ascription.
        type_ann: Option<HirType>,
        /// Initializer.
        value: HirExpr,
        /// `mut` flag.
        mutable: bool,
        /// Span.
        span: Span,
    },
    /// Assignment.
    Assign {
        /// Target l-value.
        target: HirExpr,
        /// New value.
        value: HirExpr,
        /// Span covering the assignment.
        span: Span,
    },
    /// `ret` statement.
    Return {
        /// Return value, if any.
        value: Option<HirExpr>,
        /// Span covering `ret` / value.
        span: Span,
    },
    /// Expression statement.
    Expr {
        /// Evaluated expression.
        expr: HirExpr,
        /// Span covering the statement.
        span: Span,
    },
}

/// JSX element with children.
#[derive(Debug, Clone)]
pub struct HirJsxElement {
    /// Tag name.
    pub tag: String,
    /// Attributes.
    pub attributes: Vec<HirJsxAttr>,
    /// Child expressions.
    pub children: Vec<HirExpr>,
    /// Span covering the element.
    pub span: Span,
}

/// Self-closing JSX element.
#[derive(Debug, Clone)]
pub struct HirJsxSelfClosing {
    /// Tag name.
    pub tag: String,
    /// Attributes.
    pub attributes: Vec<HirJsxAttr>,
    /// Span covering `<tag .../>`.
    pub span: Span,
}

/// JSX attribute.
#[derive(Debug, Clone)]
pub struct HirJsxAttr {
    /// Attribute name.
    pub name: String,
    /// Attribute value expression.
    pub value: HirExpr,
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
