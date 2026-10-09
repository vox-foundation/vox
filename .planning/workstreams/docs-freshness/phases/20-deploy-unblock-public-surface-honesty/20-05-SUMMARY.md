---
phase: 20-deploy-unblock-public-surface-honesty
plan: 05
subsystem: docs-links
tags: [docs, links, guard, node-test, honesty]
status: complete

requires: []
provides:
  - "docs-astro/tests/unit/source-link-targets.test.mjs: one node:test per live docs/src page (git ls-files -z, minus archive/ and .well-known/) asserting every relative link target exists; exports relativeLinkTargets(markdown) (inline + reference-definition links; fenced/inline code and [^footnote] definitions skipped); picked up by docs-quality CI's tests/unit glob"
  - "Zero dead relative link targets in the live docs corpus (671/671 page tests pass) — precondition for 20-08 strict link rewrite and 20-10 blocking lychee"
affects: [20-08, 20-10]

actuals:
  tokens: 30000
  tasks: 3
  commits: 3
plan_head_before: c79b7a0afcfa84c88f81560c99edefc9dab04fff

tech-stack:
  added: []
  patterns:
    - "Source-level link guard: per-page node:test so the failure report names each page and its missing targets"
    - "Retired-code links are unlinked to inline code (visible text kept), never redirected to a successor"

key-files:
  created:
    - docs-astro/tests/unit/source-link-targets.test.mjs
  modified:
    - docs/src/architecture/v1-llm-target-implementation-plan-2026.md
    - docs/src/architecture/vox-as-llm-target-audit-and-plan-2026.md
    - docs/src/adr/036-webir-hir-unification-compare-both.md
    - docs/src/architecture/webir-hir-split-brain-inventory-2026.md
    - docs/src/architecture/2026-05-08-llm-misleading-content-cleanup-plan.md
    - docs/src/architecture/agentic-version-control-automation-research-2026.md
    - docs/src/architecture/dependency-consolidation-plan-2026.md
    - docs/src/architecture/gui-authoring-syntax-2026.md
    - docs/src/architecture/mesh-and-language-distribution-ssot-2026.md
    - docs/src/architecture/mesh-dashboard-and-distributed-compute-research-2026.md
    - docs/src/architecture/unified-task-hopper-research-2026.md
    - docs/src/reference/vox-fullstack-artifacts.md

key-decisions:
  - "vox-ssg link in reference/vox-fullstack-artifacts.md is RE-LINKED (not unlinked) to crates/vox-cli/src/utils/ssg/mod.rs: its module doc still names itself `vox-ssg`, and vox build calls generate_static_site to write public/ssg-shells — a clear current home"
  - "All 55 root-relative links in the two LLM-target pages pointed at paths that still exist, so every one was a depth fix (none unlinked)"
  - "vox-checksum-manifest does not exist anywhere in the workspace, so it was unlinked like the retired crates"
  - "Footnote definitions ([^label]: text) are not link references and are excluded from the guard"

metrics:
  duration: "~25 min"
  completed: 2026-10-09
---

# Phase 20 Plan 05: Dead relative link pre-fix + source-link guard Summary

A per-page node:test now fails CI on any dead relative link target in live `docs/src`. All 77 dead targets in 12 pages were fixed: 55 wrong-depth links corrected, 2 deleted-file links unlinked, 19 retired-crate links unlinked, and 1 `vox-ssg` link moved to its current home.

## Before / after

| | Pages tested | Failing pages | Dead targets |
|---|---|---|---|
| Before (guard first run, after footnote fix) | 671 | 12 | 77 |
| After Task 1 | 671 | 8 | 20 |
| After Task 2 | 671 | 4 | 12 |
| After Task 3 | 671 | 0 | 0 |

The plan measured 71 dead links in 13 pages on 2026-10-08. The live census found 77 in 12. That is within the plan's tolerance, so no stop-and-report was needed.

## Task commits

| Task | Commit | What |
|---|---|---|
| 1 | c7802fd72 | Guard added; 55 wrong-depth links fixed in the two LLM-target pages; `tauri_stub.rs` unlinked in ADR-036 and the WebIR inventory |
| 2 | 4ea6a511b | `_frozen.md`, `vox-dashboard`, `vox-install-policy`, `vox-checksum-manifest` unlinked in four architecture pages |
| 3 | 7cc26c291 | `vox-dashboard/**` unlinked in mesh and task-hopper pages; `vox-ssg` relinked to `crates/vox-cli/src/utils/ssg/mod.rs` |

## Verification

- Task 1 verifies: pass (663 pass, 8 fail, none of the four Task 1 pages failing).
- Task 2 verify: pass (667 pass, none of the Task 2 pages failing).
- Task 3: `node --test tests/unit/source-link-targets.test.mjs` exits 0 with 671/671 passing. `cargo run -q -p vox-doc-pipeline -- --lint-only --paths architecture,reference/vox-fullstack-artifacts.md,adr/036-webir-hir-unification-compare-both.md` reports no hard errors.
- `git status --porcelain -- docs/src docs-astro/tests` listed only this plan's files before each commit. Every staged diff was checked: no `lean-ctx` text and no unexpected deletions.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Guard counted footnote definitions as links**
- **Found during:** Task 1, first run
- **Issue:** `[^label]: *Paper title* …` matched the reference-definition pattern. That produced 86 false "dead targets" in `autonomous-orchestration-policy-research-2026.md`, a page outside the plan.
- **Fix:** The reference-definition regex now excludes `[^…]` labels, and the helper's unit test covers this case. After the fix, that page passes and the failure set equals the plan's pages.
- **Commit:** c7802fd72

**2. [Verify harness] Node 26 default reporter is `spec`, not TAP**
- Node v26.10.0 prints `ℹ pass N`, not `# pass N`, even when output goes to a file. The plan's verifies parse `# pass` and `# fail`, so they were run with `--test-reporter=tap`. The commands are otherwise unchanged.

**3. [Scope] `2026-05-08-crate-org-followup-plan.md` needed no edit**
- It is listed in `files_modified`, but it had no dead targets when this plan ran; it was already clean before the plan started. It was left untouched.

**4. [Scope] Line 426 of dependency-consolidation**
- The first scripted pass appended a `(`crates/vox-install-policy/`)` parenthetical. It was reverted before commit to the original visible text, "`vox-install-policy` crate", to keep the edit link-only.

**5. [Honesty] LINKS-01 / LINKS-03 left Pending in REQUIREMENTS.md**
- `requirements.mark-complete` was run, then reverted. This plan only clears the precondition. LINKS-01 (links rendered as site routes) lands with 20-08, and LINKS-03 (a blocking `dist/` check with a broken fixture) lands with 20-10.


## Known Stubs

None.

## Self-Check: PASSED

- FOUND: docs-astro/tests/unit/source-link-targets.test.mjs
- FOUND commits: c7802fd72, 4ea6a511b, 7cc26c291 (`git rev-list --count c79b7a0af..HEAD` = 3)
