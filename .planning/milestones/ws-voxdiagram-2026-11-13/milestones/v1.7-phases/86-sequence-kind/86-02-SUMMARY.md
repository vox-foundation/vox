---
phase: 86-sequence-kind
plan: 02
subsystem: vox-diagram
tags: [sequence-diagram, diagnostics, fixes, mermaid-syntax, repair, proptest]

requires:
  - phase: 86-sequence-kind
    provides: sequence DSL, resolve_sequence, Owner::Note, count arms (plan 01)
  - phase: 83-core-crate-parser-ir-diagnostics
    provides: fixgen, apply_fixes/repair, Mermaid-ism machinery for graph
provides:
  - "undeclared-actor declare fix (one per id, after the last actor line, first-mention lane order) and repair in one round"
  - "reserved-word for actor ids; actors/events wording in limit-nodes/limit-edges"
  - "fixgen::insert_after_line"
  - "parser/seq_mermaid.rs: sequenceDiagram header, participant / actor ... as, ->>, -->>, -->, Note over spellings with canonical rewrites; other links and side notes reported without a fix"
  - "tests/support/seq.rs: SeqModel, model_sequence, model_seq_ir_json, noisy_seq_source; sequence_props S1-S4"
affects: [86-03, 86-04]

plan_head_before: 84699c63473794f3551a6d92394a704520897215
plan_head_after: e43ea423a76df0a437b7bd07245fd08a0fc32230

actuals:
  tokens: 15400
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "sequence declare fix mirrors graph Rule R: insert_after_line for a multi-line scope with actors, insert_decl otherwise"
    - "Mermaid recognizers are Parser methods in one file, called from sequence_statement; every scan spends fuel and consumes what it scans"

key-files:
  created:
    - crates/vox-diagram/src/parser/seq_mermaid.rs
    - crates/vox-diagram/tests/sequence_check.rs
    - crates/vox-diagram/tests/sequence_mermaid.rs
    - crates/vox-diagram/tests/sequence_props.rs
    - crates/vox-diagram/tests/support/seq.rs
    - crates/vox-diagram/tests/fixtures/broken/23-sequence-reserved-actor.vdg
    - crates/vox-diagram/tests/fixtures/broken/24-sequence-mermaid-arrows.vdg
    - crates/vox-diagram/tests/fixtures/repairable/14-sequence-undeclared-actors.vdg
    - crates/vox-diagram/tests/fixtures/repairable/15-sequence-mermaid.vdg
    - crates/vox-diagram/tests/snapshots/fixes_repair__14-sequence-undeclared-actors.snap
    - crates/vox-diagram/tests/snapshots/fixes_repair__15-sequence-mermaid.snap
  modified:
    - crates/vox-diagram/src/fixgen.rs
    - crates/vox-diagram/src/resolve/seq.rs
    - crates/vox-diagram/src/parser/{mod,seq,mermaid,recover}.rs
    - crates/vox-diagram/tests/support/{mod,noisy}.rs

key-decisions:
  - "Insertion point for a declared actor: after the line of the last `actor` statement, so new lanes follow declared ones in first-mention order; `insert_decl` (first statement) when the block has no actor or its `}` shares that line. Same-offset inserts keep diagnostic order, which is source order."
  - "`-->` (headless dashed) rewrites to `reply a -> b`, like `-->>`: the DSL has no other dashed form."
  - "`participant` with a tail that is neither `as` nor the end of the statement is an unexpected-token error (actor still declared), not a Mermaid rewrite."
  - "A lexically bad id (`BadIdent`) gets the mermaid-syntax diagnostic without a fix; the lexer already reported it."
  - "The sequence printer reuses the graph `Printer` (visibility change only) rather than copying 70 lines of die-stream helpers."

patterns-established:
  - "Mutation proof: copy, mutate with a grep presence check, named test fails, restore with diff -q."

requirements-completed: []

duration: one session
completed: 2026-10-17
status: complete
---

# Phase 86 Plan 02: Sequence check/fmt parity Summary

