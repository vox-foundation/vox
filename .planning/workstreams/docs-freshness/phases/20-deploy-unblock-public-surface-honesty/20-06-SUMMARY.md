---
phase: 20-deploy-unblock-public-surface-honesty
plan: 06
subsystem: docs-site
tags: [docs, astro, starlight, pagefind, sitemap, robots, honesty]
status: complete

requires:
  - 20-01 (package.json/lockfile with pinned wrangler)
  - 20-02 (routeData.ts git dates, tests/lib/dist.ts, build-output tests)
provides:
  - "docs-astro/src/utils/page-status.mjs: STATUS_POLICY, statusPolicy(status), isInternalsStatus(status), bannerFor(status) — the only status -> {banner, noindex, internals} rule"
  - "docs-astro/src/utils/doc-slug.mjs: docSlug(relPath) — Starlight's route id (github-slugger per segment, drop /index, bare index -> \"\")"
  - "docs-astro/src/utils/page-index.mjs: listDocPages(docsSrc), noindexRoutes(pages), internalsDocIds(pages) — for 20-07 llms exclusion and 20-09 repo mounts"
  - "Banner + robots noindex on research/roadmap/deprecated/legacy pages; Pagefind section=Internals filter and 'Internals — ' title on research/roadmap pages"
  - "Collapsed Internals sidebar group (last), filtered sitemap-index.xml, robots.txt with no Disallow"
affects: [20-07, 20-09, 20-12]

actuals:
  tokens: 7600
  tasks: 3
  commits: 3
plan_head_before: 4aae461ae931a8efa7a2e7acba3ca7e14a1ad766

tech-stack:
  added:
    - "github-slugger 2.0.0 (direct, exact; already in lockfile)"
    - "@astrojs/sitemap 3.7.2 (direct, exact; already in lockfile)"
  patterns:
    - "Status rule lives in one module; route middleware, sidebar, sitemap filter all read it"
    - "Build-output tests read dist/ (HTML, Pagefind fragments, sitemap XML) without a browser"

key-files:
  created:
    - docs-astro/src/utils/page-status.mjs
    - docs-astro/src/utils/doc-slug.mjs
    - docs-astro/src/utils/page-index.mjs
    - docs-astro/tests/unit/page-status.test.mjs
    - docs-astro/tests/unit/doc-slug.test.mjs
    - docs-astro/tests/build-output/honesty.spec.ts
  modified:
    - docs-astro/src/routeData.ts
    - docs-astro/src/utils/sidebar.mjs
    - docs-astro/astro.config.mjs
    - docs-astro/package.json
    - docs-astro/pnpm-lock.yaml
    - docs/src/robots.txt

key-decisions:
  - "Sidebar links now use the route id from listDocPages instead of the raw file path; dotted or uppercase file names (qwen-3.7, README, *.generated) previously produced sidebar hrefs to paths that were never built"
  - "The Internals page title is passed unescaped to the Pagefind meta attribute because Astro escapes attribute values when rendering head entries; pre-escaping would double-encode"
  - "Internals sub-groups list categories in SECTION_ORDER, then the rest alphabetically; main groups keep today's first-seen order for categories outside SECTION_ORDER"

metrics:
  duration: "~9 min"
  completed: 2026-10-09
---

# Phase 20 Plan 06: Status banners, Internals section, honest sitemap Summary

Frontmatter `status` now drives everything public about a page from one module (`page-status.mjs`): research/roadmap/deprecated/legacy pages get a Starlight banner and robots `noindex`. The 133 research/roadmap pages sit only in a collapsed `Internals` sidebar group placed last, and Pagefind labels them `section: Internals` with an `Internals — ` title. The sitemap leaves out all 142 noindex routes, and robots.txt points at `sitemap-index.xml` with no `Disallow`.

## Measured counts

- Docs pages indexed by `listDocPages`: 670. Every id has a built `dist/<id>/index.html` (doc-slug cross-check).
- Internals (research + roadmap) pages: **133** (the plan measured 131 earlier; two more pages have since been marked research/roadmap).
- Noindex routes (Internals + deprecated + legacy): 142. Sitemap URLs after filtering: 826.

## Tasks

