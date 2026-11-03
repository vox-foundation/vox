---
phase: 86-sequence-kind
plan: 03
subsystem: vox-diagram
tags: [sequence-diagram, svg, layout, geometry-oracle, proptest, caps, determinism]

requires:
  - phase: 86-sequence-kind
    provides: layout::seq::place, svg::seq::render_sequence, svgcheck read-back of sequence SVG (plan 01)
  - phase: 85-graph-layout-deterministic-svg
    provides: the single escaping Writer, svgcheck allowlist checker, render caps pattern
provides:
  - "layout::seq: 60-cell soft wrap of message and note text (MSG_WRAP_CELLS), self-message gap rule, fail-closed validate() before any placement"
  - "limits::MAX_RENDER_ACTORS (100) and limits::MAX_RENDER_EVENTS (200)"
  - "tests/support/svgcheck/seq.rs: independent assert_sequence_geometry oracle and sequence_ir() strategy"
  - "svg_sequence_geometry and svg_sequence_safety test binaries; goldens 30-33 in svg/cases"
affects: [86-04]

plan_head_before: 8caf232d28293ff3d3030864d7743dc9ae78a15f
plan_head_after: e1b63240a4a4183fe800a4fc020ec5a655f10c38

actuals:
  tokens: 14100
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "sequence oracle reads only svgcheck's read-back plus IR order (actors to nodes/lifelines, messages to edge_paths, non-blank messages to edge_labels, notes to notes)"
    - "validate() returns the id-to-lane map, so place() resolves lanes once and validation order is one function"

key-files:
  created:
    - crates/vox-diagram/tests/support/svgcheck/seq.rs
    - crates/vox-diagram/tests/svg_sequence_geometry.rs
    - crates/vox-diagram/tests/svg_sequence_safety.rs
    - crates/vox-diagram/tests/fixtures/svg/cases/30-seq-long-labels.{vdg,svg}
    - crates/vox-diagram/tests/fixtures/svg/cases/31-seq-self-and-notes.{vdg,svg}
    - crates/vox-diagram/tests/fixtures/svg/cases/32-seq-replies-and-wide-actors.{vdg,svg}
    - crates/vox-diagram/tests/fixtures/svg/cases/33-seq-unicode.{vdg,svg}
  modified:
    - crates/vox-diagram/src/layout/seq.rs
    - crates/vox-diagram/src/layout/seq/tests.rs
    - crates/vox-diagram/src/layout/input.rs
    - crates/vox-diagram/src/limits.rs
    - crates/vox-diagram/src/svg/mod.rs
    - crates/vox-diagram/tests/support/svgcheck.rs
    - crates/vox-diagram/tests/determinism_guard.rs

key-decisions:
  - "Note text soft-wraps at the same 60 cells as message text (Claude's discretion; the context only fixed the message rule). Actor labels keep the 28-cell graph node wrap."
  - "The oracle does not forbid an intermediate lifeline passing behind a label box that crosses it (the label background is filled); it requires clearance only from the message's own two lifelines, as the plan specifies."
  - "REND-05 is not ticked: SC3 (the corpus block renders, golden plus oracle) and the by-eye review of the sequence SVGs are still open in 86-04."

patterns-established:
  - "Mutation proof: copy, mutate with a grep presence check, named tests fail, restore with diff, rerun green."

requirements-completed: []

duration: one session
completed: 2026-10-17
status: complete
---

# Phase 86 Plan 03: Sequence SVG width-aware lanes Summary

**An 80+ character sequence message soft-wraps at 60 cells and widens every lane it crosses (and the lane of a self-message), proven for hand-picked and 128 generated diagrams by an oracle that reads only the drawn SVG; oversized or inconsistent IR fails closed before any placement, hostile text stays in text content and renders are byte-identical with integer-only geometry.**

## Accomplishments

