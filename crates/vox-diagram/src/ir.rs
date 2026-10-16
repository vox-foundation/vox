//! The resolved, serialisable diagram IR. `Vec`s in declaration order only, so the JSON is
//! byte-identical across runs.

use serde::{Deserialize, Serialize};

use crate::span::Location;

/// Resolved diagram. Phase 86 adds a `Sequence` variant additively.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Ir {
    /// A `graph` diagram.
    Graph(GraphIr),
}

/// Flow direction of a graph.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// Top to bottom (default).
    #[default]
    Down,
    /// Left to right.
    Right,
    /// Bottom to top.
    Up,
    /// Right to left.
    Left,
}

/// Node shape.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Shape {
    /// Rectangle (default).
    #[default]
    Rect,
    /// Rounded rectangle.
    Rounded,
    /// Diamond (decision).
    Diamond,
    /// Cylinder (storage).
    Cylinder,
    /// Start terminal.
    Start,
    /// End terminal.
    End,
}

/// A resolved `graph`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct GraphIr {
    /// Flow direction.
    pub direction: Direction,
    /// Nodes in declaration order.
    pub nodes: Vec<IrNode>,
    /// Containers in declaration order.
    pub containers: Vec<IrContainer>,
    /// Edges in source order.
    pub edges: Vec<IrEdge>,
}

/// A resolved node.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IrNode {
    /// Full dotted path.
    pub id: String,
    /// Id as written at the declaration.
    pub local_id: String,
    /// Decoded label.
    pub label: String,
    /// Shape.
    pub shape: Shape,
    /// Id of the enclosing container, if any.
    pub container: Option<String>,
    /// Declaration location.
    pub span: Location,
}

/// A resolved container.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IrContainer {
    /// Full dotted path.
    pub id: String,
    /// Id as written at the declaration.
    pub local_id: String,
    /// Decoded label.
    pub label: String,
    /// Id of the enclosing container, if any.
    pub parent: Option<String>,
    /// Nesting depth (top-level container = 1).
    pub depth: u32,
    /// Declaration location.
    pub span: Location,
}

/// A resolved edge between two node ids.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IrEdge {
    /// Source node id (full path).
    pub from: String,
    /// Target node id (full path).
    pub to: String,
    /// Decoded label.
    pub label: Option<String>,
    /// Statement location.
    pub span: Location,
}
