//! Unified Vox compiler pipeline.
//!
//! This crate consolidates all core compiler stages: lexing, parsing,
//! AST definition, HIR lowering, type checking, and code generation.

pub mod app_contract;
pub mod ast;
pub mod builtin_registry;
pub mod codegen_rust;
pub mod codegen_ts;
pub mod eval;
pub mod fmt;
pub mod generated_vox;
pub mod hir;
pub mod lexer;
pub mod parser;
pub mod react_bridge;
pub mod runtime_projection;
pub mod serialization;
pub mod syntax_k;
pub mod typeck;
pub mod web_ir;
pub mod web_prefixes;

/// Re-export of common types if needed.
pub use ast::decl::Module;
pub use hir::{HirModule, TypedCoreIR_v2};
pub use typeck::checker::Checker;

/// Re-export the canonical formatter so callers use `vox_compiler::format(src)`.
pub use fmt::format;
/// Re-export canonical compact serializer for deterministic `.vox` output.
pub use serialization::canonicalize_vox;
