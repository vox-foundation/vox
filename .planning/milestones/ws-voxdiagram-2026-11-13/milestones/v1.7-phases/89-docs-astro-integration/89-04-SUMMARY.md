---
phase: 89-docs-astro-integration
plan: 04
subsystem: docs-astro-diagrams
tags: [vox-diagram, playwright, contrast, font-fit, forced-colors, ci]

requires:
  - phase: 89-docs-astro-integration
    provides: "89-02 padding floor, 89-03 figure markup, theme CSS and sizing policy"
provides:
  - "reference page: Layout stress examples (shapes, nested containers, accented Latin and CJK, wide sequence)"
  - "tests/vox-diagram.spec.ts + tests/lib/diagrams.ts: ids, font probe, fit x3 stacks, contrast x2 + self-test, forced colors, wide policy x2, review bundle"
  - "docs-quality installs Chromium and fonts and runs the diagram spec after the build"
  - "browser-confirmed font policy and wide-diagram policy in the go/no-go record"
affects: [90]

actuals:
  tokens: 14000
  tasks: 3
  commits: 3
plan_head_before: 1f404eef5d254e7d8912fc209fc04e4f64c667cd
plan_head_after: 183bc9ad8a49da46fe3c3b3931f9aee16b8eb335
commits: 3

key-files:
  created:
    - docs-astro/tests/vox-diagram.spec.ts
    - docs-astro/tests/lib/diagrams.ts
  modified:
    - docs/src/reference/vox-diagram.md
    - .github/workflows/docs-quality.yml
    - docs-astro/tests/unit/workflow-paths.test.mjs
    - docs/src/architecture/voxdiagram-layout-go-no-go-2026.md
    - contracts/reports/test-inventory.v1.json

key-decisions:
  - "Pages are discovered from dist/ (`figureRoutes`), so Phase 90's migrated pages join the checks with no spec edit"
  - "The review bundle holds two PNGs per figure and mode: as shipped (0.6 floor, scroll-clipped) and `-natural` (full diagram lifted over the page chrome), because the shipped capture of a wide figure shows only its visible part"

requirements-completed: [DOCS-04, DOCS-03]

duration: one session
completed: 2026-10-27
status: complete
---

# Phase 89 Plan 04: Browser proof, stress diagrams and CI step Summary

**A Playwright spec now proves in Chromium, on every page of the built site that has a figure, that ids resolve, every label fits under three font stacks, contrast holds in light, dark and forced colors (with a self-test that shows the checker can fail), and the wide-diagram policy holds; docs-quality runs it after the pre-render build.**

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 (tracer) | `01fd0b4aa` | feat(89-04): stress diagrams and a browser ids check run in docs-quality |
| 2 | `ac448de4a` | test(89-04): fit under three font stacks, contrast, forced colors and wide-diagram policy |
| 3 | `183bc9ad8` | docs(89-04): record browser-confirmed font policy and the wide-diagram policy |

## Verification (all run in this session, tree at `183bc9ad8` plus this SUMMARY)

