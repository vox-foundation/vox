//! Typechecking pipeline helpers.

use vox_ast::decl::Module;
use vox_typeck::{typecheck_module, Diagnostic};
use crate::pipeline::parser::parse_str_unwrap;
use crate::assertions::assert_no_errors;

/// Lex, parse, and typecheck `src`, returning the module and all diagnostics.
///
/// Parse errors are panicked on — use `parse_str_unwrap` first if you want
/// to check typecheck output only.
pub fn typecheck_str(src: &str) -> (Module, Vec<Diagnostic>) {
    let module = parse_str_unwrap(src);
    let diags = typecheck_module(&module, "");
    (module, diags)
}

/// Typecheck `src` and assert that there are zero error-severity diagnostics.
///
/// Warnings are permitted. Panics with all error messages on failure.
#[track_caller]
pub fn assert_typechecks_cleanly(src: &str) {
    let (_module, diags) = typecheck_str(src);
    assert_no_errors(&diags);
}
