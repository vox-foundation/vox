use crate::ast::span::Span;
use crate::ast::types::TypeExpr;

/// ADT variant in a type definition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<VariantField>,
    /// Optional string literal value: `| User = "user"` emits `"user"` in TS union.
    pub literal_value: Option<String>,
    pub span: Span,
}

/// A field within an ADT variant.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VariantField {
    pub name: String,
    pub type_ann: TypeExpr,
    pub span: Span,
}

/// Type / ADT / struct declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TypeDefDecl {
    pub name: String,
    /// Generic type parameters: `type Response[T]:`
    pub generics: Vec<String>,
    /// ADT variants (sum type). Empty for struct types and type aliases.
    pub variants: Vec<Variant>,
    /// Struct fields (product type). Empty for ADTs and type aliases.
    pub fields: Vec<VariantField>,
    /// The aliased type, if this is a type alias.
    pub type_alias: Option<TypeExpr>,
    pub json_layout: Option<String>,
    pub is_pub: bool,
    pub is_deprecated: bool,
    pub span: Span,
}

