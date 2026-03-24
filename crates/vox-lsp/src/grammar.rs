//! Semantic token type mapping for the Vox LSP.
//!
//! Maps [`vox_compiler::lexer::token::Token`] variants to LSP semantic token type indices
//! matching the `SEMANTIC_TOKEN_TYPES` legend order.

use tower_lsp::lsp_types::SemanticTokenType;
use vox_compiler::lexer::token::Token;

/// Maps a Vox lexer [`Token`] to an LSP semantic token type index, or `None`
/// for tokens that do not need syntax highlighting (punctuation, whitespace, etc.).
///
/// The returned index corresponds to the position in [`SEMANTIC_TOKEN_TYPES`].
pub fn token_to_semantic_type(token: &Token) -> Option<u32> {
    match token {
        // ── Keywords (index 2) ────────────────────────────────────────────────
        Token::Fn
        | Token::Let
        | Token::Mut
        | Token::If
        | Token::Else
        | Token::For
        | Token::Match
        | Token::TypeKw
        | Token::Actor
        | Token::Workflow
        | Token::Activity
        | Token::Import
        | Token::True
        | Token::False
        | Token::And
        | Token::Or
        | Token::Not
        | Token::Spawn
        | Token::On
        | Token::To
        | Token::With
        | Token::Pub
        | Token::Ret
        | Token::Http
        | Token::Get
        | Token::Post
        | Token::Put
        | Token::Delete
        | Token::In
        | Token::Is
        | Token::Isnt => Some(2), // KEYWORD

        // ── Literals ─────────────────────────────────────────────────────────
        Token::IntLit(_) | Token::FloatLit(_) => Some(5), // NUMBER
        Token::StringLit(_) | Token::SingleQuoteStringLit(_) => Some(4), // STRING

        // ── Comments ─────────────────────────────────────────────────────────
        Token::Comment => Some(6), // COMMENT

        // ── Decorators (index 7) ─────────────────────────────────────────────
        Token::AtComponent
        | Token::AtMcpTool
        | Token::AtExternal
        | Token::AtTest
        | Token::AtServer
        | Token::AtTable
        | Token::AtIndex
        | Token::AtV0
        | Token::AtIsland => Some(7), // DECORATOR

        // ── Identifiers ───────────────────────────────────────────────────────
        Token::Ident(_) => Some(1),     // VARIABLE
        Token::TypeIdent(_) => Some(3), // TYPE

        // Punctuation, newlines, JSX, EOF — no highlighting
        _ => None,
    }
}

/// The ordered list of semantic token types supported by the Vox LSP.
///
/// The slice index MUST match the integer returned by [`token_to_semantic_type`].
pub const SEMANTIC_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::FUNCTION,  // 0
    SemanticTokenType::VARIABLE,  // 1
    SemanticTokenType::KEYWORD,   // 2
    SemanticTokenType::TYPE,      // 3
    SemanticTokenType::STRING,    // 4
    SemanticTokenType::NUMBER,    // 5
    SemanticTokenType::COMMENT,   // 6
    SemanticTokenType::DECORATOR, // 7
    SemanticTokenType::PARAMETER, // 8
];
