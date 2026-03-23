use crate::ast::decl::fundecl::FnDecl;
use crate::ast::span::Span;
use crate::ast::types::TypeExpr;

/// v0.dev AI-generated component declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct V0ComponentDecl {
    pub prompt: String,
    pub image_path: Option<String>,
    pub name: String,
    pub return_type: Option<TypeExpr>,
    pub span: Span,
}

/// Client-side routing declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RoutesDecl {
    pub entries: Vec<RouteEntry>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RouteEntry {
    pub path: String,
    pub component_name: String,
    pub children: Vec<RouteEntry>,
    pub redirect: Option<String>,
    pub is_wildcard: bool,
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
