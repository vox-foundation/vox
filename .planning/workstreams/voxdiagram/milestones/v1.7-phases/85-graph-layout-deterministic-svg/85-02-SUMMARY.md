---
phase: 85-graph-layout-deterministic-svg
plan: 02
subsystem: diagram-render
tags: [rust, dagre, svg, layout, edges, verify, fail-closed, golden]
status: complete

requires:
  - phase: 85-01
    provides: "Engine seam, guarded/quantize, text model, single escaping writer, svg_render golden harness"
provides:
  - "edge labels (LLabel) through the dagre seam with label centres read back and quantized"
  - "svg::paths: integer-only rounded polylines (isqrt), diamond endpoint clipping, edge and edge-label drawing"
  - "layout::verify::{verify, Violation}: NodeOverlap / LabelOverflow / OutsideView on integer half-pixel geometry"
  - "lay_out as a three-rung separation ladder; Layout errors final, all-rung failure is RenderError::Geometry"
  - "Engine test doubles (panic, NaN, huge, missing, overlap-then-clean, always-overlap)"
  - "goldens 10-edges, 11-right, 12-up, 13-left"
affects: [85-03, 85-04, 85-05]

actuals:
  tokens: 10200   # chars/4 over the diff excluding fixtures (40712 chars)
  tasks: 2
  commits: 2
plan_head_before: 19644857438f3b812ce709933cd914b4d1b97486
plan_head_after: 474efbeb1c6ad7bc2fc2890e2d335ec390b10dcb

tech-stack:
  added: []
  patterns:
    - "ladder: guarded engine call, quantize, verify; first verified rung wins"
    - "edge geometry in i64 with isqrt and truncating integer division; no float after quantize"

key-files:
  created:
    - crates/vox-diagram/src/layout/verify.rs
    - crates/vox-diagram/src/svg/paths.rs
    - crates/vox-diagram/tests/fixtures/svg/cases/ (10-edges, 11-right, 12-up, 13-left, each .vdg and .svg)
  modified:
    - crates/vox-diagram/src/layout/{mod,input,dagre_impl,tests}.rs
    - crates/vox-diagram/src/svg/{mod,shapes}.rs
    - crates/vox-diagram/tests/svg_render.rs

key-decisions:
  - "verify padding rule for rectangular shapes is w >= tw + 4*PAD_X and h >= th + 4*PAD_Y half-pixels (the nominal node_size padding, which every shape's node_size satisfies); diamond uses tw*h + th*w <= w*h"
  - "Edge label box is the wrapped text block plus 4 px each side; the engine receives that box, the label rect is placed from dagre's centre read-back and the input size"
  - "The vd-edge-labels group is emitted only when at least one edge has a label, so the 85-01 goldens are byte-unchanged"
  - "shapes::label is shared (class and fill parameters) so edge labels reuse the single tspan/baseline layout"

requirements-completed: []

duration: 40min
completed: 2026-10-17
---

# Phase 85 Plan 02: Edges and fail-closed seam Summary

**Labelled edges, 2-cycles, self-loops, parallel edges and all four directions now render as integer-rounded paths with clipped diamond endpoints and `--vd-bg` label backgrounds, and a geometry verifier plus a three-rung separation ladder turn every engine failure or unfixable overlap into a typed `RenderError`.**

## Accomplishments

- Task 1: `LEdge` carries `Option<LLabel>`; the dagre adapter passes the label box with `LabelPos::Center` and reads the label centre back; `place` quantizes it to a top-left `Rect` and includes label rects in the margin/viewBox bounds. The 85-01 labelled-edge refusal is gone (a labelled edge renders Ok; blank or whitespace labels become `None`). `svg::paths` has `rounded` (radius `min(8 px, leg1/2, leg2/2)`, legs by `i64::isqrt`, offsets by integer `d*r/len`, repeated points dropped, collinear points just get `L`), `clip_diamond` (`p = c + d*w*h / (2|dx|h + 2|dy|w)` in `i64`, a point on or inside the outline is untouched) and the edge and edge-label drawing. Draw order is edges, nodes, edge labels. Four goldens blessed and inspected (all path coordinates inside the viewBox, diamond endpoints on the outline, self-loop and parallel edges distinct).
- Task 2: `layout::verify` (exact integer checks, no epsilon, no label text in messages). `lay_out` loops `RUNGS` in order, `guarded` then `place` then `verify`, returns the first Ok, returns a `Layout` error at once, and returns `Geometry(<last violation>)` when all three rungs fail. Soft defects (edge through a non-endpoint node, label touching a node) are deliberately not checked.

