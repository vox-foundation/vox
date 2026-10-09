---
phase: 20-deploy-unblock-public-surface-honesty
plan: 02
subsystem: docs-site
tags: [starlight, astro, git-log, playwright, node-test, ci]

requires: []
provides:
  - "git-dates.mjs: parseGitLog / readIgnoreRevs / getGitDates (repo-path keys) / DEFAULT_IGNORE_SUBJECTS"
  - ".git-blame-ignore-revs (5 mechanical docs commits)"
  - "routeData.ts sets starlightRoute.lastUpdated; Starlight lastUpdated off"
  - "Wave-0 test scaffold: tests/unit (node --test), tests/lib/dist.ts, tests/build-output (Playwright, no browser)"
  - "docs-quality CI: docs-astro/** trigger, fetch-depth 0, unit + build-output steps"
affects: [20-03, 20-04, 20-05, 20-06, 20-07, 20-08, 20-09, 20-10, 20-11, 20-12, docs-deploy]

actuals:
  tokens: 6000
  tasks: 3
  commits: 3
plan_head_before: 383408ef3dd5ee8881d7c91bb14379b5fb1aa03b

tech-stack:
  added: []
  patterns:
    - "Build-output specs are pure-fs Playwright tests (no page fixture) reading dist/ via tests/lib/dist.ts"
    - "Doc date-map key = 'docs/src/' + entry.filePath minus 'src/content/docs/' (routeData + feed share the rule)"

key-files:
  created:
    - .git-blame-ignore-revs
    - docs-astro/tests/unit/git-dates.test.mjs
    - docs-astro/tests/lib/dist.ts
    - docs-astro/tests/build-output/dates.spec.ts
  modified:
    - docs-astro/src/utils/git-dates.mjs
    - docs-astro/src/routeData.ts
    - docs-astro/astro.config.mjs
    - docs-astro/src/pages/feed.xml.ts
    - docs-astro/playwright.config.ts
    - .github/workflows/docs-quality.yml

key-decisions:
  - "An A or D record ends a path's history in parseGitLog, so a previous file at a re-used path never lends its dates"
  - "getGitDates memoises per (root, paths): one git log call per build shared by routeData and feed.xml"
  - "feed.xml.ts re-key moved into Task 1: the new repo-path keys make the old id lookup return nothing and the feed endpoint fails the build"

requirements-completed: [DEPLOY-04]

coverage:
  - id: D1
    description: "Mechanical-commit-aware, rename-following git date parser"
    requirement: DEPLOY-04
    verification:
      - kind: unit
        ref: "docs-astro/tests/unit/git-dates.test.mjs (10 tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Built pages render true Last updated dates through the content symlink"
    requirement: DEPLOY-04
    verification:
      - kind: integration
        ref: "docs-astro/tests/build-output/dates.spec.ts (4 tests)"
        status: pass
    human_judgment: false
  - id: D3
    description: "docs-quality CI runs the scaffold with full history"
    requirement: DEPLOY-04
    verification:
      - kind: other
        ref: "actionlint + yq checks + vox ci workflow-concurrency-guard/cache-key-lint/yaml-parse-check/ssot-drift"
        status: pass
    human_judgment: false

duration: 12min
completed: 2026-10-09
status: complete
---

# Phase 20 Plan 02: True per-page "Last updated" dates Summary

**Page dates now come from a tested git-log parser that skips the five bulk docs commits, ssot auto-regen/fmt/style commits and >100-file commits, follows renames, and reaches Starlight through the route middleware — 967 of 969 built pages show a date (previously 0).**

## Performance

- **Duration:** ~12 min (plus a 5 min cold `cargo build -p vox-cli` for the guards)
- **Tasks:** 3/3
- **Commits:** 3

## Accomplishments

- `parseGitLog` + `readIgnoreRevs` + memoised `getGitDates` keyed by repo path. Build log line: `[git-dates] 1034 dated paths from git log in /Users/brbrainerd/dev/vox/.claude/worktrees/docs-freshness-p20` (printed once per build — one `git log` call).
- `routeData.ts` sets `starlightRoute.lastUpdated`; `astro.config.mjs` has `lastUpdated: false`; the false "noindex makes Pagefind skip the page" comment now says noindex affects crawlers only.
- `tutorials/tut-getting-started` renders `2026-09-07T20:07:04.000Z` = newest substantive commit `c5502e567` (13:07:04 −07:00).
- `dates.spec.ts`: 4 passed. Coverage 967/969 docs pages dated. Ignored-commit assertion used `docs/src/adr/001-burn-backend-selection.md -> /adr/001-burn-backend-selection/` (newest commit is `4e98a0f87`; rendered date is earlier).
- `dist/feed.xml` carries 30 `<pubDate>` items; `playwright test --list` collects nothing from `unit/`, `lib/`, `fixtures/`.
- `docs-quality.yml`: 4 new trigger paths on PR and push, `fetch-depth: 0`, "Docs unit tests" and "Build-output checks" after "Build Starlight"; generator-drift untouched.