- `node --test tests/unit/workflow-paths.test.mjs`: 9 tests, 9 pass (RED first: the new ordering test failed before the workflow edit).
- `node --test "tests/unit/*.test.mjs"`: 968 tests, 967 pass, 1 fail (`tutorial-record`, pre-existing, see below).
- `playwright test tests/vox-diagram.spec.ts`: 13 passed (plan floor 13). With `tests/build-output/`: 47 passed, 1 skipped, 1 failed (`llms.spec.ts` TRUST-02, pre-existing).
- Fresh `pnpm --dir docs-astro build`: exit 0, 844 pages; 6 `vd-figure` on the reference page; 36 PNGs in the bundle (plan floor 18).
- Logged numbers: font probe narrow 401.6 px, wide 462.8 px, ratio 1.152 (floor 1.05); fit, 57/57 texts mapped, widest text fills 82.9% (Arial-metric), 86.9% (site system-ui), 95.7% (Verdana) of its box; contrast light min text 11.71 / min stroke 7.02, dark 10.06 / 5.27; forced colors 21.00 / 21.00.
- Mutation (fit): replacing the wide stack with `"Courier New", monospace` made `fits: wide` fail with 20 overflows (up to 22.7 units); lib restored from backup, `git diff --quiet` clean. The pale-palette self-test is itself a permanent mutation proof for contrast.
- Doc lint (`reference/vox-diagram.md`, the go/no-go record, `where-things-live.md`): no hard errors. `cargo nextest -p vox-doc-pipeline` render_manifest + diagrams filter: 19/19. `cargo nextest -p vox-diagram --test grammar_card`: 7/7 (card fence untouched).
- Gates: `vox ci ssot-drift` OK, `crate-edges` OK (one stale baseline edge `vox-gui -> vox-db-types` warning, not from this phase, not tightened), `check-links` 5368 links OK, `vox ci pre-push` (fast) all passed. `cargo fmt -p vox-diagram -p vox-doc-pipeline --check` clean. `cargo clippy -p vox-doc-pipeline --all-targets -D warnings` fails only inside `src/claims/` (known pre-existing items); no finding names `render_manifest.rs`.
- `Cargo.lock`, `crate-edges.allow.v1.json`, `fan-in-snapshot.v1.json`, `docs-astro/pnpm-lock.yaml` unchanged vs `b176936db`. No removed workflow line mentions concurrency, permissions, timeout-minutes or package-manager-cache.
- Generators: `scripts/ssot-regen.vox` changed only `contracts/reports/test-inventory.v1.json` (committed in `183bc9ad8`).

## By-eye review

Read with the Read tool from `/Users/brbrainerd/dev/vox-voxdiagram/docs-astro/test-results/review-bundle/vox-diagram/` (`NN-{light,dark,forced}.png` as shipped at 1280 px, `NN-{light,dark,forced}-natural.png` full size; gitignored, regenerated by the spec). Figure numbers are page order. I opened: 01 light, dark; 02 light, dark; 03 light, dark, forced-natural; 04 light (shipped), light, dark, forced-natural; 05 light, dark; 06 light (shipped), light, dark, forced-natural. The remaining PNGs were not opened (shipped variants of 03 and 05, forced variants of 01, 02, 05, dark shipped of 03 to 06); their natural-size siblings were.

- **01 Examples graph (nested container, cylinder), 02 sequence (note, reply).** Light and dark: no clipping or overlap, all text clear, container title above its nodes, "dispatch" label sits on a white (light) or dark (dark) patch that is slightly off the container fill (see defect D2).
- **03 Every shape (start, rect, rounded, diamond, cylinder, end), long and wrapped labels, 50-char edge labels.** Light and dark: nothing clipped, wrapped lines centred in their boxes, diamond text well inside the diamond, cylinder text below the rim. Edge labels interrupt their lines cleanly. Forced (natural): pure black on white, still legible. Defect-free apart from D2.
- **04 Nested containers, long container title, edges into nested nodes.** Light, dark, forced: all labels fully inside their boxes.
  - **D1 (known, record only):** the edge `Public API -> Metrics` ("emit") runs straight through the wrapped four-line title of the "Operations and observability tooling..." container; "Operations", "tooling" and "on-call dashboards" are struck through. This is 89-RESEARCH Open Question 6 / the Phase 85 title-crossing soft metric. Not fixed.
  - **D2 (cosmetic):** edge-label backgrounds use `--vd-bg` (page colour), so labels inside a container ("append", "apply", "routes the request", "emit") show as small white patches on the grey container fill in light, and darker patches in dark.
  - **D3 (layout quality, Phase 85):** containers are far larger than their content (the "Persistence layer" holds two small nodes in a ~640x400 box); the diagram is 1696 px wide and about 1500 px tall.
  - As shipped at 1280 px (04-light): the diagram is at its 0.6 floor and scrolls horizontally inside the figure. Node text is about 8 px: small but readable. The capture also shows the sticky site header drawn over the top of the figure; that is an element-screenshot artifact of a figure taller than the viewport, not a site defect.
