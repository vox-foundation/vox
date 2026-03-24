//! Shared assertion helpers for compiler diagnostic tests.
//!
//! Provides typed helpers over `Vec<Diagnostic>` so test files don't
//! repeat the same filter/map patterns.

use vox_compiler::typeck::{diagnostics::Severity, Diagnostic};

/// Returns `true` if `diags` contains at least one error-severity diagnostic.
pub fn has_error(diags: &[Diagnostic]) -> bool {
    diags.iter().any(|d| d.severity == Severity::Error)
}

/// Returns the `message` field of every error-severity diagnostic in `diags`.
pub fn error_messages(diags: &[Diagnostic]) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.message.clone())
        .collect()
}

/// Returns the `message` field of every warning-severity diagnostic in `diags`.
pub fn warning_messages(diags: &[Diagnostic]) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .map(|d| d.message.clone())
        .collect()
}

/// Asserts that `diags` contains zero error-severity diagnostics.
///
/// Prints all error messages on failure.
#[track_caller]
pub fn assert_no_errors(diags: &[Diagnostic]) {
    let errs = error_messages(diags);
    assert!(errs.is_empty(), "Expected no type errors, got:\n{}", errs.join("\n"));
}
