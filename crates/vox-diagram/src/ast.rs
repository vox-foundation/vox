//! Syntax tree produced by the parser. Every node carries its byte [`Span`].

use serde::Serialize;

use crate::ir::Shape;
use crate::span::Span;

/// A parsed `graph { ... }` document.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Statements of the graph block.
    pub body: Block,
    /// Whole document span.
    pub span: Span,
}

/// Ordered entries of a `{ ... }` block (comments and trivia arrive with the formatter).
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Block {
    /// Entries in source order.
    pub entries: Vec<Entry>,
}

/// One block entry.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// A statement.
    Item(Item),
}

/// A statement plus its leading blank-line flag.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// What the statement is.
    pub kind: ItemKind,
    /// True when a blank line precedes the statement.
    pub blank_before: bool,
    /// Statement span.
    pub span: Span,
}

/// Statement kinds.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    /// `node id: "label"`.
    Node(NodeDecl),
    /// `a -> b` with optional `: "label"`.
    Edge(EdgeStmt),
}

/// A node declaration.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct NodeDecl {
    /// Declared id.
    pub id: Ident,
    /// Display label.
    pub label: Label,
    /// Shape (default rect).
    pub shape: Shape,
}

/// An edge statement.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct EdgeStmt {
    /// Source reference.
    pub from: Ref,
    /// Target reference.
    pub to: Ref,
    /// Optional edge label.
    pub label: Option<Label>,
}

/// A dotted reference such as `backend.gateway`.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Ref {
    /// Path segments.
    pub segments: Vec<Ident>,
    /// Whole reference span.
    pub span: Span,
}

/// An identifier.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    /// Identifier text.
    pub name: String,
    /// Where it was written.
    pub span: Span,
}

/// A decoded quoted label.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// Decoded text.
    pub value: String,
    /// Span including the quotes.
    pub span: Span,
}
