//! Lexer pipeline helpers.

use vox_compiler::lexer::lex;
use vox_compiler::lexer::cursor::Spanned;

/// Lex the given source into a vector of tokens.
pub fn lex_str(src: &str) -> Vec<Spanned> {
    lex(src)
}
