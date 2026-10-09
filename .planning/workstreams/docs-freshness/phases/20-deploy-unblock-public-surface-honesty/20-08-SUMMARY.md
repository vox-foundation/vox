---
phase: 20-deploy-unblock-public-surface-honesty
plan: 08
subsystem: docs-site
tags: [docs, astro, starlight, remark, links, honesty]
status: complete

requires:
  - 20-05 (dead relative targets fixed; source-link-targets guard)
  - 20-06 (doc-slug.mjs docSlug)
  - 20-07 (astro.config.mjs llms wiring; llms.spec)
  - 20-12 (content mirror of per-entry symlinks; archive not built)
provides:
  - "docs-astro/src/plugins/remark-doc-links.mjs: pure resolveDocLink(url, fromFile, ctx) + remarkDocLinks(options) remark plugin + docLinksGate() Astro integration"
  - "Relative links between docs pages render as Starlight routes (docSlug); out-of-tree / non-Markdown / archive / superpowers targets render as GitHub blob/tree URLs with :N / :N-M -> #LN / #LN-LM"
  - "Dead or repo-escaping relative links on docs/src pages fail `astro build` (exit 1) naming the source file and URL"
  - "docs-astro/tests/build-output/links.spec.ts: whole-site zero relative .md hrefs, no href above the site root, every GitHub blob/tree link names a repo path"
affects: [20-09, 20-10]

actuals:
  tokens: 4900
  tasks: 3
  commits: 5
plan_head_before: d7b3029d81c5bfe562dfab98a0330cfadc2a3ae7

tech-stack:
  added: []
  patterns:
    - "Remark errors are swallowed by Astro's content loader, so build-failing checks record into module state and throw from an integration's astro:build:done hook"
    - "Resolve links from realpath(file.path), since the content dir is a mirror of symlinks into docs/src"

key-files:
  created:
    - docs-astro/src/plugins/remark-doc-links.mjs
    - docs-astro/tests/unit/remark-doc-links.test.mjs
    - docs-astro/tests/build-output/links.spec.ts
  modified:
    - docs-astro/astro.config.mjs
    - docs-astro/tests/unit/source-link-targets.test.mjs
    - docs/src/architecture/plugin-system-redesign-sp1-plan-2026.md
    - docs/src/explanation/expl-architecture.md

key-decisions:
  - "docLinksGate() fails the build at astro:build:done: a remark plugin's throw is only logged by starlight-docs-loader and the build exits 0"
  - "A docs/src directory with an index page links to its route; other directories go to tree/main"
  - "Markdown inside docs/src but not a page (MIRROR_EXCLUDED top-level entries, _-prefixed files) goes to blob, reusing setup-content.mjs MIRROR_EXCLUDED"
  - "Definitions used by an imageReference are never rewritten (a blob URL would break the image)"
  - "href attributes inside raw HTML nodes are rewritten too; JSX in .mdx is not (index.mdx uses site-absolute links only)"

metrics:
  duration: "~25 min"
  completed: 2026-10-09
---

# Phase 20 Plan 08: Repo-relative link rewrite Summary

A remark plugin now rewrites every relative link at build time. Links between docs pages become Starlight routes, and everything else in the repository becomes a GitHub blob/tree URL. A dead or repo-escaping link fails `astro build`, and a whole-site spec proves the built HTML has no relative `.md` href and no GitHub link to a missing path.

## Rewrite counts

Measured by running `resolveDocLink` over the inline, reference and raw-HTML links of the 670 live pages, with code stripped:

| Destination | Links |
|---|---|
| Site route (in-docs page) | 2,280 |
| GitHub `blob/main` | 1,875 |
| GitHub `tree/main` | 393 |
| **GitHub total** | **2,268** |
| Left untouched (external, `#anchor`, site-absolute, mailto) | 1,097 |

Pages with at least one rewrite: 459. The built site carries 2,296 GitHub blob/tree hrefs, which includes hand-written absolute GitHub URLs. All of them name a path that exists.

## Task commits

| Task | Commit | What |
|---|---|---|
| 1 (deviation) | 32596816e | Nested ```` ```markdown ```` fence in the plugin-system plan widened to 4 backticks; guard fence-closing rule fixed |
| 1 | fc4651f59 | `remarkDocLinks` + `resolveDocLink` for in-docs pages, `docLinksGate`, config registration, tutorial spec |
| 2 | f37a0f12b | GitHub blob/tree for out-of-tree, non-Markdown, archive, superpowers; `#LN` / `#LN-LM`; repo-escape throw; image definitions skipped |
| 3 (deviation) | 31e43440c | Raw-HTML `href` rewriting; 3 dead hand-written GitHub links in `expl-architecture.md` fixed |
| 3 | 0d7c66a25 | Whole-site `links.spec.ts` assertions |

