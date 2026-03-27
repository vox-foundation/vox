//! Completion engine for `vox-lsp`.
//!
//! Provides context-aware completions for keywords, decorators, types, and builtins.
//!
//! Decorators listed here match the **canonical parser** in `vox-compiler` (see
//! `crates/vox-compiler/src/parser/mod.rs`). Do not suggest top-level forms the lexer
//! cannot parse.

use tower_lsp::lsp_types::*;

pub struct CompletionEngine;

impl CompletionEngine {
    /// Returns a list of all completions at a given position.
    /// Initially keyword-only, expanding to be context-aware.
    pub fn completions(_params: CompletionParams) -> CompletionList {
        let mut items = Vec::new();

        // 1. Keyword completions
        Self::add_keywords(&mut items);

        // 2. Decorator completions (if triggered by @)
        Self::add_decorators(&mut items);

        // 3. Type completions
        Self::add_types(&mut items);

        // 4. Builtin completions
        Self::add_builtins(&mut items);

        CompletionList {
            is_incomplete: false,
            items,
        }
    }

    fn add_keywords(items: &mut Vec<CompletionItem>) {
        let keywords = vec![
            ("fn", "fn $1($2): \n\t$0"),
            ("let", "let $1 = $0"),
            ("mut", "mut $1 = $0"),
            ("if", "if $1: \n\t$0"),
            ("else", "else: \n\t$0"),
            ("for", "for $1 in $2: \n\t$0"),
            ("match", "match $1: \n\t$2 -> $0"),
            ("ret", "ret $0"),
            ("type", "type $1 = $2"),
            ("import", "import \"$1\""),
            ("actor", "actor $1($2): \n\t$0"),
            ("workflow", "workflow $1($2): \n\t$0"),
            ("activity", "activity $1($2): \n\t$0"),
            ("spawn", "spawn $0"),
            ("http", "http $1: \n\t$0"),
            ("pub", "pub $0"),
            ("with", "with $1: \n\t$0"),
            ("on", "on $1($2): \n\t$0"),
            ("struct", "struct $1: \n\t$0"),
            ("enum", "enum $1: \n\t$0"),
            ("trait", "trait $1: \n\t$0"),
            ("impl", "impl $1 for $2: \n\t$0"),
            ("const", "const $1 = $0"),
            ("message", "message $1($2)"),
            ("state", "state $1: $2"),
            ("routes", "routes: \n\t$0"),
            ("to", "to $0"),
            ("from", "from $0"),
            ("use", "use $0"),
        ];

        for (name, snippet) in keywords {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }
    }

    fn add_decorators(items: &mut Vec<CompletionItem>) {
        // SSOT: `vox-compiler` lexer tokens `@component`, `@island`, `@server`, …
        let decorators = vec![
            (
                "@component",
                "Reactive or legacy `fn` component (see Vox web stack docs).",
            ),
            (
                "@island",
                "Typed stub for a React island under `islands/` (hydration mount point).",
            ),
            (
                "@loading",
                "Route suspense UI (`fn` → `*.tsx`); TanStack Router `pendingComponent` when `routes:` exists.",
            ),
            (
                "@server",
                "Server function (Axum route + TS client wrapper).",
            ),
            ("@table", "Declares a persistent database table."),
            ("@index", "Declares a database index."),
            ("@query", "Declares a database query function."),
            ("@mutation", "Declares a database mutation function."),
            ("@mcp.tool", "Exposes a function as an MCP tool."),
            ("@test", "Marks a function as a test case."),
            ("@v0", "Placeholder for v0.dev-generated UI hook."),
        ];

        for (name, doc) in decorators {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                documentation: Some(Documentation::String(doc.to_string())),
                insert_text: Some(name.to_string()),
                ..Default::default()
            });
        }
    }

    fn add_types(items: &mut Vec<CompletionItem>) {
        let types = vec![
            "int",
            "str",
            "bool",
            "float",
            "Unit",
            "Element",
            "List[T]",
            "Map[K, V]",
            "Set[T]",
            "Result[T, E]",
            "Option[T]",
        ];

        for t in types {
            items.push(CompletionItem {
                label: t.to_string(),
                kind: Some(CompletionItemKind::STRUCT),
                detail: Some("Built-in Type".to_string()),
                ..Default::default()
            });
        }
    }

    fn add_builtins(items: &mut Vec<CompletionItem>) {
        let builtins = vec![
            ("print", "print($0)"),
            ("len", "len($0)"),
            ("push", "push($1, $0)"),
            ("pop", "pop($0)"),
            ("now", "now()"),
            ("sleep", "sleep($1)"),
            ("hash", "hash($0)"),
            ("uuid", "uuid()"),
            ("random", "random()"),
        ];

        for (name, sig) in builtins {
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
