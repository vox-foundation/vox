//! # Vox Type Checker
//!
//! This crate implements the name resolution and bidirectional type checking
//! logic for the Vox compiler. It operates on the High-Level Intermediate
//! Representation (HIR).
//!
//! Key components:
//! - Unification-based type inference
//! - ADT and pattern matching validation
//! - Builtin type registration
//! - Diagnostic reporting for semantic errors

#![allow(clippy::collapsible_if)]

mod ast_decl_lints;
/// Automated fix suggestions for type-check diagnostics.
pub mod autofix;
/// Logic for registering and accessing builtin types and functions.
pub mod builtins;
// DEPRECATED typecheck_module (AST path) removed in Wave 1.
// All type checking now flows through typecheck_hir in Checker.rs.
/// Central state machine for the type checking process.
pub mod checker;
/// Diagnostic structures and error reporting for type checking.
pub mod diagnostics;
/// Environment management for symbols, types, and scopes.
pub mod env;
/// Core logic for unification-based type inference.
pub mod infer;
pub mod policy;
/// Logic for registering declarations into the global environment.
pub mod registration;
/// Representation of internal types used during inference and checking.
pub mod ty;
/// Logic for unification of types and solving constraints.
pub mod unify;

pub use builtins::BuiltinTypes;
pub use checker::{Checker, typecheck_hir};
/// A single semantic diagnostic (error or warning) produced during type checking.
pub use diagnostics::{Diagnostic, Severity};
pub use env::TypeEnv;
pub use ty::ty_display;
pub use unify::InferenceContext;

use crate::ast::decl::Module;
use crate::hir::lower::lower_module;

/// Lower `module` to HIR and run the type Checker (replacement for the removed AST-only path).
#[must_use]
pub fn typecheck_ast_module(source: &str, module: &Module) -> Vec<Diagnostic> {
    let mut diags = ast_decl_lints::lint_ast_declarations(module);
    let hir = lower_module(module);
    let mut env = TypeEnv::new();
    let builtins = BuiltinTypes::register_all(&mut env);
    diags.extend(typecheck_hir(&hir, &mut env, &builtins, source));
    diags
}

/// Type-check a parsed module. Pass `source` for diagnostic context (may be `""`).
///
/// This is the stable name used by the CLI, LSP, and integration tests; it delegates to the HIR pipeline.
#[must_use]
pub fn typecheck_module(module: &Module, source: &str) -> Vec<Diagnostic> {
    typecheck_ast_module(source, module)
}
