//! HIR representation for typed URL declarations (TASK-4.3).
use crate::ast::span::Span;
use crate::ast::types::TypeExpr;

/// A `url TypeName { Variant, Variant(arg: Type), ... }` HIR node.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirUrlDecl {
    /// The name of the URL type (e.g. `Path`).
    pub name: String,
    /// All declared variants.
    pub variants: Vec<HirUrlVariant>,
    pub span: Span,
}

/// One variant in a URL declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirUrlVariant {
    /// Variant name (PascalCase).
    pub name: String,
    /// Arguments (empty = unit variant).
    pub args: Vec<HirUrlArg>,
    pub span: Span,
}

/// One argument in a URL variant.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirUrlArg {
    pub name: String,
    pub optional: bool,
    pub ty: TypeExpr,
    pub span: Span,
}
