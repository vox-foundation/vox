---
phase: 342-docs-authoring-parity-and-starlight-0-42-upgrade
plan: 01
subsystem: docs
tags: [docs-astro, remark-doc-links, vox-doc-pipeline, openwiki, subsystem-hubs, baseline]
requires: []
provides:
  - t3 wave4 audit page code links that resolve inside the repo
  - subsystem hub generator that links crate API pages only when they exist
  - frozen pre-upgrade (Astro 6) unit and Playwright failure baselines
affects: [342-02, 342-03, 342-04]
tech-stack:
  added: []
  patterns: ["generator fix + regeneration instead of hand-editing *.generated.md"]
key-files:
  created:
    - .planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-baseline-unit-failed.txt
    - .planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-baseline-playwright-failed.txt
  modified:
    - docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md
    - crates/vox-doc-pipeline/src/pipeline/openwiki.rs
    - docs/src/architecture/subsystems/l5-binaries.generated.md
    - docs/src/architecture/subsystems/l2-domain-libs.generated.md
    - docs/src/architecture/subsystems/index.generated.md
key-decisions:
  - "Fixed links, not the remark-doc-links outside-root guard (T-342-04)"
  - "crate_link_cell stays private; no new public API"
requirements-completed: [DOCS-PARITY-04]
status: complete
completed: 2026-11-05
commits: 3
plan_head_before: eb2fb05cc69b416da77400cd0820a77238433a72
plan_head_after: a926f555ed77231e49718def44214b74111293bb
actuals:
  tokens: 3500
  tasks: 3
  commits: 3
---

# Phase 342 Plan 01: Link-failure source fixes and pre-upgrade baseline Summary

Both existing `remark-doc-links` failures are fixed at source (16 four-segment links on the t3 audit page; hub generator no longer links the API page that `vox-cargo-shim` never gets), and the Astro 6 failing-test sets are frozen for Plans 02-04.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | t3 audit page code links resolve inside the repo | 1f6e2512b |
| 2 (tdd) | `crate_link_cell` + regenerated hubs | 64c4f23db |
| 3 | Pre-upgrade baseline freeze | a926f555e |

- **Task 1:** replaced `](../../../../crates/` with `](../../../crates/` on 16 lines; nothing else on the page changed. Full `pnpm --dir docs-astro build` (Astro 6): exit 0, `907 page(s) built`, zero `resolves outside repository root`.
- **Task 2:** RED first. `cargo test -p vox-doc-pipeline --lib crate_link_cell` failed to compile with `E0425: cannot find function crate_link_cell in this scope` (2 errors) before the helper existed. GREEN: private `crate_link_cell(repo_root, name)` returns the link form only when `docs/src/api/<name>.generated.md` is a file, else a backtick-only span; `generate_subsystem_hubs` uses it for the crate column. `cargo test -p vox-doc-pipeline --lib openwiki`: 8 passed, including `crate_link_cell_links_only_existing_api_pages`. Hubs regenerated with `VOX_WIKI_SYNC_WRITE=1 cargo test -p vox-doc-pipeline --lib generates_and_verifies_subsystem_hubs_in_repo` (the same `generate_subsystem_hubs(root, true)` that `vox ci wiki-sync --write` calls). Row is now ``| `vox-cargo-shim` | `binary` | — | — |``; all `../../api/*.generated.md` hub links resolve.
- **Task 3:** final-tree build: exit 0, `907 page(s) built`, neither remark-doc-links error string present. Baselines written with the plan's exact extraction.

## Regression baseline (pre-upgrade, Astro 6)

Files: `342-baseline-unit-failed.txt` (3 lines, sorted) and `342-baseline-playwright-failed.txt` (20 lines, sorted, equals Playwright's "20 failed").

**Unit (`node --test --test-reporter=tap "tests/unit/*.test.mjs"`): 903 tests, 900 pass, 3 fail**
1. `docs-deploy push paths cover every mounted repo file and build input`
2. `docs-quality push and pull_request paths cover every mounted repo file`
3. `tutorial verification record parses and matches git blob hashes`

**Playwright (build-output, reader-layout, trust-signals, smoke, baseline specs): 20 failed, 1 skipped, 61 passed**
- `tests/build-output/links.spec.ts:9` tutorials/tut-getting-started has clean installation links and no .md href (1)
- `tests/build-output/links.spec.ts:122` every GitHub blob/tree link in built HTML names a path that exists in repo working tree (1)
- `tests/trust-signals.spec.ts:48` TRUST-01 & TRUST-04: `.vox-trust-banner` not visible for verified / adr-009-verified / adr-001-verified x desktop/tablet/phone x dark/light (18)

These are not owned by this phase and were not touched.

## Deviations from Plan

**1. [Info] Hub regeneration also changed L2 and the index.** The generator output for the committed `layers.toml` includes `vox-python-bridge` (added by an earlier phase-325 commit) and bumps the L2 crate count 17 to 18 in `index.generated.md`. Plan said to keep every resulting hub diff; kept.

Otherwise: None - plan executed as written.

## Observations (out of scope, not fixed)

- The build log still carries a non-fatal `remark-doc-links: Target not found: 'milestones/v4.3-ROADMAP.md' in .planning/ROADMAP.md` warning (mounted planning file, owned elsewhere). Plan 04 should account for it when the validator becomes strict.
- The `tdd-guard`/`fmt-fix` hooks ran clean on the Rust commit.

## Known Stubs

None.

## Threat Flags

None.

## State updates deferred to the orchestrator

Per orchestrator notes, ROADMAP.md / STATE.md not touched. Would record: plan 342-01 complete, requirement DOCS-PARITY-04 part 1 done (Plan 04 completes the validator half), 3 tasks, 3 commits, no decisions beyond the two above.

## Self-Check: PASSED

Commits 1f6e2512b, 64c4f23db, a926f555e are ancestors of HEAD; both baseline files, the regenerated L5 hub, and `crate_link_cell` exist.