**`sequence` now behaves like `graph` under `check`, `repair` and `fmt`: every mistake at once with codes and spans, undeclared actors declared mechanically in lane order, Mermaid sequence syntax rewritten to the canonical form (the frozen corpus block repairs to exactly the strict importer's IR), and S1-S4 properties over generated documents.**

## Accomplishments

- Task 1 (tracer): `diagram/undeclared-actor` carries a declare fix (`actor <id>: "<id>"`), once per id, after the last actor line; `repair` fixes `a -> b / b -> c / c -> b` in one round with lanes `a, b, c`. Actor ids in `RESERVED` give `diagram/reserved-word` (no fix). Cap messages say "actors" / "events" for sequences. `fixgen::insert_after_line` added (LF, CRLF, trailing comment kept, out of range `None`).
- Task 2: `sequenceDiagram` header (rewrite to `sequence {` plus the EOF `}`), `participant X [as Y]` / `actor X [as Y]`, `->>`, `-->>`, `-->`, `Note`/`NOTE OVER`, unquoted text; `--x`, `==>`, `-.->`, `-->|t|` and `Note right of` are reported without a fix and the statement still parses. `repair` of the corpus block `19-scientia-...-l112.mmd` is valid, stable and equals the importer's IR.
- Task 3: `sequence_props` (128 cases each): S1 model oracle, S2 fmt idempotence/canonical shape, S3 AST and IR preserved by fmt, S4 five Mermaid corruptions (plus brace deletion fallback) repair back to the original IR; `every_corruption_kind_applies_and_repairs` proves each kind applies.

## Task Commits

1. Task 1: `7b33eca39` feat(86-02): undeclared sequence actors repair mechanically in lane order
2. Task 2: `8b77cd58f` feat(86-02): Mermaid-isms in sequence blocks with canonical rewrites
3. Task 3: `e43ea423a` test(86-02): sequence properties S1-S4 over a model-first generator

## TDD Evidence

- Task 1 RED (`sequence_check` + `fixgen::`): 16 run, 9 passed, 7 failed: `tracer_undeclared_actors_repair_in_one_round`, `declare_fix_goes_first_without_actors`, `declare_fix_respects_a_single_line_scope`, `no_declare_fix_without_both_braces`, `reserved_actor_ids`, `caps_wording`, `all_errors_at_once`. `adversarial_sequence_inputs_are_bounded` was already green (the 86-01 parser is bounded; the test pins it). The four `insert_after_line` unit tests were written with the function (they cannot fail before it exists). GREEN: 65 targeted tests pass.
- Task 2 RED (`sequence_mermaid`): 10 run, 0 passed, 10 failed. GREEN: all 10 pass, full crate 518 passed.
- Task 3: the properties exercise code from 86-01 and Tasks 1-2, so there is no separate RED; they passed on the first run (128 cases) and at 2000 cases (temporary `with_cases(2000)` edit, restored; `PROPTEST_CASES` is ignored by an explicit `with_cases`): 5 passed in under 1 s, no `proptest-regressions/` written.
- Mutation proof 1 (Task 1): copy of `resolve/seq.rs`, `insert_after_line(self.src, at, &decl)` replaced by `insert_decl(self.src, open, close, &decl)` (`grep -c insert_after_line` 2 before, 1 after). `tracer_undeclared_actors_repair_in_one_round` failed on lane order (`["actor b","actor c","actor a"]` vs `a, b, c`); the other 7 passed. Restored from the copy (`diff -q` identical, count back to 2), suite green.
- Mutation proof 2 (Task 2): copy of `seq_mermaid.rs`, the `->>` fix `replace(span, "->")` changed to `"-->"` (presence grep 1 before, 0 after, mutation present 1). `arrow_solid`, `unquoted_message_text` and `corpus_block_repairs_to_the_imported_ir` failed (7 passed, 3 failed). Restored (`diff -q` identical), full suite green.

## Verification

- `cargo nextest run -p vox-diagram` (whole crate, `INSTA_UPDATE=no`): 523 passed, 2 skipped (pre-existing slow tests).
- `cargo clippy -p vox-diagram --all-targets -- -D warnings`: clean. `cargo fmt -p vox-diagram` applied.
- New snapshots created with `INSTA_UPDATE=always` on `fixes_repair_seeded_set` only and read before commit: `14` shows `gateway`, `db`, `cache` declared after `server` in first-mention order; `15` shows the header rewritten, `%%` turned into `//`, participants as actors, `reply ... ->` for the dashed returns and a lowercase `note over U,S`.
- No `*.snap.new`, no `proptest-regressions/`; `Cargo.toml`, `Cargo.lock`, `crates/workspace-hack` untouched.
- Non-blank line counts, all under 400: `resolve/seq.rs` 196, `fixgen.rs` 211, `parser/mod.rs` 355, `parser/seq.rs` 135, `parser/seq_mermaid.rs` 208, `parser/mermaid.rs` 144, `tests/sequence_mermaid.rs` 183, `tests/support/seq.rs` 191, `tests/sequence_props.rs` 212.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Plan arithmetic] Tracer diagnostic count**
- **Found during:** Task 1 test design
- **Issue:** the plan says `tracer_undeclared_actors_repair_in_one_round` gives "three" undeclared-actor errors for `a -> b / b -> c / c -> b`. `check` reports every use (both endpoints of each message), so the source has five: `b`@3, `b`@4, `c`@4, `c`@5, `b`@5.
- **Fix:** the test asserts those five (lines `[3,4,4,5,5]`) and that exactly the first per id (`b`@3, `c`@4) carries a fix, which is the behaviour the plan describes.
- **Commit:** 7b33eca39

