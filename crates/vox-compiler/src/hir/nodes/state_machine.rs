//! HIR types for `state_machine` declarations (TASK-4.1).

use crate::ast::span::Span;
use crate::ast::types::TypeExpr;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirStateMachineDecl {
    pub name: String,
    pub states: Vec<HirStateDecl>,
    pub transitions: Vec<HirTransitionDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirStateDecl {
    pub name: String,
    pub fields: Vec<HirStateField>,
    pub terminal: bool,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirStateField {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirTransitionDecl {
    pub event: String,
    pub event_params: Vec<HirEventParam>,
    pub from: HirTransitionSource,
    pub to: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HirEventParam {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum HirTransitionSource {
    State(String),
    Any,
}