| Task | Commit | Result |
|---|---|---|
| 1 Status policy → banner + noindex + Pagefind label | bc95cf8c2 | `page-status.mjs`, route middleware, unit test (5/5), honesty spec |
| 2 Slug + page index helpers; direct deps | cf84a8963 | `doc-slug.mjs`, `page-index.mjs`, unit test (4/4, incl. dist cross-check); no new lockfile package keys |
| 3 Internals sidebar, filtered sitemap, robots | b8129d5f6 | sidebar rewritten on `listDocPages`; `@astrojs/sitemap` registered before Starlight with noindex filter; robots.txt rewritten |

## Verification

- `node --test tests/unit/page-status.test.mjs`: pass 5, fail 0. `doc-slug.test.mjs`: pass 4, fail 0 (dist cross-check ran, nothing skipped). The other unit suites still pass: git-dates 10, source-link-targets 671, tutorial-record 7.
- Fresh `pnpm@11 build` (969 pages), then `playwright test tests/build-output/honesty.spec.ts tests/build-output/dates.spec.ts`: 14 passed.
- `grep -c 'name="robots" content="noindex"' dist/reference/cli/index.html` → 0.
- `ls dist/sitemap*.xml` → `sitemap-0.xml`, `sitemap-index.xml` only (no `sitemap.xml`; Starlight's own sitemap was suppressed).
- `jq` pins → `2.0.0`, `3.7.2`. `pnpm@11 install --frozen-lockfile` → up to date. `packages:` keys identical before/after `pnpm add` (634 keys).
- Pagefind 1.5.2 fragments decode as gzip + leading `pagefind_dcd` + JSON, as the plan expected. The research page fragment has `filters.section` = Internals and a title starting `Internals — `; the `reference/cli` fragment has neither.

### Mutation checks

- Task 1: set the `research` banner to `null` and rebuilt. The spec failed on "a research page has the Internals banner…" (1 failed, 5 passed). Restored from backup and confirmed the `research note` string was back.
- Task 3: replaced the `sitemap({ filter })` call with `sitemap()` and rebuilt. The spec failed on "the sitemap lists no noindex page and not /retired/" (1 failed, 9 passed). Restored and confirmed `noindex.has` was back; a final clean rebuild was green.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Sidebar hrefs pointed at unbuilt paths**
- **Found during:** Task 2 (slug cross-check)
- **Issue:** `sidebar.mjs` used the raw file path as the link, for example `/architecture/qwen-3.7-…`, but Starlight builds that page at `/architecture/qwen-37-…`. Uppercase and `*.generated` names had the same mismatch.
- **Fix:** The sidebar now builds on `listDocPages` and links to `page.id`. Because the walk is shared, the separate `collectPages` copy is gone. `SUMMARY.md` and `_`-prefixed files are now excluded the same way the content collection excludes them.
- **Files modified:** docs-astro/src/utils/sidebar.mjs
- **Commit:** b8129d5f6

**2. [Rule 1 - Bug] Spec matchers were stricter than Astro's real markup**
- **Found during:** Task 1
- **Issue:** The first spec draft expected `<meta …>` and `class="sl-banner"`. Astro actually emits `<meta …/>` and a scoped class (`sl-banner astro-…`). The rendering itself was correct.
- **Fix:** Matchers became regexes on the noindex meta and the banner `<div>` content. No assertion was weakened: the banner text and the `<strong>Label:</strong>` prefix are still checked.
- **Commit:** bc95cf8c2

**3. Task 1 page walk, later replaced**
- Task 1's "every research/roadmap page has noindex" check first used a naive lower-cased route walk, because `docSlug` did not exist until Task 2. Task 3 switched it to `internalsDocIds(listDocPages())`, which covers all 133 pages with no existence filter.

## Requirements

HONEST-01 is marked complete. HONEST-02 stays open: its llms.txt exclusion is 20-07's work, which reads `internalsDocIds` from this plan.

## Threat Flags

None. Banner HTML comes only from constants in `page-status.mjs`, and the status value is used as a lookup key (T-06-1; the unit test checks that a `<script>` status gets no banner). The new direct dependencies are exact pins of versions already in the lockfile (T-06-3).

## Self-Check: PASSED

- Created files exist: page-status.mjs, doc-slug.mjs, page-index.mjs, page-status.test.mjs, doc-slug.test.mjs, honesty.spec.ts.
- Commits bc95cf8c2, cf84a8963, b8129d5f6 are present on `docs-freshness/phase-20` (`git rev-list --count 4aae461ae..HEAD` = 3 before this SUMMARY commit).
