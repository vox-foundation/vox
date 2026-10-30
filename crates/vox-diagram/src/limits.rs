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
/// Maximum IR nodes plus containers the SVG renderer lays out (every container is a layout
/// node). Lower than the parse caps because dagre has no work budget and is super-quadratic.
pub const MAX_RENDER_NODES: usize = 100;
/// Maximum IR edges the SVG renderer lays out; see [`MAX_RENDER_NODES`] for why this is lower
/// than the parse cap.
pub const MAX_RENDER_EDGES: usize = 200;
/// Maximum actors the SVG renderer lays out in one sequence diagram. Sequence layout is plain
/// integer arithmetic (no engine), so these render caps only bound output size.
pub const MAX_RENDER_ACTORS: usize = 100;
/// Maximum events (messages plus notes) the SVG renderer lays out in one sequence diagram; see
/// [`MAX_RENDER_ACTORS`].
pub const MAX_RENDER_EVENTS: usize = 200;
