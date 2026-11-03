---
phase: 86-sequence-kind
plan: 01
subsystem: vox-diagram
tags: [sequence-diagram, dsl, parser, svg, layout, mermaid-export]

requires:
  - phase: 83-core-crate-parser-ir-diagnostics
    provides: lexer, error-recovering parser, resolver idioms, SequenceIr types
  - phase: 84-mermaid-emitter-strict-importer
    provides: export_sequence (opaque p<i> participants)
  - phase: 85-graph-layout-deterministic-svg
    provides: single escaping Writer, shapes::write_node/label, svgcheck oracle
provides:
  - "Ir::Sequence(SequenceIr) and DiagramKind { Graph, Sequence } on Document"
  - "sequence DSL: actor declarations, messages, reply (dashed), self-messages, note over one or two actors"
  - "diagram/undeclared-actor (30 codes)"
  - "check dispatches on the document kind; fmt, mermaid::export and svg::render handle Ir::Sequence"
  - "layout::seq::place: integer half-pixel lane/row/note/label geometry with the width-aware gap rule"
  - "svg::seq::render_sequence and the shared svg::open_document"
  - "svgcheck allowlist split into support/svgcheck/allow.rs, extended for lifelines, notes, dashed strokes"
affects: [86-02, 86-03, 86-04, phase-87, phase-88, phase-89]

plan_head_before: f2db2b26b8847bd4c0dad8e62f8570d92d22bde6
plan_head_after: f1f80142151f3bd3800d1674691b44168472adbd

actuals:
  tokens: 22300
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "contextual keywords: actor/reply/note/over are plain identifiers unless the next token says otherwise"
    - "sequence geometry in i64 half-pixels, narrowed once in a canvas stage that fails closed past 100 000 px"
    - "one SVG preamble (open_document) shared by graph and sequence"

key-files:
  created:
    - crates/vox-diagram/src/parser/seq.rs
    - crates/vox-diagram/src/resolve/seq.rs
    - crates/vox-diagram/src/layout/seq.rs
    - crates/vox-diagram/src/layout/seq/canvas.rs
    - crates/vox-diagram/src/layout/seq/tests.rs
    - crates/vox-diagram/src/svg/seq.rs
    - crates/vox-diagram/tests/sequence_dsl.rs
    - crates/vox-diagram/tests/svg_sequence.rs
    - crates/vox-diagram/tests/support/svgcheck/allow.rs
    - crates/vox-diagram/tests/fixtures/valid/sequence_all_constructs.vdg
    - crates/vox-diagram/tests/fixtures/broken/21-sequence-undeclared-actor.vdg
    - crates/vox-diagram/tests/fixtures/broken/22-sequence-duplicate-actor.vdg
  modified:
    - crates/vox-diagram/src/{ir,ast,check,codes,diag,lib,print,sequence_ir,resolve}.rs
    - crates/vox-diagram/src/parser/{mod,stmt,recover}.rs
    - crates/vox-diagram/src/svg/mod.rs
    - crates/vox-diagram/src/layout/mod.rs
    - crates/vox-diagram/src/mermaid/{export,mod,export_seq}.rs
    - crates/vox-diagram/tests/support/svgcheck.rs
    - crates/vox-diagram/tests/fixtures/broken/10-unknown-diagram-kind.vdg
    - crates/vox-diagram/tests/fixtures/codes.golden.txt

key-decisions:
  - "actor, reply, note and over are contextual, not reserved: `actor <id>` declares, `reply <id> ->` is a dashed message, `note over` starts a note, and the same words before `->` are ordinary actor ids. Mermaid's parser accepts `participant note` and the importer case 12-seq-words declares Note/loop, so reserving them would refuse blocks Mermaid reads. The graph RESERVED list stays the only reserved list (reserved-word for actor ids is 86-02)."
  - "Stats for a sequence: nodes = actors, edges = events (messages plus notes), containers and max_depth 0; documented on the Stats fields."
  - "Sequence geometry runs in i64 and is narrowed to Half once, after a 100 000 px canvas check, so a hand-built IR with huge labels returns RenderError::Geometry instead of overflowing."
  - "Lifeline bottoms are part of the canvas bounds, so nothing drawn can lie outside the viewBox."

