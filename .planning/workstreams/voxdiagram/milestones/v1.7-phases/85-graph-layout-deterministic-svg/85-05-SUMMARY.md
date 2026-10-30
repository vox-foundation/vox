---
phase: 85-graph-layout-deterministic-svg
plan: 05
subsystem: diagram-render
tags: [rust, dagre, svg, go-no-go, goldens, contact-sheet, governance]
status: complete

requires:
  - phase: 85
    provides: "svg::render, layout seam, svgcheck/theme test support (85-01..04)"
  - phase: 84
    provides: "mermaid::import and the frozen 33-block corpus"
provides:
  - "tests/svg_corpus.rs: corpus render, frozen top-10 ranking, ten byte-exact SVG goldens, ignored contact-sheet generator"
  - "85-CONTACT-SHEET.html: light/dark panes of the ten spike diagrams from one markup"
  - "docs/src/architecture/voxdiagram-layout-go-no-go-2026.md: recorded GO (dagre 0.1.1), font policy, caps, theming contract, residual risks"
  - "where-things-live row for layout/ and svg/; regenerated vox-diagram API page"
affects: [86, 87, 89]

requirements-completed: [REND-01, REND-02, REND-04]

actuals:
  tokens: 14500   # chars/4 over the diff excluding the contact sheet and SVG goldens (about 58,000 chars)
  tasks: 3
  commits: 3
plan_head_before: b83c1d226c0feca5b62be4b7669880192fe50a86
plan_head_after: 13b1257c9b47c2a90b68f664c67f4c4e84a605b9

key-files:
  created:
    - crates/vox-diagram/tests/svg_corpus.rs
    - crates/vox-diagram/tests/fixtures/svg/corpus/ (10 .svg goldens)
    - .planning/workstreams/voxdiagram/phases/85-graph-layout-deterministic-svg/85-CONTACT-SHEET.html
    - docs/src/architecture/voxdiagram-layout-go-no-go-2026.md
  modified:
    - crates/vox-diagram/src/svg/mod.rs
    - docs/src/architecture/where-things-live.md
    - docs/src/api/vox-diagram.generated.md
    - docs/src/api/index.generated.md

key-decisions:
  - "GO on dagre 0.1.1: 30 of 30 importable corpus graphs render Ok; 0 of 10 need a DSL tweak (threshold 2)"
  - "Diagram 10 (edge crossing a container title) is a recorded residual risk, not a DSL tweak and not a renderer change here"

duration: ~70 min
completed: 2026-10-17
---

# Phase 85 Plan 05: Go/No-Go, Goldens and Governance Summary

**The shipped renderer lays out all 30 importable corpus graphs, the ten largest are pinned as byte-exact goldens that also pass on Linux aarch64 and x86_64, a reproducible light/dark contact sheet was reviewed screenshot by screenshot, and the GO on dagre 0.1.1 is recorded with its limits.**

## Accomplishments

- Task 1: `svg_corpus.rs` with `corpus_graphs_all_render` (33 blocks split 30 graphs / 1 sequence / 2 refused; each of the 30 checks, renders Ok and passes the independent allowlist checker), `corpus_top10_ranking` (frozen list, tenth scores 20, eleventh 19), `corpus_top10_goldens` (ten goldens, directory holds exactly those) and the ignored `contact_sheet` generator (asserts the dark SVG equals the light SVG after the id-prefix swap). The computed top 10 matched the frozen list and no graph returned `Err`, so nothing had to be stopped or reported. The sheet was generated twice: same SHA-1 `09e9ee01d81a1ad4a4e2b9624390992df3d6c925`, 171,257 bytes, 20 `class="vd-diagram"`.
- Task 2: visual review, Linux comparison and the decision record (below).
- Task 3: where-things-live row, `crate-api-sync --write`, and the closing gates.

## TDD record

- Task 1 RED: `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 --no-fail-fast -E 'binary(svg_corpus)'` before blessing: `corpus_top10_goldens` FAIL (`golden .../13-deep-research-waves-...-l110.svg is missing`); `corpus_top10_ranking` and `corpus_graphs_all_render` PASS at once, because they assert the shipped behaviour the earlier plans built (no behavioural RED exists for them; they would fail on a different corpus, a changed ranking or an `Err`). GREEN: blessed with `UPDATE_GOLDEN=1` running `test(corpus_top10_goldens)` alone, then 3/3 pass without the variable.
- No proptest-regressions file was produced by this plan.

