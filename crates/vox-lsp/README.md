# Vox Language Server (`vox-lsp`)

Language Server Protocol (LSP) implementation for the Vox programming language (`v0.6`), available both as a standalone binary (`vox-lsp`) and via the unified CLI (`vox lsp`).

## Features

- **Diagnostics**: Real-time lexer, parser, bidirectional type checker, and effect/boundary lint diagnostics.
- **Hover**: Type signatures, doc comments, and declaration metadata on hover.
- **Completion**: Context-aware keyword, declaration (`table`, `query`, `mutation`, `server`, `component`, `workflow`, `activity`, `actor`, `tool`, `resource`), decorator, and symbol completions.
- **Document Symbols**: Outline view for functions, components, tables, actors, workflows, and types.
- **Semantic Tokens**: Compiler-backed semantic highlighting for `.vox` files.
- **Code Actions**: Quick fixes and compiler-suggested refactorings.
- **Go-to-Definition**: Partial support for local and module-level symbol navigation.

Canonical capability matrix: [`docs/src/architecture/vox-lsp-capabilities-ssot-2026.md`](../../docs/src/architecture/vox-lsp-capabilities-ssot-2026.md).

## Usage

Run via the `vox` CLI (recommended):

```bash
vox lsp
```

Or build and run the standalone `vox-lsp` server binary over stdio:

```bash
cargo build --release -p vox-lsp
./target/release/vox-lsp
```

## Editor Configuration

### VS Code

Install the [`vox-vscode`](../../apps/editor/vox-vscode/README.md) extension or configure the server path manually in `settings.json`:

```json
{
  "vox.lsp.enabled": true,
  "vox.lsp.serverPath": "/path/to/target/release/vox-lsp"
}
```
