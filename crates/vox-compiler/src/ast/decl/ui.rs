use crate::ast::decl::fundecl::{FnDecl, StyleBlock};
use crate::ast::span::Span;
use crate::ast::types::TypeExpr;

/// v0.dev AI-generated component declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct V0ComponentDecl {
    pub v0_id: String,
    /// When set, this stub was generated from `@v0 from "path.png"` (design asset) instead of a v0 chat id string.
    #[serde(default)]
    pub image_path: Option<String>,
    pub name: String,
    pub props: Vec<IslandProp>,
    pub span: Span,
}

/// Client-side routing declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoutesDecl {
    pub entries: Vec<RouteEntry>,
    /// Optional 404 screen component (PascalCase name).
    #[serde(default)]
    pub not_found_component: Option<String>,
    /// Optional global error screen component.
    #[serde(default)]
    pub error_component: Option<String>,
    pub span: Span,
}

/// Deterministic parse metrics for a [`RoutesDecl`] (tooling / gates; OP-0027).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoutesParseSummary {
    pub entry_count: usize,
    pub paths: Vec<String>,
}

impl RoutesDecl {
    #[must_use]
    /// Paths are listed in source order (stable for snapshot-style tests).
    pub fn parse_summary(&self) -> RoutesParseSummary {
        RoutesParseSummary {
            entry_count: self.entries.len(),
            paths: self.entries.iter().map(|e| e.path.clone()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RouteEntry {
    pub path: String,
    pub component_name: String,
    pub children: Vec<RouteEntry>,
    pub redirect: Option<String>,
    pub is_wildcard: bool,
    /// `@query` / loader function name for this route (emitted into `routes.manifest.ts`).
    #[serde(default)]
    pub loader_name: Option<String>,
    /// Loading / pending component for this route.
    #[serde(default)]
    pub pending_component_name: Option<String>,
    pub span: Span,
}

/// Frontend React Context wrapper.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ContextDecl {
    pub name: String,
    pub state_type: Option<TypeExpr>,
    pub default_expr: Option<crate::ast::expr::Expr>,
    pub span: Span,
}

/// A frontend provider component declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProviderDecl {
    pub context_name: String,
    pub func: FnDecl,
    pub span: Span,
}

/// Layout component wrapper — wraps child routes with shared UI.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LayoutDecl {
    pub func: FnDecl,
}

/// Loading state component — shown during route suspense.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LoadingDecl {
    pub func: FnDecl,
}

/// 404 / not-found page component.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct NotFoundDecl {
    pub func: FnDecl,
}

/// Error boundary component — catches render errors.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ErrorBoundaryDecl {
    pub func: FnDecl,
}

/// CSS keyframes declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeyframeDecl {
    pub name: String,
    pub steps: Vec<KeyframeStep>,
    pub span: Span,
}

/// A single keyframe step (e.g., `from:`, `to:`, `50%:`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeyframeStep {
    pub selector: String,
    pub properties: Vec<(String, String)>,
}

/// Theme declaration with light/dark variants.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThemeDecl {
    pub name: String,
    pub light: Vec<(String, String)>,
    pub dark: Vec<(String, String)>,
    pub span: Span,
}

/// Static site generation (SSG) page hint: URL path + handler body in Vox.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PageDecl {
    pub path: String,
    pub func: FnDecl,
    pub span: Span,
}

/// React island component stub.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IslandDecl {
    pub name: String,
    pub props: Vec<IslandProp>,
    pub span: Span,
}
/// Prop declaration for an island.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct IslandProp {
    pub name: String,
    pub ty: TypeExpr,
    pub is_optional: bool,
}

/// Reactive component declaration (Path C reactive model).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReactiveComponentDecl {
    /// Component name.
    pub name: String,
    /// Props/parameters.
    pub params: Vec<crate::ast::expr::Param>,
    /// Body members: state, derived, effect, on_mount, on_cleanup.
    pub members: Vec<ReactiveMemberDecl>,
    /// Optional view expression (JSX).
    pub view: Option<crate::ast::expr::Expr>,
    /// Scoped CSS blocks following the component body (same as classic `@component fn`).
    #[serde(default)]
    pub styles: Vec<StyleBlock>,
    /// Source span.
    pub span: Span,
}

/// Member of a reactive component body.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReactiveMemberDecl {
    /// Reactive state binding.
    State(StateDecl),
    /// Derived (computed) value.
    Derived(DerivedDecl),
    /// Side-effect hook.
    Effect(EffectDecl),
    /// Runs once after first render.
    OnMount(OnMountDecl),
    /// Runs on component teardown.
    OnCleanup(OnCleanupDecl),
    /// Imperative prelude: `let` bindings, hook calls (`use_effect(...)`), assignments, etc.
    Stmt(crate::ast::stmt::Stmt),
}

/// `state name: Type = init_expr`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StateDecl {
    /// Binding name.
    pub name: String,
    /// Optional type annotation.
    pub ty: Option<TypeExpr>,
    /// Initial value expression.
    pub init: crate::ast::expr::Expr,
    /// Source span.
    pub span: Span,
}

/// `derived name: Type = expr`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DerivedDecl {
    /// Binding name.
    pub name: String,
    /// Optional type annotation.
    pub ty: Option<TypeExpr>,
    /// Computation expression.
    pub expr: crate::ast::expr::Expr,
    /// Source span.
    pub span: Span,
}

/// `effect: { body }`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EffectDecl {
    /// Effect body expression.
    pub body: crate::ast::expr::Expr,
    /// Source span.
    pub span: Span,
}

/// `mount: { body }` — runs once after first render.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OnMountDecl {
    /// Mount body expression.
    pub body: crate::ast::expr::Expr,
    /// Source span.
    pub span: Span,
}

/// `cleanup: { body }` — runs on component teardown.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OnCleanupDecl {
    /// Cleanup body expression.
    pub body: crate::ast::expr::Expr,
    /// Source span.
    pub span: Span,
}
