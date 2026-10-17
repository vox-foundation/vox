//! Mermaid export and strict import.
//!
//! [`export`] writes one Mermaid statement per output line, so a label can never smuggle a
//! second statement; every label goes through the single encoder in `escape`. Mermaid's
//! default configuration rejects more than 500 edges while the DSL allows 1000, so a very
//! large diagram can export validly yet fail to render; the exporter cannot fix that.
//! [`import`] is fail-closed: a block it cannot read in full yields no output, only the list
//! of offending lines. `Unsupported::line` is 1-based within the block.

use crate::sequence_ir::SequenceIr;

mod escape;
mod export;
mod export_seq;
mod import;

pub use export::export;
/// Sequence IR to Mermaid text; Phase 86 routes `Ir::Sequence` here.
pub use export_seq::export_sequence;
pub use import::import;

/// Options for [`import`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportOpts {
    /// Drop styling statements (reported in `ImportReport::dropped`) instead of refusing them.
    pub drop_styling: bool,
}

/// A construct the importer did not accept (or dropped), with its block-relative line.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Unsupported {
    /// 1-based physical line within the block.
    pub line: u32,
    /// Short construct name, for example `statement` or `header:gantt`.
    pub construct: String,
}

/// Result of [`import`]. `unsupported` is non-empty iff both `source` and `sequence` are
/// `None`; otherwise exactly one of them is `Some`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ImportReport {
    /// Canonical VoxDiagram source for a graph or state block.
    pub source: Option<String>,
    /// IR for a `sequenceDiagram` block (there is no sequence DSL yet).
    pub sequence: Option<SequenceIr>,
    /// Every line the importer refused; the import failed closed when non-empty.
    pub unsupported: Vec<Unsupported>,
    /// Lines deliberately dropped (styling under `drop_styling`, redundant directions).
    pub dropped: Vec<Unsupported>,
}
