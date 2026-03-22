# vox-parser

**Primary recursive-descent parser** for the Vox compiler pipeline.
Converts a `vox-lexer` token stream into a `vox-ast::Module`.

> **Scope boundary**: this parser handles the **core brace-delimited language surface**.
> Extended full-stack syntax (`@page`, `@partial`, `@theme`, `@layout`, `@i18n`, `@schema`, `@action`)
> is **not** in scope here — those declarations are processed by downstream crates
> (`vox-codegen-ts`, `vox-codegen-rust`) that consume `vox-ast` output.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | Public API + detailed scope table |
| `src/parser.rs` | Core recursive-descent parser — `parse()` entry point |
| `src/error.rs` | `ParseError` type with source spans and recovery info |
| `src/indent.rs` | Indentation-aware formatting and scope detection |

## What This Parser Handles

| Construct | Example |
|---|---|
| Functions / closures | `fn add(a, b) to int { ret a + b }` |
| Type definitions & ADTs | `type Shape = \| Circle(r: float) \| Point` |
| Imports | `import react.use_state` |
| Components | `@component fn App() to Element { ... }` |
| Islands | `@island Counter { count: int }` |
| Database tables & indices | `@table type Task { title: str }` |
| MCP tools | `@mcp.tool "description" fn myTool(...) { ... }` |
| Tests | `@test fn it_works() { ... }` |
| Server functions | `@server fn getData() to Data { ... }` |
| v0 components | `@v0 "A dashboard with charts" fn Dashboard() to Element` |
| Actors & workflows | `actor Worker { on receive(msg) { ... } }` |
| HTTP routes | `http post "/api/data" to Result { ... }` |
| JSX | `<div class="app"><span>hello</span></div>` |

## What This Parser Does NOT Handle

`@page`, `@partial`, `@theme`, `@layout`, `@i18n`, `@schema`, `@action`
— these use an extended syntax processed downstream.

## Error Strategy

The parser **never panics**. All errors are accumulated into `Vec<ParseError>` and returned
at the end, enabling partial ASTs useful for LSP diagnostics and incremental compilation.

## Usage

```rust
use vox_parser::parse;
use vox_lexer::cursor::lex;

let tokens = lex("fn hello() to int { ret 42 }");
let module = parse(tokens).expect("parse failed");
// module: Module with one Function declaration
```

## Golden Tests

End-to-end parse correctness is verified by `tests/parity_test.rs` against
the `.vox` files in `tests/golden/`. All golden examples must parse without errors.
The full extended-syntax corpus (`@page` etc.) is tested at the integration-test level
in `crates/vox-integration-tests`.
