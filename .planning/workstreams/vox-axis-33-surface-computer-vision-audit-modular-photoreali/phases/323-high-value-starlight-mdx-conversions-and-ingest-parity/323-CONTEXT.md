# Phase 323: High-Value Starlight MDX Conversions and Ingest Parity - Context

**Gathered:** 2026-11-01
**Status:** Ready for planning
**Mode:** Autonomous execution

<domain>
## Phase Boundary

Make `.mdx` a first-class, zero-regression citizen across `vox-kb-facets`, `vox-search` (Library index, `kb_ids`, RAG `ingest_markdown_tree`, `chunk_document`), `vox-doc-pipeline`, and `vox-corpus` MENS training extraction; then upgrade `docs/src/index.mdx` and high-value tutorial/reference surfaces to leverage Starlight MDX components (`<Steps>`, `<Tabs>`, `<TabItem>`, `<CardGrid>`, `<Card>`, `<Aside>`, `<VoxPlayground>`) while keeping prose-heavy architecture/ADR/research docs in CommonMark `.md`.

</domain>

<decisions>
## Implementation Decisions

### Stratified MDX & Zero-Regression Ingest
- **Dual Summary Marker Support (`MDX-INGEST-01`):** MDX v3 rejects HTML comments (`<!-- kb:summary -->`). `vox-kb-facets::summary` must support both `<!-- kb:summary -->` / `<!-- /kb:summary -->` and `{/* kb:summary */}` / `{/* /kb:summary */}` and skip top-level ESM `import` / `export` lines between `# H1` and the summary block.
- **Shared L0 MDX Boilerplate Stripper (`MDX-INGEST-03`):** Define `is_mdx_boilerplate_line` and `strip_mdx_boilerplate` in `vox-kb-facets` (Layer 0) so `vox-search` (`first_paragraph`, `chunk_document`, `ingest_markdown_tree`) and `vox-corpus` (`extract_from_md_file`) strip ESM `import`/`export` statements, `{/* ... */}` comments, and Starlight JSX container tags (`<Tabs>`, `<TabItem>`, `<Steps>`, `<CardGrid>`, `<Card>`, `<LinkCard>`, `<LinkButton>`, `<Aside>`, `<Badge>`, `<VoxPlayground>`) while preserving inner prose and fenced code blocks.
- **Corpus Path & Facet Lint Parity (`MDX-INGEST-02`):** `vox-search::kb_ids::is_kb_corpus_path`, `doc_title`, `ingest_markdown_tree`, and `vox-corpus::extract_docs::walk_docs_dir` accept both `.md` and `.mdx`. `vox-doc-pipeline::pipeline::facets::facet_lint_rel_path` lints authored `.mdx` pages under `docs/src/` while continuing to exempt `docs/src/index.mdx` (landing page without `# H1`).
- **Non-Destructive High-Value MDX Adoption (`MDX-CONV-01..03`):** Preserve existing `.md` paths locked by E2E tests (`docs/src/tutorials/tut-getting-started.md`), add `.md` ↔ `.mdx` link fallback in `docs-astro/src/plugins/remark-doc-links.mjs`, upgrade `docs/src/index.mdx` with Starlight `<Tabs>`/`<TabItem>`/`<CardGrid>`, and add an interactive MDX full-stack walkthrough tutorial (`docs/src/tutorials/tut-fullstack-walkthrough.mdx`) cross-linked from `tut-getting-started.md` and `ref-syntax.md`.

</decisions>

<code_context>
## Existing Code Insights

- `crates/vox-kb-facets/src/summary.rs` currently hardcodes `SUMMARY_OPEN = "<!-- kb:summary -->"` and `SUMMARY_CLOSE = "<!-- /kb:summary -->"`.
- `crates/vox-search/src/kb_ids.rs:152` checks `!rel.ends_with(".md")`, dropping `.mdx` files from the Tantivy/VoxDb Library index.
- `crates/vox-search/src/ingest.rs:41` and `crates/vox-corpus/src/corpus/extract_docs.rs:563` only walk `.md` files.
- `crates/vox-doc-pipeline/tests/docs_production_audit_e2e.rs` locks specific strings in `docs/src/index.mdx` (`"60s:"`, `"5-Min:"`, `"15-Min:"`, `"Run that now"`) and requires `docs/src/tutorials/tut-getting-started.md` to exist at its `.md` path.

</code_context>
