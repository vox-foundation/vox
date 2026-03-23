//! Lexer pipeline helpers.

use vox_lexer::lex;
use vox_lexer::cursor::Spanned;

/// Lex the given source into a vector of tokens.
pub fn lex_str(src: &str) -> Vec<Spanned> {
    lex(src)
}