## Task Commits

1. **Task 1: parser + ignore-revs + middleware (tracer)** — `50607bc85`
2. **Task 2: build-output date spec, dist helper, Playwright ignore** — `e216545c7`
3. **Task 3: docs-quality CI** — `66c038f62`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] feed.xml.ts re-key moved from Task 2 into Task 1**
- **Found during:** Task 1 tracer build
- **Issue:** Task 1 changes the map keys to repo paths; the feed still looked up `doc.id`, got 0 items and threw `feed.xml produced 0 items…`, failing the build the tracer verify needs.
- **Fix:** Re-keyed the feed with the same rule as routeData in Task 1; also added a sample `filePath` and map key to that error message so a future key mismatch is diagnosable from the build log.
- **Files modified:** docs-astro/src/pages/feed.xml.ts
- **Commit:** 50607bc85

**2. [Rule 2 - Correctness] A/D records end a path's history**
- **Issue:** The plan's alias rule alone would credit a deleted predecessor's commits to a new file re-created at the same path.
- **Fix:** Tombstone the raw path on `A`/`D`; covered by an extra unit test. Also two extra unit tests beyond (a)–(g): style/fmt subjects, re-added path.
- **Commit:** 50607bc85

**3. [Rule 3 - Blocking] Guards run with a freshly built `vox`**
- **Issue:** The installed `vox` is stale (commit 6382 vs tree 6407) and refuses to run guards; `yaml-parse-check` requires a glob argument.
- **Fix:** Built `target/debug/vox` in this worktree and ran the guards with it; `yaml-parse-check .github/workflows/docs-quality.yml`. `ssot-drift` ran directly in this worktree (it has no foreign dirty files, so the throwaway HEAD worktree was unnecessary).

## Verification Results

| Check | Result |
|---|---|
| `node --test tests/unit/git-dates.test.mjs` (and CI form `"tests/unit/*.test.mjs"`) | 10 pass, 0 fail |
| ignore-revs SHAs resolve / count | 0 BAD, 5 |
| `pnpm@11 build` + `<time datetime>` on tut-getting-started | exit 0, 969 pages, date present |
| `playwright test tests/build-output/dates.spec.ts` | 4 passed |
| `playwright test --list` unit/lib/fixtures count | 0 |
| `dist/feed.xml` pubDates | 30 |
| actionlint; yq paths (PR/push), fetch-depth, step count | OK; 1/1, 0, 2 |
| `vox ci workflow-concurrency-guard`, `cache-key-lint`, `yaml-parse-check`, `ssot-drift` | all exit 0 |
| `vox ci workflow-scripts` | exit 1 — pre-existing, see below |

## Deferred Issues

- `vox ci workflow-scripts` fails on HEAD for references in `ci.yml`, `nightly.yml`, `setup-e2e.yml`, `release-prepare.yml`, `release-binaries.yml` (e.g. `scripts/ssot-regen.vox`, `scripts/install.sh`) missing from its allowlist. `docs-quality.yml` references no `scripts/` path and is not in the output; not caused by this plan.

## Notes

- The agent shell wrapper (lean-ctx) twice injected a `[lean-ctx] no compression applied…` banner as line 1 of an edited file (`feed.xml.ts`, `docs-quality.yml`) and dropped edits. Both were caught (esbuild/yaml parse errors), cleaned, and every committed file was grepped for `lean-ctx` before commit.
- A `pkill -f "astro build"` was used once to restart this worktree's build; it matches by command line machine-wide.

## Self-Check: PASSED

- Files: `.git-blame-ignore-revs`, `tests/unit/git-dates.test.mjs`, `tests/lib/dist.ts`, `tests/build-output/dates.spec.ts` exist.
- Commits `50607bc85`, `e216545c7`, `66c038f62` exist on `docs-freshness/phase-20`.
