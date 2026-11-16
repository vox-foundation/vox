# Phase 342: Docs Authoring Parity and Starlight 0.42 Upgrade - Research

**Researched:** 2026-11-05
**Domain:** Astro 6 -> 7 / Starlight 0.38 -> 0.42 framework upgrade of `docs-astro`, plus two Starlight plugins (GitHub alerts, links validator) on a symlink-mirrored content tree
**Confidence:** HIGH (the whole upgrade, both plugins and the validator mutation probe were executed end-to-end in a throwaway copy; see "Trial Evidence")

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **DOCS-PARITY-01 (done, uncommitted):** Extend the existing dependency-free `remark-vox-include.mjs` instead of replacing it with the blueprint's `unist-util-visit`/`remark-directive` rewrite. Exported `extractRegion(content, selector, srcPath)`; selectors `name`, `a,b`, `N..M`/`N-M`; markers `ANCHOR:`/`ANCHOR_END:` and `tag::x[]`/`end::x[]` behind `//` or `#`; exact-name match (blueprint's `\b` matched `foo` inside `foo-2`). `llms.txt` description reads version from root `Cargo.toml` (AGENTS.md version SSOT, not `CITATION.cff`).
- **DOCS-PARITY-02 first:** `starlight-links-validator@0.27.0` peers `astro >=7.2.10`, `@astrojs/starlight >=0.42.0`; `starlight-github-alerts@0.5.0` peers starlight `>=0.42.0`. Repo pins starlight `^0.38.3`, astro `^6.0.1`. No plugin lands before the upgrade.
- **Banners stay ledger-driven:** `Banner.astro` already renders drifting/broken/historical from `contracts/reports/docs-freshness/ledger.v1.json`; `page-status.mjs` covers research/roadmap/deprecated/legacy. `docs-reality-audit/inventory.v1.json` has `claims` with no `verification_status`, `findings.v1.json` is empty. Banner accuracy is `docs-freshness` Phase 32's job — do not add a second status source.
- **No new contracts:** `contracts/documentation/stability-matrix.v1.json` does not exist; tables that should be data use the `*.generated.md` + `ssot-drift` pattern.
- **`.mdx` is already first-class** (Phase 323); this phase neither forces `.md` purity nor converts more pages.
- **YAGNI cuts:** `remark-vox-attributes` `{token}` substitution, `::include{}` partials, `// !hide`, auto Expressive Code titles, `examples/golden/anti/` (directory absent), `starlight-markdown-blocks` custom asides.

### Claude's Discretion
- Upgrade sequencing (Astro then Starlight vs together) and whether Vite alias shims for `@astrojs/starlight/components` are still needed after 0.42.

### Deferred Ideas (OUT OF SCOPE)
- xref empty-label title resolution in `remark-doc-links.mjs` — revisit once the links validator lands and there is author demand.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DOCS-PARITY-02 | Starlight `^0.38` -> `>=0.42`, Astro `^6` -> `>=7.2.10`; fix overrides, `routeData.ts`, alias shims; build + unit + Playwright stay green | Trial build green on astro 7.4.7 / starlight 0.42.9 (907 pages, exit 0); only two things needed code changes (alias shim path, link failures that became fatal). Breaking-change map below. |
| DOCS-PARITY-03 | `starlight-github-alerts` renders the GitHub-alert docs as asides | Trial: 29 built pages contain `starlight-aside` (baseline: 1). Plugin works on the `unified()` processor. Zero config needed. |
| DOCS-PARITY-04 | `starlight-links-validator` in the build; fix the two `remark-doc-links` failures | Both failures fixed at source (exact edits below). Validator on this site reports 12 findings, 11 of them one false-positive class (`.mdx` pages), 1 real local link; a 3-clause `exclude` function brings it to 0 with `failOnError: true`. Mutation probe fails the build as required. |
</phase_requirements>

## Summary