- Task 1 (tracer): message and note text come from `wrap_at(text, 60)` (explicit `\n` kept), so the existing gap rule widens each crossing gap to the wrapped label box plus 8 px per side. `tests/support/svgcheck/seq.rs` holds `assert_sequence_geometry`: every box inside the viewBox, actor boxes and note boxes pairwise disjoint, every label box disjoint from other labels, actors and notes, lifeline x at the actor box centre and top at its bottom, non-self labels at least 8 px inside both endpoint lifelines, self-message loops starting and ending on their lifeline with the label right of the loop and 8 px before the next lifeline, one-lane notes centred, two-lane notes spanning both lifelines. `oracle_rejects_a_forged_overlap` edits the SVG text (label background moved onto a lifeline) and the oracle returns `Err`. Golden `30-seq-long-labels`.
- Task 2: self-message gap rule: gap `i` is also at least `SELF_W + SELF_LABEL_GAP + label width + LANE_CLEAR` for a labelled self-message on lane `i`; on the last lane the canvas grows instead. `generated_sequences_never_overlap_or_clip` (128 cases, 1-6 actors, 0-14 events, labels to 130 characters with CJK, emoji and newlines) passes, asserting all 128 drew. Goldens `31-seq-self-and-notes`, `32-seq-replies-and-wide-actors`, `33-seq-unicode`; `sequence_goldens_pass_the_oracle` also runs the oracle over every sequence case in the directory.
- Task 3: `limits::MAX_RENDER_ACTORS = 100` / `MAX_RENDER_EVENTS = 200`; `layout::seq::validate` runs before any placement in the plan's order (caps, duplicate actor id, label length through `check_label` (now `pub(crate)`), note arity, then 86-01's unknown actor in `steps`). `RenderError::TooLarge` docs and the `svg::render` doc cover sequences. Hostile text in all four slots (actor label, message, note, title) equals the sanitized, whitespace-squeezed fragment; a 64-case property over arbitrary text is accepted by the checker; no user text in any attribute; 20 fresh renders are identical for `sequence_all_constructs.vdg` and the four goldens; 64 generated sequences render to equal bytes twice; `sequence_geometry_is_integer_only` guards `layout/seq.rs`, `layout/seq/canvas.rs`, `layout/seq/tests.rs` and `svg/seq.rs` (float needles built at run time).

## Task Commits

1. Task 1: `aa4e4face` feat(86-03): 60-cell soft wrap for sequence messages with a read-back geometry oracle
2. Task 2: `bde1fa547` feat(86-03): self-message gap rule and the no-overlap/no-clip sequence property
3. Task 3: `e1b63240a` feat(86-03): sequence render caps, IR validation, hostile and determinism tests

## TDD Evidence

- Task 1 RED (`svg_sequence_geometry` + `layout::seq::`, 13 run): 10 passed, 3 failed: `tracer_long_message_wraps_and_fits` (one 87-character line, expected 2), `explicit_newlines_are_kept_and_long_lines_wrap`, `layout::seq::tests::message_lines_wrap_at_60`. `MSG_WRAP_CELLS` was defined first (unused) so the failures were runtime, not compile errors. GREEN: 40 targeted tests pass.
- Task 2 RED (18 run): 15 passed, 3 failed: `self_gap_rule`, `self_message_label_stays_before_the_next_lifeline`, and `generated_sequences_never_overlap_or_clip` (first shrunk counterexample: two actors, `a0 -> a0 "GET /orders/{id}"`, "self label ... too near lifeline"). The property found only that one defect class; I did not narrow the strategy. GREEN: 18 pass; temporary `with_cases(5000)` also green in 2.5 s (edit reverted, `with_cases(128)` back). `last_lane_self_message_widens_the_canvas` and `notes_cover_their_lanes` were already green (the 86-01 canvas shift and note width already satisfied them; they pin that).
- Task 3 RED (17 run): 13 passed, 4 failed: `caps_are_checked_before_layout`, `duplicate_actor_is_invalid_ir`, `overlong_labels_are_invalid_ir`, `note_arity_is_invalid_ir`. The hostile, determinism, attribute and integer-only tests passed on the first run: they exercise the 86-01 writer path and no defect was found there.
- Mutation proof 1 (Task 2, crossing-label term of the gap rule): copy of `layout/seq.rs`; `grep -c 'LANE_CLEAR'` 4 before and 4 after, the original line count 1 before and 1 for the mutated `max(2 * LANE_CLEAR)` after. `*g = (*g).max(l.w + 2 * LANE_CLEAR)` became `(*g).max(2 * LANE_CLEAR)`. Failed (8 of 19): `tracer_long_message_wraps_and_fits`, `multi_lane_long_message_fits_between_its_endpoints`, `explicit_newlines_are_kept_and_long_lines_wrap`, `generated_sequences_never_overlap_or_clip`, `sequence_goldens_pass_the_oracle`, `oracle_rejects_a_forged_overlap` (its un-forged precondition), `layout::seq::tests::a_label_across_two_lanes_widens_both_gaps`, `a_gap_fits_its_crossing_label_with_clearance`. Restored from the copy (`diff` identical, original line present, no `if false`), 46 targeted tests green.
- Mutation proof 2 (Task 3, render caps check): copy of `layout/seq.rs`; `grep -c MAX_RENDER_ACTORS` 2 before; the cap condition wrapped as `if false && (...)` (mutated line present 1). `caps_are_checked_before_layout` failed (1 of 12; the other 11 passed). Restored (`diff` identical, `grep -c 'if false'` 0), then 151 targeted tests green.

