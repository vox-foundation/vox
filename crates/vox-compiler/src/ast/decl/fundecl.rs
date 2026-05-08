use crate::ast::expr::{Expr, Param};
use crate::ast::span::Span;
use crate::ast::stmt::Stmt;
use crate::ast::types::TypeExpr;

/// Verification mode for contracts and assertions.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum VerifyMode {
    /// No runtime checks; contracts are stripped.
    Off,
    /// Preconditions are checked; postconditions are ignored (development safe).
    RequireOnly,
    /// Both preconditions and postconditions are checked.
    Full,
}

/// A postcondition requirement with an optional fallback for repair.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PostCondition {
    /// The condition expression to check.
    pub condition: Expr,
    /// Optional fallback function name to call if the condition fails.
    pub fallback: Option<String>,
}

/// Function declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FnDecl {
    /// The name of the function.
    pub name: String,
    /// Generic parameter names.
    pub generics: Vec<String>,
    /// List of function parameters.
    pub params: Vec<Param>,
    /// Explicit return type annotation.
    pub return_type: Option<TypeExpr>,
    /// The function body (sequence of statements).
    pub body: Vec<Stmt>,
    /// Whether the function is asynchronous.
    pub is_async: bool,
    /// Whether the function is marked as deprecated.
    pub is_deprecated: bool,
    /// Whether the function is pure (no side effects).
    pub is_pure: bool,
    /// Whether the function carries an `@reactive` decorator. When set, the auto-dep
    /// inference pass at `vox_codegen::codegen_ts::hir_emit::state_deps` is allowed to
    /// recurse into the body when this function is called from a reactive `derived` /
    /// `effect`, so that reads of reactive bindings through the call participate in
    /// dependency tracking. Default: `false` (the dep walker stops at the call site).
    #[serde(default)]
    pub is_reactive: bool,
    /// Explicit effect annotations from the `uses` clause.
    /// Empty means unannotated (unconstrained); `[Nothing]` means `uses nothing` (pure).
    #[serde(default)]
    pub effects: Vec<super::effect::EffectAnnotation>,
    /// Whether the function is subject to observability tracing.
    pub is_traced: bool,
    /// Whether the function body is implemented via an LLM.
    pub is_llm: bool,
    /// Optional specific LLM model to use for implementation.
    pub llm_model: Option<String>,
    /// Whether the function serves as a page layout.
    /// Whether the function is public.
    pub is_pub: bool,
    /// Whether the function records custom metrics.
    /// The name of the recorded metric.
    /// Whether the function is a health check endpoint.
    /// Optional authentication provider name.
    pub auth_provider: Option<String>,
    /// List of roles required to access the function.
    pub roles: Vec<String>,
    /// Optional CORS policy configuration.
    pub cors: Option<String>,
    /// Precondition expressions from `@require(expr)` decorators.
    pub preconditions: Vec<Expr>,
    /// Postcondition expressions from `@ensure(expr)` decorators.
    pub postconditions: Vec<PostCondition>,
    /// Class invariants (if this is a method).
    pub invariants: Vec<Expr>,
    /// How strictly to enforce contracts at runtime.
    pub verify_mode: VerifyMode,
    /// Optional strategy override for property testing.
    pub test_strategy: Option<String>,
    /// Whether this function is a mobile native implementation bridge.
    pub is_mobile_native: bool,
    /// When `Some`, this function is a TS-source FFI extern: the body is empty
    /// and codegen-TS emits an `import { name } from "<module>"` instead of
    /// generating a body. Codegen-Rust treats calls to such functions as an
    /// error (see plan 6).
    #[serde(default)]
    pub ts_extern_module: Option<String>,
    /// Source location.
    pub span: Span,
}

/// Component declaration (wraps a function with @component semantics).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ComponentDecl {
    /// The underlying function implementing the component.
    pub func: FnDecl,
    /// Scoped CSS styles associated with the component.
    pub styles: Vec<StyleBlock>,
}