**2. [Rule 1 - Plan definition] "Plain" label in S4**
- **Found during:** Task 3
- **Issue:** "ASCII letters, digits and single inner spaces" admits a word that starts with a digit; unquoted, that is a lexer `invalid-identifier` error with no fix, so the corruption would not be repairable (not a repair defect).
- **Fix:** `plain` is the one `repair_props.rs` already uses (each word starts with a letter or `_`).
- **Commit:** e43ea423a

**3. [Rule 3 - Blocking] `tests/support/noisy.rs` edited**
- **Issue:** the plan lists only `support/seq.rs` and `support/mod.rs`; reusing the graph die-stream `Printer` needs it visible to `support::seq`.
- **Fix:** visibility only (`pub(super)` on the struct, three fields and the helper methods) plus a `Printer::new`. No behaviour change; the graph suites are unchanged and green.
- **Commit:** e43ea423a

### Other notes

- `reply` followed by an `MArrow` (`reply -->> b`, an actor named `reply`) is now a message from `reply`, not the "a message after `reply`" error (the guard excludes `MArrow` as well as `Arrow`).
- `resolve.rs` was not touched: `braces()` and `RESERVED` are reachable from the child module `resolve::seq`.
- The S4 corruption kinds are 0-4 plus the brace-deletion fallback (kind 5), as in `repair_props.rs`.
- Known limit (inherited from the graph path): unquoted Mermaid text containing non-ASCII words lexes as `invalid-identifier` (no fix) before any rewrite sees it; quoted text is unaffected. Unquoted message text also keeps a trailing `;` inside the label (the graph `unquoted_label` behaves the same); the fixture uses `participant DB;` and a quoted label instead.

## Auth Gates

None.

## Known Stubs

None.

## Threat Flags

None. Every new fix text is built by `fixgen` (`quote`, `mermaid_label`, `declarable` ids) or is a fixed keyword/arrow; fix spans are token spans or `insert_after_line`/`insert_decl` offsets that `apply_fixes` re-vets (T-86-04). The Mermaid recognizers spend fuel and consume what they scan; `adversarial_sequence_inputs_are_bounded` runs five inputs just under the 64 KiB cap, all finishing well under 10 s (T-86-05). `corpus_block_repairs_to_the_imported_ir` ties the lenient path to the strict importer (T-86-06). No dependency or manifest change (T-86-SC).

## Requirements

- DGM-02 was already ticked by 86-01; nothing newly completed here. REND-05 stays open for 86-03.

## For the next plans

- 86-03: this plan touched no `src/layout/`, `src/svg/`, `src/limits.rs` or `tests/support/svgcheck*`.
- 86-04: `repair` of a Mermaid sequence block gives valid DSL whose IR equals `import(...).sequence` (spans aside); the snapshot of `15-sequence-mermaid` shows the repaired (not yet `fmt`-ed) text.

## Self-Check: PASSED

All created files exist (see `git status`/`git diff --name-only 84699c63..HEAD`), commits `7b33eca39`, `8b77cd58f`, `e43ea423a` are in `git log`, full crate suite and clippy pass.
