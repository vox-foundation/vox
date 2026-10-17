//! VoxDiagram core: graph and sequence DSL lexer, error-recovering parser, resolved IR,
//! `diagram/*` diagnostics with machine-applicable fixes, canonical formatter, markdown fence
//! scanner, graph layout and deterministic SVG rendering of both kinds.
//! Pure and synchronous: no I/O, no async runtime, no workspace dependencies.
//!
//! ```text
//! graph {
//!     node a: "A"
//!     node b: "B"
//!     a -> b
//! }
//! ```
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

/// Syntax tree produced by the parser.
pub mod ast;
/// Stable `diagram/*` diagnostic codes.
pub mod codes;
/// Diagnostics, fixes and the check report.
pub mod diag;
/// Markdown fenced-block scanner shared by every diagram consumer.
pub mod fence;
/// Resolved serialisable diagram IR.
pub mod ir;
/// Named input caps.
pub mod limits;
/// Mermaid export (verified against a pinned real Mermaid parser) and strict, fail-closed Mermaid import.
pub mod mermaid;
/// Sequence diagram IR types, the payload of `Ir::Sequence`.
pub mod sequence_ir;
/// Byte spans and line/col locations.
pub mod span;
/// Deterministic, injection-safe, themeable SVG rendering of graph and sequence diagrams (graph layout through the dagre adapter).
pub mod svg;
/// One-call validation envelope (check, canonical form, optional Mermaid import and SVG/Mermaid emit) shared by the MCP tool and the CLI.
pub mod validate;

mod check;
/// Machine-applicable fixes and the bounded repair loop.
mod fixes;
mod fixgen;
mod layout;
mod lexer;
mod parser;
/// Canonical formatter.
mod print;
mod resolve;

/// Draft VoxDiagram grammar card for agent authoring guidance (finalized in Phase 88).
pub const GRAMMAR_CARD: &str = include_str!("../grammar-card.txt");

pub use check::{Parsed, check, parse};
pub use fixes::{FixOutcome, Repaired, apply_fixes, repair};
pub use print::fmt;
pub use validate::{ValidateOpts, Validation, validate};
