//! Unified Vox compiler pipeline.
//!
//! This crate consolidates all core compiler stages: lexing, parsing,
//! AST definition, HIR lowering, type checking, and code generation.

pub mod ast;
pub mod codegen_rust;
pub mod codegen_ts;
pub mod eval;
pub mod fmt;
pub mod hir;
pub mod lexer;
pub mod parser;
pub mod react_bridge;
pub mod ssg;
pub mod typeck;
pub mod web_prefixes;

/// Re-export of common types if needed.
pub use ast::decl::Module;
pub use hir::hir::HirModule;
pub use typeck::checker::Checker;

/// Re-export the canonical formatter so callers use `vox_compiler::format(src)`.
pub use fmt::format;
