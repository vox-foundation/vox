//! High-level intermediate representation (HIR): AST lowered to a flatter module for codegen and
//! typechecking.
//!
//! **Pipeline position:** after `vox-ast` (CST/AST), before `vox-typeck` and `vox-codegen-*`.
//! See repo-root [`AGENTS.md`](../../../AGENTS.md) §2.1 for the full compiler graph.
//!
//! Items mirror compiler pipeline needs rather than user-facing API. Public types and functions
//! are documented at the definition site (`hir`, `lower`, `def_map`).

/// Name resolution maps (`use`, re-exports).
pub mod def_map;
/// HIR node definitions (expressions, items, spans).
pub mod hir;
/// AST → HIR lowering entrypoints.
pub mod lower;
pub use hir::*;
pub use lower::lower_module;