## Visual review (SC2): per-diagram verdicts

Method: the committed sheet opened in headless Chromium (playwright-core 1.63.0 from `docs-astro/node_modules/.pnpm`, throwaway script in the scratchpad), one screenshot per row at 1700 px, 1.5x scale, light and dark pane read side by side.

| # | Fixture | Labels in shapes | Container titles | Arrowheads both panes | Overlap | Soft defects | Verdict |
|---|---------|------------------|------------------|-----------------------|---------|--------------|---------|
| 1 | 13-...-l110 | yes | inside band | yes | none | two edge labels sit on the band's top border; long diagonals from the diamond | no tweak |
| 2 | 04-...-l46 | yes | n/a | yes | none | one 31-character identifier hard-broken mid-word (`..._stat` / `us: completed`) | no tweak |
| 3 | 21-...-l45 | yes | n/a | yes | none | none | no tweak |
| 4 | 12-...-l76 | yes | n/a | yes | none | none | no tweak |
| 5 | 26-...-l51 | yes | n/a | yes | none | none | no tweak |
| 6 | 03-...-l83 | yes | 4 inside bands | yes | none | cross-container edges pass through a sibling band (by the graph) | no tweak |
| 7 | 01-...-l147 | yes | n/a | yes | none | none | no tweak |
| 8 | 11-...-l45 | yes | n/a | yes | none | none | no tweak |
| 9 | 20-...-l117 | yes | n/a | yes | none | none | no tweak |
| 10 | 06-...-l238 | yes | inside band | yes | none | the edge from outside `UserQuery` to the first container's first node runs through that container's title text (title stays readable, struck through) | no tweak, recorded as a residual risk |

**DSL tweaks: 0 of 10** (threshold 2). Even if diagram 10 were counted, the total is 1. Decision: GO, so the `dugong` fallback and its new-dependency sign-off were not needed.

## Linux golden comparison (SC3, non-gating)

Docker (Colima) was available. A throwaway copy of the crate (src, tests, fixtures, `grammar-card.txt`) was placed under `~/dev/_scratch/vd85-linux/` with a standalone manifest pinning the locked versions (`dagre =0.1.1`, `unicode-width =0.2.2`, serde 1.0.228, serde_json 1.0.150, pulldown-cmark 0.13.4, strsim 0.11.1, quick-xml 0.41.0, proptest 1.11.0, insta 1.48.0) and no workspace-hack. `cargo test --test svg_corpus --test svg_render --test svg_determinism` in `rust:1.98.1`:

| Platform | Result |
|----------|--------|
| Linux aarch64 (native Colima, `uname -m` aarch64) | pass: 3 + 2 + 16 = 21 tests, ten corpus goldens equal the macOS-blessed bytes |
| Linux x86_64 (`--platform linux/amd64`, QEMU, `uname -m` x86_64) | pass: the same 21 tests |

The scratch copy and the two Docker volumes were deleted. Hosted CI on the milestone PR remains the authoritative run (QEMU float emulation is not real x86_64 hardware).

## Closing gates

