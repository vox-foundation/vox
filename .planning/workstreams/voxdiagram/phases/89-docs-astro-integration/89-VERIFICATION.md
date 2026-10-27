---
phase: 89-docs-astro-integration
verified: 2026-10-27T06:50:00Z
status: passed
score: 6/6 must-haves verified
covered_files: [".github/workflows/docs-deploy.yml",".github/workflows/docs-quality.yml",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-01-PLAN.md",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-01-SUMMARY.md",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-02-PLAN.md",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-02-SUMMARY.md",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-03-PLAN.md",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-03-SUMMARY.md",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-04-PLAN.md",".planning/workstreams/voxdiagram/phases/89-docs-astro-integration/89-04-SUMMARY.md","crates/vox-diagram/src/layout/dagre_impl.rs","crates/vox-diagram/src/layout/finish.rs","crates/vox-diagram/src/layout/input.rs","crates/vox-diagram/src/layout/tests/containers.rs","crates/vox-diagram/src/svg/mod.rs","crates/vox-doc-pipeline/src/pipeline/render_manifest.rs","docs-astro/astro.config.mjs","docs-astro/package.json","docs-astro/scripts/render-diagrams.mjs","docs-astro/src/plugins/remark-vox-diagram.mjs","docs-astro/src/plugins/vox-diagram-integration.mjs","docs-astro/src/styles/vox-diagram.css","docs-astro/tests/build-output/diagrams.spec.ts","docs-astro/tests/lib/diagrams.ts","docs-astro/tests/unit/vox-diagram-rebuild.test.mjs","docs-astro/tests/vox-diagram.spec.ts"]
covered_digest: "v3:sha256:dc0672c6d08f8232d5167b7d5588a035cda5159646d4c536a7e27a593cf78b88"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Run docs-quality on hosted CI (push the branch or open a draft PR) and confirm the 'Pre-render VoxDiagram fences', 'Build Starlight', 'Install Playwright Chromium and test fonts' and 'VoxDiagram browser checks' steps go green within the 30-minute cap"
    expected: "All four steps pass on ubuntu. The apt install of fonts-dejavu-core and fonts-liberation runs without a preceding apt-get update. vox-diagram.spec.ts passes, including the wide (DejaVu Sans) fit stack and the Arimo webfont test."
    why_human: "The branch has no upstream and no docs-quality run exists for it. These steps have never run on a hosted runner."
  - test: "Run docs-deploy on hosted CI after merge, or as a dry run"
    expected: "The pre-render step and the prebuild hook both succeed. The built site serves /fonts/arimo/*.woff2 and the six reference figures."
    why_human: "This is a deploy workflow and can only run on hosted CI."
  - test: "Read the Linux font-fit numbers from the first docs-quality log (the 'fits wide' line, which reports the widest fill ratio)"
    expected: "Every text fits under real DejaVu Sans and Liberation Sans. Locally only Verdana and Arial stood in."
    why_human: "These metrics can only be measured on the CI image."
  - test: "Optional: view the reference page under Windows High Contrast (a dark theme)"
    expected: "Diagrams stay legible."
    why_human: "Forced colours were checked only under Playwright emulation, with a white Canvas."
---

# Phase 89: Docs-Astro Integration Verification Report

**Phase goal:** voxlang.org builds and shows every `vox-diagram` fence as an inline, accessible, themed SVG, and the build fails rather than ship a missing render.

**Verified:** 2026-10-27
**Status:** human_needed
**Re-verification:** No. This is the first verification. The code review and its fix report (89-REVIEW, 89-REVIEW-FIX) came before it.
**Scope:** HEAD `56592855d`. This includes the self-hosted Arimo commit.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: `pnpm build` pre-renders every fence into a gitignored manifest. The remark plugin inlines each SVG in `.md` and `.mdx`. A fence with no render fails the build, and the message names the file and the exact command. | VERIFIED | `prebuild` runs `render-diagrams.mjs` (package.json:9), and `.vox-diagram/` is gitignored. The manifest has 6 entries, one for each of the 6 fences in `docs/src` outside the archive; `dist/reference/vox-diagram/index.html` has 6 `vd-figure`s. **md:** `vox-diagram-rebuild.test.mjs` re-run here and passed. Two real `astro build` runs both fail with "page.md fence 1". **mdx:** my own probe builds a one-page MDX project with `@astrojs/mdx` and no manifest. It fails twice with `[@mdx-js/rollup] remark-vox-diagram: .../page.mdx fence 1 has no pre-rendered SVG. Run: pnpm diagrams (cargo run -p vox-doc-pipeline -- --render-diagrams ...)`. |
| 2 | WR-01: a rebuild that would reuse a cached failed md render still fails | VERIFIED | Mutation probe: a copy of `vox-diagram-integration.mjs` with the `data-store.json` drop removed. Build 1 fails and build 2 **passes**, so the guard carries the weight and the regression test would catch its removal. With the real integration both builds fail. |
| 3 | WR-05: mounted repo docs are pre-rendered | VERIFIED | `render-diagrams.mjs` passes `--mount` for every path `discoverMounts` returns. The Rust `render_manifest` tests passed 11/11 this session (mount render, attribution, path guard). A miss on a mounted page names its `mounted_from` (remark-vox-diagram.mjs:111). Today no mounted file contains a fence. |
| 4 | SC2 / DOCS-04: Playwright against the built site finds no duplicate ids on a page with several diagrams, every `<text>` inside its box under two font stacks, and contrast passing in light and dark. Forced colours stay legible. Fixtures cover both kinds. | VERIFIED (local Chromium on macOS) | `vox-diagram.spec.ts` passed 16/16 against the build after 56592855d (established this session). The duplicate-id scan in dist is empty: 18 ids, 6 figures on one page, ids carry page-scoped prefixes from `reprefix`. The reference route is asserted to have both graph and sequence figures. **Would the checks fail on broken output?** Contrast: the pale-palette and container-fill self-tests run on every pass. Fit: the title-strike self-test runs every pass, and the 89-04 Courier mutation gave 20 overflows. The `fonts: wide stack ≥1.05×` probe stops the two stacks from silently becoming one font. Duplicate ids: no self-test, but the check is a plain `Set` over `[id]`, so any repeated `vd-title` is caught. Linux metrics are still to be confirmed (see Human Verification). |
| 5 | SC3: docs-deploy and docs-quality run the pre-render before `pnpm build` and trigger on `crates/vox-diagram/**` | VERIFIED (static) | docs-deploy runs the pre-render at line 97 and the build at line 114, with a `crates/vox-diagram/**` path filter. docs-quality runs the pre-render at line 152 and the build at line 157, with filters on both push and pull_request, followed by the Chromium/fonts install and the VoxDiagram browser step. `workflow-paths.test.mjs` passed 9/9 when re-run here. No hosted run exists yet (see Human Verification). |
| 6 | Carried Phase 86 by-eye review, plus 56592855d (Arimo self-hosted, metric-identical fallbacks) | VERIFIED | Notes on the PNGs follow this table. Arimo: `dist/fonts/arimo/` has 4 woff2 files and OFL.txt, the built CSS contains `Vox Arimo`, and the manifest SVGs carry `Arimo,Arial,Liberation Sans,...` in all 6 entries. The spec test checks that the face loads, cyrillic is not fetched, and the fallback width is within 0.5%. Diagram SVGs use no bold or italic, so the single 400-weight face covers everything. |

**Score:** 6/6 truths verified (0 present-but-behavior-unverified)

#### What the review-bundle PNGs show (regenerated 23:34, after the Arimo build)

- **01 light/dark natural:** a left-to-right graph with an Orchestrator container and a cylinder. Text is crisp and the themes are correct. The external "Desktop Client" sits below the container instead of to its left: the title column takes rank 0. Its edge climbs diagonally through the frame, and the "TLS" label sits just inside the frame. Legible, but awkward.
- **02 light/dark natural (sequence):** actors, a solid call, a note on Gateway and a dashed reply. Clean in both themes. The carried Phase 86 review passes.
- **04 light/dark natural (stress nested):** 1284×536, against 1696×1482 before. Titles sit top-left, nothing overlaps, and the `emit` edge is clear of the Operations title (D1 fixed). There is a large empty area in Platform services and Core runtime. Browser is pushed below the outer frame, as in 01. The edge-label patches don't match the container fill (IN-06, cosmetic).
- **05 light/dark natural:** accented Latin and CJK all render, wrap inside their boxes, and nothing is clipped.
- **06 light/dark natural (wide sequence):** the multi-hop labels sit in their own lifeline gaps ("Cancel the reservation..." between Gateway and Billing, "Send the receipt..." between Billing and Stock). The note, the self-loop and the dashed reply are clean. The carried multi-hop check passes.

#### Left-to-right title column: judgement

These are Chromium renders of the goldens before (`07b66a573^`) and after:

- **21-nested** is 1805×382.5 after, against 1147×1247 before. It is clearly better: frames hug their content and titles sit top-left. The empty container "Nothing here yet" now looks like a plain node, which is acceptable. Verdict: acceptable.
- **Corpus 03** (axis-harness-river, three left-to-right containers) has regressed:
  - Before, it was a clean three-column layout.
  - After, it is a diagonal staircase, and 4 to 5 edges run in near-parallel vertical bundles that are hard to trace to their targets.
  - The 16-cell side-title wrap hard-breaks `HarnessRiverPanel` into `HarnessRiverPane` / `l (Dockable +`. The golden tspan confirms it.
- **Corpus 06** is top-down and unaffected.

**Verdict: a defect to fix before Phase 90.** It does not block Phase 89: the site's 6 figures render correctly and pass every check. But Phase 90 will migrate left-to-right subgraphs, and some titles there contain words longer than 16 cells (HarnessRiverPanel, EnforcementLayers, Inbound_discovery_planned). Minimum fix:

- never hard-break a title word that would fit the normal wrap width. Widen the side column to the longest word, or fall back to the top band.
- check corpus 03 by eye again.

The external-source-below-container pattern in 01 and 04 is a smaller issue. It is worth fixing in the same change, for example by not putting the title node in the rank of external predecessors.

### Required Artifacts

| Artifact | Status | Details |
|---|---|---|
| `crates/vox-doc-pipeline/src/pipeline/render_manifest.rs` | VERIFIED | `--render-diagrams` with `--mount`; tests 11/11 |
| `docs-astro/scripts/render-diagrams.mjs` | VERIFIED | cwd = repo root, passes mounts, exits 1 in build mode and 0 in dev |
| `docs-astro/src/plugins/remark-vox-diagram.mjs` | VERIFIED | Fail-closed tag-allowlist parser, id re-prefix, strict throw, miss registry, page-unique titles |
| `docs-astro/src/plugins/vox-diagram-integration.mjs` | VERIFIED | Wired in `astro.config.mjs:32`; `assertNoMisses` on build start and done; pending marker and store drop |
| `docs-astro/src/styles/vox-diagram.css` | VERIFIED | Theme map, viewBox cap with a 0.6 floor, forced colours, Arimo @font-face; registered in `customCss` |
| `docs-astro/tests/vox-diagram.spec.ts`, `tests/lib/diagrams.ts` | VERIFIED | Routes are discovered from `dist/`, so Phase 90 pages join automatically |
| `.github/workflows/docs-{deploy,quality}.yml` | VERIFIED (static) | Pre-render before build; path filters; browser step |

### Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| `package.json` prebuild | `render-diagrams.mjs` → cargo `--render-diagrams` | pnpm pre-hook | WIRED |
| `astro.config.mjs` | `voxDiagram({ manifestFile })` | integrations[] | WIRED |
| integration | `remarkVoxDiagram` (strict when command is build) | `updateConfig.markdown.remarkPlugins` | WIRED; the MDX probe shows the plugin also runs under `@astrojs/mdx` |
| manifest SVG | inline `<svg class="vd-diagram">` in dist | `svgToHast` + `reprefix` | WIRED and FLOWING (6/6 figures in dist) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Docs unit tests for the phase | `node --test tests/unit/{remark-vox-diagram,render-diagrams,vox-diagram-rebuild,workflow-paths}.test.mjs` | 26/26 pass | PASS |
| MDX miss fails the build, twice | scratch `verify89.mjs` (temporary project under `node_modules/.vd-verify-*`, removed afterwards) | both builds FAILED with the expected message | PASS |
| The WR-01 guard is load-bearing | the same probe, with the store drop removed from a copy of the integration | build 2 PASSED, so the mutation is detected | PASS |
| No duplicate ids in the built page | `grep -o 'id="vd-…"' … \| uniq -d` | empty (18 ids) | PASS |
| Browser spec, cargo nextest vox-diagram, render_manifest | established earlier this session | 16/16; 592/593 (one load-sensitive lexer timing test, which passes 3/3 alone); 11/11 | PASS |

### Probe Execution

None applicable. No `scripts/*/tests/probe-*.sh` is declared by this phase.

### Requirements Coverage

| Requirement | Source plan | Status | Evidence |
|---|---|---|---|
| DOCS-03 | 89-01, 89-03, 89-04 | SATISFIED | Truths 1 to 3 |
| DOCS-04 | 89-02, 89-04 | SATISFIED (local); Linux CI pending | Truth 4 |

No orphaned requirements: REQUIREMENTS.md maps only DOCS-03 and DOCS-04 to Phase 89.

### Anti-Patterns Found

| File | Pattern | Severity | Impact |
|---|---|---|---|
| (phase diff `b176936db..HEAD`) | TBD/FIXME/XXX/TODO/HACK | none found | None |
| `crates/vox-diagram/src/layout/input.rs:21` `SIDE_TITLE_CELLS = 16` + `wrap_at` hard break | Mid-word title break in left-to-right layouts | Warning | Reaches the site in Phase 90; see the judgement above |
| `.github/workflows/docs-quality.yml` font install | `apt-get install` with no `apt-get update` | Info | Usually fine, because both packages are preinstalled on ubuntu-latest; confirm on the first hosted run |
| docs-deploy/docs-quality | The pre-render runs twice (explicit step plus prebuild) | Info | IN-01, skipped deliberately; redundant but harmless |

Known failures outside this phase, not counted: playwright `llms.spec.ts` TRUST-02, unit `tutorial-record`, and 27 clippy errors in `vox-doc-pipeline` `src/claims/*`.

### Human Verification Required

1. **Left-to-right container-title layout.** Verifier judgement: defect, fix before Phase 90. Look at corpus 03 (`HarnessRiverPane` / `l`, staircase layout, bundled edges) against its pre-`07b66a573` render, then choose a gap-closure plan in 89 or a pre-step in 90.
2. **Hosted docs-quality run.** The pre-render, build, apt fonts plus Chromium, and VoxDiagram browser steps must be green within 30 minutes.
3. **Hosted docs-deploy run.** The pre-render, prebuild and Arimo font assets must be served.
4. **Linux font metrics.** Every text must fit under real DejaVu Sans and Liberation Sans; read the fill ratios from the CI log.
5. **Optional: Windows High Contrast with a dark theme.**

### Gaps Summary

There are no gaps against Phase 89's must-haves:

- The build fails closed for md, for mdx, and on a cached rebuild.
- Mounted docs are pre-rendered.
- The browser checks hold and, apart from the duplicate-id scan, are shown able to fail.
- The workflows are wired.

What remains is CI-only confirmation, plus one layout defect introduced by the review fix: the 16-cell side-title wrap breaks words, and corpus 03 regressed. My judgement is that it must be fixed before Phase 90 migrates the corpus.

---

_Verified: 2026-10-27T06:50:00Z_
_Verifier: Claude (gsd-verifier)_

## Re-verification (2026-10-27, orchestrator)

The left-to-right container-title defect found above (word split by the 16-cell title column, corpus 03 staircase) was fixed in 40e95fba9 ("lay sideways container titles in their content's ranks"), with red-first, mutation-proven tests `a_title_never_breaks_a_word_that_fits_a_line` and `sideways_containers_follow_the_flow` (crates/vox-diagram/src/layout/tests/containers.rs) plus a `nested_figures_stay_compact` ceiling. Corpus 03 is three side-by-side columns again with whole top-band titles (964x761 vs 1024x999.5 before CR-01, inspected by eye); the reference stress figure is 1392.5x446 with no title strike-through. vox-diagram 597/597, render_manifest 11/11, docs-astro diagrams Playwright 22/22 after a fresh build. Remaining human items are hosted-CI-only and are carried to the branch's PR CI run.
