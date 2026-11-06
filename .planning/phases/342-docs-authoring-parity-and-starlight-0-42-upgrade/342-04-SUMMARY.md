---
phase: 342-docs-authoring-parity-and-starlight-0-42-upgrade
plan: 04
subsystem: docs
tags: [docs-astro, starlight, links-validator, mutation-probe]
requires: [342-01, 342-03]
provides:
  - starlight-links-validator 0.27.0 gates every docs build (default fatal options, zero findings)
  - linksValidatorExclude(docPages) pure predicate with 10 unit cases
affects: []
tech-stack:
  added: ["starlight-links-validator 0.27.0 (exact pin)"]
  patterns: ["exclude predicate limited to .mdx pages/routes + documented 127.0.0.1:11434 probe URL"]
key-files:
  created:
    - docs-astro/src/utils/links-validator-exclude.mjs
    - docs-astro/tests/unit/links-validator-exclude.test.mjs
  modified:
    - docs-astro/package.json
    - docs-astro/pnpm-lock.yaml
    - docs-astro/astro.config.mjs
    - docs/src/architecture/docs-site-asciidoc-parity-findings-2026.md
key-decisions:
  - "Validator runs on defaults (failOnError true); no warn-only phase because baseline is zero"
  - "Same-page #hash is excluded only when the page itself is .mdx (closes T-342-12 hole)"
requirements-completed: [DOCS-PARITY-04]
status: complete
completed: 2026-11-06
commits: 2
plan_head_before: 5f2182657e7bb6f2e9a978ede2e3e75834a9daf1
plan_head_after: 845c04e78
actuals:
  tokens: 6000
  tasks: 2
  commits: 2
---

# Phase 342 Plan 04: starlight-links-validator build gate Summary

`starlight-links-validator@0.27.0` (exact, default fatal options) now runs in `pnpm build` with a unit-tested exclude predicate; baseline is zero findings across 907 pages, and a mutation probe proves a bad page link and a bad hash fail the build.

## Package approval

Human approval recorded in 342-02-SUMMARY.md ("Approved mature-only (Recommended)"). Installed with `--save-exact`; +7 transitive packages, pnpm supply-chain policy passed, no `minimumReleaseAgeExclude` lines.

## Tasks

| Task | Commit | Result |
| ---- | ------ | ------ |
| 1 Tracer: predicate test-first, pin, wire, build | 3d9de7a1d | RED: test failed (module missing). GREEN: 10/10. Build: `All internal links are valid.`, 907 page(s) built |
| 2 Mutation probe, findings SSOT, phase gate | 845c04e78 | Probe fired and restored; SSOT updated; no new failures vs baseline |

## Mutation probe evidence

Before: `git diff --quiet` on installation.md clean; backup taken; seeded strings counted 1 and 1.
Failing build (exit 1), log excerpt:

```
268 | /reference/no-such-page-zzz/            ╰── invalid link
270 | /reference/installation/#no-such-heading-zzz   ╰── invalid hash
· Found 2 invalid links in 1 file. ·
[AstroUserError] Links validation failed.
```

After the failing build the seeded strings were still present (1 and 1). Restore by copying the backup back; `cmp` reported identical, `git status` clean for installation.md. Clean rebuild: exit 0, `All internal links are valid.`, 907 page(s) built.

## Phase gate vs frozen baseline

- Unit (full TAP): pass 910, fail 3; failing names minus `342-baseline-unit-failed.txt` = empty (no new failures). The two named files (links-validator-exclude, remark-vox-include) pass 16/16.
- Playwright (build-output incl. alerts.spec.ts, reader-layout, trust-signals, smoke, baseline): 64 passed, 1 skipped, 20 failed; failing IDs minus `342-baseline-playwright-failed.txt` = empty (the 20 are the baseline set, e.g. trust-signals phone/light layout).

## Deviations from Plan

None - plan executed as written. The known `remark-doc-links` "Target not found: milestones/v4.3-ROADMAP.md" (mounted .planning/ROADMAP.md) is a non-fatal remark warning and the validator did not flag it, so the optional extra exclude clause was not needed.

Process notes: the Claude Code process restarted mid-run (scratchpad wiped, root-pin guard missing); I stopped, the coordinator restored the guard, and the plan resumed from the build step.

## Known Stubs

None.

## Threat Flags

None. T-342-12 mitigated (clause (b) plus 4 negative unit cases plus mutation probe); T-342-14 mitigated (backup, presence checks, `cmp`).

## Self-Check: PASSED

Files exist; commits 3d9de7a1d and 845c04e78 are ancestors of HEAD; installation.md identical to pre-probe copy; findings SSOT lint exit 0.
