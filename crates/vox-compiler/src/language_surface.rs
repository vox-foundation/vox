//! Single source of truth for **keyword** and **decorator** strings exposed to LSP, MCP, and docs.
//!
//! Keep lexer-derived lists aligned with [`crate::lexer::token::Token`] (`token.rs`). LSP snippets may
//! include forms that parse as `Ident` today; those live only in [`LSP_KEYWORD_SNIPPETS`].
//!
//! See `docs/src/architecture/language-surface-ssot.md`.

/// LSP keyword completions: `(label, snippet)`.
/// Includes parser-level keywords that may still lex as identifiers until dedicated tokens exist.
pub const LSP_KEYWORD_SNIPPETS: &[(&str, &str)] = &[
    ("fn", "fn $1($2): \n\t$0"),
    ("let", "let $1 = $0"),
    ("mut", "mut $1 = $0"),
    ("if", "if $1: \n\t$0"),
    ("else", "else: \n\t$0"),
    ("for", "for $1 in $2: \n\t$0"),
    ("match", "match $1: \n\t$2 -> $0"),
    ("ret", "ret $0"),
    ("return", "return $0"),
    ("while", "while $1: \n\t$0"),
    ("loop", "loop: \n\t$0"),
    ("break", "break"),
    ("continue", "continue"),
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

/// Decorators with dedicated lexer tokens — `(spelling, LSP doc)`.
pub const LSP_DECORATOR_DOCS: &[(&str, &str)] = &[
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
    (
        "@mcp.resource",
        "Read-only MCP resource (URI + description; nullary fn body).",
    ),
    ("@test", "Marks a function as a test case."),
    ("@v0", "Placeholder for v0.dev-generated UI hook."),
    (
        "@external",
        "External binding marker (see parser / web stack docs).",
    ),
];

/// Keywords that have dedicated single-word lexer tokens (speech / strict introspection).
pub const LEXER_KEYWORDS: &[&str] = &[
    "fn",
    "let",
    "mut",
    "if",
    "else",
    "match",
    "for",
    "in",
    "to",
    "ret",
    "return",
    "while",
    "loop",
    "break",
    "continue",
    "type",
    "import",
    "actor",
    "workflow",
    "activity",
    "spawn",
    "http",
    "pub",
    "with",
    "on",
    "state",
    "derived",
    "effect",
    "mount",
    "cleanup",
    "view",
    "component",
    "and",
    "or",
    "not",
    "is",
    "isnt",
    "true",
    "false",
    "get",
    "post",
    "put",
    "delete",
];

/// `@decorator` spellings from the lexer (stable order).
pub const LEXER_DECORATORS: &[&str] = &[
    "@component",
    "@mcp.tool",
    "@mcp.resource",
    "@external",
    "@test",
    "@server",
    "@query",
    "@mutation",
    "@table",
    "@index",
    "@v0",
    "@island",
    "@loading",
];

/// Builtin names for LSP / MCP “surface” introspection (aligned with common runtime helpers).
pub const SURFACE_BUILTIN_NAMES: &[&str] = &[
    "print", "len", "push", "pop", "now", "sleep", "hash", "uuid", "random",
];

/// LSP builtin snippets: `(name, insertText)`.
pub const LSP_BUILTIN_SNIPPETS: &[(&str, &str)] = &[
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

/// Surface type names shown in completions / introspection.
pub const SURFACE_TYPE_NAMES: &[&str] = &[
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
