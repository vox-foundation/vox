//! High-level intermediate representation (HIR): AST lowered to a flatter module for codegen and
//! typechecking.
//!
//! **Pipeline position:** after the AST (`parse` → `ast::decl::Module`), before `typeck` and `codegen_*`.
//! See [`docs/src/explanation/expl-architecture.md`](../../../docs/src/explanation/expl-architecture.md).
//!
//! Items mirror compiler pipeline needs rather than user-facing API. Public types and functions
//! are documented at the definition site (`nodes`, `lower`, `def_map`).
//!
//! Note: historical cross-module import resolver prototypes were retired; active
//! import binding now flows through type registration/checker passes.

/// Name resolution maps (`use`, re-exports).
pub mod def_map;
/// AST → HIR lowering entrypoints.
pub mod lower;
/// HIR node definitions (expressions, items, spans).
pub mod nodes;
/// Structural validation after lowering (invariants for codegen/type consumers).
pub mod validate;
pub use lower::{lower_classic_component_view, lower_module};
pub use nodes::*;
pub use validate::{HirValidationError, validate_module};
