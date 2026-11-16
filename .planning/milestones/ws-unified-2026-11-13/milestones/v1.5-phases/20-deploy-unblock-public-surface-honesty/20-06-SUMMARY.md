---
phase: 20-deploy-unblock-public-surface-honesty
plan: 06
status: complete
completed_at: "2026-10-12T21:16:00-07:00"
requirements:
  - HONEST-01
  - HONEST-02
measured:
  internals_pages: 135
  total_sitemap_urls: 826
---

# Plan 20-06 Summary: Public Surface Honesty (Banners, Noindex, Internals Sidebar, Sitemap, Robots)

## Accomplishments

1. **Status Policy SSOT (`docs-astro/src/utils/page-status.mjs`)**
   - Established single source of truth for page status policy (`research`, `roadmap`, `deprecated`, `legacy`, `current`).
   - Defined `statusPolicy(status)`, `isInternalsStatus(status)`, `bannerFor(status)`.
   - Built unit test suite `tests/unit/page-status.test.mjs` (all cases, casing, whitespace).

2. **Route Middleware Integration (`docs-astro/src/routeData.ts`)**
   - Injected Starlight banners for `research`, `roadmap`, `deprecated`, and `legacy` pages.
   - Pushed `<meta name="robots" content="noindex">` to page head for all non-current pages.
   - For `internals` pages, preserved `data-pagefind-body` while injecting Pagefind metadata:
     - `<meta data-pagefind-filter="section[content]" content="Internals">`
     - `<meta data-pagefind-meta="title[content]" content="Internals — <title>">`

3. **Slug & Page Index Utilities (`doc-slug.mjs`, `page-index.mjs`)**
   - Created `docSlug(relPath)` mirroring Starlight's glob loader id assignment (github-slugger per segment, dropping `/index`).
   - Created `listDocPages(docsSrc)`, `noindexRoutes(pages)`, `internalsDocIds(pages)`.
   - Pinned direct dependencies `github-slugger@2.0.0` and `@astrojs/sitemap@3.7.2` in `package.json`.

4. **Sidebar Internals Group (`docs-astro/src/utils/sidebar.mjs`)**
   - Segregated `mainPages` and `internalsPages` (135 measured pages).
   - Preserved main category groupings without any research/roadmap contamination.
   - Appended a collapsed `Internals` group at the end of the sidebar, containing category sub-groups ordered by `SECTION_ORDER` and remaining categories alphabetically.

5. **Filtered Sitemap & Honest Robots.txt (`astro.config.mjs`, `docs/src/robots.txt`)**
   - Registered `@astrojs/sitemap` before Starlight with filter excluding all `noindex` routes and `/retired/`.
   - Updated `robots.txt` to honest configuration (`User-agent: *`, `Allow: /`, `Sitemap: https://voxlang.org/sitemap-index.xml`) with zero `Disallow` directives so crawlers can discover `noindex` meta tags.

6. **Verification & Mutation Testing (`tests/build-output/honesty.spec.ts`)**
   - 9 automated build-output tests passing:
     - Research banner, noindex, and `data-pagefind-body` presence.
     - Deprecated and legacy banner and noindex verification.
     - Zero banners or noindex tags leaking onto `reference/cli`.
     - Zero research/roadmap pages missing noindex across the entire site.
     - Pagefind `pf_fragment` search index metadata verification.
     - Internals group placement last in sidebar with zero link leaks to main groups.
     - `sitemap-index.xml` generated without `sitemap.xml`, and 0 noindex URLs present.
     - Honest `robots.txt` verification.
   - **Mutation Test:** Temporarily removed the sitemap `filter`. Verified `honesty.spec.ts` failed loudly (`Error: Noindex URL /adr/012-internal-web-ir-strategy/ found in sitemap sitemap-0.xml`). Restored filter and verified all 9 tests green.
