---
phase: 01-dead-crate-cleanup-remove-confirm
plan: 02
subsystem: infra
tags: [toml, catalog, crate-lifecycle]

requires:
  - phase: 01-01
    provides: "D-03 verification pass, D-04 batching precedent, SC#3 checkpoint ratification"
provides:
  - "catalog.toml as the single documented home for both ghost-entry hygiene and frozen-crate status (D-02)"
  - "ROADMAP Phase 1 Success Criterion 4 fully satisfied"
affects: [phase-2, phase-3]

actuals:
  tokens: 3400
  tasks: 2
  commits: 1

tech-stack:
  added: []
  patterns: ["catalog.toml records crate disposition as a prose `#`-comment block when the schema (no deny_unknown_fields, 4 renamed tables) would silently swallow a new table kind"]

key-files:
  created: []
  modified: ["crates/vox-plugin-catalog/catalog.toml"]

key-decisions:
  - "Frozen-crate annotation placed as a new comment block between the file header and the first plugin section, not inside/near the unrelated execution-api/stub-check block -- keeps two distinct disposition records visually separate"

requirements-completed: [REQ-dead-crate-catalog-cleanup, REQ-dead-crate-keep-frozen]

coverage:
  - id: D1
    description: "execution-api and stub-check are already documented as removed, with no live catalog entry or workspace package for either (ROADMAP SC#4, clause 1)"
    requirement: "REQ-dead-crate-catalog-cleanup"
    verification:
      - kind: other
        ref: "grep -nE '^id = \"(execution-api|stub-check)\"' catalog.toml -- 0 matches; cargo metadata -- 0 matches; ghost-entry comment present at line 75 (pre-edit)/84 (post-edit), unmodified"
        status: pass
    human_judgment: false
  - id: D2
    description: "vox-workflow-runtime, vox-integration-tests, vox-test-harness present, unmodified by this phase, and annotated as intentionally frozen (ROADMAP SC#4, clause 2)"
    requirement: "REQ-dead-crate-keep-frozen"
    verification:
      - kind: other
        ref: "git ls-files -- all 3 Cargo.toml tracked; git diff --stat ab3554261..HEAD over the 3 crate dirs -- empty; catalog.toml new comment block names all 3 verbatim"
        status: pass
    human_judgment: false
  - id: D3
    description: "Frozen-crate annotation is a comment, not a new TOML table (D-02, schema constraint)"
    requirement: "REQ-dead-crate-keep-frozen"
    verification:
      - kind: other
        ref: "lib.rs schema check: exactly 4 rename=\"...\" tables (plugin/bundle/component/skill-bundle), no deny_unknown_fields; committed diff ccc29d5cf contains zero non-# non-blank added lines; table-kind set post-edit is unchanged (plugin/bundle/component/skill-bundle only)"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-09-22
status: complete
---

# Phase 1 Plan 02: KEEP-FROZEN Catalog Annotation Summary

**Verified catalog.toml's execution-api/stub-check ghost-entry bookkeeping is already accurate and added a KEEP-FROZEN disposition comment for the three frozen crates, closing ROADMAP Phase 1 Success Criterion 4.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-09-22T21:25:00Z
- **Completed:** 2026-09-22T21:45:00Z
- **Tasks:** 2 of 2
- **Files modified:** 1 (`crates/vox-plugin-catalog/catalog.toml`)

## Accomplishments
- Confirmed `execution-api`/`stub-check` have zero live catalog entries and zero matching workspace packages; the existing removal comment is present and accurate — REQ-dead-crate-catalog-cleanup was already satisfied, this plan only verified it
- Confirmed all three KEEP-FROZEN crates (`vox-workflow-runtime`, `vox-integration-tests`, `vox-test-harness`) are git-tracked and untouched by any Phase 1 commit
- Confirmed `CatalogFile`'s schema (exactly 4 `rename = "..."` tables, no `deny_unknown_fields`) forces the D-02 annotation to be a comment, not a new table
- Added the KEEP-FROZEN disposition comment block and verified the commit's diff is comment-only, single-file, and leaves the generated docs untouched

## Verification Evidence (ROADMAP Success Criterion 4)

### Clause 1 — execution-api / stub-check ghost-entry hygiene

