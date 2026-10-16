//! Syntax tree produced by the parser. Every node carries its byte [`Span`].

use serde::Serialize;

use crate::ir::{Direction, Shape};
use crate::span::Span;

/// A parsed `graph { ... }` document, with the comments around it.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Own-line comments before `graph`.
    pub leading: Vec<Comment>,
    /// Statements of the graph block.
    pub body: Block,
    /// Comment on the `graph {` line.
    pub header_trailing: Option<Comment>,
    /// Comment on the closing `}` line.
    pub close_trailing: Option<Comment>,
    /// Own-line comments after the closing `}`.
    pub trailing: Vec<Comment>,
    /// Whole document span.
    pub span: Span,
}

/// A `//` comment. `blank_before` follows the same rule as [`Item::blank_before`].
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// Text after `//`, with trailing whitespace and `\r` trimmed.
    pub text: String,
    /// True when a blank line separates it from the previous entry of its block.
    pub blank_before: bool,
    /// Span from `//` to the end of the text.
    pub span: Span,
}

/// Ordered entries of a `{ ... }` block.
#[derive(Serialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Block {
    /// Entries in source order.
    pub entries: Vec<Entry>,
    /// Byte offsets of the block's `{` and `}`, when it has both; not part of the JSON form.
    /// Two bare offsets (not spans) keep `Entry` small.
    #[serde(skip)]
    pub braces: Option<(u32, u32)>,
}

/// One block entry.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum Entry {
    /// An own-line comment.
    Comment(Comment),
    /// A statement.
    Item(Item),
}

/// A statement plus its trivia.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Item {
    /// What the statement is.
    pub kind: ItemKind,
    /// True iff at least one blank line separates it from the previous entry in the same
    /// block (always false for the first entry of a block).
    pub blank_before: bool,
    /// Comment on the same line after the statement.
    pub trailing: Option<Comment>,
    /// Statement span.
    pub span: Span,
}

/// Statement kinds.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    /// `direction: down`.
    Direction(DirectionStmt),
    /// `node id: "label"`.
    Node(NodeDecl),
    /// `container id: "label" { ... }`.
    Container(ContainerDecl),
    /// `a -> b` with optional `: "label"`.
    Edge(EdgeStmt),
}

/// `direction: <down|right|up|left>`.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct DirectionStmt {
    /// The flow direction.
    pub value: Direction,
    /// Statement span.
    pub span: Span,
}

/// A container declaration with its nested body.
#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ContainerDecl {
    /// Declared id.
    pub id: Ident,
    /// Display label.
    pub label: Label,
    /// Nested statements.
    pub body: Block,
    /// Comment on the `container ... {` line.
    pub header_trailing: Option<Comment>,
    /// Comment on the closing `}` line.
    pub close_trailing: Option<Comment>,
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
    /// Set when it was written as Mermaid `id[Label]`; the resolver reports it.
    #[serde(skip)]
    pub bracket: Option<Box<Bracket>>,
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
    /// Set when it was written as Mermaid `id[Label]`; the resolver declares and reports it.
    #[serde(skip)]
    pub bracket: Option<Box<Bracket>>,
}

/// Mermaid `id[Label]` text where a plain id belongs: it declares the node (M3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bracket {
    /// The bracketed text, decoded.
    pub label: String,
    /// The canonical quoted spelling of the label.
    pub quoted: String,
    /// The shape the brackets stand for; `None` when VoxDiagram has no exact equivalent.
    pub shape: Option<Shape>,
    /// From the last id segment through the closing bracket.
    pub form: Span,
    /// False when the line ended before the closing bracket.
    pub closed: bool,
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
