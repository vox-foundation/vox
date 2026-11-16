---
phase: 323-high-value-starlight-mdx-conversions-and-ingest-parity
plan: 01
subsystem: kb-search-corpus
tags: [mdx, starlight, vox-kb-facets, vox-search, vox-doc-pipeline, vox-corpus]
requires: []
provides:
  - "SUMMARY_OPEN_MDX and SUMMARY_CLOSE_MDX ({/* kb:summary */}) in vox-kb-facets"
  - "is_mdx_boilerplate_line and strip_mdx_boilerplate in vox-kb-facets"
  - ".mdx corpus discovery and JSX boilerplate stripping in vox-search (kb_ids, ingest, library::chunk) and vox-corpus (extract_docs)"
  - "Facet lint support for authored .mdx files under docs/src/ in vox-doc-pipeline"
affects:
  - crates/vox-kb-facets
  - crates/vox-search
  - crates/vox-doc-pipeline
  - crates/vox-corpus
key-files:
  modified:
    - crates/vox-kb-facets/src/summary.rs
    - crates/vox-kb-facets/src/lib.rs
    - crates/vox-search/src/kb_ids.rs
    - crates/vox-search/src/ingest.rs
    - crates/vox-search/src/library/chunk.rs
    - crates/vox-doc-pipeline/src/pipeline/facets.rs
    - crates/vox-doc-pipeline/src/pipeline/facets_tests.rs
    - crates/vox-corpus/src/corpus/extract_docs.rs
requirements-completed:
  - MDX-INGEST-01
  - MDX-INGEST-02
  - MDX-INGEST-03
completed: 2026-11-01
---

# Phase 323 Plan 01: Dual HTML/MDX Summary Markers and L0 MDX Boilerplate Stripping Summary

## Accomplishments
- **`MDX-INGEST-01` (`vox-kb-facets`):** Added `SUMMARY_OPEN_MDX` (`{/* kb:summary */}`) and `SUMMARY_CLOSE_MDX` (`{/* /kb:summary */}`) alongside HTML summary markers, updated `inspect_summary` to ignore top-level ESM `import`/`export` statements before the summary block, and added `is_mdx_boilerplate_line` + `strip_mdx_boilerplate` in L0 `vox-kb-facets`.
- **`MDX-INGEST-02` (`vox-search` & `vox-doc-pipeline`):** Updated `vox-search::kb_ids::is_kb_corpus_path`, `doc_title`, and `ingest::ingest_markdown_tree` to accept `.mdx` alongside `.md`. Updated `vox-doc-pipeline::pipeline::facets::facet_lint_rel_path` to lint authored `.mdx` pages while continuing to exempt `docs/src/index.mdx`.
- **`MDX-INGEST-03` (`vox-search::library::chunk` & `vox-corpus::extract_docs`):** Stripped ESM `import`/`export` lines, JSX comments, and Starlight wrapper tags (`<Tabs>`, `<TabItem>`, `<Steps>`, `<CardGrid>`, `<Card>`, `<Aside>`, `<VoxPlayground>`) from Tantivy summary cards, section chunks, and MENS training pairs.