patterns-established:
  - "Parser.seq selects sequence_statement() in statement(); the graph arm moved into graph_statement()."
  - "Owner::Note: an empty note text is reported with no fix."
  - "Mutation proof on the gap rule: deleting the label term fails three tests."

requirements-completed: [DGM-02]

duration: one session
completed: 2026-10-17
status: complete
---

# Phase 86 Plan 01: Sequence Kind Summary

**`sequence { actor .. a -> b: ".." reply .. note over .. }` now goes lexer, parser, resolver, `check`, `Ir::Sequence`, `fmt`, `mermaid::export` and a checker-clean SVG with width-aware lanes, using the Phase 85 single escaping writer.**

## Accomplishments

- Tracer (Task 1): two actors and one message pass through every stage. `check` dispatches on `Document.kind`, `fmt` prints the canonical form, `mermaid::export` equals `export_sequence`, and `svg::render` draws lifelines, actor boxes, an arrow with its marker and a label whose background box lies strictly between the lifelines. 20 renders give one string.
- Width-aware lanes: each gap is at least the widest crossing label box plus 8 px each side (a 70-character label from lane 0 to lane 2 widens both gaps).
- Every `SeqEvent` shape renders: solid and dashed messages, self-message loops on the right, notes over one or two actors. A hand-built IR naming an unknown actor is `RenderError::InvalidIr`, never a panic.
- Expansion (Task 2): `reply`, self-messages and `note over a` / `note over a, b` parse, format canonically and round trip through the canonical fixture `sequence_all_constructs.vdg` (fmt fixed point, export has `-->>` and `Note over p0,p2`, render passes the checker with 2 notes).
- `diagram/undeclared-actor` (30 codes), `duplicate-id` with a related span; the IR is withheld on either.
- Graph behaviour unchanged: every graph golden is byte-identical (`git diff --quiet -- tests/fixtures/svg`).

## Task Commits

1. Task 1 (tracer): `88d76fdf8` feat(86-01): sequence kind tracer from DSL to checker-clean SVG
2. Task 2: `f1f801421` feat(86-01): reply, self-messages and note over in the sequence DSL

## TDD Evidence

- Task 1 RED: `cargo nextest ... binary(sequence_dsl) | binary(svg_sequence)` failed to compile: E0432 unresolved import `vox_diagram::ast::DiagramKind`, E0599 no variant `Sequence` on `Ir`, E0609 no field `kind` on `Document`, no field `lifelines` / `edge_paths` on `Svg`. GREEN: 75 targeted tests pass; full crate 487 then 489 pass.
- Task 2 RED (runtime): 16 run, 13 passed, 3 failed: `empty_labels_follow_the_owner_rules`, `reply_self_and_notes_round_trip`, `keywords_are_contextual`. `reply_needs_a_message` and `note_over_three_actors_is_one_error` were already green at that point (Task 1 reported one `unexpected-token` for them by accident), so I tightened them with message assertions ("a message after `reply`", "expected `:`") before implementing. I did not rerun them red afterwards; they pass for the right reason now. GREEN: full crate 494 passed, 2 skipped (pre-existing slow tests).
- Mutation proof (repo rule): `cp` of `layout/seq.rs`, then `*g = (*g).max(l.w + 2 * LANE_CLEAR)` replaced by `.max(0)` (`grep -c LANE_CLEAR` 3 before, 2 after). `layout::seq::tests::a_label_across_two_lanes_widens_both_gaps`, `a_gap_fits_its_crossing_label_with_clearance` and `svg_sequence::tracer_gap_fits_crossing_labels` failed (7 passed, 3 failed). Restored from the copy (`diff -q` identical, count back to 3), then the full suite was rerun green.

## Verification

