//! # vox-parser — primary compiler parser
//!
//! **This is the canonical, single-source-of-truth parser** for the Vox compiler
//! pipeline. It transforms a [`crate::lexer`] token stream into a [`crate::ast::decl::Module`].
//!
//! ## Scope: what this parser covers
//!
//! | Construct | Token(s) |
//! |---|---|
//! | Functions / closures | `fn`, `pub fn` |
//! | Type definitions & ADTs | `type`, `pub type` |
//! | Imports | `import` |
//! | Components | `@component` |
//! | Islands | `@island` |
//! | Database tables & indices | `@table`, `@index` |
//! | MCP tools | `@mcp.tool` |
//! | Tests | `@test` |
//! | Server functions | `@server` |
//! | v0 components | `@v0` |
//! | Actors & workflows | `actor`, `workflow`, `activity` |
//! | HTTP routes | `http get/post/put/delete` |
//! | JSX | `<Tag ...>` / `<Tag ... />` |
//! | Expressions | arithmetic, logic, match, if/else, for, spawn, pipe `|>` |
//!
//! ## Out of scope (parsed downstream)
//!
//! Extended full-stack declarations — `@page`, `@partial`, `@theme`, `@layout`,
//! `@i18n`, `@schema`, `@action` — are **not** part of this parser's grammar.
//! They are handled by crates that consume `vox-ast` output and augment it:
//! `vox-codegen-ts`, `vox-codegen-rust`, and related pipeline stages.
//!
//! ## Error strategy
//!
//! The parser collects all errors into a [`Vec<ParseError>`] and returns
//! `Err(errors)` only at the end, allowing partial ASTs to be produced for
//! LSP and tooling use-cases. It never panics on well-formed or malformed input.
//!
//! Block structure uses `{` / `}` (`LBrace` / `RBrace`). Indentation is
//! advisory; the brace tokens are authoritative. Grammar and construct names
//! mirror `docs/src/reference/lexicon.md`.

/// Parse errors and recovery hints.
pub mod error;
/// Recursive-descent parser implementation.
pub mod parser;

pub use error::ParseError;
pub use parser::parse;