## Verification

- `node --test tests/unit/remark-doc-links.test.mjs`: 24/24 pass. Full unit suite: 726/726 pass, including the 671-page source-link guard.
- Fresh build (data store cleared, `dist` removed): exit 0, no `remark-doc-links` errors, `test ! -e dist/archive` holds.
- `npx playwright test tests/build-output/links.spec.ts tests/build-output/llms.spec.ts`: 10/10 pass. All of `tests/build-output`: 30/30 pass.
- Mutations, each confirmed then restored:
  - A tutorial link pointed at `ref-decorators-nope.md` makes the build exit 1 with `dead link '../reference/ref-decorators-nope.md' in …/docs/src/tutorials/tut-getting-started.md`.
  - Removing the repo-escape check fails the escape unit test.
  - Injecting a relative `.md` href, an above-root href and a missing-path GitHub link into one built page fails all three whole-site assertions.
  - Restoring the old nested fence fails the source-link guard on that page.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A thrown remark error did not fail the build**
- **Found during:** Task 1, first build
- **Issue:** `starlight-docs-loader` catches the plugin's error, logs `Error rendering …`, and `astro build` still exits 0. The plan's "missing target fails the build" truth did not hold.
- **Fix:** The plugin records every dead link in module state and still throws per file. `docLinksGate()`, registered in `integrations`, throws at `astro:build:done` when any were recorded. A unit test covers the gate.
- **Commit:** fc4651f59

**2. [Rule 1 - Bug] Dead link hidden by a nested fence; guard disagreed with CommonMark**
- **Found during:** Task 1, first build
- **Issue:** `plugin-system-redesign-sp1-plan-2026.md` wrapped a ```` ```bash ```` block in a 3-backtick ```` ```markdown ```` fence. CommonMark closes the outer fence only at a bare fence line, so line 1946 rendered as a live link to a missing `architecture/distribution-bundles.generated.md`. The 20-05 guard treated ```` ```bash ```` as a closer and missed it.
- **Fix:** The outer fence is now 4 backticks. The guard closes a fence only on a line with no info string, and it has a unit case for this.
- **Commit:** 32596816e

**3. [Rule 2 - Missing] Raw HTML links and dead hand-written GitHub links**
- **Found during:** Task 3, first whole-site run
- **Issue:** `expl-architecture.md` links three ADRs as relative `.md` inside a raw HTML `<a href>`, which is not an mdast link. It also had 5 absolute GitHub links (3 distinct targets) to paths that no longer exist.
- **Fix:** The plugin rewrites `href="…"` in `html` nodes, with a unit test. Content fixes: `rules.v1.yaml` now links `contracts/code-audit/rules.v1.yaml`, and `vox-exec-grammar` now links `vox-container-types/src/exec_grammar/` (per where-things-live.md). The retired `vox-protocol` link was removed and its text kept.
- **Commit:** 31e43440c

**4. [Verify harness] Stale content cache**
- After a plugin-source change, Astro's `node_modules/.astro/data-store.json` reuses previously rendered Markdown. The first Task 3 run therefore saw Task 1 output (only 32 GitHub links). Builds used for verification deleted the data store first. This is logged in `deferred-items.md`. CI is unaffected because it builds from a clean checkout.

**5. [Spec scope] Tutorial `.md` check limited to relative hrefs**
- The Starlight "Edit page" link is an absolute GitHub URL ending in `.md`. The spec counts relative hrefs only, as the plan's truth states.

### Deferred (logged in deferred-items.md)

- Every "Edit page" URL is malformed (`…/edit/main/docs/src/src/content/docs/…`). This predates 20-08.
- A raw-HTML `<img src="../assets/…webp">` in `expl-architecture.md` 404s. It is an image, not a link.
- The Astro data-store cache issue described in deviation 4.

## Known Stubs

None.

## Threat Flags

None. Targets are only `existsSync`/`statSync`'d after the repo-root containment check. Only the fixed `repoUrl` from config is ever prefixed.

## Self-Check: PASSED

- FOUND: docs-astro/src/plugins/remark-doc-links.mjs, docs-astro/tests/unit/remark-doc-links.test.mjs, docs-astro/tests/build-output/links.spec.ts
- FOUND commits: 32596816e, fc4651f59, f37a0f12b, 31e43440c, 0d7c66a25 (`git rev-list --count d7b3029d8..HEAD` = 5)
