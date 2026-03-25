//! Lexer pipeline helpers.

use vox_compiler::lexer::cursor::Spanned;
use vox_compiler::lexer::lex;

/// Lex the given source into a vector of tokens.
pub fn lex_str(src: &str) -> Vec<Spanned> {
    lex(src)
}
