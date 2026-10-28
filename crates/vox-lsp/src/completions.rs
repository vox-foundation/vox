//! Completion engine for `vox-lsp`.
//!
//! Provides context-aware completions for keywords, decorators, types, and builtins.
//!
//! Keywords and lexer-backed decorators come from [`vox_compiler::language_surface`] (SSOT).

use tower_lsp_server::ls_types::*;
use vox_compiler::language_surface;

pub struct CompletionEngine;

impl CompletionEngine {
    /// Returns a list of all completions at a given position.
    pub fn completions(params: CompletionParams) -> CompletionList {
        Self::completions_for_document(&params, None)
    }

    /// Context-aware completions inspecting document text, cursor position, trigger character, and AST symbols.
    pub fn completions_for_document(
        params: &CompletionParams,
        source: Option<&str>,
    ) -> CompletionList {
        let position = params.text_document_position.position;
        let line_text = source
            .and_then(|s| s.lines().nth(position.line as usize))
            .unwrap_or("");
        let char_offset = (position.character as usize).min(line_text.len());
        let prefix_up_to_cursor = &line_text[..char_offset];

        let trigger_char = params
            .context
            .as_ref()
            .and_then(|c| c.trigger_character.as_deref());

        let last_token = prefix_up_to_cursor.split_whitespace().last().unwrap_or("");

        let is_at_trigger = trigger_char == Some("@")
            || prefix_up_to_cursor.trim_end().ends_with('@')
            || last_token.starts_with('@');

        let mut items = Vec::new();

        if is_at_trigger {
            let at_filter = if last_token.starts_with('@') {
                &last_token[1..]
            } else {
                ""
            };
            Self::add_decorators_filtered(&mut items, at_filter);
            return CompletionList {
                is_incomplete: false,
                items,
            };
        }

        let (word_prefix, before_word) = {
            let trimmed = prefix_up_to_cursor;
            if let Some(pos) = trimmed.rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '@')
            {
                (&trimmed[pos + 1..], &trimmed[..=pos])
            } else {
                (trimmed, "")
            }
        };

        let is_type_position =
            before_word.trim_end().ends_with(':') || before_word.trim_end().ends_with("->");

        if is_type_position {
            Self::add_types_filtered(&mut items, word_prefix);
            if let Some(src) = source {
                Self::add_ast_types_filtered(
                    &mut items,
                    src,
                    word_prefix,
                    Some(position.line as usize),
                );
            }
            return CompletionList {
                is_incomplete: false,
                items,
            };
        }

        // General completions: keywords, builtins, types, decorators, and AST symbols
        Self::add_keywords_filtered(&mut items, word_prefix);
        Self::add_builtins_filtered(&mut items, word_prefix);
        Self::add_types_filtered(&mut items, word_prefix);
        Self::add_decorators_filtered(&mut items, word_prefix);

        if let Some(src) = source {
            Self::add_ast_symbols_filtered(
                &mut items,
                src,
                word_prefix,
                Some(position.line as usize),
            );
        }

