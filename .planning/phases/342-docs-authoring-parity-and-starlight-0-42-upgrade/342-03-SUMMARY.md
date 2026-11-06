---
phase: 342-docs-authoring-parity-and-starlight-0-42-upgrade
plan: 03
subsystem: docs
tags: [docs-astro, starlight, github-alerts, playwright]
requires: [342-02]
provides:
  - GitHub alert blockquotes render as Starlight asides on the site
  - alerts.spec.ts (3 build-output tests) guarding presence, absence, and no-nesting
affects: [342-04]
tech-stack:
  added: ["starlight-github-alerts 0.5.0 (exact pin)"]
  patterns: ["plugin registered first in Starlight plugins, no options (default GitHub-parity mapping)"]
key-files:
  created:
    - docs-astro/tests/build-output/alerts.spec.ts
  modified:
    - docs-astro/package.json
    - docs-astro/pnpm-lock.yaml
    - docs-astro/astro.config.mjs
    - docs/src/how-to/how-to-train-mens-4080.md
    - docs/src/reference/mens-training.md
key-decisions:
  - "Default alert mapping kept: NOTE note, TIP tip, IMPORTANT caution, WARNING caution, CAUTION danger"
  - "Two list-nested alerts made top-level (GitHub does not render nested alerts)"
requirements-completed: [DOCS-PARITY-03]
status: complete
completed: 2026-11-05
commits: 2
plan_head_before: 6749bc39d16c7f381b77ddbf721625045880bd17
plan_head_after: b3075b590035bdae1cc1da67265bc57fc229bc67
actuals:
  tokens: 6000
  tasks: 2
  commits: 2
---

# Phase 342 Plan 03: GitHub alerts as Starlight asides Summary

`starlight-github-alerts@0.5.0` (exact, no options) registered first in the Starlight plugins; every top-level `> [!TYPE]` alert now builds to a mapped `starlight-aside--<type>` aside, guarded by a 3-test build-output spec.

## Package approval

Plan 02 recorded the human answer "Approved mature-only (Recommended)" covering starlight-github-alerts 0.5.0 (see 342-02-SUMMARY.md). Installed with `--save-exact`; no `minimumReleaseAgeExclude`. The lockfile adds transitive `satteri 0.10.5` and `unist-util-visit 5.1.0`; pnpm reported "Lockfile passes supply-chain policies".

## Tasks

| Task | Commit | Result |
| ---- | ------ | ------ |
| 1 Tracer: spec red, plugin, build, spec green | eb8195011 | 2 tests red before plugin, 2 passed after |
| 2 Un-nest two alerts, third test, regression diff | b3075b590 | 3 tests green, baseline diff empty |

### RED run (before plugin, against Plan 02 dist)

```
2 failed
  alerts.spec.ts:54 alert pages render mapped asides
  alerts.spec.ts:65 no raw alert blockquote survives   (listed 20+ raw-blockquote pages, e.g. how-to/how-to-train-mens-4080, reference/mens-training, tutorials/tut-getting-started)
```

### GREEN run (after plugin + build, 907 pages built)

```
2 passed (1.9s)
```

### Task 2 RED (third test, source-only)

```
Error: indented alert markers: how-to/how-to-train-mens-4080.md:22, reference/mens-training.md:170
1 failed
```

Names exactly the two expected lines. After un-nesting and rebuild (907 pages): `3 passed (1.2s)`; built pages carry `starlight-aside--caution` for both the WARNING and IMPORTANT alerts. `git diff -w --ignore-blank-lines` on the two docs prints no changed lines (whitespace and blank lines only).

## Regression vs 342-01 baseline

- Unit (`node --test`, TAP): pass 900, fail 3; baseline 3 failures; new failures: none.
- Playwright (build-output, reader-layout, trust-signals, smoke, baseline): 64 passed, 1 skipped, 20 failed; baseline 20; new failure IDs: none.
- Verdict: diff empty (`REGRESSION-DIFF-EMPTY`).

## Deviations from Plan

None - plan executed as written. The two verification scripts were run via files in the scratchpad (inline interpreters are blocked in this environment); the commands are the plan's, unchanged.

## Known Stubs

None.

## Threat Flags

None. T-342-11 mitigated by the spec (mapped aside per type per page, no raw marker in any dist page); T-342-SC mitigated by the exact pin and committed lockfile.

## Self-Check: PASSED

- docs-astro/tests/build-output/alerts.spec.ts: FOUND
- commits eb8195011 and b3075b590 are ancestors of HEAD: FOUND
