---
phase: 67
phase_name: "user-hybrid-search-patch-scoped-tables"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 67 Learnings: user-hybrid-search-patch-scoped-tables

## Decisions

### Expose vox-search / SQLite FTS + phonetic + vector hybrid retrieval on ....
Expose `vox-search` / SQLite FTS + phonetic + vector hybrid retrieval on `.vox` `table` declarations (`search_hybrid`, `emit_search_index_ddl`, `emit_vector_index_ddl`).

**Rationale:** Primary phase objective established in plan.
**Source:** 67-01-PLAN.md

---

### Emit partial column updates (patch), declarative bucket_count / join_whe...
Emit partial column updates (`patch`), declarative `bucket_count` / `join_where` aggregations, and tenant-scoped (`all_for_tenant`, `patch_for_tenant`) queries on generated `.vox` table structs.

**Rationale:** Primary phase objective established in plan.
**Source:** 67-02-PLAN.md

## Lessons

## Patterns

## Surprises

