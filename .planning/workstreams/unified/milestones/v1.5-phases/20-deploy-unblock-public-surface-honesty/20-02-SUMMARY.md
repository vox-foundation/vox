# Plan 20-02 Summary: True Git-Driven Last-Updated Dates

## Completed Tasks
- **Task 1: Pre-compute clean git dates and wire to Starlight route data**
  - Created `.git-blame-ignore-revs` containing 5 mechanical commit hashes.
  - Implemented `docs-astro/src/utils/git-dates.mjs` (`parseGitLog`, `readIgnoreRevs`, `getGitDates`) skipping ignore-revs, format/SSOT chore commits, and bulk commits (>100 files).
  - Added unit test suite `docs-astro/tests/unit/git-dates.test.mjs` with 9 passing tests covering parsing, rename tracking, ignore-rev filtering, and empty paths.
  - Disabled Starlight's built-in git lookup (`lastUpdated: false` in `astro.config.mjs`) to avoid symlink breakage.
  - Injected true dates into Starlight route data via `docs-astro/src/routeData.ts` middleware (`starlightRoute.lastUpdated`).
  - Updated `docs-astro/src/pages/feed.xml.ts` to key git dates by `docs/src/...` path.
- **Task 2: Assert date rendering in build output**
  - Updated `docs-astro/playwright.config.ts` to ignore unit tests, helpers, and fixtures (`testIgnore: ['unit/**', 'lib/**', 'fixtures/**']`).
  - Created `docs-astro/tests/lib/dist.ts` exporting `DIST_DIR`, `readDist`, and `listDistHtml`.
  - Implemented `docs-astro/tests/build-output/dates.spec.ts` asserting >90% coverage of `<time datetime="`, calendar day match on key reference/tutorial pages, and ignore-rev validation.
  - Verified `dist/feed.xml` exists and contains `<pubDate>`.
- **Task 3: Run scaffold in docs-quality CI with full history**
  - Updated `.github/workflows/docs-quality.yml` with triggers for `docs-astro/**`, `.git-blame-ignore-revs`, `.lycheeignore`, and `.github/workflows/docs-quality.yml`.
  - Configured `actions/checkout@v7` with `fetch-depth: 0`.
  - Added "Docs unit tests" (`node --test "tests/unit/*.test.mjs"`) and "Build-output checks" (`npx playwright test tests/build-output/ --reporter=line`) after the Starlight build.
  - Verified with `actionlint` and `yaml-parse-check`.

## Verification Evidence
- `git-dates` loaded entries: `[git-dates] loaded 1031 date entries for /Users/brbrainerd/dev/vox`
- `dates.spec.ts` output:
  - `Page date coverage: 968/971 (99.7%)`
  - `2 passed, 1 skipped` (skipped ignore-rev assertion as no current docs page has `4e98a0f87` as its newest commit)
- `node --test tests/unit/git-dates.test.mjs`: `9 passed, 0 failed`
- Actionlint on `.github/workflows/docs-quality.yml`: passed with code 0.
