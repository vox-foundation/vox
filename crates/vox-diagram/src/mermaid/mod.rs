//! Mermaid export and strict import.
//!
//! [`export()`](crate::mermaid::export()) writes one Mermaid statement per output line, so a label can never smuggle a
//! second statement; every label goes through the single encoder in `escape`. Mermaid's
//! default configuration rejects more than 500 edges while the DSL allows 1000, so a very
//! large diagram can export validly yet fail to render; the exporter cannot fix that.
//! [`import()`](crate::mermaid::import()) is fail-closed: a block it cannot read in full yields no output, only the list
//! of offending lines. `Unsupported::line` is 1-based within the block.
//!
//! The importer must agree with Mermaid's own parser (pinned at 11.17.2 and checked by the
//! `docs-astro` oracle): it reads exactly what Mermaid accepts, with the same meaning, and refuses
//! the rest. Reserved words are position-aware (`subgraph style` and `A --> Note: hi` import,
//! `A --> Note` does not). Deliberate over-refusals: `direction <TB|TD|LR|RL|BT>` in any case
//! inside text or split across lines (Mermaid's flowchart lexer is case-sensitive, its state
//! lexer is not); a state transition `A --> B: ` whose label is only a space (lines are trimmed
//! before scanning); a lone `o` / `x` id fused to an arrow. Render-only semantics (`fa:` icons,
//! `$$` maths, the 50,000-character text limit) are outside the parser and not checked.

use crate::sequence_ir::SequenceIr;

mod escape;
mod export;
mod export_seq;
mod import;

pub use export::export;
/// Sequence IR to Mermaid text; [`export()`](crate::mermaid::export()) routes `Ir::Sequence` here.
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

/// Result of [`import`]. Invariant, for every diagram kind: `source.is_some()` exactly when
/// `unsupported` is empty (the import failed closed otherwise, and `sequence` is `None` too).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ImportReport {
    /// Canonical VoxDiagram source (a `fmt` fixed point) for an imported graph, state or
    /// `sequenceDiagram` block.
    pub source: Option<String>,
    /// For a `sequenceDiagram` block only: the same diagram as IR, with spans located in the
    /// Mermaid block. Kept next to `source` on purpose: the IR of `source` carries spans into
    /// the DSL text, so only this field lets a caller migrating a fence point back at the
    /// original block's line numbers. Re-checking `source` yields the same IR, spans aside.
    pub sequence: Option<SequenceIr>,
    /// Every line the importer refused; the import failed closed when non-empty.
    pub unsupported: Vec<Unsupported>,
    /// Lines deliberately dropped (styling under `drop_styling`, redundant directions).
    pub dropped: Vec<Unsupported>,
}
