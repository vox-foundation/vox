//! Named input caps shared by the parser and resolver (locked by the phase 83 context).

/// Maximum accepted source size in bytes; larger input is rejected before lexing.
pub const MAX_SOURCE_BYTES: usize = 65_536;
/// Maximum number of nodes in one diagram.
pub const MAX_NODES: usize = 500;
/// Maximum number of edges in one diagram.
pub const MAX_EDGES: usize = 1_000;
/// Maximum container nesting depth (top-level container = 1).
pub const MAX_DEPTH: usize = 4;
/// Maximum decoded label length in Unicode scalar values.
pub const MAX_LABEL_CHARS: usize = 200;
/// Maximum diagnostics reported by one check.
pub const MAX_DIAGNOSTICS: usize = 100;
