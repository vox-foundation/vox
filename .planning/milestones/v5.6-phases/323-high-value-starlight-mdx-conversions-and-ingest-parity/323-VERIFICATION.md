---
phase: 323-high-value-starlight-mdx-conversions-and-ingest-parity
status: passed
verified_at: 2026-11-01T10:59:00Z
requirements:
  - MDX-INGEST-01
  - MDX-INGEST-02
  - MDX-INGEST-03
  - MDX-CONV-01
  - MDX-CONV-02
  - MDX-CONV-03
---

# Phase 323 Verification Report — High-Value Starlight MDX Conversions and Ingest Parity

## Verdict

**Status: PASSED** (`6/6` requirements verified)

## Requirement Verification Matrix

| Requirement | Description | Evidence | Status |
|---|---|---|---|
| `MDX-INGEST-01` | `vox-kb-facets` accepts both HTML (`<!-- kb:summary -->`) and MDX JSX (`{/* kb:summary */}`) summary markers and skips top-level ESM `import`/`export` statements before H1/summary blocks | `cargo test -p vox-kb-facets` (`59 passed` unit tests + `1 passed` golden schema test) | Passed |
| `MDX-INGEST-02` | `vox-search::kb_ids`, `vox-search::ingest::ingest_markdown_tree`, and `vox-doc-pipeline::pipeline::facets::facet_lint_rel_path` index and lint `.mdx` files under `docs/src/` while exempting `docs/src/index.mdx` | `cargo test -p vox-search --lib` (`270 passed; 0 failed`), `cargo test -p vox-doc-pipeline --lib facets` (`20 passed; 0 failed`) | Passed |
| `MDX-INGEST-03` | `vox-search::library::chunk` and `vox-corpus::extract_docs` strip top-level ESM `import`/`export` lines, `{/* ... */}` comments, and Starlight JSX wrapper tags (`<Tabs>`, `<TabItem>`, `<Steps>`, `<CardGrid>`, `<Card>`, `<Aside>`, `<VoxPlayground />`, etc.) while preserving prose and fenced `.vox` blocks | `cargo test -p vox-search --lib` (`chunk_document_strips_mdx_imports_and_wrapper_tags`), `cargo test -p vox-corpus --lib extract_docs` (`23 passed; 0 failed`, including `extract_from_mdx_file_strips_jsx_and_summary`) | Passed |
| `MDX-CONV-01` | `docs/src/index.mdx` uses Starlight `<CardGrid>` and `<Card>` components while preserving `SYNC-FROM-README: why_vox`, `VoxPlayground.astro`, and E2E-tested onboarding copy | `cargo test -p vox-doc-pipeline --test docs_production_audit_e2e` (`4 passed; 0 failed`), `pnpm --dir docs-astro build` (`dist/index.html` generated) | Passed |
| `MDX-CONV-02` | Interactive Starlight MDX tutorial `docs/src/tutorials/tut-fullstack-walkthrough.mdx` authored with `<Steps>`, `<Tabs>`/`<TabItem>` (comparing `.vox` source with generated Turso SQL, Rust Axum, and React TSX), `<CardGrid>`, `<Aside>`, `<VoxPlayground />`, `{/* kb:summary */}`, and compilable `vox` doctests | `cargo run -p vox-doc-pipeline -- --lint-only --paths index.mdx,tutorials/tut-fullstack-walkthrough.mdx,tutorials/tut-getting-started.md,reference/ref-syntax.md` (`no hard errors`), `pnpm --dir docs-astro build` (`dist/tutorials/tut-fullstack-walkthrough/index.html` generated) | Passed |
| `MDX-CONV-03` | `docs-astro/src/plugins/remark-doc-links.mjs` resolves `.md` ↔ `.mdx` links transparently, `docs-astro/astro.config.mjs` aliases `@astrojs/starlight/components` for symlinked `docs/src/` MDX files, and `tut-getting-started.md` + `ref-syntax.md` cross-link `tut-fullstack-walkthrough.mdx` | `node -e` smoke test on `remarkDocLinks` (`0`), `pnpm --dir docs-astro build` (`901 page(s) built`, `919 HTML files` indexed by Pagefind, exit code `0`) | Passed |
