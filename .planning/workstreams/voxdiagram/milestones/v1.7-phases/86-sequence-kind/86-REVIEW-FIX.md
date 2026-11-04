---
phase: 86-sequence-kind
fixed_at: 2026-10-17T00:00:00Z
review_path: .planning/workstreams/voxdiagram/phases/86-sequence-kind/86-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 86: Code Review Fix Report

**Fixed at:** 2026-10-17
**Source review:** .planning/workstreams/voxdiagram/phases/86-sequence-kind/86-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-01, WR-01..WR-04; Info excluded by `fix_scope: critical_warning`)
- Fixed: 5
- Skipped: 0

Verification ran in the main checkout (`workflow.use_worktrees` is `false`, no worktree was
created): `INSTA_UPDATE=no cargo nextest run -p vox-diagram` 563 passed / 3 skipped;
`cargo clippy -p vox-diagram --all-targets -- -D warnings` clean; `cargo fmt -p vox-diagram
--check` clean; `node --test tests/unit/mermaid-oracle.test.mjs` (docs-astro) 77/77. The
no-overlap geometry property was also run once at 4000 cases (temporary bump, reverted) with no
counterexample. No `proptest-regressions` files and no Cargo/lock edits are committed.

Every finding started from a failing regression test on the reviewer's exact input. CR-01 was
additionally proven by mutation: with the `;` stop disabled the new tests and property S5 fail.

## Fixed Issues

### CR-01: `repair` of a Mermaid message with `;` silently merges or corrupts statements

**Files modified:** `crates/vox-diagram/src/parser/recover.rs`, `crates/vox-diagram/tests/sequence_props.rs`, `crates/vox-diagram/tests/sequence_repair_review.rs` (new)
**Commit:** 52aacfad4 (shared with WR-02)
**Applied fix:** In sequence mode `unquoted_label` now stops at `Semi`, so the existing M8
terminator fix owns the `;`. A `;` that closes a `#59;`-style entity does not stop the label
(otherwise `x#59;y` would be cut, a new regression against the importer, which decodes it as
text). Graph mode is untouched. Semantics versus the importer: the importer refuses any
statement containing a raw `;`; repair treats it as the Mermaid statement terminator, which is
what Mermaid does, so the reviewer's input now repairs to the three messages `x`, `y`, `z`.
S4 corruption kinds were extended as a new property `s5_semicolons_br_and_undeclared_actors_repair`
(plus a deterministic `every_review_corruption_applies_and_repairs`): `;` joining two statements
after an unquoted label, a trailing `;`, unquoted `<br>` / `<br/>` / `<BR>`, and a deleted actor
declaration still mentioned by an event. S5 builds on a "tidy" model so each kind always has a
site and repairs back to the original IR.

### WR-01: unterminated Mermaid alias is repaired into garbage

**Files modified:** `crates/vox-diagram/src/parser/seq_mermaid.rs`
**Commit:** 5efafaa9c (regression test `wr01_...` lives in `tests/sequence_repair_review.rs`, committed with 52aacfad4)
**Applied fix:** `mermaid_actor` omits the participant rewrite when the scanned tail ends in an
unterminated string; the lexer's closing-quote fix lands first and the next round rewrites the
now well-formed `participant a as "unterminated"` to `actor a: "unterminated"`. Converges, and the
document is a repair fixed point. Unit test added in the module.

### WR-02: Mermaid `<br>` decoded for aliases but kept literal for messages and notes

**Files modified:** `crates/vox-diagram/src/parser/recover.rs`, `crates/vox-diagram/src/fixgen.rs`, `crates/vox-diagram/tests/sequence_repair_review.rs`
**Commit:** 52aacfad4 (shared with CR-01, same code path)
**Applied fix:** In sequence mode the unquoted label goes through `mermaid_label`, like aliases.
`mermaid_label` now decodes `<br>`, `<br/>`, `<br />` in any case (as the importer does) and
trims after decoding. The test asserts repair output equals the strict importer's IR for alias,
message and note including `<BR>` and `<br />`. Note: single-quote stripping (`'x'`) is now graph-only; the importer keeps such quotes literally in
sequence text, and no existing test relied on it.

### WR-03: multi-hop message label centred on the lifelines it crosses

**Files modified:** `crates/vox-diagram/src/layout/seq.rs`, `crates/vox-diagram/tests/support/svgcheck/seq.rs`, `crates/vox-diagram/tests/svg_sequence_geometry.rs`, goldens below
**Commit:** 6aa16ebf5 (shared with WR-04)
**Applied fix:** `between` centres the label in the single gap at the middle of its span
(`lo + (hi - lo - 1) / 2`) instead of on the arrow midpoint. `lane_xs` already widened every
spanned gap to hold the label, so no lane moved. The independent oracle now rejects a label
within 8 px of any lifeline strictly between its endpoints; a forged-placement test proves the
oracle fires. Status: fixed. Layout choice for human review: label position is no longer the
arrow midpoint for multi-hop messages (it sits in the middle gap).

### WR-04: wide one-actor note covers neighbouring lifelines

**Files modified:** `crates/vox-diagram/src/layout/seq.rs`, `crates/vox-diagram/src/layout/seq/lanes.rs` (new, `lane_xs` + `note_w` moved out to keep `seq.rs` under 400 lines), `crates/vox-diagram/src/layout/seq/tests.rs`, `crates/vox-diagram/tests/support/svgcheck/seq.rs`, `crates/vox-diagram/tests/svg_sequence_geometry.rs`
**Commit:** 6aa16ebf5 (shared with WR-03)
**Applied fix:** `lane_xs` gains a notes pass: the gaps directly outside a note's lanes are
widened to its overhang plus 8 px clearance (+1 half-px rounding slack), using one shared
`note_w`. First/last lane have no outer gap and the canvas grows as before. The oracle now also
requires that no lifeline of an actor outside the note lies under it (with 8 px clearance); a
forged-note test proves it. Applies to two-actor notes with wide text as well.

**Goldens re-blessed (blessing tests alone, diffs reviewed):** `tests/fixtures/svg/cases/30-seq-long-labels.svg`,
`tests/fixtures/svg/cases/32-seq-replies-and-wide-actors.svg`,
`tests/fixtures/svg/sequence-corpus/19-scientia-automated-research-historical-extension-research-2026-l112.svg`.
Only `x` of multi-hop message label rects/text changed (labels moved left into the middle gap);
lane positions, canvas size and all other elements are byte-identical. No golden contained a
wide note, so no note golden changed.

## Skipped Issues

None. IN-01..IN-04 are out of scope (`fix_scope: critical_warning`).

---

_Fixed: 2026-10-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