| # | Gate | Result |
|---|------|--------|
| 1 | `cargo fmt -p vox-diagram --check` | pass |
| 2 | `cargo clippy -p vox-diagram --all-targets -- -D warnings` | pass (re-run after the doc fix) |
| 3 | `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps` | first run FAILED, fixed (Deviation 1), re-run pass |
| 4 | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0` | pass: 460 run, 460 passed (1 leaky), 2 skipped (340 pre-phase + this phase's binaries; threshold 341) |
| 5 | `target/debug/vox ci crate-edges` | pass, no baseline edit |
| 6 | `cargo run -p vox-arch-check` | pass |
| 7 | `cargo hakari generate --diff` | pass |
| 8 | `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md` | pass; the go/no-go doc also lints clean (`--paths architecture/voxdiagram-layout-go-no-go-2026.md`) |
| 9 | new files under `src/{layout,svg}/` and `tests/svg_*.rs`, `tests/support/*.rs` below 400 non-blank lines | pass (none at or above 400; largest `svgcheck.rs` 383) |
| 10 | `git diff --quiet 5247a56ea -- contracts/ci docs/src/architecture/layers.toml crates/workspace-hack` | exit 0 |

Also: `vox ci crate-api-sync --write` then `crate-api-sync` -> `OK (130 crates)`, both before and after the doc fix. The first `vox ci` attempt was refused because `target/debug/vox` was stale (built at commit 7274, tree at 7316); `cargo build -p vox-cli` was run in the background and the freshness check was never bypassed.

## Task Commits

1. **Task 1: Corpus render, top-10 goldens, contact sheet** - `9b2b5ee59` (test)
2. **Task 2: Go/no-go decision record** - `1a8491213` (docs)
3. **Task 3: where-things-live row, API page, rustdoc fix** - `13b1257c9` (docs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] rustdoc broken intra-doc links in the `svg` module doc**
- **Found during:** Task 3 gate 3 (`RUSTDOCFLAGS='-D warnings' cargo doc`)
- **Issue:** `crates/vox-diagram/src/svg/mod.rs` linked `[`writer`]` and `[`THEME_VARS`]` in its first doc paragraph; that paragraph is reused as the module summary in the crate index, where neither name resolves (two `unresolved link` errors).
- **Fix:** plain backticks instead of links. Comment-only change; fmt, clippy, doc re-run clean.
- **Commit:** `13b1257c9`

### Plan acceptance wording that did not hold

**2. API page grep.** The acceptance check `grep -c 'svg::render\|pub fn render' docs/src/api/vox-diagram.generated.md` prints 0: the generated page lists public modules (`svg` now appears in the Modules table with its description) and constants, not module items such as `render` or `MAX_RENDER_*`. The page is regenerated only by `crate-api-sync --write` and `crate-api-sync` reports OK, so it was not hand-edited. `grep -c '`svg`'` is 1.

**3. Contact-sheet pane classes.** `theme::pane_css` scopes to `.pane-light` / `.pane-dark`; the generator renames them to the plan's `.vd-light` / `.vd-dark` with `str::replace` rather than changing the shared support function.

## Issues Encountered

- 85-04's note was honoured: the decision doc states the roughly 4% generated-graph `Geometry` fail-closed rate next to the corpus 30/30 result.
- Diagram 10's edge-through-title defect is the one thing a reader of the sheet will notice; it needs title-aware routing or a text halo, which is outside this plan (no renderer change was made to force a clean sheet).
- The bash allowlist blocks `$VAR` command names, `sleep` in the foreground and `bash -c`; long waits used a scratchpad wait script and plain literal commands.

## Known Stubs

None.

## Threat Flags

None. T-85-15 (decision derived from tests over the frozen corpus, byte-identical sheet generated twice, verdicts and Linux results recorded), T-85-16 (static sheet built only from the renderer's own escaped SVG plus test CSS, no script, never served), T-85-17 (the `git diff --quiet` gate exit 0; API page only through `crate-api-sync --write`) and T-85-SC (no dependency change; the Linux check used a throwaway copy pinning locked versions) are mitigated or accepted as planned.

## Requirements

REND-01 (graph renders via dagre behind the one-file adapter with containers, edge labels, cycles, self-loops; the 10 largest corpus diagrams pass the go/no-go), REND-02 (byte-identical across runs and across macOS and Linux, with the Linux evidence above and no hash-order, transcendental-float or version/timestamp source, each guarded by a test) and REND-04 (one inline SVG themes in light and dark via `--vd-*` variables mapped to Starlight tokens, proven by `svg_theme` and the contact sheet) are ticked. Hosted CI on the milestone PR is the remaining independent confirmation of the Linux claim.

## Self-Check: PASSED

Present: `tests/svg_corpus.rs`, ten files under `tests/fixtures/svg/corpus/`, `85-CONTACT-SHEET.html`, `voxdiagram-layout-go-no-go-2026.md`. Commits `9b2b5ee59`, `1a8491213`, `13b1257c9` are ancestors of HEAD; `git rev-list --count b83c1d226..13b1257c9` is 3. `.planning/active-workstream` and `.planning/workstreams/voxdiagram/milestone.lock` were left unstaged.