## Task Commits

1. **Task 1: Edge labels, cycles, self-loops, parallel edges, directions, rounded paths, diamond clipping** - `6dfcf808d` (feat)
2. **Task 2: Geometry verifier, separation ladder and Engine test doubles** - `474efbeb1` (feat)

## TDD record

- Task 1 RED: with `svg/paths.rs` holding only its unit tests (and `mod paths;` declared) plus the new `svg_render` assertions and `.vdg` inputs, `cargo check -p vox-diagram --all-targets` failed with 15 errors (E0425 `rounded` / `clip_diamond` not found, E0422). GREEN after implementation: 59 tests pass. The first `UPDATE_GOLDEN=1` run had three assertion tests fail only because nextest ran them in parallel with the blessing test (known from 85-01); the rerun without the variable was green.
- Task 2 RED: with `verify.rs` holding only its tests and the new doubles in `layout/tests.rs`, the check failed with 41 errors (E0425 `verify`, E0433 `Violation`, E0425 `Half`/`Rect`/`Placed`/`LayoutInput`). GREEN: 70 tests pass.
- Mutation (guard proof): with a before/after presence check (`grep -c 'verify::verify(input, &placed)'` 1 before, 0 mutated, 1 restored; restored file `diff`-identical to the saved copy), `lay_out` was changed to skip `verify`. `cargo nextest run -p vox-diagram -E 'test(/^layout::/)'` then failed `layout::tests::an_unfixable_overlap_is_a_geometry_error_after_three_rungs`, `layout::tests::an_overlap_escalates_to_the_next_rung` and `layout::tests::the_public_path_is_guarded_too` (3 failed, 31 passed). Restored; all green again.
- Final: `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_render) | test(/^svg::/) | test(/^layout::/) | binary(determinism_guard)'` -> 70 passed; `cargo clippy -p vox-diagram --all-targets -- -D warnings` clean; `cargo fmt -p vox-diagram --check` clean.

## Deviations from Plan

None - plan executed as written. (Doubles `PanicEngine` etc. live in `layout/tests.rs`; the `render_with` public-path assertions are therefore in that file too, since `layout::tests` is private to `layout`. The 85-01 `Panicky` double was replaced by `PanicEngine`.)

## Issues Encountered

- A path can contain a zero-length `L` segment after a `Q` when two corners on a short leg meet at the same point (visible in the self-loop paths). It is valid SVG and deterministic; not suppressed to keep `rounded` small. Candidate tidy-up for a later plan if path size matters.

## Known Stubs

None. Containers are still refused with `RenderError::InvalidIr` until 85-03 (intentional, documented in 85-01).

## Not done (belongs to later plans)

- Containers, corpus spike, allowlist checker, theme/contrast tests: 85-03..85-05.
- `vox ci crate-api-sync --write` and the `where-things-live.md` row: closing plan 85-05.
- REND-01 and REND-02 are NOT ticked: containers, the corpus spike and the determinism/checker work are still open.

## Threat Flags

None. T-85-05 (at most three engine calls, Layout error stops at once) and T-85-06 (fail-closed verifier, each check has a failing unit test, skip-verify mutation recorded above) are mitigated. T-85-07: edge-label text is written only through `shapes::label` -> `Writer::leaf` (the single escaper); no label text reaches an attribute.

## Self-Check: PASSED

Verified present: `crates/vox-diagram/src/layout/verify.rs`, `crates/vox-diagram/src/svg/paths.rs`, `10-edges`, `11-right`, `12-up`, `13-left` `.vdg` and `.svg`. Commits `6dfcf808d` and `474efbeb1` are ancestors of HEAD. `grep -rl 'dagre::' crates/vox-diagram/src` prints only `layout/dagre_impl.rs`; no Cargo.toml / Cargo.lock change since 85-01; every touched file is under 400 non-blank lines (largest `layout/tests.rs` 261).
