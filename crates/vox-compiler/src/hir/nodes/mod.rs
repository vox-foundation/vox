//! Concrete HIR types: one lowered module with separate vectors per top-level construct.
//!
//! **Names:** Identifiers and resolved paths in [`HirExpr`] / patterns reflect the lowering pass
//! ([`crate::hir::lower`]), not necessarily raw source spelling. Prefer [`crate::ast::span::Span`] on each node for
//! diagnostics rather than re-parsing names.
//!
//! **Consumers:** `vox-typeck` and codegen read these types; keep new variants backward-compatible
//! or bump all match sites in the same change.

mod decl;
mod expr;
mod stmt;
mod stmt_expr;

pub use decl::*;
pub use expr::*;
pub use stmt::*;
pub use stmt_expr::{DefId, HirParam, HirType};
