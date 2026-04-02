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
//! | Imports | `import` (`react.use_state`, `rust:serde_json`) |
//! | Components | `@component` |
//! | Islands | `@island` |
//! | Database tables & indices | `@table`, `@index` |
//! | MCP tools / resources | `@mcp.tool`, `@mcp.resource` |
//! | Tests | `@test` |
//! | Server functions | `@server` |
//! | v0 components | `@v0` |
//! | Route loading UI | `@loading` |
//! | Actors & workflows | `actor`, `workflow`, `activity` |
//! | HTTP routes | `http get/post/put/delete` |
//! | JSX | `<Tag ...>` / `<Tag ... />` |
//! | Expressions | arithmetic, logic, match, if/else, for, spawn, pipe `|>` |
//!
//! ## Out of scope today
//!
//! Declarations named in older roadmaps (`@page`, `@layout`, `@theme`, …) appear in the AST type
//! definitions for future work but are **not** produced by this parser. Use `routes:`, `@component`,
//! `@server`, and `@island` for the supported web stack (see `docs/src/reference/vox-web-stack.md`).
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

/// Recursive-descent parser implementation.
pub mod descent;
/// Parse errors and recovery hints.
pub mod error;

pub use descent::parse;
pub use error::{ParseError, ParseErrorClass};

/// Brace-first web declaration forms the descent parser accepts (doc extraction / inventory; OP-0015).
pub const WEB_SURFACE_SYNTAX_INVENTORY: &[&str] = &[
    "`@island Name { prop: Type }` / `prop?: Type` — `{` is required immediately after the island name",
    "`@component fn Name(...) to Type { ... }` — classic component; only `fn` may follow `@component` for this form",
    "`@component Name(...) { ... }` or `component Name(...) { ... }` — Path C reactive body: `state`, `derived`, `effect`, `mount`, `cleanup`, `view:`",
    "`routes { \"/path\" to Component ... }` — string literal path, keyword `to`, then component identifier; `{` right after `routes`",
];
