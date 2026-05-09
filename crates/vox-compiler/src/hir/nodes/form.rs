//! HIR form declarations — lowered from `@form` AST nodes.

use crate::ast::span::Span;

use super::expr::HirExpr;
use super::stmt_expr::HirType;

/// A lowered `@form` declaration in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirForm {
    /// Form name (PascalCase).
    pub name: String,
    /// Declared form fields.
    pub fields: Vec<HirFormField>,
    /// `@endpoint` function called on submit.
    pub on_submit: Option<String>,
    /// Path to redirect to after successful submit.
    pub success_redirect: Option<String>,
    /// Error message shown on submit failure.
    pub error_message: Option<String>,
    /// Source span.
    pub span: Span,
}

/// A single field in a lowered `@form`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HirFormField {
    pub name: String,
    pub ty: HirType,
    pub label: Option<String>,
    pub required: bool,
    pub hidden: bool,
    pub default: Option<HirExpr>,
    pub constraints: Vec<HirFieldConstraint>,
    pub span: Span,
}

/// Constraints on a form field value in HIR.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum HirFieldConstraint {
    Range(HirExpr, HirExpr),
    MaxLen(usize),
    MinLen(usize),
    Pattern(String),
    Enum(Vec<HirExpr>),
    Custom(String),
}