- `cargo check -p vox-diagram --all-targets`: clean, no warnings.
- `INSTA_UPDATE=no cargo nextest run -p vox-diagram`: 494 passed, 2 skipped.
- `cargo clippy -p vox-diagram --all-targets -- -D warnings`: clean.
- Line counts (non-blank): `parser/mod.rs` 354, `parser/stmt.rs` 353, `resolve.rs` 339, `svg/mod.rs` 269, `layout/seq.rs` 318, `layout/seq/canvas.rs` 134, `svg/seq.rs` 262, `parser/seq.rs` 131, `resolve/seq.rs` 145, `svgcheck.rs` 358; all under 400.
- `Cargo.toml`, `Cargo.lock`, `crates/workspace-hack/` untouched (`git diff --quiet` between the plan base and HEAD).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `layout/seq.rs` over the 400-line cap**
- **Found during:** Task 1 (after `cargo fmt -p vox-diagram`, 419 non-blank lines)
- **Fix:** moved the canvas stage (`Bx`, `Raw`, `finish`, `MAX_EXTENT`) into `layout/seq/canvas.rs`, with two inline tests there. `place` and the geometry constants stay in `layout/seq.rs` as specified.
- **Commit:** 88d76fdf8

**2. [Rule 2 - Missing critical functionality] Overflow-safe geometry**
- **Found during:** Task 1
- **Issue:** a hand-built `SequenceIr` with huge labels could overflow `i32` half-pixel sums (`measure` saturates at `i32::MAX`; lanes multiply it).
- **Fix:** working arithmetic in `i64`; the finished canvas is checked against 100 000 px (the graph layout's `MAX_COORD_PX`) and narrowed once; over the limit is `RenderError::Geometry`. Test `a_huge_canvas_is_rejected_not_wrapped`. Still integer-only (no float type).
- **Commit:** 88d76fdf8

**3. [Rule 1 - Bug] Self-message label could start above the row cursor**
- **Found during:** Task 1 geometry design
- **Issue:** with y0 = cursor and the label centred on the loop, a label taller than the 20 px loop (a single line is 26 px) starts above the cursor and can overlap the previous row.
- **Fix:** the label box top is clamped to the cursor and the loop is centred on the label (identical to the spec when the label fits inside the loop). Label left edge, loop shape and row advance are as specified.
- **Commit:** 88d76fdf8

### Other notes

- `statement()` now delegates the existing graph dispatch to a new `graph_statement()` (behaviour unchanged) so the `seq` branch stays small. `mermaid/mod.rs` and `mermaid/export_seq.rs` doc comments no longer say "Phase 86 routes". Neither file is in the plan's file list.
- Lifeline bottoms are included in the canvas bounds (the plan lists rects and message points only); otherwise the last row gap would be clipped.
- The plan's Task 1 verify filter and acceptance greps all hold (`Sequence(SequenceIr)`, `diagram/undeclared-actor` in codes and golden, `pie {` in fixture 10, `vd-lifeline` and `stroke-dasharray` in `allow.rs`).

## Auth Gates

None.

## Known Stubs

None.

## Threat Flags

None. Sequence labels reach the SVG only through `Writer` (`esc`) as `<tspan>` text; ids come from the validated prefix via the shared `open_document`; the independent `svgcheck` allowlist accepts only the new classes `vd-lifelines`, `vd-lifeline`, `vd-notes`, `vd-note`, `vd-note-label` and the style property `stroke-dasharray` (T-86-01). Sequence statements use the existing statement loop, fuel and `count` caps (T-86-02). Actor lookup is a `BTreeMap`; an unknown id is `InvalidIr` (T-86-03).

## Requirements

- DGM-02 marked complete: actors, ordered messages, dashed returns and `note over` are authorable, format canonically, check and render.
- REND-05 left open: lane widening for crossing messages is in, but self-message labels are not yet kept before the next lifeline and long labels are not soft-wrapped (both are 86-03).

## For the next plans

- 86-02: `Parser.seq` and `sequence_statement` are in `parser/seq.rs`; `Owner::Note` exists; `resolve_sequence` reports `undeclared-actor` with no fix yet; `RESERVED` is not applied to actor ids yet.
- 86-03: constants (`ACTOR_GAP`, `LANE_CLEAR`, `LABEL_PAD`, `FIRST_ROW`, `ROW_GAP`, `SELF_W`, `SELF_H`, `SELF_LABEL_GAP`, `NOTE_MIN_W`, `NOTE_OVERHANG`) live in `layout/seq.rs`; `label_box` uses `wrap_at(text, usize::MAX)`; the checker exposes `lifelines`, `notes`, `edge_paths`.

## Self-Check: PASSED

Created files exist (verified with `git status` and the test runs), commits `88d76fdf8` and `f1f801421` are in `git log`, full crate suite and clippy pass.