/// A scoped style block within a component.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StyleBlock {
    /// The CSS selector (e.g. ".btn", "h1").
    pub selector: String,
    /// List of (property, value) pairs.
    pub properties: Vec<(String, String)>,
    /// `true` when this block came from a `raw_css { }` escape hatch.
    /// Raw CSS values are allowed (as a warning) inside these blocks.
    pub is_raw_css: bool,
    /// Source location.
    pub span: Span,
}

/// Test declaration (wraps a function with @test semantics).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TestDecl {
    /// Human-readable label/description for the test.
    pub label: String,
    /// The underlying function implementing the test.
    pub func: FnDecl,
}

/// Property-based test declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ForallDecl {
    /// Human-readable description.
    pub label: String,
    /// How many generated iterations to run.
    pub iterations: u32,
    /// The underlying function implementing the property to check.
    pub func: FnDecl,
}

/// Server function declaration (wraps a function with @server semantics).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ServerFnDecl {
    /// The underlying function implementing the server logic.
    pub func: FnDecl,
}

/// Query declaration: a read-only database function.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct QueryDecl {
    /// The underlying function implementing the query.
    pub func: FnDecl,
}

/// Mutation declaration: a write database function with transaction semantics.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MutationDecl {
    /// The underlying function implementing the mutation.
    pub func: FnDecl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EndpointKind {
    Query,
    Mutation,
    Server,
}

/// Unified endpoint declaration (wraps a function with @endpoint semantics).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EndpointDecl {
    pub kind: EndpointKind,
    pub func: FnDecl,
}

/// Skill declaration: a modular AI capability.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SkillDecl {
    /// The underlying function implementing the skill.
    pub func: FnDecl,
}

/// Agent definition declaration: defines the core logic and interface for an AI agent.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgentDefDecl {
    /// The underlying function implementing the agent's logic.
    pub func: FnDecl,
}

/// Scheduled function declaration — runs at a fixed interval or cron schedule.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ScheduledDecl {
    /// The interval or cron schedule (e.g. "1h", "0 0 * * *").
    pub interval: String,
    /// The function to execute on schedule.
    pub func: FnDecl,
}

/// MCP tool declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpToolDecl {
    /// Human-readable description of the tool's purpose.
    pub description: String,
    /// The function implementing the tool's logic.
    pub func: FnDecl,
}

/// MCP resource declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct McpResourceDecl {
    /// The URI identifying the resource.
    pub uri: String,
    /// Human-readable description of the resource.
    pub description: String,
    /// The function that serves the resource content.
    pub func: FnDecl,
}

/// Mock declaration for testing.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MockDecl {
    /// The target function/service to mock.
    pub target: String,
    /// The mock implementation function.
    pub func: FnDecl,
}

/// A frontend hook function declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HookDecl {
    /// The underlying function implementing the hook.
    pub func: FnDecl,
}

/// Fixture declaration: setup code for tests.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FixtureDecl {
    /// The underlying function implementing the fixture.
    pub func: FnDecl,
    /// Source location.
    pub span: Span,
}

/// Task declaration — a trust-gated, capability-checked execution unit (Wave 4).
///
/// Syntax:
/// ```vox
/// @task(trust = "user", caps = ["network.read", "db.write"])
/// fn send_notification(user_id: int, msg: str) to Result[Unit, str]:
///     ...
/// ```
///
/// Generated code gates execution behind `TrustPolicy::check()` and
/// `CapabilityPolicy::require_all()` before the function body runs.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TaskDecl {
    /// The function implementing the task body.
    pub func: FnDecl,
    /// Minimum trust class required to invoke this task.
    /// Defaults to `"user"` if not specified.
    pub trust_class: String,
    /// List of required capabilities (e.g. `["network.read", "db.write"]`).
    pub capabilities: Vec<String>,
    /// Source span.
    pub span: Span,
}
