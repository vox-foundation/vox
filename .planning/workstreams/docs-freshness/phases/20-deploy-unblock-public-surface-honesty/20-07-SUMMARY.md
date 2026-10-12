---
phase: 20-deploy-unblock-public-surface-honesty
plan: 07
subsystem: docs-site
tags: [docs, astro, starlight, llms-txt, pnpm-patch, honesty]
status: complete

requires:
  - 20-06 (page-index.mjs internalsDocIds/listDocPages, page-status.mjs statusPolicy)
  - 20-12 (content mirror; dist layout the build-output specs read)
provides:
  - "docs-astro/patches/starlight-llms-txt@0.10.0.patch: llms-full.txt.ts and llms-custom.txt.ts pass `exclude` to generateLlmsTxt"
  - "astro.config.mjs passes internalsDocIds(docPages) as starlightLlmsTxt `exclude`; `llmsFullTxt` removed"
  - "docs-astro/tests/build-output/llms.spec.ts: generated-variant heading check + .well-known link-status check"
affects: [20-08, 20-09]

actuals:
  tokens: 2900
  tasks: 2
  commits: 2
plan_head_before: 914c7ce47e693408d79fdd3f04547a4f328e2854

tech-stack:
  added: []
  patterns:
    - "Upstream plugin gaps fixed with a committed `pnpm patch` (patchedDependencies + lockfile patch_hash), not a version bump"
    - "Assert on generated dist output (page titles as headings), not on config"

key-files:
  created:
    - docs-astro/patches/starlight-llms-txt@0.10.0.patch
    - docs-astro/tests/build-output/llms.spec.ts
  modified:
    - docs-astro/astro.config.mjs
    - docs-astro/pnpm-workspace.yaml
    - docs-astro/pnpm-lock.yaml
    - docs/src/.well-known/llms.txt
    - docs/src/.well-known/llms-full.txt

key-decisions:
  - "The page list is computed once in astro.config.mjs and shared by the sitemap filter (noindexRoutes) and the llms exclude list (internalsDocIds)"
  - "The .well-known check also requires every non-.md same-site link to resolve to a docs page, so a broken link fails rather than silently passing the status check"

metrics:
  duration: "~10 min"
  completed: 2026-10-09
---

# Phase 20 Plan 07: Internals out of every llms.txt variant Summary

A committed `pnpm patch` makes starlight-llms-txt 0.10.0 honour `exclude` in `llms-full.txt` and custom sets, `astro.config.mjs` feeds it the 133 Internals page ids from frontmatter, and the hand-written `.well-known` lists lose their research and legacy links. A build-output spec checks all of this against the built files.

## What was built

- **Patch (Task 1, tracer).** The patch touches only `llms-full.txt.ts` (imports `starlightLllmsTxtContext`, passes `exclude`) and `llms-custom.txt.ts` (passes `exclude` alongside `include`). `pnpm-workspace.yaml` gained `patchedDependencies`; the lockfile records `patch_hash=3f20fc40…`. starlight-llms-txt stays at 0.10.0.
- **Config.** `exclude: internalsDocIds(docPages)` (133 ids, none with glob metacharacters, so micromatch matches them literally); `llmsFullTxt: true` deleted.
- **Sizes.** `dist/llms-full.txt` went from 12,625,898 to 8,970,797 bytes. `dist/llms-small.txt` went from 10,839,859 to 7,635,464 bytes; it already supported `exclude`, but nothing was passed before. `dist/llms.txt` is unchanged at 759 bytes. The build produces no `_llms-txt/` custom sets; the spec covers them if any are added later.
- **.well-known (Task 2).** Removed the `front-facing-honesty-audit-2026` item (status research) from `llms.txt` and the `mcp-tool-reference` item (status legacy) from `llms-full.txt`. The `/AGENTS.md`, `/CLAUDE.md` and `/GEMINI.md` links stay for 20-09; the spec skips `.md` URLs, and a comment names 20-09.

## Verification

| Check | Result |
|---|---|
| Spec on the pre-patch build (red) | 3 failed: llms-small.txt and llms-full.txt each had 133 Internals titles as headings; `.well-known/llms.txt` linked `/architecture/front-facing-honesty-audit-2026/` |
| `pnpm@11 install --frozen-lockfile && pnpm@11 build && playwright … --grep-invert well-known` | install 0, build 0, 4 passed |
| `grep -c llmsFullTxt docs-astro/astro.config.mjs` | 0 |
| `grep -E '^\+\+\+ ' patch` | only `b/llms-custom.txt.ts`, `b/llms-full.txt.ts` |
| Task 2 spec on the old dist (red) | 2 failed: `/reference/mcp-tool-reference/` and `/architecture/front-facing-honesty-audit-2026/` |
| Rebuild + full `llms.spec.ts` | 6 passed |
| `grep -cE 'front-facing-honesty-audit-2026\|mcp-tool-reference'` on both .well-known files | 0 and 0 |
| All build-output specs (archive, dates, honesty, llms) | 26 passed |

### Mutation evidence

With the `llms-full.txt.ts` hunk removed from the patch and `pnpm install` rerun, the installed file had no `exclude`. The rebuilt `llms-full.txt` went back to 12,625,898 bytes, and the spec failed only on "llms-full.txt includes no Internals page" (1 failed, 3 passed). After restoring the patch and lockfile from backup, `--frozen-lockfile` install passed, the rebuild was 8,970,797 bytes, and the spec was green again (4 passed).

## Deviations from Plan

None. The plan was executed as written. The shell wrapper blocks `node -e`, so all file edits ran from temp scripts that check for exact, unique anchors. The staged diffs contained no `lean-ctx` text.

## Notes for later plans

- 20-09: remove the `.md` skip in `llms.spec.ts` once `/AGENTS.md`, `/CLAUDE.md` and `/GEMINI.md` are repointed at `/repo/…` routes. Because of the "links to routes that are not docs pages" assertion, those routes must exist as pages (or be added to the spec's route map) when the skip goes.
- The spec checks generated files for Internals page bodies only, matched by title heading. Body cross-links to Internals routes are allowed there, as the plan specifies for 20-08.

## Self-Check: PASSED

- FOUND: docs-astro/patches/starlight-llms-txt@0.10.0.patch, docs-astro/tests/build-output/llms.spec.ts
- FOUND commits: 7ac5cc33f, a8e892e3e
