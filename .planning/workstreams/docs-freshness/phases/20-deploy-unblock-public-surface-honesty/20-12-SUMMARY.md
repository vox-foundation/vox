---
phase: 20-deploy-unblock-public-surface-honesty
plan: 12
subsystem: docs-site
tags: [docs, astro, starlight, archive, redirects, pagefind, sitemap]
status: complete

requires:
  - 20-01 (lockfile, pnpm@11 build)
  - 20-02 (tests/lib/dist.ts, dates.spec through the content dir)
provides:
  - "docs-astro/scripts/setup-content.mjs: MIRROR_MARKER, MIRROR_EXCLUDED, prepareMirrorDir(mirrorPath), buildMirror(sourceDir, mirrorPath); script body runs only when executed directly"
  - "docs-astro/src/content/docs is a gitignored, marker-guarded mirror dir of per-entry symlinks into docs/src (no archive, .well-known, SUMMARY.md)"
  - "docs-astro/src/pages/retired.astro: /retired/ notice page (noindex, pagefind false)"
  - "public/_redirects: /archive/* /retired/ 301"
affects: [20-08, 20-09, 20-10]

actuals:
  tokens: 4800
  tasks: 2
  commits: 2
plan_head_before: f0365a77aadb9486d153e4a0d53ff673a44f5bdb

tech-stack:
  added: []
  patterns:
    - "Content exclusion happens by not mirroring an entry, since docsLoader() has no exclude option"
    - "Destructive rebuild guarded by a marker file; links inside the mirror are unlinked, never followed"

key-files:
  created:
    - docs-astro/src/pages/retired.astro
    - docs-astro/tests/build-output/archive.spec.ts
    - docs-astro/tests/unit/setup-content.test.mjs
  modified:
    - docs-astro/scripts/setup-content.mjs
    - docs-astro/src/content.config.ts
    - docs-astro/public/_redirects

key-decisions:
  - "prepareMirrorDir empties a marked mirror entry by entry (unlink for links, rmSync only for real copies) instead of one recursive rmSync, so a win32 junction inside the mirror is never traversed"
  - "The direct-run check compares import.meta.url with the realpath of argv[1], so the script still runs when invoked through a symlinked path"
  - "The retired page's off-site check covers links inside <main>; Starlight header/social chrome is outside the page's control"

metrics:
  duration: "~10 min"
  completed: 2026-10-09
---

# Phase 20 Plan 12: Unpublish the archive, retire its URLs Summary

`docs/src/archive/` is no longer built: setup-content.mjs now builds `src/content/docs` as a marker-guarded mirror of per-entry symlinks that leaves the archive out, and `/archive/*` 301s to one noindex, Pagefind-excluded `/retired/` page.

## What was built

- **Mirror (Task 1, tracer).** 20 of 22 top-level `docs/src` entries are mirrored (excluded: `archive`, `.well-known`; `SUMMARY.md` is not present in this tree). The mirror holds 21 entries: those 20 links plus `.vox-docs-mirror`. No win32 copy fallback was used (macOS run; the EPERM fallback is in place for Windows without symlink privilege).
- **Archive removed from dist.** Before: 970 `index.html` files in dist, 298 of them under `dist/archive/`, and 298 archive URLs in the sitemap. After Task 1: 672 `index.html` files, no `dist/archive/`, and no archive URL in the sitemap.
- **content.config.ts** no longer passes the `exclude` option that `docsLoader()` ignored.
- **Retired page and redirect (Task 2).** `dist/retired/index.html` carries `<meta name="robots" content="noindex"/>`, has no `data-pagefind-body`, and links only to `/` and the GitHub archive tree. `dist/_redirects` has `/archive/* /retired/ 301` exactly once, and all 29 earlier rules are unchanged.

## Verification

| Check | Result |
|---|---|
| `node --test tests/unit/setup-content.test.mjs` | 5 pass, 0 fail |
| Two consecutive `setup-content.mjs` runs: `docs/src` modified/untracked set unchanged, `ls-files -d` empty, marker present, no `archive` | pass |
| `git status --short docs/src` before/after the whole plan | identical (empty) |
| `git check-ignore` on the mirror; `git status --porcelain docs-astro/src/content` empty | pass |
| Fresh build + `archive.spec.ts` + `dates.spec.ts` | 8/8 (Task 1), then archive.spec 6/6 (Task 2) |
| All build-output specs (archive, dates, honesty) after Task 2 | 20 passed |
| All docs-astro unit tests | 702 pass, 0 fail |
| `grep -c '^/archive/\* /retired/ 301' dist/_redirects` | 1 |

### Mutation evidence

- **Delete guard:** `grep -c 'existsSync(.*MIRROR_MARKER' scripts/setup-content.mjs` read **1** before, **0** with the marker check removed, **1** after restore. With the check removed, case (a) failed (`not ok 1 - (a) an unmarked real directory is refused…`, 4 pass / 1 fail); after restore, 5 pass / 0 fail and the file was byte-identical to the backup.
- **Archive exclusion:** with `'archive'` removed from `MIRROR_EXCLUDED`, a rebuild put 298 pages back under `dist/archive/`, and archive.spec failed "the archive is not built" and "no sitemap URL points into the archive" (2 failed, 2 passed). After restoring the file (byte-identical), a clean rebuild was green again.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Shell wrapper blocked `perl -0pi`, `node -e`, `cmp` and `od`**
- **Found during:** Task 1 mutation, Task 2 redirect append
- **Fix:** mutations ran from a temp script (`/tmp/p20-12-mutate.mjs`), with `diff -q` in place of `cmp` and `tail -c 1 | wc -l` in place of `od`. Nothing ran in the blocked attempts (the wrapper rejects the whole pipeline).

**2. [Rule 1 - Bug] Wrapper banner injected into archive.spec.ts; one edit silently dropped**
- **Found during:** Task 2
- **Issue:** one exact-text edit to archive.spec.ts was not applied, and a later edit inserted a `[lean-ctx] …` banner as line 1, so Playwright reported a syntax error.
- **Fix:** rewrote the spec in one Write, then stripped the banner with a script. Checked all six plan files and the committed Task 1 objects for banner text before each commit; none found.
- **Commit:** 02ab8968c

**3. [Rule 1 - Bug] Wrong sanity floor in the redirect-preservation test**
- The first draft asserted more than 30 committed rules; there are 29. Lowered the floor to 25 (the real check is that every committed rule is still present).

## Notes for later plans

- LINKS-02 is left **Pending** in REQUIREMENTS.md: this plan is a prerequisite only; the repo-Markdown mounts that satisfy it come in 20-09.
- `src/utils/page-index.mjs` still says the excluded paths match "the config"; the exclusion now lives in setup-content.mjs. The behaviour is the same; only the comment is stale (outside this plan's files).
- The `archive/` noindex branch in `src/routeData.ts` is now dead and harmless, as the plan expected.
- 20-09 adds `repo/` mounts inside this mirror. Add them in `buildMirror` (after `prepareMirrorDir`), so a rebuild clears them through the same guard.

## Self-Check: PASSED

- FOUND: docs-astro/scripts/setup-content.mjs, docs-astro/src/pages/retired.astro, docs-astro/tests/build-output/archive.spec.ts, docs-astro/tests/unit/setup-content.test.mjs
- FOUND commits: 477f8432f, 02ab8968c
