---
phase: 85-graph-layout-deterministic-svg
plan: 01
subsystem: diagram-render
tags: [rust, dagre, svg, layout, determinism, escaping, golden]
status: complete

requires:
  - phase: 84
    provides: "Ir/GraphIr, check(), tests/support (roundtrip.rs assert_golden/stems)"
provides:
  - "vox_diagram::svg::{render, SvgOpts, RenderError, THEME_VARS} (final public API for the phase)"
  - "limits::MAX_RENDER_NODES = 100, MAX_RENDER_EDGES = 200"
  - "crate-internal layout seam (Engine, guarded, quantize, lay_out, Placed) with dagre_impl.rs as the only dagre importer"
  - "text model: Arimo advance table, measure, wrap (28 cells), node_size for all six shapes"
  - "single escaping writer (esc, num, Writer) and shapes.rs node drawing"
  - "svg_render golden harness with cases 00-minimal, 01-shapes, 02-wrap, 03-single"
  - "every dependency edit of the phase: dagre =0.1.1, unicode-width 0.2, dev quick-xml"
affects: [85-02, 85-03, 85-04, 85-05, 87, 89]

actuals:
  tokens: 15700   # chars/4 over the diff excluding Cargo.lock and fixtures (62909 chars)
  tasks: 2
  commits: 2
plan_head_before: a54e258b0775c85b4ff420247fe34d802590f325
plan_head_after: 3a769b162ed5e7a75ac0cf457d3eb7a01a6b907f

tech-stack:
  added: ["dagre =0.1.1 (default features)", "unicode-width 0.2 (already locked)", "quick-xml (dev, workspace)"]
  patterns:
    - "engine seam + catch_unwind guard + quantize to integer half-pixels, no float after the seam"
    - "one writer: ordered (&'static str, &str) attribute slices, every value through esc(.., true)"
    - "every paint is var(--vd-NAME,FALLBACK) built from THEME_VARS"

key-files:
  created:
    - crates/vox-diagram/src/layout/{mod,tests,input,text,dagre_impl}.rs
    - crates/vox-diagram/src/layout/text/tests.rs
    - crates/vox-diagram/src/svg/{mod,writer,shapes}.rs
    - crates/vox-diagram/tests/svg_render.rs
    - crates/vox-diagram/tests/fixtures/svg/cases/ (4 x .vdg and .svg)
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/vox-diagram/Cargo.toml
    - crates/vox-diagram/src/lib.rs
    - crates/vox-diagram/src/limits.rs

key-decisions:
  - "Property and unit tests for text live in the sibling src/layout/text/tests.rs (as src/lexer.rs does), keeping text.rs at 134 non-blank lines"
  - "<desc> and the 24-char endpoint labels are cut to at most the stated length INCLUDING the ellipsis (23 or 299 chars plus U+2026)"
  - "Cylinder is one rect (rx = half width, ry = 8) plus a vd-node-rim path, so every outline stays a single rect or polygon for the 85-04 checker"

requirements-completed: []

duration: 45min
completed: 2026-10-17
---

# Phase 85 Plan 01: Tracer and node shapes Summary

**A two-node graph now renders through check, IR caps, text sizing, dagre (behind an `Engine` seam inside `catch_unwind`), half-pixel quantization and a single escaping writer to a byte-exact, themeable, accessible SVG golden, and all six node shapes draw with wrapped, property-tested labels.**

## Accomplishments

- Tracer (Task 1): `svg::render` on `graph { node a: "A"  node b: "B"  a -> b }` gives `00-minimal.svg` byte for byte: `role="img"`, `aria-labelledby="vd-title vd-desc"`, `viewBox="0 0 96 168"`, no width/height, `<desc>` `2 nodes, 1 edge: A → B`, arrow marker, every paint a `var(--vd-*,fallback)` pair from `THEME_VARS`, no `<style>`/`foreignObject`.
- Fail-closed paths: bad `id_prefix` -> `InvalidIdPrefix`; 101 boxes or 201 edges -> `TooLarge` before layout; dangling endpoint or duplicate node id -> `InvalidIr("edge 0: unknown endpoint")`; engine panic / NaN / missing coordinate -> `Layout(..)` (tested through `Engine` doubles because dagre itself never panics).
- Expansion (Task 2): `wrap` (explicit `\n`, 28 cells, words split only on space/tab/CR, hard break only for an over-long word, zero-width followers stay with their base), the full width model (ASCII Arimo table, U+00A0-017F 600, wide 1000, emoji/VS16 1300, zero-width 0) and `node_size` for rect, rounded, diamond (corner-containment rule), cylinder, start, end; `shapes.rs` draws them.
- Dependencies: `Cargo.lock` diff is +14/-0 (one `dagre 0.1.1` package, three vox-diagram dependency lines); `cargo hakari generate --diff` printed nothing (rc 0); `cargo deny check licenses bans sources` -> bans ok, licenses ok, sources ok.

