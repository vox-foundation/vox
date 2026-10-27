---
phase: 89-docs-astro-integration
plan: 03
subsystem: docs-astro-diagrams
tags: [vox-diagram, astro, remark, build-guard, css, llms]

requires:
  - phase: 89-docs-astro-integration
    provides: "89-01 pre-render manifest, remark-vox-diagram plugin, inline figure"
provides:
  - "fail-closed miss path: strict throw, shared miss registry, assertNoMisses on astro:build:start/done, dev placeholder"
  - "closed-grammar svgToHast proven against every committed golden and a hostile list; Node half of the fence-key parity"
  - "vox-diagram.css: --vd-* to --sl-* theme map, viewBox-capped sizing with 0.6 scroll floor, forced-colors, placeholder"
  - "diagrams.spec.ts: fs-only built-site guarantees; where-things-live row"
affects: [89-04]

actuals:
  tokens: 7500
  tasks: 3
  commits: 4
plan_head_before: 2571c2cd5eb6d045487bbef31e58bfcb8bc2134f
plan_head_after: f8637d78fa64f008af362268b3169c6a6759f068
commits: 4

key-files:
  created:
    - docs-astro/src/styles/vox-diagram.css
    - docs-astro/tests/unit/render-diagrams.test.mjs
    - docs-astro/tests/build-output/diagrams.spec.ts
  modified:
    - docs-astro/src/plugins/remark-vox-diagram.mjs
    - docs-astro/astro.config.mjs
    - docs-astro/scripts/render-diagrams.mjs
    - docs-astro/tests/unit/remark-vox-diagram.test.mjs
    - docs/src/architecture/where-things-live.md

key-decisions:
  - "Dev refresh uses refreshContent() in astro:server:setup, not deleting Astro's data store (the planned D-16 mechanism was wrong, see Deviations)"
  - "customSelectors for svg.vd-diagram is kept as planned although llms-full.txt is already free of SVG text without it (mutation-proven, see below)"

requirements-completed: [DOCS-03]

duration: resumed after a rate-limit death; Tasks 2-3 plus evidence re-runs
completed: 2026-10-26
status: complete
---

# Phase 89 Plan 03: Fail-closed diagram figures Summary

**A missing diagram render now fails `astro build` for md and mdx (naming the file and `pnpm diagrams`) and only shows a placeholder in dev; the SVG crosses the manifest boundary through a fail-closed parser, figures are themed and sized, and the built site is checked from disk.**

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 (tracer) | `cb29dd79b` | feat(89-03): fail the build on a missing diagram render, placeholder in dev |
| 2 | `92b9363bd` | feat(89-03): harden the diagram plugin and clear the dev content store |
| 2 (fix) | `71c21c5b4` | fix(89-03): refresh dev content when the diagram manifest changes |
| 3 | `f8637d78f` | feat(89-03): theme and size diagram figures, retain llms source, guard the built site |

## Negative controls (Task 1 gate; re-run in this session, the earlier logs were lost with the dead executor)

- **md, manifest removed, `astro build` (no prebuild): exit 1.** `Error: Failed to parse Markdown file ".../reference/vox-diagram.md": remark-vox-diagram: docs/src/reference/vox-diagram.md fence 1 has no pre-rendered SVG. Run: pnpm diagrams (...)`, then `[ERROR] [vox-diagram] An unhandled error occurred while running the "astro:build:start" hook` listing `docs/src/reference/vox-diagram.md fence 1` and the command. The remark throw alone does not fail md; the guard hook does.
- **Guard mutation (both hooks removed, grep count 0, manifest removed AND `.astro/data-store.json` deleted so the miss is really encountered): exit 0**, `844 page(s) built`, log still contains the `no pre-rendered SVG` error line twice. Guards restored from the committed file (`git checkout`, grep count 2, `git diff --quiet` clean). A first mutation run without clearing the store also exited 0 but logged no miss (cached store), so it proved nothing; the second run is the evidence.
- **mdx (manifest restored, fence appended to `docs/src/index.mdx`, `astro build`): exit 1**: `[@mdx-js/rollup] remark-vox-diagram: docs/src/index.mdx fence 1 has no pre-rendered SVG. Run: pnpm diagrams (...)` with `file: .../docs/src/index.mdx`. `git checkout -- docs/src/index.mdx`; `git diff --quiet b176936db -- docs/src/index.mdx docs-astro/pnpm-lock.yaml` exits 0. Manifest regenerated afterwards.

## Verification

