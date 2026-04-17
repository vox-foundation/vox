//! Single source of truth for **keyword** and **decorator** strings exposed to LSP, MCP, and docs.
//!
//! Keep lexer-derived lists aligned with [`crate::lexer::token::Token`] (`token.rs`). LSP snippets may
//! include forms that parse as `Ident` today; those live only in [`LSP_KEYWORD_SNIPPETS`].
//!
//! See `docs/src/architecture/language-surface-ssot.md`.

/// LSP keyword completions: `(label, snippet)`.
/// Includes parser-level keywords that may still lex as identifiers until dedicated tokens exist.
pub const LSP_KEYWORD_SNIPPETS: &[(&str, &str)] = &[
    ("fn", "fn $1($2) { \n\t$0 \n}"),
    ("let", "let $1 = $0"),
    ("mut", "mut $1 = $0"),
    ("if", "if $1 { \n\t$0 \n}"),
    ("else", "else { \n\t$0 \n}"),
    ("for", "for $1 in $2 { \n\t$0 \n}"),
    ("match", "match $1 { \n\t$2 -> $0 \n}"),
    ("return", "return $0"),
    ("while", "while $1 { \n\t$0 \n}"),
    ("loop", "loop { \n\t$0 \n}"),
    ("break", "break"),
    ("continue", "continue"),
    ("type", "type $1 = $2"),
    ("import", "import \"$1\""),
    ("actor", "actor $1($2) { \n\t$0 \n}"),
    ("workflow", "workflow $1($2) { \n\t$0 \n}"),
    ("activity", "activity $1($2) { \n\t$0 \n}"),
    ("spawn", "spawn $0"),
    ("http", "http $1 { \n\t$0 \n}"),
    ("pub", "pub $0"),
    ("with", "with $1 { \n\t$0 \n}"),
    ("on", "on $1($2) { \n\t$0 \n}"),
    ("struct", "struct $1 { \n\t$0 \n}"),
    ("enum", "enum $1 { \n\t$0 \n}"),
    ("trait", "trait $1 { \n\t$0 \n}"),
    ("impl", "impl $1 for $2 { \n\t$0 \n}"),
    ("const", "const $1 = $0"),
    ("message", "message $1($2)"),
    ("state", "state $1: $2"),
    ("routes", "routes { \n\t$0 \n}"),
    ("to", "to $0"),
    ("from", "from $0"),
    ("use", "use $0"),
];

/// Decorators with dedicated lexer tokens — `(spelling, LSP doc)`.
pub const LSP_DECORATOR_DOCS: &[(&str, &str)] = &[
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
        "@require",
        "Adds a runtime validation guard (precondition).",
    ),
    (
        "@ensure",
        "Adds a runtime validation guard (postcondition).",
    ),
    (
        "@invariant",
        "Adds a runtime validation guard evaluated on both bounds.",
    ),
    ("@forall", "Marks a test for property-based generation."),
    ("@fuzz", "Marks a test for fuzzing iteration bounds."),
    (
        "@pure",
        "Marks a function as side-effect free (optimization / tooling contracts).",
    ),
    (
        "@scheduled",
        "Declares a periodic job with an interval or cron string before `fn`.",
    ),
    (
        "@deprecated",
        "Marks a declaration as deprecated for diagnostics and documentation.",
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
    "@deprecated",
    "@mcp.tool",
    "@mcp.resource",
    "@pure",
    "@require",
    "@scheduled",
    "@ensure",
    "@invariant",
    "@forall",
    "@fuzz",
    "@test",
    "@server",
    "@query",
    "@mutation",
    "@table",
    "@index",
    "@v0",
    "@mobile.native",
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