- **05 Accented Latin and CJK graph.** Light and dark: every glyph renders (CJK through the platform fallback font), "Élève à l'école, naïve résumé" and the Japanese node wrap inside their boxes, no clipping. 1539 px natural width for four nodes (long edge labels), so it also sits at the 0.6 floor and scrolls at 1280 px.
- **06 Wide sequence (five actors, multi-hop messages, 3-line note, reply, self-message).** Light, dark, forced: legible everywhere. **WR-03 multi-hop labels confirmed:** "Cancel the reservation ... three times in a row" (Client to Stock, three hops) sits in the Gateway-Billing gap and "Send the receipt ..." (Gateway to Mailer) in the Billing-Stock gap, each clear of every lifeline it does not belong to; "reservation released" is clear of the Billing lifeline. The note covers the Billing lifeline as intended and its text is centred with room to spare. Self-message loop and its label on Billing are clean. Minor: "Send the receipt ... paid / for" leaves a one-word second line (wrap at 60 cells), cosmetic. At 1280 px the figure scrolls at about 0.6 (1986 px natural), text about 8 px: acceptable but at the lower edge of readable; scrolling reveals Stock and Mailer.
- **Forced colors** (emulated, Canvas white): black text and strokes, container fills drop out, legible in every figure I opened; the `<summary>` "Diagram source" is the UA link blue.
- **Contrast in dark** was judged by eye as well as measured: edge labels use the muted grey and are the dimmest text, still clearly readable.
- **Accessible name:** every figure's `aria-labelledby` resolves to a `<title>` whose text is the generic "Diagram" (all six). The check only requires non-empty, so it passes; a per-diagram title would be better for screen-reader users (Phase 90 or a later renderer change).

**Font-policy line: confirmed on macOS, Chromium 1243.** The static width table plus the 89-02 floor holds under `system-ui` (San Francisco), an Arial-metric stack and Verdana (1.152 times the Arial-metric probe); the widest label fills 95.7% of its box under Verdana. Not measured: real DejaVu Sans or Liberation (not installed here; the Arial and Verdana fallbacks stood in, and the CI job installs both). No slack proposal is needed from these numbers; the Linux numbers will come from the first docs-quality run.

## Deviations from Plan

None to the plan's logic. Additions: natural-size screenshots (`*-natural.png`) beside the shipped ones, the fit test logs the widest fill ratio, and the review capture hides the site chrome (`header, nav, aside`) so the lifted figure is not overlaid.

## Pre-existing failures (checked, not caused by this plan or 89-03)

- `tests/build-output/llms.spec.ts` TRUST-02: the test reads only committed files (`docs/src/.well-known/llms.txt`, `contracts/reports/docs-freshness/ledger.v1.json`, `doc-slug.mjs`, the spec) plus the existence of dist routes. `git diff --stat 2571c2cd5 HEAD` over all four is empty, and the ledger blob at `2571c2cd5` already has `docs/src/reference/cli.md` `state: "broken"`. So the six pages were `broken` at the base. I did not rebuild the site at the base commit; the conclusion follows from identical inputs.
- `tests/unit/tutorial-record.test.mjs`: tutorial blob hash mismatch for `tut-workflow-durability.md`; neither that tutorial nor the test changed since `2571c2cd5` (empty `git diff --stat`), and `109601d37` that edited the tutorial is an ancestor of `2571c2cd5`. Same reasoning, not run at the base.

## Known Stubs

None.

## CI budget

`gh run list --workflow docs-quality.yml`: of the last 8 runs seven are `action_required` (never executed) and one failed after about 14 minutes (2026-10-20) before reaching the Playwright steps. No complete run exists, so the 30-minute margin cannot be measured. Expect roughly 2 more minutes (apt fonts, Chromium with deps, a 15 s spec).

## Threat Flags

None. T-89-10 mitigated (discovery asserts the reference route plus a graph and a sequence figure; fit asserts every text is mapped; font probe; contrast self-test). T-89-11/12 accepted/mitigated as planned; T-89-SC: no lockfile change.

## Unverified

- docs-quality on hosted CI (apt font install, `playwright install chromium --with-deps` inside that job, the new steps' `if:` conditions) has not run.
- DejaVu Sans and Liberation Sans metrics; Linux Chromium rendering.
- Forced colors only under Playwright emulation (white Canvas); no Windows High Contrast dark theme.
- The wide-policy test was not mutation-proven (only fit and contrast were).
- Shipped-size screenshots 03, 05 and several forced/dark variants were not opened.

## Self-Check: PASSED

`docs-astro/tests/vox-diagram.spec.ts` and `docs-astro/tests/lib/diagrams.ts` exist; commits `01fd0b4aa`, `ac448de4a`, `183bc9ad8` are ancestors of HEAD; 36 PNGs present in the review bundle.