- `node --test tests/unit/remark-vox-diagram.test.mjs tests/unit/render-diagrams.test.mjs`: 14 tests, 14 pass (plan floor was 13).
- `pnpm --dir docs-astro build`: exit 0, 844 pages, 0 `Error while highlighting code block using language "vox-diagram"` lines.
- `playwright test tests/build-output/`: 34 passed, 1 skipped, 1 failed. The 6 new `diagrams.spec.ts` tests all pass. The failure is `llms.spec.ts` "curated .well-known/llms.txt links point only to verified reader-facing pages (TRUST-02)": six curated reference pages are page-status `broken`. It does not touch files this plan changed (see Pre-existing failures).
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md`: no hard errors.
- `pnpm-lock.yaml` unchanged vs `b176936db`; no dependency added.
- **Dev loop (evidence, final mechanism):** dev server on 4399; base reference page 2 figures; after changing the first fence label, 1 figure + 1 `vd-missing` placeholder and the server keeps running; after `pnpm diagrams` the server restarted ("Astro config changed / Clearing content store") and the page returned to 2 figures with no placeholder. Fence reverted, manifest regenerated, `git status --porcelain docs/src` empty.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The planned dev data-store clearing (D-16) was wrong in two ways**
- **Found during:** Task 2 dev-loop check
- **Issue:** (a) Astro 6.1.9 keeps the store at `docs-astro/.astro/data-store.json`; the plan, 89-RESEARCH Pitfall 3 and the first draft of `clearStaleStore` pointed at `node_modules/.astro/`, so the CLI never deleted anything. (b) After pointing it at the right file, deleting the store under a running dev server left the `docs` collection empty and every page 404 until a cold restart, because a dev restart (triggered by the watched manifest) does not re-sync content. Without deletion, md pages stayed stale (the 89-RESEARCH A4 risk).
- **Fix:** `astro.config.mjs` integration adds `'astro:server:setup': ({ refreshContent }) => refreshContent?.({})`. The manifest digest is already in the plugin options, which is part of Astro's `astro-config-digest`; a sync after a restart then logs "Astro config changed" and rebuilds the store. The wrapper no longer deletes anything from the CLI path; `clearStaleStore` stays exported and unit-tested (plan acceptance) with its correct default path and a comment explaining why it is not wired.
- **Files modified:** docs-astro/astro.config.mjs, docs-astro/scripts/render-diagrams.mjs
- **Commit:** `71c21c5b4`

### Observations

- **`customSelectors: { all: ['svg.vd-diagram'] }` is not observable at this version.** Mutation: removed it, rebuilt, ran `diagrams.spec.ts`: 6/6 still pass, and `llms-full.txt` has no `<desc>` or label text either way (the HTML-to-Markdown conversion already drops the `svg`). It is kept because the plan requires it and it states intent; the spec guards the outcome (fence present, desc absent), not the mechanism. Config restored and rebuilt afterwards.
- `llms-full.txt` shows the fence source with doubled blank lines (Expressive Code line divs), the same as every code block in that file.
- The built figure nests Expressive Code's own `<figure class="frame">` inside `details`, so the spec matches `<figure class="vd-figure"` up to `</details></figure>` and identifies the source block by `data-language="vox-diagram"` (the `language-vox-diagram` class exists only in the llms conversion).

## Pre-existing failures (not caused by this plan, not fixed)

- `tests/unit/tutorial-record.test.mjs`: tutorial verification record hash mismatch for `tut-workflow-durability.md` (the tutorial changed in `109601d37`, unrelated to this plan).
- `tests/unit/doc-slug.test.mjs` "cross-checks against built dist": failed only while `dist/` was a partial build left by a failed negative-control run; it needs a full `pnpm build` first.
- `tests/build-output/llms.spec.ts` TRUST-02 page-status check (above). I did not run these against the base commit to prove they predate this plan; the evidence is that none of them read a file this plan touched.

## Known Stubs

None. The `vd-missing` placeholder is the intentional dev-only fallback (build fails instead).

## Threat Flags

None beyond the plan's threat model. T-89-06 (parser, hostile list), T-89-07 (guards, mutation-proven), T-89-08 (viewBox width regex) and T-89-09 (Mermaid selectors, fs spec) are covered by tests above.

## Self-Check: PASSED

`vox-diagram.css`, `diagrams.spec.ts`, `render-diagrams.test.mjs` exist; commits `cb29dd79b`, `92b9363bd`, `71c21c5b4`, `f8637d78f` are ancestors of HEAD; `grep -c 'assertNoMisses()' astro.config.mjs` prints 2.
