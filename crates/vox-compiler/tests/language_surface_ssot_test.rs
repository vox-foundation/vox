//! Guardrails: LSP decorator docs must map to lexer `@` tokens (see `language_surface.rs`).

use vox_compiler::language_surface;

#[test]
fn lsp_decorator_spellings_exist_in_lexer_list() {
    for &(d, _) in language_surface::LSP_DECORATOR_DOCS {
        assert!(
            language_surface::LEXER_DECORATORS.contains(&d),
            "{d} is documented for LSP but missing from LEXER_DECORATORS — add to lexer `Token` or trim LSP list"
        );
    }
}
