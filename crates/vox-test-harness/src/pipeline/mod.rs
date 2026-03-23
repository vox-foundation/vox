//! Shared pipeline helpers: lex → parse → typecheck shortcuts for test code.

pub mod lexer;
pub mod parser;
pub mod typeck;

pub use parser::parse_str_unwrap;
pub use typeck::{assert_typechecks_cleanly, typecheck_str};
