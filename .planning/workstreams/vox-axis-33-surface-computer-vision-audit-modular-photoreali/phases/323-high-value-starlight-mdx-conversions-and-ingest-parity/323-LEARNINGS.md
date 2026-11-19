---
phase: 323
phase_name: "high-value-starlight-mdx-conversions-and-ingest-parity"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 6
  lessons: 3
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 323 Learnings: high-value-starlight-mdx-conversions-and-ingest-parity

## Decisions

### MDX-INGEST-01 (vox-kb-facets)
**`MDX-INGEST-01` (`vox-kb-facets`):** Added `SUMMARY_OPEN_MDX` (`{/* kb:summary */}`) and `SUMMARY_CLOSE_MDX` (`{/* /kb:summary */}`) alongside HTML summary markers, updated `inspect_summary` to ignore top-level ESM `import`/`export` statements before the summary block, and added `is_mdx_boilerplate_line` + `strip_mdx_boilerplate` in L0 `vox-kb-facets`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 323-01-SUMMARY.md

---

### MDX-INGEST-02 (vox-search & vox-doc-pipeline)
**`MDX-INGEST-02` (`vox-search` & `vox-doc-pipeline`):** Updated `vox-search::kb_ids::is_kb_corpus_path`, `doc_title`, and `ingest::ingest_markdown_tree` to accept `.mdx` alongside `.md`. Updated `vox-doc-pipeline::pipeline::facets::facet_lint_rel_path` to lint authored `.mdx` pages while continuing to exempt `docs/src/index.mdx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 323-01-SUMMARY.md

---

### MDX-INGEST-03 (vox-search::library::chunk & vox-corpus::extract_docs)
**`MDX-INGEST-03` (`vox-search::library::chunk` & `vox-corpus::extract_docs`):** Stripped ESM `import`/`export` lines, JSX comments, and Starlight wrapper tags (`<Tabs>`, `<TabItem>`, `<Steps>`, `<CardGrid>`, `<Card>`, `<Aside>`, `<VoxPlayground>`) from Tantivy summary cards, section chunks, and MENS training pairs.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 323-01-SUMMARY.md

---

### MDX-CONV-01 (docs/src/index.mdx)
**`MDX-CONV-01` (`docs/src/index.mdx`):** Replaced raw inline-styled HTML feature cards with native Starlight `<CardGrid>` and `<Card>` components and updated beginner walkthrough code snippets to current bare-keyword Vox syntax while preserving `SYNC-FROM-README: why_vox` and E2E-tested onboarding copy.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 323-02-SUMMARY.md

---

### MDX-CONV-02 (docs/src/tutorials/tut-fullstack-walkthrough.mdx)
**`MDX-CONV-02` (`docs/src/tutorials/tut-fullstack-walkthrough.mdx`):** Authored a full-stack interactive Starlight MDX tutorial using `<Steps>`, `<Tabs>` / `<TabItem>` (comparing `.vox` source with generated Turso SQL, Rust Axum, and React TSX), `<CardGrid>`, `<Aside>`, `<VoxPlayground />`, and `{/* kb:summary */}` with valid KB frontmatter facets and compilable `vox` doctests.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 323-02-SUMMARY.md

---

### MDX-CONV-03 (remark-doc-links.mjs & Cross-Links)
**`MDX-CONV-03` (`remark-doc-links.mjs` & Cross-Links):** Added transparent `.md` ↔ `.mdx` link resolution fallback in `docs-astro/src/plugins/remark-doc-links.mjs` and cross-linked the interactive MDX walkthrough from `tut-getting-started.md` and `ref-syntax.md`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 323-02-SUMMARY.md

## Lessons

### [WR-01] Mismatched HTML/MDX summary open and close markers accepted (cra...
**`[WR-01]` Mismatched HTML/MDX summary open and close markers accepted (`crates/vox-kb-facets/src/summary.rs`)**

**Context:** Surfaced in structured phase code review.
**Source:** 323-REVIEW.md

---

### Issue: inspect_summary and strip_summary_blocks allowed opening with <!-...
**Issue:** `inspect_summary` and `strip_summary_blocks` allowed opening with `<!-- kb:summary -->` and closing with `{/* /kb:summary */}` (or vice versa). Because MDX v3 rejects HTML comments outright, a mixed pair is always a syntax error.

**Context:** Surfaced in structured phase code review.
**Source:** 323-REVIEW.md

---

### Fix: Paired SUMMARY_OPEN strictly with SUMMARY_CLOSE and SUMMARY_OPEN_MD...
**Fix:** Paired `SUMMARY_OPEN` strictly with `SUMMARY_CLOSE` and `SUMMARY_OPEN_MDX` strictly with `SUMMARY_CLOSE_MDX` in both `inspect_summary` and `strip_summary_blocks`, and added unit test coverage verifying `SummaryStatus::Unterminated` and fail-closed stripping on mismatched markers.

**Context:** Surfaced in structured phase code review.
**Source:** 323-REVIEW.md

## Patterns

## Surprises

