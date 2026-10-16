---
phase: 02-library-index-and-kb-tool-surface
plan: 02
subsystem: docs
tags: [docs, doc-lint, sidebar, library]
requires: []
provides:
  - "Library docs category (lint vocabulary, sidebar contract, governance table)"
  - "docs/src/library/index.md seed page"
  - "valid_categories_match_sidebar_sections parity test"
affects: [kb-indexer]
tech-stack:
  added: []
  patterns: ["vocabulary parity test reading the contract JSON via CARGO_MANIFEST_DIR"]
key-files:
  created: [docs/src/library/index.md]
  modified:
    - crates/vox-doc-pipeline/src/pipeline/lint.rs
    - contracts/documentation/docs-sidebar-section-order.v1.json
    - docs/src/contributors/documentation-governance.md
decisions:
  - "Library placed directly after Concepts, not collapsed (AP-19)"
requirements-completed: [IDX-06]
status: complete
metrics:
  completed: 2026-10-05
actuals:
  tokens: 2500
  tasks: 2
  commits: 2
plan_head_before: 89a4835b590cf546a82f3b6e61135820d68f9efe
---

# Phase 2 Plan 02: Library category and parity guard Summary

`Library` is now a legal docs category (lint, sidebar section after Concepts, governance row) with a seed page at `docs/src/library/index.md`, plus a parity test that fails whenever `VALID_CATEGORIES` (minus archive) and the sidebar `sections` array drift.

## Commits

- `2a3fad788` feat(02-02): add Library docs category and seed page (Task 1, tracer)
- `5a7084da1` test(02-02): guard VALID_CATEGORIES against sidebar sections drift (Task 2)

## Verification

- Red: before the vocabulary edit, `vox-doc-pipeline --lint-only --paths library/index.md` reported `unknown category "Library"`.
- Green: after the edit, lint of `library/index.md` and `contributors/documentation-governance.md` reports no hard errors.
- Mutation check (Task 2): presence count of `"Library"` in the sidebar JSON was 1 before, 0 after removal. The test then failed with `In lint but not sidebar: ["Library"]`. After restore from a backup the count was 1 and `diff -q` against the backup was identical; the test passed again.
- `cargo nextest run -p vox-doc-pipeline`: 25 passed (includes the existing governance-table test).
- Not run locally: the docs-astro build (node_modules not installed in this worktree); hosted CI covers it.

## Deviations from Plan

None in substance. The plan's `lint.rs` edits spanned both tasks, so the parity test was temporarily held back from the Task 1 commit to keep the two commits atomic.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- docs/src/library/index.md, lint.rs, sidebar JSON, governance doc: present in HEAD~2..HEAD diff.
- Commits 2a3fad788 and 5a7084da1: present on branch.
