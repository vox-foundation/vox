---
phase: 34
phase_name: "docs-corpus-consolidation"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 34 Learnings: docs-corpus-consolidation

## Decisions

### Implemented plan_duplicate_cluster_consolidation and render_redirects_bl...
Implemented `plan_duplicate_cluster_consolidation` and `render_redirects_block` in [`crates/vox-doc-pipeline/src/claims/consolidation.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/consolidation.rs) to pick `.generated.md` or `*-ssot*.md` canonical pages and emit 301 redirects (`DEBT-01`, `DEBT-02`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 34-01-SUMMARY.md

---

### Implemented ratchet_kb_facet_backlog to guarantee contracts/documentatio...
Implemented `ratchet_kb_facet_backlog` to guarantee `contracts/documentation/kb-facet-backlog.v1.yaml` only shrinks (`DEBT-03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 34-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

