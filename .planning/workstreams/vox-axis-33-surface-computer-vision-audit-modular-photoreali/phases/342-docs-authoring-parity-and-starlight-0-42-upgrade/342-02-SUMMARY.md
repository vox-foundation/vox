---
phase: 342-docs-authoring-parity-and-starlight-0-42-upgrade
plan: 02
subsystem: docs
tags: [docs-astro, astro-7, starlight-0.42, llms-txt, dependabot, playwright]
requires: [342-01]
provides:
  - docs-astro on astro 7.4.5 / @astrojs/starlight 0.42.9 (peers satisfied for Plans 03 and 04)
  - unified() Markdown processor running the two remark plugins
  - agent-safe Playwright preview, unpinned dependabot
affects: [342-03, 342-04]
tech-stack:
  added: ["@astrojs/markdown-remark 7.3.5"]
  patterns: ["caret-minor floors under the release-age gate; no minimumReleaseAgeExclude"]
key-files:
  modified:
    - docs-astro/package.json
    - docs-astro/pnpm-lock.yaml
    - docs-astro/pnpm-workspace.yaml
    - docs-astro/astro.config.mjs
    - docs-astro/playwright.config.ts
    - docs-astro/src/content.config.ts
    - .github/dependabot.yml
    - docs-astro/patches/starlight-llms-txt@0.12.0.patch (renamed from 0.10.0, content unchanged)
key-decisions:
  - "Approved mature-only: no minimumReleaseAgeExclude lines; caret-minor floors"
  - "Bare @astrojs/starlight/components alias re-pointed via import.meta.resolve; sub-path alias deleted"
requirements-completed: [DOCS-PARITY-02]
status: complete
completed: 2026-11-05
commits: 2
plan_head_before: c083b82bfe68e521bb5bc105178d7c749807ef16
plan_head_after: 1ea19bbd5b2b7e7b2b89c7764c1601ecd1373527
actuals:
  tokens: 9000
  tasks: 3
  commits: 2
---

# Phase 342 Plan 02: Astro 7 / Starlight 0.42 upgrade Summary

docs-astro now builds (907 pages) on astro 7.4.5 / Starlight 0.42.9 with a `unified()` Markdown processor, a re-pointed components alias and the re-keyed llms-txt patch; unit and Playwright suites are identical to the 342-01 frozen baseline (zero new failures).

## Package approval record (Task 1, blocking-human)

Human answer, verbatim: **"Approved mature-only (Recommended)"**. No `minimumReleaseAgeExclude` lines were written (`grep -c minimumReleaseAge docs-astro/pnpm-workspace.yaml` = 0). All have empty postinstall.

Approved name@version entries (all eight):
1. astro 7.4.7 (installed via floor ^7.4.0)
2. @astrojs/starlight 0.42.9 (floor ^0.42.0)
3. @astrojs/markdown-remark 7.3.5 (floor ^7.3.0)
4. @astrojs/check 0.9.12 (floor ^0.9.12)
5. @astrojs/sitemap 3.7.4 (exact)
6. starlight-llms-txt 0.12.0 (exact)
7. starlight-github-alerts 0.5.0 (exact; approved for Plan 03, not installed here)
8. starlight-links-validator 0.27.0 (exact; approved for Plan 04, not installed here)

## Actual resolved versions (pnpm ls / lockfile)

| Package | Spec in package.json | Resolved |
|---|---|---|
| astro | ^7.4.0 | **7.4.5** (7.4.7 too young for the age gate, as intended) |
| @astrojs/starlight | ^0.42.0 | 0.42.9 |
| @astrojs/markdown-remark | ^7.3.0 | 7.3.5 |
| @astrojs/check | ^0.9.12 | 0.9.12 |
| @astrojs/sitemap | 3.7.4 | 3.7.4 |
| starlight-llms-txt | 0.12.0 | 0.12.0 |

`@playwright/test` lock pin 1.63.0 retained. `allowBuilds` unchanged (esbuild/sharp true, workerd false); install finished with no `ERR_PNPM_IGNORED_BUILDS`.

## Commits

- 5ddcf7145 feat(342-02): upgrade docs-astro to astro 7 / starlight 0.42 (Task 2 tracer)
- 1ea19bbd5 chore(342-02): agent-safe preview, drop dependabot ignores, astro/zod import (Task 3)

Task 1 was a checkpoint with no commit.

## Verification

- Tracer: `pnpm build` exit 0, `907 page(s) built`; `tests/build-output/llms.spec.ts` 3 passed; Internals exclusion patch applies on 0.12.0.
- Task 3: `pnpm install --frozen-lockfile` exit 0; rebuild 907 pages (exit 0), no deprecation warnings.
- Unit (node TAP): 903 tests, 900 pass, 3 fail; failing names are exactly the 3 baseline names. New-failure diff: empty.
- Playwright (`build-output/`, reader-layout, trust-signals, smoke, baseline): 61 passed, 20 failed, 1 skipped; the 20 failed IDs are exactly the 20 in 342-baseline-playwright-failed.txt (both `comm -23` new-failure and `comm -13` fixed-vs-baseline diffs empty). Webserver started via `--ignore-lock` from an agent shell, no stray preview left.
- `git diff --quiet -- docs-astro/src/components docs-astro/src/routeData.ts` clean (overrides and routeData byte-identical).

## Observation: milestones/v4.3-ROADMAP.md warning

`[remark-doc-links] Target not found: 'milestones/v4.3-ROADMAP.md' in '.planning/ROADMAP.md'` is still emitted on Astro 7 and remains **non-fatal** (build exits 0). No checkpoint needed; ROADMAP.md untouched.

## Deviations from Plan

- Version floors use the checkpoint's caret-minor floors (^7.4.0 / ^0.42.0 / ^7.3.0) instead of the plan's ^7.4.7 / ^0.42.9 / ^7.3.2, per the human's "mature-only" answer. Acceptance greps on `package.json` text for the higher floors are therefore superseded by the checkpoint resolution; the lockfile is the source of truth (above).
- Tracer commit contains only Task 2 files; Task 3's three files were committed separately (as planned).

Otherwise: plan executed as written.

## Known Stubs

None.

## Threat Flags

None. No new endpoints, auth paths or trust-boundary surface; T-342-05/06/07 mitigated (allowBuilds unchanged, no age-gate excludes), T-342-08 verified by llms spec, T-342-09 closed (dependabot ignore list removed).

## State updates deferred to the orchestrator

STATE.md / ROADMAP.md / REQUIREMENTS.md not touched (orchestrator owns them).

## Self-Check: PASSED

Files verified present (patch 0.12.0, SUMMARY); commits 5ddcf7145 and 1ea19bbd5 are ancestors of HEAD.
