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
#[test]
fn retired_decorators_not_in_lsp_list() {
    // @component is retired; must not appear in LSP suggestions
    assert!(
        !language_surface::LEXER_DECORATORS.contains(&"@component"),
        "@component is retired and must not be in LEXER_DECORATORS"
    );
}

#[test]
fn deprecated_keywords_not_in_lexer_list() {
    // ret is deprecated; must not appear in main keyword list
    assert!(
        !language_surface::LEXER_KEYWORDS.contains(&"ret"),
        "ret is deprecated and must not be in LEXER_KEYWORDS"
    );
}
