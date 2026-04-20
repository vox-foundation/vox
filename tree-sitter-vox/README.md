---
title: "Tree-sitter Vox"
description: "Tree-sitter grammar for the Vox language (grammar, queries, corpus tests)."
category: "reference"
status: "current"
training_eligible: true
training_rationale: "Defines the grammar for the Vox language, essential for toolchain development."
---
# tree-sitter-vox

Tree-sitter grammar for the Vox language (grammar, queries, corpus tests).

**React interop migration (2026):** the rustc parser now accepts `routes { }` entries with `with loader:` / `with pending:`, nested child routes, and `not_found:` / `error:` lines. This grammar should be extended to match those surfaces for highlighting and structural queries; track against [`docs/src/architecture/react-interop-backlog-2026.md`](../docs/src/architecture/react-interop-backlog-2026.md) (WS23).

## Development

From this directory:

```bash
npm ci   # or: npm install
npx tree-sitter generate
npx tree-sitter test
```

`node_modules/` is gitignored; reinstall after clone. The grammar was flattened to this top-level folder (no nested `tree-sitter-vox/` package dir).
