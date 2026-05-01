//! Shared pipeline helpers: lex → parse → typecheck → codegen shortcuts.

pub mod codegen;
pub mod lexer;
pub mod parser;
pub mod typeck;

pub use codegen::{codegen_ts_file, codegen_ts_str, lower_str};
pub use parser::parse_str_unwrap;
pub use typeck::{assert_typechecks_cleanly, typecheck_str};
