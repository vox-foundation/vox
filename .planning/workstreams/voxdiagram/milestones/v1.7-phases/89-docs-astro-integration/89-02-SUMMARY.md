---
phase: 89-docs-astro-integration
plan: 02
subsystem: vox-diagram-layout
tags: [vox-diagram, layout, font-fit, goldens]

requires:
  - phase: 85-svg-renderer
    provides: "static width table, 8% slack, padded node/edge-label boxes"
  - phase: 86-sequence-kind
    provides: "sequence label/note/lane layout and its geometry oracles"
provides:
  - "layout::text::pad_x + WIDE_FONT_PCT: the one integer horizontal padding floor (box >= 112% of measured width)"
  - "floor applied to nodes (rect/rounded/cylinder/diamond), sequence actors, edge labels, message labels, notes and container title struts"
  - "re-blessed goldens (number-only diffs) and an updated font policy in the go/no-go record"
affects: [89-04]

actuals:
  tokens: 6500
  tasks: 2
  commits: 2
plan_head_before: 18c07fc2f8eebb47f220c1515372797bb1cf702a
plan_head_after: 63c4693832202248e6cba47bfce36127f811aee6
commits: 2

key-files:
  created: []
  modified:
    - crates/vox-diagram/src/layout/text.rs
    - crates/vox-diagram/src/layout/text/tests.rs
    - crates/vox-diagram/src/layout/seq.rs
    - crates/vox-diagram/src/layout/seq/lanes.rs
    - crates/vox-diagram/src/layout/seq/tests.rs
    - crates/vox-diagram/src/layout/input.rs
    - docs/src/architecture/voxdiagram-layout-go-no-go-2026.md

key-decisions:
  - "Sequence call sites reach the floor through a private `hpad` (W=i64 wrapper over pad_x, saturating at the Half range) so there is still exactly one copy of the formula"
  - "start/end nodes keep their fixed 40 px per side (exceeds the floor under the label cap)"

requirements-completed: []  # DOCS-04 needs the browser proof in plan 89-04; not ticked here

duration: ~40min
completed: 2026-10-17
status: complete
---

# Phase 89 Plan 02: Padding floor for wide fonts Summary

**Every horizontally padded text box the layout produces is now at least 112% of its measured width through one integer function (`pad_x`), so a font up to ~21% wider than the Arimo table fits; the table and 8% slack are unchanged.**

## Accomplishments

- `WIDE_FONT_PCT = 12` and `pad_x(tw, base) = max(base, ceil(tw*12/200))` in `layout/text.rs` (integer only, negative width clamped, saturating).
- Call sites: `node_size` (rect, rounded, cylinder, diamond; actors via `node_size`), `edge_label`, container `strut_w`, `seq::label_box`, `lanes::note_w`. `verify.rs` unchanged (its node rule is a lower bound).
- Go/no-go record: new "Padding floor (Phase 89)" bullet; the Verdana sentence now says 89-RESEARCH measured overflow on 4 px-padded edge and message labels (up to ~7 px per side).

## Re-blessed goldens (blessing tests only, number-only diff check over `tests/fixtures/svg` vs `b176936db`: exit 0, 317 changed lines)

- Task 1: `cases/30..33-seq-*.svg`, `sequence-corpus/19-scientia-...-l112.svg`. Widest growth: `30-seq-long-labels` viewBox width 1328.5 -> 1445.5.
- Task 2: `cases/21-nested.svg` (1145 -> 1147 wide), `cases/31-seq-self-and-notes.svg` (again, notes/actors), `corpus/06-...-l238.svg`, `corpus/11-...-l45.svg`, `corpus/13-...-l110.svg` (1541 -> 1563 wide). The other 20 graph goldens did not change (floor inactive at their widths).
- No export golden changed, so the Node mermaid oracle was not re-run.

## Pinned expectation that moved

- `layout::text::tests::node_size_formulas_for_a_wide_label` (40-char `W` line, tw = 1142 half-px): rect/rounded/cylinder `tw + 56` -> `tw + 2*pad_x(tw, 28)` (floor active: pad 69 vs 28 half-px base), diamond `2*(tw+28)` -> `2*(tw+pad_x(tw,28))`. No other pinned size moved (seq and layout tests unchanged).

## TDD Evidence

- Task 1 RED (stub `pad_x` returning base): `a_long_message_label_box_fits_a_wide_font`, `node_boxes_fit_a_wide_font`, `pad_x_fits_a_font_21_percent_wider_than_the_table` FAILED (3 failed, 23 passed). GREEN: 25/25.
- Task 2 RED: `a_long_title_strut_uses_the_padding_floor` failed to compile without the call site's `pad_x` import (compile-level), and `node_size_formulas_for_a_wide_label` failed behaviorally the moment `node_size` used the floor, before its expectation was updated. The edge-label, note and node-fit tests were not run separately in a behavioral-RED state; their sensitivity is shown by the mutation proofs below. GREEN: 216/216 lib tests.
- **Mutation proofs** (revert one call site, named test must fail, restore with presence check and `cmp`): `label_box` -> `a_long_message_label_box_fits_a_wide_font` FAILED; `edge_label` -> `a_long_edge_label_box_fits_a_wide_font` FAILED; strut -> `a_long_title_strut_uses_the_padding_floor` FAILED; `note_w` -> `a_long_note_is_as_wide_as_a_wide_font_needs` FAILED; `node_size` -> `node_boxes_fit_a_wide_font` FAILED. Presence of the mutated text was 1 before and 0 after each mutation; all five files restored byte-identical.

## Verification Run

- `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0`: 587 passed, 3 skipped (includes `corpus_graphs_all_render` 30/30, `geometry_holds` floor, determinism, both SVG geometry oracles, hostile, theme).
- `cargo clippy -p vox-diagram --all-targets -- -D warnings`: clean. `cargo fmt -p vox-diagram --check`: clean.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/voxdiagram-layout-go-no-go-2026.md`: no hard errors.
- `Cargo.toml`/`Cargo.lock` unchanged vs `b176936db`. Touched src files all under 400 non-blank lines (largest `input.rs` 325).

## Deviations from Plan

1. **[Minor] `lanes.rs` uses `hpad(` rather than a literal `pad_x(`**: the plan's acceptance grep (`pad_x(` >= 1 in lanes.rs) is not literally met; `hpad` is a 3-line `W`-to-`Half` wrapper in `seq.rs` that calls `pad_x`, which is the plan's own instruction ("convert at the call"). One copy of the formula remains.
2. **[Process]** Scratchpad log names collide across sessions; one stale log was briefly read as mine. Re-ran with unique names; no results were affected.

## Known Stubs

None.

## Self-Check: PASSED

`pad_x` present once in `text.rs`; commits `199e9f49e` and `63c469383` are ancestors of HEAD; goldens, record and tests present.
