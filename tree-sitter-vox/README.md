# tree-sitter-vox

Tree-sitter grammar for the Vox language (grammar, queries, corpus tests).

## Development

From this directory:

```bash
npm ci   # or: npm install
npx tree-sitter generate
npx tree-sitter test
```

`node_modules/` is gitignored; reinstall after clone. The grammar was flattened to this top-level folder (no nested `tree-sitter-vox/` package dir).