```
$ grep -nE '^id = "(execution-api|stub-check)"' crates/vox-plugin-catalog/catalog.toml
(0 matches, exit 1)

$ cargo metadata --format-version 1 --offline --no-deps | grep -oE '"name":"[^"]*(execution-api|stub-check)[^"]*"'
(0 matches, exit 1)

$ grep -n 'execution-api and stub-check removed 2026-05-08' crates/vox-plugin-catalog/catalog.toml
75:# execution-api and stub-check removed 2026-05-08: no crate exists for either;
```
Result: **PASS** — already satisfied verbatim, pre-existing comment untouched by this plan.

### Clause 2 — frozen crates present and untouched

```
$ git ls-files crates/vox-workflow-runtime/Cargo.toml crates/vox-integration-tests/Cargo.toml crates/vox-test-harness/Cargo.toml
crates/vox-integration-tests/Cargo.toml
crates/vox-test-harness/Cargo.toml
crates/vox-workflow-runtime/Cargo.toml
```
All 3 tracked.

**Pre-phase base SHA:** `ab3554261e97aede2e9bbae7483b778e68f72f3b` (parent of Phase 1's first committing task, `1d81d7ae9` — the CI matrix fix).

```
$ git diff --stat ab3554261..HEAD -- crates/vox-workflow-runtime crates/vox-integration-tests crates/vox-test-harness
(empty — no output)
```
Result: **PASS** — zero changes to any of the three frozen crate directories across all of Phase 1's own commits.

### Schema evidence (D-02's comment-not-table justification)

```
$ grep -nE 'deny_unknown_fields|rename = "(plugin|bundle|component|skill-bundle)"' crates/vox-plugin-catalog/src/lib.rs
17:    #[serde(default, rename = "plugin")]
19:    #[serde(default, rename = "bundle")]
21:    #[serde(default, rename = "component")]
23:    #[serde(default, rename = "skill-bundle")]
```
Exactly 4 renamed tables, zero `deny_unknown_fields` matches. A `[[frozen]]` (or any other unrecognized) table would parse successfully, expose through no accessor, and become dead data — confirming the annotation must be a `#`-comment.

## New Comment Block (verbatim)

```toml
# ── KEEP-FROZEN crates (not plugins) ───────────────────────────────────────────
#
# vox-workflow-runtime, vox-integration-tests, and vox-test-harness are not
# plugins and deliberately have no [[plugin]] entry here -- do not go looking
# for one. Their disposition per docs/src/architecture/dead-crate-fate-plan-2026-05-08.md
# is KEEP-FROZEN: present in the workspace, unmodified, intentionally inactive.
# This comment is their disposition record. They are not to be removed as dead
# crates; that disposition is revisited only through a new PRD.
```

Placed between the file header block and the `# ── Hardware / ML plugins ──` section header, separated from both by a blank line.

## Task Commits

1. **Task 2: Add KEEP-FROZEN annotation comment block to catalog.toml** — `ccc29d5cf` (docs)

(Task 1 was read-only verification; no commit.)

## Files Created/Modified
- `crates/vox-plugin-catalog/catalog.toml` — new 8-line comment block; zero non-comment, non-blank lines added (verified via `git diff` grep gate); pre-existing execution-api/stub-check comment byte-identical

## Decisions Made
- Placed the new block as its own section rather than appending to or near the execution-api/stub-check block, keeping the two disposition records (ghost-entry removal vs. frozen-crate retention) visually distinct despite both being "crate lifecycle" bookkeeping.

## Deviations from Plan

None — both tasks executed exactly as specified.

## Issues Encountered

None specific to this plan. (Phase-level git-merge interruption is recorded in `01-01-SUMMARY.md`.)

## D-04 Commit Batching — Final Record

Per D-04, Phase 1's crate-deletion work was batched into three groups:
- **Group (1)** vox-scientia-* trio: verification-only, no diff — recorded in `01-01-SUMMARY.md`.
- **Group (2)** seven standalone deletes: verification-only, no diff — recorded in `01-01-SUMMARY.md`.
- **Group (3)** KEEP-FROZEN documentation + catalog-cleanup verification: **this plan**, commit `ccc29d5cf`.

## Next Phase Readiness

**Complete.** ROADMAP Phase 1 Success Criterion 4 is fully satisfied. All four of Phase 1's success criteria (1-4) are now machine-verified with evidence across both plans. Phase 1 is ready to close; Phase 2 (Wire Up & Reclassify Dormant Crates) depends on Phase 1 and can begin.

---
*Phase: 01-dead-crate-cleanup-remove-confirm*
*Completed: 2026-09-22*