## Verification

- `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_sequence_safety) | binary(svg_sequence) | binary(svg_sequence_geometry) | binary(determinism_guard) | binary(svg_errors) | binary(svg_render) | binary(svg_theme) | test(/^layout::/) | test(/^svg::/)'`: 151 passed, 0 failed.
- `cargo clippy -p vox-diagram --all-targets -- -D warnings`: clean. `cargo fmt -p vox-diagram` applied. No workspace-wide command was run (lean verification rule).
- Goldens blessed by running `UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_render) & test(svg_cases)'` alone, then rerun without the variable. The first bless attempt failed on `30-seq-long-labels.vdg` itself (a 212-character label is over the 200 limit); shortened to 167 and re-blessed. Graph goldens unchanged.
- Visual review: no browser could be driven (headless Chrome, `qlmanage` and `cmp` are outside the shell allowlist), so I rendered each new SVG with ImageMagick after flattening `var()` fallbacks. ImageMagick's renderer draws text and filled rects only (no strokes or arrow markers) and has no CJK glyphs, so the by-eye check covers text placement and wrapping (60-cell wraps, labels centred between lanes, loops' labels right of their lane with clearance, notes over the right lanes, nothing clipped) but not arrows, lifelines or CJK/emoji glyphs. Those are covered by the oracle and checker (30-33 all pass `assert_sequence_geometry`) and left for the 86-04 browser review.
- `proptest-regressions` never written; `Cargo.toml`, `Cargo.lock`, `crates/workspace-hack` untouched.
- Non-blank lines, all under 400: `layout/seq.rs` 358, `layout/seq/tests.rs` 134, `layout/input.rs` 304, `svg/mod.rs` 271, `tests/support/svgcheck.rs` 359, `svgcheck/seq.rs` 274, `svg_sequence_geometry.rs` 232, `svg_sequence_safety.rs` 312.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] 86-01 test `a_huge_canvas_is_rejected_not_wrapped` invalidated by validation**
- **Found during:** Task 3 verify
- **Issue:** the test feeds a 2,000,000-character message label and expected `Geometry`. With the plan's label-length validation (`check_label`, 200) that input is now `InvalidIr` first. The canvas bound is no longer reachable through valid IR: the widest legal diagram (100 actors, 60 cells of wide Latin letters per label line, about 855 px) is about 90,000 px, under the 100,000 px `MAX_EXTENT`.
- **Fix:** the test is now `a_huge_label_is_rejected_before_layout` (asserts `InvalidIr`); the bound itself stays covered by `canvas::tests::an_oversized_canvas_is_rejected`. The canvas check remains as a defence.
- **Files modified:** `src/layout/seq/tests.rs`
- **Commit:** e1b63240a

**2. [Rule 1 - Plan arithmetic] Tracer label length**
- **Issue:** the plan names an "85-character" sentence; the one used is 87 characters (asserted in the test), which also wraps to exactly two lines at 60.

**3. [Rule 2 - Missing coverage] Goldens run through the oracle**
- Added `sequence_goldens_pass_the_oracle` so the four new cases (and any later sequence case in that directory) are checked by the geometry oracle, not only byte-compared. Also added `layout/seq/canvas.rs` to the integer-only guard.

### Other notes

- `src/svg/seq.rs` needed no change (listed in the plan's `files_modified`): the writer path was already correct for every hostile and determinism test.
- `RenderError::TooLarge` still displays "N nodes and containers, M edges" for a sequence (existing graph tests may pin the text); only the field docs were updated as the plan states.
- REND-05 left unticked on purpose (see key-decisions); `requirements-completed` is empty.

## Auth Gates

None.

## Known Stubs

None.

## Threat Flags

None. T-86-07 to T-86-10 are mitigated as planned (single writer, caps before placement mutation-proven, `InvalidIr` by kind and index, integer-only geometry guard).

## Self-Check: PASSED

- Files present: `tests/support/svgcheck/seq.rs`, `tests/svg_sequence_geometry.rs`, `tests/svg_sequence_safety.rs`, goldens `30`-`33` (`.vdg` and `.svg`).
- Commits `aa4e4face`, `bde1fa547`, `e1b63240a` are ancestors of HEAD; `git rev-list --count 8caf232d2..HEAD` is 3 and equals `actuals.commits`.
