//! Unified Vox compiler pipeline.
//!
//! This crate consolidates all core compiler stages: lexing, parsing,
//! AST definition, HIR lowering, type checking, and code generation.

pub mod lexer;
pub mod parser;
pub mod ast;
pub mod hir;
pub mod typeck;
pub mod codegen_rust;
pub mod codegen_ts;
pub mod fmt;
pub mod ssg;
pub mod eval;

/// Re-export of common types if needed.
pub use ast::decl::Module;
pub use hir::hir::HirModule;
pub use typeck::checker::Checker;
