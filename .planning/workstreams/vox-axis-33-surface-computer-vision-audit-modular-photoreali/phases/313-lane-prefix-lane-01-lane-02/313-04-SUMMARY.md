---
phase: 313-lane-prefix-lane-01-lane-02
plan: 04
subsystem: training
tags: [lane, qwen3, chatml, fusion, tests-only, loss-mask]
status: complete

requires:
  - phase: 313-lane-prefix-lane-01-lane-02
    provides: "Lane, chatml_prefix_for_lane, lane-aware serve (313-01..03)"
provides:
  - "LANE-02 lock tests: empty think block stays in the prompt (outside the loss), reasoning rows are never rewritten, no double prefix, no second flag"
affects: []

requirements-completed: [LANE-02]

actuals:
  tokens: 1500
  tasks: 2
  commits: 3
plan_head_before: 5bd33c8980393a42c6d4f8b97458bec1404d98d7
plan_head_after: 013bd55c2

tech-stack:
  added: []
  patterns:
    - "Prefix string is the loss boundary (encoding.rs aligned_prefix_token_len), so string-level prefix asserts lock the mask"

key-files:
  created: []
  modified:
    - crates/vox-eval/src/qwen_reasoning.rs
    - crates/vox-plugin-mens-candle-core/src/training_text.rs
    - crates/vox-populi/src/mens/tensor/training_text.rs

key-decisions:
  - "Task 1 committed as two commits (vox-eval, then candle-core), each naming only its own file"
  - "Populi file committed whole: it carried pre-existing uncommitted production hunks (see Deviations)"

duration: ~10min
completed: 2026-10-26
---

# Phase 313 Plan 04: LANE-02 fusion locks Summary

Tests-only locks for the codegen training fusion on vox-eval, candle-core and the legacy vox-populi copy; every new test passed against current production code, so there are no findings.

## Tests added

| File | Test |
|---|---|
| vox-eval `qwen_reasoning.rs` | `assistant_body_does_not_double_the_empty_think_block` |
| vox-eval `qwen_reasoning.rs` | `codegen_user_with_an_existing_flag_gets_no_second_flag` |
| candle-core `training_text.rs` | `message_rows_leave_a_reasoning_turn_unrewritten` |
| candle-core `training_text.rs` | `supervised_prefix_adds_no_second_flag` |
| populi `training_text.rs` | `direct_answer_puts_empty_think_in_the_prompt_not_the_loss` |
| populi `training_text.rs` | `reasoning_trace_keeps_the_thinking_mode_entry_point` |

candle-core already had same-named `direct_answer_...` and `reasoning_trace_...` tests; they were left as is and populi mirrors them.

## Verification

- `cargo test -p vox-eval qwen_reasoning`: 13 passed, 0 failed; both new tests `... ok`.
- `cargo test -p vox-plugin-mens-candle-core training_text`: 12 passed, 0 failed; `message_rows_leave_a_reasoning_turn_unrewritten`, `supervised_prefix_adds_no_second_flag` and `reasoning_trace_keeps_the_thinking_mode_entry_point` all `... ok`.
- `cargo test -p vox-populi --features mens-train --lib training_text`: 5 passed, 0 failed; both new populi tests `... ok`. (The plan's literal command without `--lib` also passes but prints the lib result far above many 0-test integration binaries.)
- Diff of each file against its pre-task `/tmp` copy: 0 removed lines; only lines inside `#[cfg(test)] mod tests` added (eval +15, candle-core +23, populi +30).
- `rustfmt --edition 2024 --check` clean on all three files.
- Mutation check (populi): replaced the `QWEN_EMPTY_THINK` tail in `chatml_prefix_open_assistant` with `""`, confirmed edit present; `direct_answer_puts_empty_think_in_the_prompt_not_the_loss` FAILED (4 passed, 1 failed). Restored from a byte copy (`cmp` identical, mutation marker count 0).

## Commits

- `ff6782176` test(313-04): vox-eval locks
- `fa70fab00` test(313-04): candle-core locks
- `013bd55c2` test(313-04): vox-populi locks

## Deviations from Plan

**1. [Process] Task 1 split into two commits** (one per file) instead of one. No content impact.

**2. [Scope note] The populi commit includes foreign production changes.** Before this plan started, `crates/vox-populi/src/mens/tensor/training_text.rs` was already modified and uncommitted (+54/-11 vs HEAD): `chatml_prefix_open_assistant` gained an `assistant` parameter and the empty-think tail, `chatml_supervised_text` gained fusion, `chatml_turns_*`/`fuse_turns` were added, and `hf_tokenize_chatml_supervised` call sites were updated. None of that is mine; I did not edit a non-test line (diff vs the pre-task copy shows added test lines only). The new tests need that signature, so a tests-only patch on HEAD would not compile; commit `013bd55c2` therefore shows 84 insertions / 11 deletions. Reviewer: treat the production hunks in that commit as another author's work. Other callers of the 4-argument `chatml_prefix_open_assistant` in `vox-plugin-mens-candle-{cuda,metal}` and `vox-ml-cli` (`eval_local_prompt.rs`) were not checked.

## Findings

None. No new test failed against current code.

## Known Stubs

None.

## Open edge (unchanged from plan)

LANE-02 edge-probe row remains unresolved: tests are string-level proxies; tokenizer alignment at the prefix boundary is not asserted.

## Self-Check: PASSED

- Files exist; commits `ff6782176`, `fa70fab00`, `013bd55c2` are ancestors of HEAD.
