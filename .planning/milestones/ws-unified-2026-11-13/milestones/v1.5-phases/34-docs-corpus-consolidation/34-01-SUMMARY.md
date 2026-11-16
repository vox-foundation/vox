---
phase: 34-docs-corpus-consolidation
plan: 01
subsystem: vox-doc-pipeline
tags: [docs-consolidation, redirects, facet-backlog-ratchet]
provides:
  - Duplicate cluster SSOT selection and 301 `_redirects` synthesis (`DEBT-01`, `DEBT-02`)
  - Shrink-only KB facet backlog ratchet (`DEBT-03`)
key-files:
  created:
    - crates/vox-doc-pipeline/src/claims/consolidation.rs
  modified:
    - crates/vox-doc-pipeline/src/claims/mod.rs
requirements-completed: [DEBT-01, DEBT-02, DEBT-03]
completed: 2026-10-15
---

# Phase 34 Plan 01: Docs Corpus Consolidation Summary

## Accomplishments
- Implemented `plan_duplicate_cluster_consolidation` and `render_redirects_block` in [`crates/vox-doc-pipeline/src/claims/consolidation.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/consolidation.rs) to pick `.generated.md` or `*-ssot*.md` canonical pages and emit 301 redirects (`DEBT-01`, `DEBT-02`).
- Implemented `ratchet_kb_facet_backlog` to guarantee `contracts/documentation/kb-facet-backlog.v1.yaml` only shrinks (`DEBT-03`).

## Verification
- `timeout 900s cargo --config 'build.rustc-wrapper=""' test -p vox-doc-pipeline --lib` — **84 passed, 0 failed**.
