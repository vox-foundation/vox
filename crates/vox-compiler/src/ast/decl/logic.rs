use crate::ast::expr::Param;
use crate::ast::span::Span;
use crate::ast::stmt::Stmt;
use crate::ast::types::TypeExpr;

/// Native agent declaration
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgentDecl {
    pub name: String,
    pub version: Option<String>,
    pub state_fields: Vec<crate::ast::decl::typedef::VariantField>,
    pub handlers: Vec<AgentHandler>,
    pub migrations: Vec<MigrationRule>,
    pub is_deprecated: bool,
    pub span: Span,
}

/// Agent handler definition: `on Event(msg) to Type:`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AgentHandler {
    pub event_name: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Stmt>,
    pub is_traced: bool,
    pub span: Span,
}

/// Agent migration rule: `migrate from "1.0":`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MigrationRule {
    pub from_version: String,
    pub body: Vec<Stmt>,
    pub span: Span,
}

/// Native message declaration
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MessageDecl {
    pub name: String,
    pub fields: Vec<crate::ast::decl::typedef::VariantField>,
    pub is_deprecated: bool,
    pub span: Span,
}

/// HTTP route declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HttpRouteDecl {
    pub method: crate::ast::decl::HttpMethod,
    pub path: String,
    pub params: Vec<Param>,
    pub return_type: Option<TypeExpr>,
    pub body: Vec<Stmt>,
    pub auth_provider: Option<String>,
    pub roles: Vec<String>,
    pub cors: Option<String>,
    pub is_traced: bool,
    pub is_deprecated: bool,
    pub span: Span,
}
