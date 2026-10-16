//! Document symbols for `vox-lsp`.
//!
//! Walks the AST and maps declarations to LSP symbols (outline view).

use tower_lsp_server::ls_types::*;
pub use vox_compiler::ast::Span;
pub use vox_compiler::ast::decl::*;

/// Walk a parsed module and emit a list of LSP document symbols.
pub fn extract_symbols(module: &Module, text: &str) -> Vec<DocumentSymbol> {
    SymbolEngine::symbols(module, text)
}

pub struct SymbolEngine;

impl SymbolEngine {
    /// Walk a parsed module and emit a list of symbols.
    pub fn symbols(module: &Module, text: &str) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();

        for decl in &module.declarations {
            if let Some(symbol) = Self::decl_to_symbol(decl, text) {
                symbols.push(symbol);
            }
        }

        symbols
    }

    fn decl_to_symbol(decl: &Decl, text: &str) -> Option<DocumentSymbol> {
        match decl {
            Decl::Function(f) => Some(Self::make_symbol(
                &f.name,
                &f.span,
                SymbolKind::FUNCTION,
                vec![],
                text,
            )),
            Decl::V0Component(c) => Some(Self::make_symbol(
                &c.name,
                &c.span,
                SymbolKind::FUNCTION,
                vec![],
                text,
            )),
            Decl::ReactiveComponent(c) => Some(Self::make_symbol(
                &c.name,
                &c.span,
                SymbolKind::FUNCTION,
                vec![],
                text,
            )),
            Decl::Actor(a) => Some(Self::make_symbol(
                &a.name,
                &a.span,
                SymbolKind::CLASS,
                vec![],
                text,
            )),
            Decl::Workflow(w) => Some(Self::make_symbol(
                &w.name,
                &w.span,
                SymbolKind::FUNCTION,
                vec![],
                text,
            )),
            Decl::Activity(a) => Some(Self::make_symbol(
                &a.name,
                &a.span,
                SymbolKind::FUNCTION,
                vec![],
                text,
            )),
            Decl::TypeDef(t) => Some(Self::make_symbol(
                &t.name,
                &t.span,
                SymbolKind::STRUCT,
                vec![],
                text,
            )),
            Decl::Const(c) => Some(Self::make_symbol(
                &c.name,
                &c.span,
                SymbolKind::CONSTANT,
                vec![],
                text,
            )),
            Decl::HttpRoute(h) => {
                let name = format!("{:?} {}", h.method, h.path);
                Some(Self::make_symbol(
                    &name,
                    &h.span,
                    SymbolKind::INTERFACE,
                    vec![],
                    text,
                ))
            }
            Decl::McpTool(m) => Some(Self::make_symbol(
                &m.func.name,
                &m.func.span,
                SymbolKind::FUNCTION,
                vec![],
                text,
            )),
            Decl::Table(t) => Some(Self::make_symbol(
                &t.name,
                &t.span,
                SymbolKind::STRUCT,
                vec![],
                text,
            )),
            Decl::Collection(c) => Some(Self::make_symbol(
                &c.name,
                &c.span,
                SymbolKind::STRUCT,
                vec![],
                text,
            )),
            Decl::Config(c) => Some(Self::make_symbol(
                &c.name,
                &c.span,
                SymbolKind::OBJECT,
                vec![],
                text,
            )),
            _ => None,
        }
    }

    fn make_symbol(
        name: &str,
        span: &Span,
        kind: SymbolKind,
        children: Vec<DocumentSymbol>,
        text: &str,
    ) -> DocumentSymbol {
        let range = Self::span_to_range(span, text);

        #[allow(deprecated)]
        DocumentSymbol {
            name: name.to_string(),
            detail: None,
            kind,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        }
    }

    fn span_to_range(span: &Span, text: &str) -> Range {
        let (start_line, start_col) = crate::byte_index_to_line_col(text, span.start);
        let (end_line, end_col) = crate::byte_index_to_line_col(text, span.end);

        Range {
            start: Position {
                line: start_line,
                character: start_col,
            },
            end: Position {
                line: end_line,
                character: end_col,
            },
        }
    }
}

/// Extract symbols from source, attempting full parse, script parse, or blanking out incomplete lines,
/// and falling back to line scanning if parsing fails.
pub fn extract_symbols_from_source(
    source: &str,
    cursor_line: Option<usize>,
) -> Vec<DocumentSymbol> {
    // 1. Try strict parse
    let tokens = vox_compiler::lexer::lex(source);
    if let Ok(module) = vox_compiler::parser::parse(tokens) {
        return SymbolEngine::symbols(&module, source);
    }

    // 2. Try script parse
    let tokens = vox_compiler::lexer::lex(source);
    if let Ok(module) = vox_compiler::parser::parse_script(tokens) {
        return SymbolEngine::symbols(&module, source);
    }

    // 3. Try blanking the cursor line (incomplete editing buffer)
    if let Some(cline) = cursor_line {
        let cleaned: String = source
            .lines()
            .enumerate()
            .map(|(idx, line)| if idx == cline { "" } else { line })
            .collect::<Vec<_>>()
            .join("\n");
        let tokens = vox_compiler::lexer::lex(&cleaned);
        if let Ok(module) = vox_compiler::parser::parse(tokens) {
            return SymbolEngine::symbols(&module, &cleaned);
        }
        let tokens = vox_compiler::lexer::lex(&cleaned);
        if let Ok(module) = vox_compiler::parser::parse_script(tokens) {
            return SymbolEngine::symbols(&module, &cleaned);
        }
    }

    // 4. Fallback line scan for declarations
    let mut symbols = Vec::new();
    for (line_idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        for (kw, kind) in [
            ("fn ", SymbolKind::FUNCTION),
            ("component ", SymbolKind::FUNCTION),
            ("actor ", SymbolKind::STRUCT),
            ("workflow ", SymbolKind::FUNCTION),
            ("activity ", SymbolKind::FUNCTION),
            ("table ", SymbolKind::STRUCT),
            ("@table ", SymbolKind::STRUCT),
            ("type ", SymbolKind::STRUCT),
        ] {
            if let Some(rest) = trimmed.strip_prefix(kw) {
                let name = rest
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .next()
                    .unwrap_or("")
                    .trim();
                if !name.is_empty() {
                    let col = line.find(name).unwrap_or(0);
                    symbols.push(DocumentSymbol {
                        name: name.to_string(),
                        detail: None,
                        kind,
                        tags: None,
                        #[allow(deprecated)]
                        deprecated: None,
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: col as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (col + name.len()) as u32,
                            },
                        },
                        selection_range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: col as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (col + name.len()) as u32,
                            },
                        },
                        children: None,
                    });
                }
                break;
            }
        }
    }
    symbols
}
