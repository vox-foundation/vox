//! Shared span helpers for test code.
//!
//! Import from here instead of defining `fn dummy_span()` locally.

use vox_ast::span::Span;

/// A zero-width span at position 0, suitable for test AST/HIR nodes.
///
/// Use this in place of constructing `Span { start: 0, end: 0 }` inline.
pub fn dummy_span() -> Span {
    Span { start: 0, end: 0 }
}
