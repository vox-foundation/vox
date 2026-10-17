//! Documentation linter: scans `docs/src/` for frontmatter, code fence, and doctest issues.

pub mod claims;
pub mod pipeline;

/// The VoxDiagram core (parser, checker, fence scanner, Mermaid and SVG), re-exported so vox-cli reaches it through this crate's existing edge.
pub use vox_diagram as diagram;
