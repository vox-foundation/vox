//! Recursive-descent parser: lexer tokens → [`vox_ast::decl::Module`].
//!
//! Block structure is delimited by `{` / `}` (`LBrace` / `RBrace`).
//! Error-tolerant entry points support LSP and tooling. Grammar and construct names are defined in
//! `docs/src/reference/lexicon.md`. Low-level parse tree shapes are covered by tests and the
//! grammar schema module; public helpers are documented below.

/// Parse errors and recovery hints.
pub mod error;
/// Recursive-descent parser implementation.
pub mod parser;

pub use error::ParseError;
pub use parser::parse;
