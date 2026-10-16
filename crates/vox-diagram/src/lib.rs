//! VoxDiagram core: graph DSL lexer, error-recovering parser, resolved IR, `diagram/*`
//! diagnostics with machine-applicable fixes, canonical formatter and markdown fence scanner.
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
/// Sequence diagram IR types (parser arrives in Phase 86).
pub mod sequence_ir;
/// Byte spans and line/col locations.
pub mod span;

mod check;
mod lexer;
mod parser;
mod resolve;

pub use check::{Parsed, check, parse};
