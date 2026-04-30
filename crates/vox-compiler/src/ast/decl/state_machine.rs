//! AST types for `state_machine` declarations (TASK-4.1).

use crate::ast::span::Span;
use crate::ast::types::TypeExpr;

/// Top-level `state_machine Name { ... }` declaration.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StateMachineDecl {
    pub name: String,
    pub states: Vec<SmStateDecl>,
    pub transitions: Vec<SmTransitionDecl>,
    pub span: Span,
}

/// A single state within a state machine.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SmStateDecl {
    pub name: String,
    /// Fields carried by this state (e.g. `state Working(task: Task)`).
    pub fields: Vec<SmStateField>,
    /// If true, no outgoing transitions are allowed.
    pub terminal: bool,
    pub span: Span,
}

/// A named field on a state constructor (e.g. `task: Task`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SmStateField {
    pub name: String,
    pub ty: TypeExpr,
    pub span: Span,
}

/// `on EventName(args) from SourcePattern -> TargetState(args)`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SmTransitionDecl {
    pub event: String,
    /// Event payload parameter names (optional typed).
    pub event_params: Vec<SmEventParam>,
    /// Source state pattern: `StateName`, `StateName(_)`, or `any`.
    pub from: SmTransitionSource,
    /// Target state name (constructor form omitted for now; just the name).
    pub to: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SmEventParam {
    pub name: String,
    pub ty: Option<TypeExpr>,
    pub span: Span,
}

/// Source pattern for a transition.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SmTransitionSource {
    /// `from StateName` or `from StateName(_)` — specific source state.
    State(String),
    /// `from any` — matches any source state.
    Any,
}
