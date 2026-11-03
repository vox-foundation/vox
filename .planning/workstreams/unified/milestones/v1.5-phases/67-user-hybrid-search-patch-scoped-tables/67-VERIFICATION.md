---
phase: 67-user-hybrid-search-patch-scoped-tables
status: passed
verified_at: "2026-10-16T08:00:00Z"
---

# Phase 67 Verification: User-Facing Hybrid Search (`@embed`, `@searchable`), Partial `db.Table.patch` & Multi-Tenancy (`@table(scoped_by: ...)`)

## Status
**PASSED** (`USERDB-01`, `USERDB-02`, `USERDB-03`, `USERDB-04` verified).

## Evidence
- `cargo test -p vox-codegen --lib table_emits_patch_search_hybrid_bucket_and_tenant_scope` (`1 passed; 0 failed`)