## Task Commits

1. **Task 1: Tracer, two-node graph through the dagre seam to a byte-exact golden** - `60602a944` (feat)
2. **Task 2: Label wrapping, full width model and all six node shapes** - `3a769b162` (feat)

## TDD record

- Task 1 RED: `cargo check -p vox-diagram --test svg_render` with only the test file and `00-minimal.vdg` present failed with E0432 (unresolved `vox_diagram::svg`) and two E0425 (`limits::MAX_RENDER_NODES/EDGES` not found). GREEN: 32 tests pass.
- Task 2 RED: with the three new `.vdg` inputs and the polygon assertion in place, `svg_cases` panicked "golden .../01-shapes.svg is missing" and `shapes_case_has_one_polygon` failed (0 polygons found, left 0 right 1). The `layout::text` unit and property tests were written before `wrap`/`node_size` were finished but compiled and ran only once those existed, so their RED is shown by mutation instead: `WRAP_CELLS` 28 -> 29 made `wrap_hard_breaks_only_an_overlong_word` and `wrap_keeps_a_zero_width_follower_with_its_base` fail (restored, presence-checked with `grep -c`). The property assertion was then changed to the literal 28 so it also bites.
- Final: `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(svg_render) | test(/^layout::/) | test(/^svg::/) | binary(determinism_guard)'` -> 45 passed; `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean; `cargo fmt -p vox-diagram` applied.

## Files Created/Modified

See frontmatter. Largest new files by non-blank lines: `svg/mod.rs` 246, `svg/writer.rs` 203, `layout/mod.rs` 197, `svg/shapes.rs` 180 (all under 400).

## Decisions Made

See frontmatter `key-decisions`. Also: `LayoutInput` carries no `containers` field yet (85-03 declares `LContainer` and adds it); `input::build` refuses a labelled edge (`edge N: labels are not drawn yet`) or any container (`container 0: ...`) with `InvalidIr` until 85-02 / 85-03 land, as the plan prescribes, so nothing is silently dropped.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `a % 2 == 0` tripped `clippy::manual_is_multiple_of` in `svg/writer.rs::num`**
- **Found during:** Task 1 (clippy gate)
- **Fix:** `a.is_multiple_of(2)`
- **Verification:** clippy clean
- **Committed in:** `60602a944`

**Total deviations:** 1 auto-fixed (Rule 1). No scope creep. The `shapes_case_has_one_polygon` test was drafted in the Task 1 test file and moved to Task 2 (where the plan lists it) before the Task 1 commit.

## Issues Encountered

- The first `UPDATE_GOLDEN=1` run of Task 2 had `shapes_case_has_one_polygon` fail because nextest ran it in parallel with the blessing test; the rerun without the variable was green. Not a defect.

## Known Stubs

None. Intentional refusals (not stubs): labelled edges and containers return `RenderError::InvalidIr` until 85-02 and 85-03.

## Not done (belongs to later plans or the closing plan)

- Diamond edge-endpoint clipping, rounded polyline corners and edge labels: 85-02.
- `vox ci crate-api-sync --write` and the `where-things-live.md` row for the new public `svg` API / `layout` module were not run here (the plan's verify list does not include them and hosted CI regenerates SSOT drift); the closing plan 85-05 should do both.
- Cross-platform byte equality of the goldens is asserted by hosted Linux CI against these macOS-blessed files (RESEARCH A4).
- REND-01..REND-04 are NOT ticked: containers, edge labels, cycles/self-loop cases, the corpus spike, the allowlist checker with per-arm mutation proof and the theme/contrast tests arrive in 85-02..85-05.

## Threat Flags

None beyond the plan's threat model (T-85-01..04 and T-85-SC mitigations are in place: single `esc` with a unit test per arm, caps before layout, `guarded` + `quantize`, validated `id_prefix`, exact `=0.1.1` pin with deny/hakari checks).

## Self-Check: PASSED

Verified present: `crates/vox-diagram/src/svg/{mod,writer,shapes}.rs`, `crates/vox-diagram/src/layout/{mod,input,text,dagre_impl,tests}.rs`, `crates/vox-diagram/src/layout/text/tests.rs`, `crates/vox-diagram/tests/svg_render.rs`, four `.vdg`/`.svg` pairs under `tests/fixtures/svg/cases/`. Commits `60602a944` and `3a769b162` are ancestors of HEAD. `grep -rl 'dagre::' crates/vox-diagram/src` prints only `layout/dagre_impl.rs`; `git diff --numstat 5247a56ea -- Cargo.lock` is `14 0`.
