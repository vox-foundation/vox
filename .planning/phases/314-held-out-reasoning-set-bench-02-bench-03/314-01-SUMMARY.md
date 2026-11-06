---
phase: 314-held-out-reasoning-set-bench-02-bench-03
plan: 01
subsystem: testing
tags: [eval, reasoning-bench, jsonl, interpreter-oracle, vox-eval, vox-compiler]

requires:
  - phase: 313
    provides: grade_final_answer / score_completions in vox-eval qwen_reasoning (unchanged here)
provides:
  - three tracked 32-item slices (gsm8k_think, math500_think, vox_think)
  - vox_eval::reasoning_bench::load_slice with SLICE_SIZE, SLICE_NAMES, MAX_SLICE_BYTES, ReasoningItem
  - interpreter oracle test proving every vox_think gold
affects: [314-02, 315, 316]

actuals:
  tokens: 28000
  tasks: 3
  commits: 3
plan_head_before: e3301eb62bf2bde08f999ad51e32f007cfece107
plan_head_after: b447422633df3e79805baf690cceb05166d73926

tech-stack:
  added: []
  patterns: [slice loader with pinned count, size cap and id uniqueness; gold derived from interpreter run]

key-files:
  created:
    - crates/vox-eval/src/reasoning_bench.rs
    - crates/vox-compiler/tests/vox_think_oracle_test.rs
    - mens/data/vox_think/reasoning.jsonl
  modified:
    - .gitignore
    - crates/vox-eval/src/lib.rs
    - mens/data/gsm8k_think/reasoning.jsonl
    - mens/data/math500_think/reasoning.jsonl

key-decisions:
  - "gsm8k_think cut to the first 32 rows byte-for-byte; only the source suffix changed from 64 to 32"
  - "math500_think content and source unchanged; no licence claim added"
  - "vox_think gold = value returned by the interpreter for main, read from the oracle failure output, never hand-computed"
  - "Size cap enforced with Read::take(MAX_SLICE_BYTES + 1); no new crate edge"

patterns-established:
  - "load_slice refuses missing/empty id (no item-N default), duplicate ids, blank question/answer, wrong count, oversize file"

requirements-completed: [BENCH-02]

coverage:
  - id: D1
    description: "Three 32-item slices tracked in git; train.jsonl still ignored"
    requirement: BENCH-02
    verification:
      - kind: unit
        ref: "cargo test -p vox-eval --lib reasoning_bench (checked_in_gsm8k_and_math500_slices_have_32_pinned_rows)"
        status: pass
    human_judgment: false
  - id: D2
    description: "load_slice refusal rules (31/33 rows, duplicate id, empty fields, missing id, oversize, malformed JSON)"
    requirement: BENCH-02
    verification:
      - kind: unit
        ref: "cargo test -p vox-eval --lib reasoning_bench"
        status: pass
    human_judgment: false
  - id: D3
    description: "32 vox_think items with interpreter-proven integer golds, no Plan/Syntax/Implementation template, no golden copy"
    requirement: BENCH-02
    verification:
      - kind: unit
        ref: "cargo test -p vox-compiler --test vox_think_oracle_test"
        status: pass
    human_judgment: false

duration: 70min
completed: 2026-10-26
status: complete
---

# Phase 314 Plan 01: Held-out reasoning slices Summary

**Three checked-in 32-item reasoning slices behind one validating `load_slice`, with 32 hand-authored `vox_think` items whose golds are the interpreter's own `main()` results.**

## Performance

- **Tasks:** 3
- **Files modified:** 7 (3 created, 4 modified)

## Accomplishments

- `.gitignore` negation `!mens/data/*_think/reasoning.jsonl` tracks the slices; `mens/data/train.jsonl` stays ignored.
- `gsm8k_think` cut to `gsm8k-test-0000..0031`; `math500_think` unchanged (`test/precalculus/807.json` .. `test/prealgebra/1302.json`).
- `vox_think`: 11 trace, 11 debug, 10 plan-check items (ids `vox-think-0000..0031`), pure programs using `fn`/`type`/`match`/lists/loops, each question ending with the fixed final-integer line.
- Oracle test runs every program through lex, parse, lower and eval and compares `VoxValue::Int` to the gold.

## Task Commits

1. Task 1 (tracer): `72f08843c` feat(314-01): check in gsm8k and math500 reasoning slices with a pinned loader
2. Task 2: `59462208e` test(314-01): pin load_slice refusal rules
3. Task 3: `b44742263` feat(314-01): add interpreter-checked vox_think slice

## Verification

| Command | Result |
|---|---|
| `cargo test -p vox-eval --lib reasoning_bench` after Task 1 | 3 passed, 0 failed |
| same, after Task 2 | 10 passed, 0 failed |
| same, after Task 3 | 14 passed, 0 failed |
| `cargo test -p vox-compiler --test vox_think_oracle_test` | 3 passed, 0 failed |
| Red runs | Task 1 with a stubbed loader: 3 passed, 7 failed (the 3 passes are the missing-file, empty-input and stale-source tests, which a stub satisfies). Task 3 with no `vox_think` file: 4 failed. |
| git checks | `git ls-files` lists all 3 slice files; `git check-ignore` exits 1 for each and 0 for `mens/data/train.jsonl`; vox_think has 32 lines; no CR bytes in any slice |

Mutation check: changed the `vox-think-0003` gold 55 to 56; the oracle failed with `vox-think-0003: expected answer 56 but main returned Ok(Int(55))`. Restored; oracle green again (3 passed).

All 96 golds round-trip through the unchanged `grade_final_answer` (boxed gold true, boxed 999999 false). No model was run and no score was produced.

## Deviations from Plan

None to scope. Notes:

- Task 1's committed test module held only its 3 tests; Task 2's 7 tests were added in its own commit, as the plan splits them.
- Interpreter quirks found while authoring, and avoided in the data: `.pop()` returns a bare value (not `Option`), `state` is a reserved word, list slicing `xs[0..n]` is unsupported, and struct construction by `Point(3, 4)` fails at run time. Programs use `.push`, indexed assignment, `.unwrap()` on indexing, `range`, ADTs with payloads, and `match`.
- The programs were run only through the interpreter; `vox check` (typecheck) was not run on them.
- `StrReplace` was not used; all file writes were whole-file writes or sed/awk against on-disk bytes, with header and line-count checks after each.

## Known Stubs

None.

## Self-Check: PASSED

Files present: `reasoning_bench.rs` (389 lines), `vox_think_oracle_test.rs` (103 lines), all three `reasoning.jsonl` files. Commits `72f08843c`, `59462208e`, `b44742263` are ancestors of HEAD.
