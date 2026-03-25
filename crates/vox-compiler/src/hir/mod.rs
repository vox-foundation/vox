//! High-level intermediate representation (HIR): AST lowered to a flatter module for codegen and
//! typechecking.
//!
//! **Pipeline position:** after `vox-ast` (CST/AST), before `vox-typeck` and `vox-codegen-*`.
//! See repo-root [`AGENTS.md`](../../../AGENTS.md) §2.1 for the full compiler graph.
//!
//! Items mirror compiler pipeline needs rather than user-facing API. Public types and functions
//! are documented at the definition site (`nodes`, `lower`, `def_map`).

/// Name resolution maps (`use`, re-exports).
pub mod def_map;
/// AST → HIR lowering entrypoints.
pub mod lower;
/// HIR node definitions (expressions, items, spans).
pub mod nodes;
pub use lower::lower_module;
pub use nodes::*;