The upgrade is far cheaper than feared. In a throwaway copy of the working tree (`/private/tmp/claude-501/-Users-brbrainerd-dev-vox/403f33b2-86be-4c98-b83c-a149ee9e1b8a/scratchpad/trial/vox`) a plain bump to `astro 7.4.7` + `@astrojs/starlight 0.42.9` + `starlight-llms-txt 0.12.0` built 907 pages with exit 0 and no behavioural regression: all Playwright and unit results are identical to the Astro 6 baseline (including the baseline's pre-existing failures, listed below). Only three source-level changes are required: (1) the Vite alias shim for `@astrojs/starlight/components` points at a path that no longer exists, (2) the two `remark-doc-links` failures change from "logged, build continues" to "build exits 1", and (3) `@astrojs/markdown-remark` must now be installed explicitly because Sätteri is Astro 7's default Markdown processor and our two remark plugins run on the `unified()` processor.

The two plugins are drop-in. `starlight-github-alerts` converts every `> [!TYPE]` blockquote in the Starlight docs collection to an aside. `starlight-links-validator` runs at `astro:build:done`; with the default options it flags only 12 links, and 11 are false positives from a single root cause (the three `.mdx` pages live behind symlinks, so Vite hands the validator their real `docs/src/...` path and it cannot derive their route). One small `exclude` function removes them, leaving a zero-error baseline that can ship with `failOnError: true` immediately, no warn-mode phase needed.

**Primary recommendation:** Sequence as (A) fix the two link failures at source on the current Astro 6 tree, (B) upgrade deps + alias path + `unified()` processor + dependabot/patch/playwright housekeeping, (C) add `starlight-github-alerts`, (D) add `starlight-links-validator` with the `exclude` function and `failOnError: true`, finished with a mutation probe. Do B only after A, because on Astro 7 the t3 audit link error fails the build.

## Trial Evidence (all executed this session)

Throwaway copy built from `git ls-files --cached --others --exclude-standard` (so it includes the uncommitted working-tree edits), `git init`-ed inside the scratchpad so `getGitDates` works. The real tree was not modified (`git status --short docs-astro` identical before/after).

| # | Variant | Result |
|---|---------|--------|
| 1 | astro 7.4.7 + starlight 0.42.9 + llms-txt 0.12.0, legacy `markdown.remarkPlugins`, alias shims as-is, link failures unfixed | **Build FAILED exit 1** at `[starlight-docs-loader] Error rendering architecture/t3-vox-wave4-...` (remark-doc-links `resolves outside repository root`). On Astro 6 the same error was only logged. |
| 2 | + link failures fixed, alias shims removed | **FAILED**: `Rolldown failed to resolve import "@astrojs/starlight/components" from docs/src/tutorials/tut-fullstack-walkthrough.mdx`. Shims are still needed (see Pitfall 2). |
| 3 | + shim re-pointed via `import.meta.resolve`, both plugins added, `errorOnRelativeLinks:false`, `failOnError:false` | **Build OK, 907 pages, exit 0**, 5m57s cold. 12 invalid links. 29 pages with `starlight-aside`. |
| 4 | + explicit `markdown.processor: unified({ remarkPlugins })` (no legacy keys), validator defaults except `failOnError:false` | **Build OK, 907 pages, 2m22s warm.** No Astro deprecation warning. Same 12 findings (so default `errorOnRelativeLinks: true` adds zero). |
| 5 | + `exclude` function (below), `failOnError` default (true) | **Build OK exit 0, 907 pages, 1m53s. Zero validator findings.** |
| 6 | Mutation probe: appended `[bad page](/reference/no-such-page-zzz/)` and `[bad hash](/reference/installation/#no-such-heading-zzz)` to `docs/src/reference/installation.md` | **Build exit 1**: `Found 2 invalid links in 1 file` (`invalid link`, `invalid hash`). Gate is real. |

Output parity: `find dist -type f` (excluding `_astro/`, `pagefind/`) was 978 files in both baseline and trial; the only diffs are the new findings page (expected, +1 page = 907) and one gitignored `assets/*.webp` absent from the copy. `sitemap-index.xml`, `sitemap-0.xml`, `feed.xml`, `llms*.txt`, Pagefind index (925 HTML files) all produced.

Test parity (trial dist vs. real baseline dist, same commands):

| Suite | Baseline (Astro 6, real tree) | Trial (Astro 7) |
|-------|-------------------------------|-----------------|
| `tests/build-output/` | 27 pass, 2 fail | 27 pass, 2 fail (same two) |
| `reader-layout` + `trust-signals` + `smoke` + `baseline` specs | 33 pass, 18 fail, 1 skipped | 33 pass, 18 fail, 1 skipped (same) |
| `node --test "tests/unit/*.test.mjs"` | 4 fail | 3 fail (the 4th, doc-slug dist cross-check, passes once `dist` contains the new findings page) |

**Pre-existing baseline failures (not caused by Phase 342; planner must baseline-diff, not demand "all green"):**
- `build-output/links.spec.ts:9` — expects `href="/reference/installation/#quick-install-from-source"` in `tut-getting-started`; page links `../reference/installation.md` with no anchor (uncommitted tutorial edit).
- `build-output/links.spec.ts:122` — `.planning/milestones/v4.3-ROADMAP.md` GitHub link targets a missing file (also logged as `[remark-doc-links] Target not found: 'milestones/v4.3-ROADMAP.md'` in the baseline build).
- `trust-signals.spec.ts` x18 — `.vox-trust-banner` not found for "verified" fixture pages; the working-tree `Banner.astro` only renders drifting/broken/historical. Spec/component drift owned by the docs-freshness stream.
- Unit: `tutorial-record.test.mjs` (blob-hash mismatch), `workflow-paths.test.mjs` x2 (mounted `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/workstreams/unified/ROADMAP.md` not covered by `docs-quality.yml`/`docs-deploy.yml` path filters).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Framework/plugin upgrade | Build tooling (`docs-astro/package.json`, lockfile, `astro.config.mjs`) | CI (`docs-quality.yml`, `docs-deploy.yml` run `pnpm install --frozen-lockfile` + `pnpm build`) | Static site generator; no runtime tier. |
| GitHub alert -> aside | Build-time Markdown pipeline (remark, `unified()` processor) | Browser (Starlight aside CSS only) | Transform happens at build; zero client JS. |
| Internal link validation | Build-time (`astro:build:done` hook, rehype collector) | CI gate (exit code of `pnpm build`) | Same build step that already gates PRs and deploy. |
| Link rewriting (`.md` -> `/slug/`) | Build-time remark plugin (`remark-doc-links.mjs`) | — | Runs before the validator sees links. |
| Subsystem-hub link correctness | Generator (`vox-doc-pipeline` openwiki) | `vox ci wiki-sync` + `ssot-drift` verify | AGENTS.md: fix the generator, never hand-edit `*.generated.md`. |
| Crate API pages | Generator (`vox ci crate-api-sync`, cargo-metadata workspace members) | — | Excludes `vox-cargo-shim` because it is `[workspace.exclude]`d. |

## Standard Stack

### Core (all versions verified with `npm view` on 2026-11-05)

| Library | Current pin | Target | Peers / notes |
|---------|-------------|--------|---------------|
| `astro` | `^6.0.1` (installed 6.1.9) | `^7.4.7` (latest 7.4.7; engines `node >=22.12.0`) | peer `@astrojs/markdown-remark ^7.3.0` (optional) [VERIFIED: npm view] |
| `@astrojs/starlight` | `^0.38.3` | `^0.42.9` | peers `astro ^7.2.10`; optional peer `@astrojs/markdown-remark ^7.3.0`; deps include `@astrojs/mdx ^8.0.3`, `@astrojs/markdown-satteri ^0.4.3`, `@astrojs/sitemap ^3.7.4` [VERIFIED: npm pack of 0.42.9 package.json] |
| `starlight-llms-txt` | `0.10.0` (exact) | `0.12.0` (exact) | peers `astro ^7.0.0`, starlight `>=0.41.0`; 0.10.0 peers `astro ^6.0.0` so it MUST move [VERIFIED: npm view] |
| `@astrojs/markdown-remark` | not a direct dep | `^7.3.2` (resolved 7.3.5) | NEW explicit dep: required for `unified()` / legacy remark keys on Astro 7 [VERIFIED: astro `validate.js` throws without it] |
| `@astrojs/sitemap` | `3.7.2` (exact) | `3.7.4` (exact) | Starlight 0.42.9 depends on `^3.7.4`; keep the exact-pin convention, avoid two copies [VERIFIED: npm view] |
| `starlight-github-alerts` | — | `0.5.0` (exact) | peer starlight `>=0.42.0`; deps `satteri ^0.10.3`, `unist-util-visit` [VERIFIED: npm pack] |
| `starlight-links-validator` | — | `0.27.0` (exact) | peers `astro >=7.2.10`, starlight `>=0.42.0`; deps incl. `@astrojs/markdown-satteri`, `satteri`, `picomatch` [VERIFIED: npm pack] |

### Supporting

| Library | Current | Target | Note |
|---------|---------|--------|------|
| `@astrojs/rss` | `^4.0.19` | unchanged (4.0.19 latest, no `astro` peer) | `feed.xml.ts` built fine in trial |
| `@astrojs/check` | `^0.9.10` | `^0.9.12` | `astro check` is not wired into any workflow [VERIFIED: grep of `.github/workflows`, `lefthook.yml`]; not exercised in trial |
| `@playwright/test` | `^1.63.0` (installed 1.63.0) | leave the lockfile pin alone | A fresh resolve picks 1.64.1; installed browsers match 1.63. Update only the targeted packages, do not `pnpm up` everything |
| `sharp`, `github-slugger`, `gray-matter`, `typescript`, `wrangler` | unchanged | unchanged | Trial resolved sharp 0.35.5, typescript 6.0.3 without issue |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `unified()` processor + our two remark plugins | Port `remark-vox-include` / `remark-doc-links` to Sätteri plugins | Both plugins read mdast `link`/`definition`/`html`/`image`/`code` nodes and call `fs`; a port is real work with no reader benefit. Out of scope; keep `unified()`. Both new plugins support both processors. |
| Legacy `markdown.remarkPlugins` (still works, folded into `unified()` with a console deprecation warning) | Explicit `markdown.processor: unified({ remarkPlugins })` | Recommended: identical output (trial 3 vs 4), no warning, survives Astro 8. |
| `exclude` function for MDX false positives | Add `slug:` frontmatter to the 3 `.mdx` files; or `vite.resolve.preserveSymlinks` | `slug:` cannot fix the home page (`index.mdx` id is `""`) and touches content; `preserveSymlinks` breaks pnpm's `.pnpm` virtual-store resolution. Not recommended. |

**Installation (in `docs-astro/`, not run in the real tree by research):**
```bash
pnpm add astro@^7.4.7 @astrojs/starlight@^0.42.9 @astrojs/markdown-remark@^7.3.2 \
  @astrojs/sitemap@3.7.4 @astrojs/check@^0.9.12 starlight-llms-txt@0.12.0
pnpm add starlight-github-alerts@0.5.0 starlight-links-validator@0.27.0 --save-exact
```
Trial `pnpm install` (pnpm 12.16.1) finished with exit 0 and **no `ERR_PNPM_IGNORED_BUILDS`**, so `allowBuilds` in `pnpm-workspace.yaml` needs no new entries. It did auto-append `minimumReleaseAgeExclude` for `@astrojs/internal-helpers@0.13.1`, `@astrojs/markdown-satteri@0.4.7`, `astro@7.4.7` (published within the release-age window); commit those lines or pick versions old enough not to need them. CI uses pnpm 11 with `--frozen-lockfile`; a pnpm-12-written lockfile reading cleanly under pnpm 11 is `[ASSUMED]` (both `lockfileVersion: '9.0'`) and is confirmed by the first docs-quality run.

## Package Legitimacy Audit

Seam: `gsd-tools query package-legitimacy check --ecosystem npm ...` (run this session). The seam's `too-new` signal keys on the *latest publish date*, not package age; every flagged package is a long-lived first-party or established-author package with a source repo and no postinstall.

| Package | Registry | Age | Weekly downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|------------------|-------------|---------|-------------|
| `starlight-links-validator` | npm | created 2023-06-23 | 273,582 | github.com/HiDeoo/starlight-links-validator | SUS (too-new: latest publish 2026-10-09) | Flagged. Named by CONTEXT.md as locked. Planner adds one `checkpoint:human-verify` before install. |
| `starlight-github-alerts` | npm | created 2025-08-07 | 6,552 | github.com/HiDeoo/starlight-github-alerts | OK | Approved (same maintainer as validator; Starlight core-team member). |
| `starlight-llms-txt` | npm | long-lived | 292,496 | github.com/delucis/starlight-llms-txt | OK | Approved |
| `@astrojs/starlight` | npm | long-lived | 2,174,238 | github.com/withastro/starlight | SUS (too-new: 0.42.9 published 2026-11-04) | Flagged for recency only; official Astro package. Covered by the same checkpoint. |
| `astro` | npm | long-lived | 11,182,144 | github.com/withastro/astro | SUS (too-new: 7.4.7 published 2026-11-05) | Same; consider a one-patch-older pin to avoid `minimumReleaseAgeExclude`. |
| `@astrojs/markdown-remark` | npm | long-lived | 5,532,536 | github.com/withastro/astro | SUS (too-new: 7.3.5 published 2026-11-05) | Same |
| `@astrojs/check` | npm | long-lived | 5,937,137 | github.com/withastro/astro | SUS (too-new: 0.9.12 published 2026-10-21) | Same |
| `@astrojs/sitemap` | npm | long-lived | 5,439,451 | github.com/withastro/astro | OK | Approved |
| `@astrojs/rss` | npm | long-lived | 1,241,166 | github.com/withastro/astro | OK | Approved |
| `satteri` (transitive) | npm | — | — | github.com/bruits/satteri | not seam-checked | `npm view` shows no `scripts.postinstall`; trial install needed no build allowance. |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** `starlight-links-validator`, `@astrojs/starlight`, `astro`, `@astrojs/markdown-remark`, `@astrojs/check` (all recency-only). The planner should add a single `checkpoint:human-verify` ahead of the Plan-B install that lists these with the registry facts above. No package has a postinstall script that touches the network [VERIFIED: seam `postinstall: null` for the seven it reported; `npm view` for `satteri`, `@astrojs/markdown-satteri`].

## Breaking Changes: Astro 6 -> 7 and Starlight 0.38 -> 0.42, mapped to this repo

Sources: [CITED: docs.astro.build/en/guides/upgrade-to/v7/], [CITED: github.com/withastro/starlight packages/starlight/CHANGELOG.md, 0.39.0-0.42.9], [CITED: github.com/withastro/starlight releases `@astrojs/starlight@0.41.0`].

| Change | Applies here? | Evidence / action |
|--------|---------------|-------------------|
| Starlight 0.41: Astro v7 supported, v6 dropped; Chromium <111 / Safari <16.4 dropped | Yes (version move) | Done by the bump. |
| Starlight 0.42: requires Astro >=7.2.10; **package now ships compiled `dist/`** (`exports` `./components` -> `./dist/components.ts`, `./components/*` -> `./dist/components/*`) | **Yes, breaks our alias shim** | `astro.config.mjs` aliases point at `node_modules/@astrojs/starlight/components.ts` (no longer exists). Re-point using `import.meta.resolve` (Pitfall 2). |
| Starlight 0.42: mobile menu markup simplified (`<starlight-menu-button>` and `aria-expanded` removed, `data-mobile-menu-expanded` removed); `tagline` option removed | No | `grep` of `reader-layout.css`, `tests/*.spec.ts`, the 5 overrides finds none of `menu-button`, `mobile-menu`, `aria-expanded` on Starlight markup, `tagline`. Our own `aria-expanded` is on our own edge-toggle buttons. [VERIFIED: grep] |
| Starlight 0.40/0.39: Sätteri processor added; sidebar `autogenerate` must be wrapped in `items`; autogenerated subgroup `collapsed` default changed | No | `sidebar` comes from `getSidebar()` in `src/utils/sidebar.mjs`, no `autogenerate` usage (build identical, sidebar-driven specs identical). |
| Astro 7: **Sätteri is the default Markdown processor; `@astrojs/markdown-remark` no longer installed by default; `markdown.remarkPlugins` deprecated but folded into `unified()`** | **Yes** | Add `@astrojs/markdown-remark`; use `markdown.processor: unified({ remarkPlugins: [...] })`. Astro throws "run on the `unified` processor from `@astrojs/markdown-remark`..." if the package is missing [VERIFIED: `node_modules/astro/dist/core/config/validate.js` in trial]. |
| Astro 7: Rust compiler; unclosed tags / invalid nesting now errors | Potentially (`.astro` files) | Trial build compiled every `.astro` file in `docs-astro/src` (`Banner`, `Sidebar`, `PageSidebar`, `TableOfContents`, `SiteTitle`, `VoxPlayground` 1099 lines, `VoxLeaderboardArena`, `pages/retired.astro`) with no error. |
| Astro 7: `compressHTML` default `'jsx'` | Possibly cosmetic | `reader-layout`/`trust-signals`/`smoke`/`baseline` specs identical to baseline; no action. Do not set `compressHTML: true` unless a visual diff appears. |
| Astro 7: Vite 8 / Rolldown | Yes | Custom `vite.resolve.alias` still honored (Rolldown error text proves the alias layer is consulted). `@docs-src` alias works. |
| Astro 7: `src/fetch.ts` reserved | No | `src/fetch.*` absent [VERIFIED: `ls`]. |
| Astro 7: experimental flags removed, `@astrojs/db` removed, `astro:transitions` internals removed | No | None used. |
| Astro 7: `import { z } from 'astro:content'` deprecated ("removed in Astro 8; use `astro/zod`") | Yes (`src/content.config.ts`) | Still works. Recommend switching to `import { z } from 'astro/zod'` (one line; trial built with the old import). [CITED: `node_modules/astro/types/content.d.ts`] |
| Astro 7: **`astro preview`/`astro dev` auto-background when run by an AI agent** (`isRunByAgent()`), unless `--ignore-lock` | **Yes, for agent executors** | `playwright.config.ts` `webServer.command: 'pnpm preview --port 4321'` fails locally in an agent shell with "Process from config.webServer exited early". Fix: `pnpm preview --port 4321 --ignore-lock` (verified: stays foreground, prints `astro v7.4.7 ready`). CI (GitHub Actions) is unaffected either way. [VERIFIED: `cli/preview/index.js`, ran it] |
| Astro 7: Node >=22.12.0 | No | CI uses node 24; local node v26.13.0. |

### The five overrides and `routeData.ts`

All unchanged and working on 0.42.9 [VERIFIED: trial build + Playwright parity]:
- `Banner.astro` imports `Default from '@astrojs/starlight/components/Banner.astro'`; 0.42 `Banner.astro` still reads `Astro.locals.starlightRoute.entry.data.banner` and renders `.sl-banner`.
- `PageSidebar.astro` wraps `Default`; 0.42 `PageSidebar` now renders `MobileTableOfContents` + `TableOfContents` via `virtual:starlight/components/...` (wrapper-compatible).
- `Sidebar.astro`, `TableOfContents.astro`, `SiteTitle.astro` (replacement, no Default): `import type { Props } from '@astrojs/starlight/props'` still resolves.
- CSS hooks used by `reader-layout.css` all still exist in 0.42 components: `.sidebar-pane`, `.sidebar-content` (PageFrame), `.right-sidebar-container`, `.main-pane` (TwoColumnContent), `.right-sidebar-panel`, `.sl-container` (PageSidebar), `header.header` (PageFrame/Header). [VERIFIED: grep of `dist/components`]
- `routeData.ts`: `defineRouteMiddleware` from `@astrojs/starlight/route-data`, `route.entry.data`, `route.entry.filePath`, `route.lastUpdated`, `route.editUrl`, `route.head` all behave the same (`dates.spec` passes: 904/906 pages carry `<time datetime>`, identical to baseline).
- `content.config.ts`: `docsLoader()` / `docsSchema({ extend })` unchanged.
- `starlight-llms-txt` patch: `patches/starlight-llms-txt@0.10.0.patch` applies **cleanly** to 0.12.0 (upstream 0.12.0 still lacks `exclude` in `llms-custom.txt.ts` and `llms-full.txt.ts`; only `llms-small.txt.ts` and `generator.ts` have it). Rename the file to `starlight-llms-txt@0.12.0.patch` and change the `patchedDependencies` key in `docs-astro/pnpm-workspace.yaml` to `starlight-llms-txt@0.12.0`. `llms.spec` (Internals exclusion) passed in the trial. [VERIFIED: trial]

## Architecture Patterns

### System Architecture Diagram

```
docs/src/**/*.md|mdx  ──(setup-content.mjs: per-entry symlink mirror, archive/.well-known/SUMMARY.md excluded)──┐
repo mounts (README/AGENTS/.planning/... via contracts/documentation/site-mounted-repo-docs.v1.json) ──────────┤
                                                                                                              v
                                     docs-astro/src/content/docs/  (symlinks; file.path = mirror path for .md,
                                                                    REAL docs/src path for .mdx via Vite realpath)
                                                                                                              |
   docsLoader() ──> Astro unified() processor
                      remarkVoxInclude -> remarkDocLinks   (our plugins; rewrite .md -> /slug/, code files -> GitHub blob)
                      [starlight-github-alerts remark, injected BEFORE Starlight's remarkDirective]
                      Starlight remark/rehype (asides, EC code blocks)
                      [starlight-links-validator rehype collector: records ids, headings, links per file]
                                                                                                              |
   Starlight routes + route middleware (routeData.ts: dates, banner, noindex) ──> static HTML (907 pages)
                                                                                                              |
   astro:build:done ──> starlight-links-validator validateLinks() ──> exit 1 on any finding (failOnError: true)
                       sitemap / pagefind / llms*.txt / feed.xml
                                                                                                              |
   CI: docs-quality.yml (PR gate) and docs-deploy.yml (main) both run `pnpm build`; lychee offline check runs on dist afterwards
```

### Recommended file layout for the changes

```
docs-astro/
├── package.json                    # bumps + 3 new deps
├── pnpm-lock.yaml                  # regenerated
├── pnpm-workspace.yaml             # patchedDependencies key -> 0.12.0 (+ minimumReleaseAgeExclude if kept)
├── patches/starlight-llms-txt@0.12.0.patch   # git mv from @0.10.0, content unchanged
├── astro.config.mjs                # unified() processor, alias fix, two plugins
├── playwright.config.ts            # webServer: --ignore-lock
├── src/utils/links-validator-exclude.mjs     # NEW: pure predicate, unit-tested
└── tests/unit/links-validator-exclude.test.mjs   # NEW
.github/dependabot.yml              # delete the two `ignore:` entries (their own comments order it)
crates/vox-doc-pipeline/src/pipeline/openwiki.rs   # link only when the api page exists (+ test)
docs/src/architecture/subsystems/l5-binaries.generated.md   # regenerated by `vox ci wiki-sync --write`, never hand-edited
docs/src/architecture/t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md   # 4 `..` -> 3 `..`
```

### Pattern 1: explicit `unified()` processor (trial variants 4, 5)

```js
// Source: astro 7 core/config/validate.js (coerceLegacyMarkdownPlugins) + trial build 4/5
import { unified } from '@astrojs/markdown-remark';

export default defineConfig({
  markdown: {
    processor: unified({
      remarkPlugins: [
        remarkVoxInclude,
        [remarkDocLinks, { repoUrl: 'https://github.com/vox-foundation/vox' }],
      ],
    }),
  },
  // ...
});
```
Both new plugins mutate `astroConfig.markdown.processor.options` in their `astro:config:setup` hook; they recognise `name === 'unified'`. `starlight-github-alerts` additionally requires Starlight's `remarkDirective` entry to be present in `options.remarkPlugins` (Starlight adds it) and silently no-ops otherwise, so a regression shows up as "alerts not rendered", which Plan C's test must catch.

### Pattern 2: alias shim that survives Starlight's layout (trial variants 3-5)

```js
// docs/src/**/*.mdx sit outside docs-astro/, so a bare `@astrojs/starlight/components`
// import cannot walk up to docs-astro/node_modules. Resolve through the package exports.
{
  find: /^@astrojs\/starlight\/components$/,
  replacement: new URL(import.meta.resolve('@astrojs/starlight/components')).pathname,
},
```
Only the bare `components` alias is needed. The `components/*` alias (second shim) is not used by any `.mdx` (all three `.mdx` import only `@astrojs/starlight/components`; the `.astro` overrides live under `docs-astro/` and resolve normally). Trial variants 3-5 ran without it. Remove it.

### Pattern 3: links-validator configuration that produces a zero-error baseline (trial 5)

```js
// Source: starlight-links-validator@0.27.0 libs/validation.js + trial build 5.
// Why these three clauses (the ONLY 12 findings on the default config):
//  - the 3 .mdx pages (index, tutorials/tut-fullstack-walkthrough, community/leaderboard) are
//    symlinked from docs/src; Vite realpaths .mdx imports, so file.history[0] is
//    docs/src/...mdx, normalizeId cannot derive their route, and 11 links TO or WITHIN them
//    (`/`, `/#try-it`, `#try-it`, `/tutorials/tut-fullstack-walkthrough/`) report invalid.
//  - env-vars.generated.md:261 documents http://127.0.0.1:11434 (local link, real finding, intentional).
starlightLinksValidator({
  exclude: ({ file, link }) => {
    const path = link.split(/[?#]/)[0] || '/';
    const target = path === '/' ? '' : path.replace(/^\/|\/$/g, '');
    return (
      file.endsWith('.mdx') ||
      docPages.some((p) => p.relPath.endsWith('.mdx') && p.id === target) ||
      link.startsWith('http://127.0.0.1:11434')
    );
  },
}),
```
Prescription: put this predicate in `docs-astro/src/utils/links-validator-exclude.mjs` as a factory taking `docPages` (from the existing `listDocPages`), unit-test it with `node:test` (the repo's unit-test style), and import it in the config. `docPages[].id` for `index.mdx` is `""` and for the others the route slug [VERIFIED: ran `docSlug` on the three paths]. Use `failOnError` default (`true`).

### Anti-Patterns to Avoid

- **Starting validator in `failOnError: false` "to see what happens":** unnecessary here; the full finding set is already known (12) and the exclude brings it to 0. A warn-only gate would be a gate nobody reads.
- **`errorOnLocalLinks: false` / `errorOnInvalidHashes: false` / `errorOnRelativeLinks: false` globally:** hides future regressions; the findings do not need them (trial 4 used defaults and still had only the 12).
- **Hand-editing `l5-binaries.generated.md`:** AGENTS.md forbids it; it would be reverted by the next `wiki-sync` and the `ssot-drift` gate.
- **`vite.resolve.preserveSymlinks`:** breaks pnpm's virtual-store dependency resolution.
- **Running Playwright locally with the stock `webServer` command in an agent shell:** fails on Astro 7 (auto-background); use `--ignore-lock` or a manual `astro preview --ignore-lock` plus `BASE_URL`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| GitHub alert syntax -> aside | A third remark plugin | `starlight-github-alerts@0.5.0` | Handles processor ordering vs `remarkDirective`, path gating (`shouldTransformPath`), Sätteri + unified. |
| Internal link/hash validation | A custom post-build HTML crawler | `starlight-links-validator@0.27.0` | Knows redirects, custom pages, hashes, trailing slash policy; mutation-proven here. (Existing lychee offline check remains as the independent post-build net.) |
| Resolving the Starlight components path | Hard-coded `node_modules/.../dist/components.ts` | `import.meta.resolve('@astrojs/starlight/components')` | The package layout already changed once (source -> `dist/`). |
| Excluding `.mdx` route identity gaps | Per-link allow-list of 11 URLs | One predicate keyed on `listDocPages` `.mdx` ids | Self-maintaining when a 4th `.mdx` appears. |

**Key insight:** every piece of this phase is "wire up an existing, maintained mechanism"; the only custom code is a ~10-line predicate and a 3-line generator guard.

## The two existing `remark-doc-links` failures (DOCS-PARITY-04) - exact fixes

### Failure 1: `t3-vox-wave4-gap-closure-and-live-pipeline-audit-2026.md`

- File lives at `docs/src/architecture/`; every code link uses `../../../../crates/...` (4 ups). From `docs/src/architecture/` 3 ups reach the repo root (`architecture` -> `src` -> `docs` -> repo); 4 ups land in `~/dev/`, hence `resolves outside repository root: '/Users/brbrainerd/dev/crates/...'` [VERIFIED: build log + Read of lines 45-151].
- **Many** links are affected, not one: `grep` finds the 4-up form on lines 48-50, 60, 72, 73, 87, 88, 101, 115, 127, 128, 139, 140, 149, 151 and more. The build reported only the first because the plugin throws on the first bad link in a file.
- Fix: replace every `](../../../../crates/` with `](../../../crates/` (and any other `../../../../` link into the repo; the trial `sed` of `(../../../../crates/` -> `(../../../crates/` left zero 4-up links and the page built and validated). Hand-authored doc (not generated), so a direct edit is correct. It is an existing `docs/src` page; keep its frontmatter untouched (doc-lint rules apply only to new/substantially authored docs).

### Failure 2: `subsystems/l5-binaries.generated.md` -> `../../api/vox-cargo-shim.generated.md`

- Root cause is a **generator mismatch**, not a missing file: the hub generator `generate_subsystem_hubs` (`crates/vox-doc-pipeline/src/pipeline/openwiki.rs:366`) lists every crate from `docs/src/architecture/layers.toml` and emits `| [`{name}`](../../api/{name}.generated.md) | ...` (lines 414-417), while the API pages come from `vox ci crate-api-sync` (`crates/vox-cli-ci/src/crate_api_docs.rs:188-206`), which iterates `cargo metadata ... workspace_packages()`. `vox-cargo-shim` is in `[workspace] exclude` of the root `Cargo.toml` and is in `layers.toml` (`vox-cargo-shim = { layer = 5, kind = "binary", ... }`), so it has a hub row but no API page. Exactly one link is missing across all seven hubs [VERIFIED: loop over every `../../api/*.generated.md` link in `subsystems/*.generated.md` against `docs/src/api/`].
- Fix at the generator (test-first, per AGENTS.md Test-First Policy): in `generate_subsystem_hubs`, emit the link only if `repo_root/docs/src/api/{name}.generated.md` is a file, else emit plain `` `{name}` ``. Add a unit test in the existing `#[cfg(test)] mod tests` of `openwiki.rs` that builds a temp dir with one present and one absent api page (std `env::temp_dir()`; `vox-doc-pipeline` has no dev-dependencies) and asserts linked vs plain. Extract the row formatting into a small private fn so the test needs no `layers.toml`. Then regenerate with `cargo run -p vox-cli -- ci wiki-sync --write` (verify path: `write_or_verify_hub` error text and `docs.rs:721-725` use this command; the repo's `ssot-drift` job checks drift and `ssot-autoregen` regenerates on PRs). Run cargo in the background per the "never block on cargo" memory; verify only `cargo check -p vox-doc-pipeline` + the new test locally (hosted CI is the gate).
- Rejected alternative: teach `crate_api_docs.rs` to also document excluded workspace members. Larger surface, needs manifest discovery outside `cargo metadata`, and would publish an API page for a crate whose `src/main.rs` shape is not exercised by that generator.

## Starlight-links-validator on this site: sizing and options

- **Estimate: 12 findings on defaults, 1 real.** Measured, not estimated (trial 4). Breakdown: `repo/planning-requirements-md.md:186` `/`; `architecture/starlight-site-strategy-2026.md:39,112` `/`; `tutorials/tut-first-app.md:15`, `tut-getting-started.md:79,183`, `reference/ref-syntax.md:14`, `docs/src/index.mdx:199` -> `/tutorials/tut-fullstack-walkthrough/`; `tut-getting-started.md:36` `/#try-it`; `docs/src/index.mdx:28,45` `#try-it` (invalid hash); `reference/env-vars.generated.md:261` `http://127.0.0.1:11434` (local link).
- All 11 non-local findings: target pages DO exist in `dist` (`dist/index.html` contains `id="try-it"`; `dist/tutorials/tut-fullstack-walkthrough/` exists). Cause: validator derives a page id from `file.history[0]` relative to `<srcDir>/content/docs`; `.md` files keep the mirror path, `.mdx` files get the real `docs/src/...` path (report shows `../../docs/src/index.mdx`). [VERIFIED: `libs/rehype.js` `normalizeId`, build output]
- **Why zero relative-link noise:** `remark-doc-links` rewrites every in-docs `.md/.mdx` link to a site-absolute `/slug/` and code/dir links to `https://github.com/.../blob|tree/main/...` before the validator's rehype pass, so `errorOnRelativeLinks` (default `true`) found nothing (trial 4 = trial 3).
- **Options recommended:** defaults for everything (`errorOnRelativeLinks`, `errorOnLocalLinks`, `errorOnInvalidHashes`, `errorOnFallbackPages`, `sameSitePolicy: 'ignore'`, `failOnError: true`) plus the `exclude` predicate above. `reporters` default `{ githubActions: true, json: false }` is fine (writes a job summary on CI). The validator is `command === 'build'` only (dev server unaffected) and clears the content-layer cache each build (hence content sync always re-runs; the 2-6 min build time is the same cost the baseline already paid, 245 s).
- **Warn-first?** No. With the predicate the baseline is 0, mutation probe proves it fails on a bad page and on a bad hash, and docs-quality already requires green `pnpm build`. Risk: `docs-deploy.yml` also builds, so a future broken link would block a deploy on `main`; the PR gate catches it first, and `failOnError: false` is the single-flag escape hatch.
- **What it does not validate:** links to/from `.mdx` pages (excluded), `https://github.com/...` code links (external; lychee nightly + `links.spec.ts` GitHub-path-exists test cover those), external sites.
- **Nightly Docs Link Checker (#578)** is lychee (`.github/workflows/link_checker.yml`, cron `0 6 * * *`); the validator does not feed it mechanically. The PR-time offline lychee step in `docs-quality.yml` stays and is complementary.

## Common Pitfalls

### Pitfall 1: The two link failures turn fatal on Astro 7
**What goes wrong:** `pnpm build` exits 1 at content sync. On Astro 6 it logged the same errors and still produced the page. (Trial 1.)
**Why:** `starlight-docs-loader` render errors for those two files are no longer swallowed.
**How to avoid:** land Failure 1 and Failure 2 fixes first (Plan A) on the current tree; they are valid on Astro 6 too.
**Warning signs:** `[ERROR] [starlight-docs-loader] Error rendering ...` followed by `[ELIFECYCLE]`.

### Pitfall 2: Alias shim silently stale after Starlight 0.42
**What goes wrong:** `Rolldown failed to resolve import "@astrojs/starlight/components" from docs/src/tutorials/tut-fullstack-walkthrough.mdx` (trial 2).
**Why:** `.mdx` files outside `docs-astro/` cannot resolve the bare specifier; the old alias target `node_modules/@astrojs/starlight/components.ts` was removed when 0.42 moved to `dist/`. Dropping the alias is NOT an option.
**How to avoid:** Pattern 2 (`import.meta.resolve`), remove the unused `components/*` alias.
**Warning signs:** build dies in "Building static entrypoints" within ~15 s.

### Pitfall 3: Forgetting `@astrojs/markdown-remark`
**What goes wrong:** Astro throws "run on the `unified` processor ... no longer installed by default".
**How to avoid:** direct dependency (listed above).

### Pitfall 4: Patch key left at `starlight-llms-txt@0.10.0`
**What goes wrong:** pnpm warns/errs about an unmatched patch and `llms-full.txt` stops excluding Internals (`llms.spec.ts` fails).
**How to avoid:** `git mv` the patch file, update the `patchedDependencies` key; re-run `llms.spec.ts`.

### Pitfall 5: Deleting dependabot ignores but not both
**What goes wrong:** `.github/dependabot.yml` lines 149-163 carry two `ignore:` entries (`@astrojs/starlight`, `starlight-llms-txt`) whose comments say "Remove both together as part of that upgrade" (they pin dependabot to astro 6). Leaving them blocks all future Starlight/llms-txt bumps. [VERIFIED: Read]
**How to avoid:** delete both entries and the explanatory comment block in the same PR as the bump.

### Pitfall 6: Local Playwright in an agent shell (Astro 7 auto-background preview)
See Breaking Changes table. Use `--ignore-lock`; never `kill` by name (repo rule: no pattern-kill; stop with `astro preview stop`, or by PID).

### Pitfall 7: Uncommitted edits sit exactly on the lines this phase changes
`git status` shows `docs-astro/astro.config.mjs`, all 4 overrides, `reader-layout.css`, `remark-doc-links.mjs`, `remark-vox-include.mjs`, `SiteTitle.astro` (untracked) etc. as modified by other work, including the alias shims themselves (the shims are not in HEAD; `git diff HEAD` shows them as additions). Plans must start from a working tree where that work is committed (or explicitly build on it) so the executor's diff to `astro.config.mjs` is reviewable.

### Pitfall 8: Pre-existing red tests mistaken for regressions
See "Pre-existing baseline failures". The plan's verify step should compare to the recorded baseline counts (27/2, 33/18/1, unit 4 -> 3 after rebuild), not demand zero failures. Optionally a separate task fixes the stale ones, but they are not in this phase's requirements.

## Runtime State Inventory

Not a rename/migration phase. State check anyway: no stored data, live-service config, OS-registered state, or secrets reference the bumped package names; build artifacts to refresh are `docs-astro/dist` (rebuild), `docs-astro/node_modules` (reinstall), and the lockfile. `docs-astro/src/content/docs` is regenerated by `prebuild`.

## Code Examples

### GitHub-alerts plugin registration (trial 3-5)
```js
// Source: starlight-github-alerts@0.5.0 index.ts (read from npm pack); no options required.
import starlightGitHubAlerts from 'starlight-github-alerts';
// inside starlight({ plugins: [ starlightGitHubAlerts(), starlightLinksValidator({...}), starlightLlmsTxt({...}) ] })
```
Default mapping [VERIFIED: `libs/config.ts` read]: `caution` -> `danger`, `important` -> `caution`, `note` -> `note`, `tip` -> `tip`, `warning` -> `caution`. `IMPORTANT` and `WARNING` render identically by default; override with `starlightGitHubAlerts({ types: { important: 'note' } })` only if authors want them distinguishable (not needed for parity).

### Playwright webServer for Astro 7
```ts
// docs-astro/playwright.config.ts
webServer: process.env.BASE_URL ? undefined : {
  command: 'pnpm preview --port 4321 --ignore-lock',
  port: 4321,
  reuseExistingServer: !process.env.CI,
  timeout: 60000,
},
```

### Validator unit test shape (node:test, matches `tests/unit/*.test.mjs` style)
Assert the factory returns `true` for `({file:'/x/docs/src/index.mdx', link:'#try-it'})`, for `/tutorials/tut-fullstack-walkthrough/` and `/#try-it` when that route is a `.mdx` id, for `http://127.0.0.1:11434`; and `false` for `/reference/installation/#x` from a `.md` file.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `markdown.remarkPlugins` | `markdown.processor: unified({ remarkPlugins })` (Sätteri default) | Astro 7 | Needs `@astrojs/markdown-remark`; legacy keys still folded in with a warning. |
| Starlight ships TS source | Starlight ships compiled `dist/` | 0.42.0 | Path-based aliases into `node_modules/@astrojs/starlight/*.ts` break. |
| Mobile menu via `<starlight-menu-button>` + JS | Popover-based, JS-free | 0.42.0 | Only matters to custom CSS/overrides that target it (none here). |
| `astro preview` always foreground | Auto-background in agent shells | Astro 7 | Playwright `webServer` needs `--ignore-lock` for agent runs. |
| `z` from `astro:content` | `z` from `astro/zod` | deprecated in 7, removed in 8 | One-line import change. |

**Deprecated/outdated:** `import { z } from 'astro:content'`; legacy `markdown.remarkPlugins/rehypePlugins/remarkRehype` keys; the `starlight-llms-txt@0.10.0` patch filename/key.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A lockfile written by local pnpm 12.16.1 installs cleanly with `--frozen-lockfile` under CI's pnpm 11 (both `lockfileVersion '9.0'`) | Standard Stack | CI install step fails; fix by regenerating the lock with pnpm 11 (`pnpm dlx pnpm@11 install`). First docs-quality run reveals it. |
| A2 | `minimumReleaseAge` gating exists in CI's pnpm 11 and would reject `astro@7.4.7` etc. unless excluded | Standard Stack | Only matters if re-resolution occurs in CI; `--frozen-lockfile` does not re-resolve, so low risk. |
| A3 | Changing `import { z } from 'astro:content'` to `'astro/zod'` compiles in `content.config.ts` | State of the Art | Trivial revert; trial built with the old import, plugins use `astro/zod` internally. |
| A4 | `@astrojs/check` 0.9.12 works with Astro 7 (`astro check` not exercised, not in CI) | Standard Stack | Only affects `pnpm check` locally. |
| A5 | The roadmap's "24 docs" count: grep finds 26 non-archive `docs/src` files (+ mounted root `README.md`) with alert blockquotes; archive (30 files) is excluded from the site | Phase scope | None functionally; the plan should assert "no `[!TYPE]` blockquote remains outside code" rather than a number. |

## Open Questions

1. **Commit state of the uncommitted docs-astro work.**
   - Known: ~17 docs-astro files (including `astro.config.mjs` alias shims, `SiteTitle.astro`, `remark-doc-links.mjs`) are modified/untracked by other sessions.
   - Unclear: who commits them before Plan B edits the same file.
   - Recommendation: Plan A (or a Wave-0 task) must begin by confirming/committing the DOCS-PARITY-01 work and the other docs-astro edits, or the planner scopes Plan B's diff to hunks on top of the working tree and says so.

2. **Baseline-red tests: fix here or leave?** (4 unit, 2 build-output, 18 trust-signals.) Recommendation: leave; verify by baseline diff. The `workflow-paths` ones (`.planning/*` mounted but absent from workflow path filters) are cheap to fix (add paths to both workflows or add `.planning/` to `never_mount_prefixes`) but belong to the docs-freshness/mounts owner, not DOCS-PARITY.

3. **Pin `astro` one patch behind latest to avoid `minimumReleaseAgeExclude` entries?** Recommendation: accept the 3 auto-added exclude lines (they are pnpm's own output) unless the maintainers dislike them.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Node | build, tests | yes | v26.13.0 local; CI node 24; Astro needs `>=22.12.0` | — |
| pnpm | install | yes | 12.16.1 local; CI pins 11 | `pnpm dlx pnpm@11` for lock regeneration |
| Network to registry | install | yes | npm view/pack/install worked | — |
| Playwright chromium | specs | yes | browsers 1208-1248 cached; 1.63 and 1.64.1 both ran | — |
| cargo (via build broker) | `vox ci wiki-sync --write`, `cargo test -p vox-doc-pipeline` | not exercised in research | — | Run backgrounded; hosted CI is the gate |
| `git` | `getGitDates` (feed.xml refuses an empty feed) | yes | — | Trial needed a `git init` copy |

**Missing dependencies with no fallback:** none.

## Validation Architecture

(`workflow.nyquist_validation` is absent from `.planning/config.json`, so enabled.)

### Test Framework
| Property | Value |
|----------|-------|
| Unit | `node --test` (built-in), files `docs-astro/tests/unit/*.test.mjs` |
| E2E / build-output | Playwright `@playwright/test` (`docs-astro/playwright.config.ts`, chromium project) |
| Rust | `cargo test -p vox-doc-pipeline` (hub generator) |
| Quick run | `cd docs-astro && node --test tests/unit/links-validator-exclude.test.mjs` (new) |
| Full suite | `cd docs-astro && pnpm build && node --test "tests/unit/*.test.mjs" && BASE_URL=http://localhost:4321 npx playwright test tests/build-output/ tests/reader-layout.spec.ts tests/trust-signals.spec.ts tests/smoke.spec.ts tests/baseline.spec.ts` (start `pnpm preview --port 4321 --ignore-lock` first when running as an agent) |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| PARITY-02 | build exits 0 on astro 7 / starlight 0.42 with 907 pages | build | `cd docs-astro && pnpm build` (grep `page(s) built`) | n/a |
| PARITY-02 | no behavioural regression | e2e | Playwright commands above; compare to baseline counts (27/2, 33/18/1) | existing specs |
| PARITY-02 | llms patch still excludes Internals | e2e | `npx playwright test tests/build-output/llms.spec.ts` | yes |
| PARITY-03 | alert blockquotes became asides; none left | build-output | NEW `tests/build-output/alerts.spec.ts`: for a fixed sample doc using each of NOTE/TIP/IMPORTANT/WARNING/CAUTION assert `starlight-aside` count; assert no built HTML (outside `<code>`/`<pre>`) has a `<blockquote>` whose first paragraph starts `[!`. Baseline: 1 page has asides, trial: 29. | Wave 0 gap |
| PARITY-04 | validator passes with 0 findings | build | `pnpm build` exit 0 | n/a |
| PARITY-04 | validator actually fails on a bad link | mutation | Temporarily append `[x](/reference/no-such-page-zzz/)` to one `docs/src` page, `pnpm build` must exit 1 with `invalid link`, then revert (guard with a before/after presence check per AGENTS.md mutation-proof rule) | manual step in plan |
| PARITY-04 | exclude predicate | unit | `node --test tests/unit/links-validator-exclude.test.mjs` | Wave 0 gap |
| PARITY-04 | hub generator links only existing api pages | unit (Rust) | `cargo test -p vox-doc-pipeline openwiki` (background) | Wave 0 gap (add test first) |
| PARITY-04 | generated hub in sync | drift | `cargo run -p vox-cli -- ci wiki-sync` (verify mode) / `vox ci ssot-drift` | existing |

### Sampling Rate
- **Per task commit:** unit test for the touched predicate / `cargo check -p vox-doc-pipeline` + new test only (repo rule: hosted CI is the gate).
- **Per wave merge:** one full `pnpm build` (budget 2-6 min; run backgrounded) + unit + build-output specs.
- **Phase gate:** full suite above, baseline-diffed, plus the validator mutation probe, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `docs-astro/tests/unit/links-validator-exclude.test.mjs` + `src/utils/links-validator-exclude.mjs`
- [ ] `docs-astro/tests/build-output/alerts.spec.ts`
- [ ] Rust unit test in `crates/vox-doc-pipeline/src/pipeline/openwiki.rs` for linked-vs-plain crate cell (write the failing test first)

## Security Domain

`security_enforcement` is absent from config, so applicable. This is a static documentation build; the relevant surface is supply chain and build integrity, not runtime auth.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication / V3 Session / V4 Access Control | no | Static site, no auth surface in this phase |
| V5 Input Validation | limited | Plugin configs validated by their own zod schemas; `exclude` predicate is pure and only reads build-time link strings |
| V6 Cryptography | no | None added (AGENTS.md crypto policy untouched) |
| V14 Configuration / dependency integrity | yes | Exact-pin the two community plugins and `starlight-llms-txt`; commit lockfile; `--frozen-lockfile` in CI; keep `pnpm-workspace.yaml` `allowBuilds` deny of `workerd` (deploy job holds the Cloudflare token); confirm no new lifecycle scripts (trial install: no `ERR_PNPM_IGNORED_BUILDS`) |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Typosquatted / slopsquatted package | Spoofing | Both new plugin names come from CONTEXT.md (locked) and are confirmed by maintainer (HiDeoo, Starlight team) + repo URL; human-verify checkpoint before install |
| Malicious postinstall in transitive deps (Sätteri native bits) | Tampering | pnpm 11 blocks dep build scripts unless allowed; trial needed none |
| Dependabot pinned to stale majors (the two `ignore:` entries) | Information disclosure (unpatched CVEs) | Remove both entries with the upgrade |
| Build-time plugin runs arbitrary code in the deploy job | Elevation of privilege | Existing split: `deploy-cloudflare` is a separate job; `workerd` build denied; do not widen `allowBuilds` |

## Project Constraints (from CLAUDE.md / AGENTS.md)

- **Fix generators, not generated files** (`docs/src/**/*.generated.md`); regenerate with the owning command (`vox ci wiki-sync --write`); do not hand-regenerate SSOT after merge (`ssot-autoregen` bot).
- **Test-First Policy:** a new `pub fn` in `crates/*/src/**` needs a test in the same file; write the failing test first (the openwiki helper stays private but test it anyway).
- **Hosted CI is the gate; verify only what you touched locally:** `cargo check -p`, `cargo nextest run -p`/`cargo test -p`; never workspace clippy/tests locally; never block a foreground tool call on cargo (background it); route cargo through the build broker.
- **Never pattern-kill** processes (`pkill -f`); stop Astro preview with `astro preview stop` or by PID.
- **VoxLang-first glue:** no new `.sh`/`.py`/`.ps1`. The new `src/utils/*.mjs` helper and `*.spec.ts` follow the existing docs-astro convention (the house "TypeScript only" rule is for TS repos; docs-astro's utilities/plugins are `.mjs` by established practice and the Playwright specs are `.ts`).
- **Doc authoring rules:** any NEW `docs/src/**` page needs frontmatter + `kb:summary`; this phase edits existing pages only (t3 audit doc, regenerated hub), so keep their frontmatter intact. Do not touch `docs/src/archive/`.
- **Dependency discipline / crates:** no new workspace crate edges (JS deps only; the Rust change is inside `vox-doc-pipeline`).
- **Work reaches `main` only via PR;** batch commits, one push; no committing secrets; commit trailer per session attribution.
- **Do not store project research in IDE memory;** this file lives under `.planning/`; if durable findings should outlive the phase, add an `docs/src/architecture/*-findings-2026.md` update (the existing `docs-site-asciidoc-parity-findings-2026.md` is the SSOT) when the phase closes.

## Sources

### Primary (HIGH confidence)
- Executed trial builds 1-6 and Playwright/unit runs in `/private/tmp/claude-501/-Users-brbrainerd-dev-vox/403f33b2-86be-4c98-b83c-a149ee9e1b8a/scratchpad/trial/vox` (logs `build1.log`..`build6.log`)
- `npm view` / `npm pack` this session: `astro@7.4.7`, `@astrojs/starlight@0.42.9` (package.json, exports, dist layout), `starlight-llms-txt@0.10.0/0.11.0/0.12.0`, `starlight-links-validator@0.27.0` (dist source read: `index.js`, `libs/validation.js`, `libs/rehype.js`, `libs/store.js`, `libs/processor.js`, `middleware.js`), `starlight-github-alerts@0.5.0` (`index.ts`, `libs/*.ts`, CHANGELOG)
- Installed `astro@7.4.7` sources in trial: `core/config/validate.js`, `cli/preview/index.js`, `types/content.d.ts`
- Repo files read this session: `docs-astro/astro.config.mjs`, `package.json`, `pnpm-workspace.yaml`, `patches/*.patch`, `src/routeData.ts`, `src/content.config.ts`, the five `src/components/*.astro` overrides, `src/plugins/remark-doc-links.mjs`, `scripts/setup-content.mjs`, `.github/workflows/docs-quality.yml`, `docs-deploy.yml`, `link_checker.yml`, `.github/dependabot.yml`, `crates/vox-doc-pipeline/src/pipeline/openwiki.rs`, `crates/vox-cli-ci/src/crate_api_docs.rs`

### Secondary (MEDIUM confidence)
- [CITED: docs.astro.build/en/guides/upgrade-to/v7/] Astro 7 upgrade guide
- [CITED: github.com/withastro/starlight packages/starlight/CHANGELOG.md] 0.39.0-0.42.9 entries
- [CITED: github.com/withastro/starlight releases/tag/@astrojs/starlight@0.41.0]
- [CITED: starlight-links-validator.vercel.app/configuration/] option table (cross-checked against the 0.27.0 schema/behaviour above)

### Tertiary (LOW confidence)
- None relied on.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, versions from `npm view` and a green install/build with exactly these versions.
- Architecture: HIGH, every recommended config variant was built; mutation probe run.
- Pitfalls: HIGH, each pitfall (1, 2) was reproduced as a real failure before being fixed; 5 and 6 verified by reading/running.
- Residual MEDIUM: pnpm 11 vs 12 lockfile interop (A1) and `@astrojs/check`.

**Research date:** 2026-11-05
**Valid until:** ~2026-11-19 (Astro/Starlight patch releases are frequent; re-run `npm view` for exact pins at plan time)