        CompletionList {
            is_incomplete: false,
            items,
        }
    }

    fn matches_filter(candidate: &str, filter: &str) -> bool {
        if filter.is_empty() {
            true
        } else {
            candidate.to_lowercase().starts_with(&filter.to_lowercase())
        }
    }

    fn add_keywords_filtered(items: &mut Vec<CompletionItem>, filter: &str) {
        for &(name, snippet) in language_surface::LSP_KEYWORD_SNIPPETS {
            if Self::matches_filter(name, filter) {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::KEYWORD),
                    insert_text: Some(snippet.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
        }
    }

    fn add_decorators_filtered(items: &mut Vec<CompletionItem>, filter: &str) {
        let docs: std::collections::HashMap<&str, &str> = language_surface::LSP_DECORATOR_DOCS
            .iter()
            .cloned()
            .collect();

        for &(name, snippet) in language_surface::LSP_DECORATOR_SNIPPETS {
            let clean_name = name.strip_prefix('@').unwrap_or(name);
            let clean_filter = filter.strip_prefix('@').unwrap_or(filter);
            if Self::matches_filter(clean_name, clean_filter) {
                let doc = docs.get(name).cloned().unwrap_or("");
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    documentation: Some(Documentation::String(doc.to_string())),
                    insert_text: Some(snippet.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
        }
    }

    fn add_types_filtered(items: &mut Vec<CompletionItem>, filter: &str) {
        for &t in language_surface::SURFACE_TYPE_NAMES {
            if Self::matches_filter(t, filter) {
                items.push(CompletionItem {
                    label: t.to_string(),
                    kind: Some(CompletionItemKind::STRUCT),
                    detail: Some("Built-in Type".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    fn add_builtins_filtered(items: &mut Vec<CompletionItem>, filter: &str) {
        for &(name, sig) in language_surface::LSP_BUILTIN_SNIPPETS {
            if Self::matches_filter(name, filter) {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    insert_text: Some(sig.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
        }
    }

    fn add_ast_types_filtered(
        items: &mut Vec<CompletionItem>,
        source: &str,
        filter: &str,
        cursor_line: Option<usize>,
    ) {
        let symbols = crate::symbols::extract_symbols_from_source(source, cursor_line);
        for sym in symbols {
            if matches!(sym.kind, SymbolKind::STRUCT | SymbolKind::INTERFACE)
                && Self::matches_filter(&sym.name, filter)
            {
                items.push(CompletionItem {
                    label: sym.name,
                    kind: Some(CompletionItemKind::STRUCT),
                    detail: Some("User-defined Type".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    fn add_ast_symbols_filtered(
        items: &mut Vec<CompletionItem>,
        source: &str,
        filter: &str,
        cursor_line: Option<usize>,
    ) {
        let symbols = crate::symbols::extract_symbols_from_source(source, cursor_line);
        for sym in symbols {
            if Self::matches_filter(&sym.name, filter) {
                let kind = match sym.kind {
                    SymbolKind::FUNCTION => CompletionItemKind::FUNCTION,
                    SymbolKind::STRUCT => CompletionItemKind::STRUCT,
                    SymbolKind::CONSTANT => CompletionItemKind::CONSTANT,
                    SymbolKind::INTERFACE => CompletionItemKind::INTERFACE,
                    SymbolKind::OBJECT => CompletionItemKind::MODULE,
                    _ => CompletionItemKind::VARIABLE,
                };
                items.push(CompletionItem {
                    label: sym.name,
                    kind: Some(kind),
                    detail: Some("Document Symbol".to_string()),
                    ..Default::default()
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_at_trigger_returns_decorators_with_doc() {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///test.vox".parse().unwrap(),
                },
                position: Position {
                    line: 0,
                    character: 1,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: Some(CompletionContext {
                trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER,
                trigger_character: Some("@".to_string()),
            }),
        };
        let res = CompletionEngine::completions_for_document(&params, Some("@"));
        assert!(!res.items.is_empty());
        assert!(
            res.items
                .iter()
                .any(|item| item.label == "@tool" || item.label.starts_with('@'))
        );
        assert!(
            !res.items
                .iter()
                .any(|item| item.label == "fn" || item.label == "let")
        );
        assert!(res.items.iter().all(|item| item.documentation.is_some()));
    }

    #[test]
    fn test_type_position_prioritizes_surface_type_names() {
        let text = "let x: ";
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///test.vox".parse().unwrap(),
                },
                position: Position {
                    line: 0,
                    character: text.len() as u32,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };
        let res = CompletionEngine::completions_for_document(&params, Some(text));
        assert!(res.items.iter().any(|item| item.label == "str"));
        assert!(res.items.iter().any(|item| item.label == "int"));
        assert!(
            !res.items
                .iter()
                .any(|item| item.label == "let" || item.label == "return")
        );
    }

    #[test]
    fn test_document_ast_symbol_included_in_completions() {
        let text = "fn calculate_tax() -> int { 0 }\ncal";
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: "file:///test.vox".parse().unwrap(),
                },
                position: Position {
                    line: 1,
                    character: 3,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };
        let res = CompletionEngine::completions_for_document(&params, Some(text));
        assert!(
            res.items.iter().any(|item| item.label == "calculate_tax"),
            "Expected calculate_tax to be present in completions"
        );
    }
}
