---
phase: 313-lane-prefix-lane-01-lane-02
reviewed: 2026-10-26T21:21:35Z
depth: standard
files_reviewed: 9
files_reviewed_list:
  - crates/vox-eval/src/qwen_reasoning.rs
  - crates/vox-eval/src/lib.rs
  - crates/vox-ml-cli/src/commands/ai/serve/schema.rs
  - crates/vox-ml-cli/src/commands/ai/serve/handlers.rs
  - crates/vox-ml-cli/src/commands/ai/serve/worker.rs
  - crates/vox-plugin-mens-candle-core/src/training_text.rs
  - crates/vox-plugin-mens-candle-cuda/src/inference.rs
  - crates/vox-plugin-mens-candle-metal/src/inference.rs
  - crates/vox-populi/src/mens/tensor/training_text.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 313: Code Review Report

**Reviewed:** 2026-10-26T21:21:35Z
**Depth:** standard
**Files Reviewed:** 9
**Status:** issues_found

## Summary

Reviewed commits 00375744d, faaf67b09, 85e0e7efb, 5994c9524, dfd6ce41a, a83d9c14e, df367d4e0, ff6782176, fa70fab00, 013bd55c2. `cargo test -p vox-eval qwen_reasoning` passes (13/13).

The phase goal holds. Codegen requests, via the plugin raw wrap and chat flatten, and codegen training rows all get `/no_think` plus the prefilled empty think block. Hub and agent get the bare `assistant\n` opener with `/no_think` stripped. Checked and found sound:

- Codegen prefix bytes are identical across `chatml_prefix_for_lane`, `chatml_prefix_open_assistant` and `chatml_supervised_text`, so the loss mask starts at the answer.
- Already-ChatML prompts are not re-wrapped, so chat completions are not double-prefixed.
- The CUDA and Metal `assemble_prompt` bodies are byte-identical.
- The `lane` field flows host to worker payload to plugin and is carried unchanged across repair retries.
- `vox-eval` is reachable from every touched crate. In `vox-populi` it is gated behind `mens-train`, matching the `training_text` module gate.
- All `chatml_prefix_open_assistant` callers were migrated to the 4-argument signature.

No blockers. The three warnings are edge-case correctness problems in the new helpers: two in the grader, one in training-row fusion.

**Known-context rulings**

- **Double lane validation (chat, then `do_generate`):** redundant, not a bug. The chat path validates first, so the `do_generate` Err arm is unreachable from chat. Dropping only the chat Err arm would still return 400 via the forwarded raw lane, but with the wrong body shape (`GenerateResponse` instead of `ChatCompletionResponse`). The chat-side check is what keeps the chat response shape correct. See IN-01 for the cleanup option and one inconsistency.
- **Commit 013bd55c2 fusion production hunks in `vox-populi/.../training_text.rs`:** reviewed. They are a faithful mirror of the `candle-core` copy (prefix tail, `user_for_mode`, `assistant_body`, `fuse_turns`, `cfg.role_assistant`). They are correct for the in-scope behavior and share WR-03 with that copy. See IN-03 for the duplication note.

## Warnings

### WR-01: Truncated thinking trace is graded as the final answer

**File:** `crates/vox-eval/src/qwen_reasoning.rs:151`
**Issue:** `answer_region` returns the whole completion when there is no `</think>`. A thinking-mode completion cut off by `max_tokens` (`<think>` opened, never closed) is therefore graded on numbers or lines inside the reasoning. `grade_final_answer` can mark such a completion correct because an intermediate number equals the gold, and `score_completions` inflates accuracy for exactly the thinking-mode lane this phase adds. `opened_nonempty_think` already treats this shape as "thinking".
**Fix:**
```rust
pub fn answer_region(completion: &str) -> &str {
    if let Some(i) = completion.rfind("</think>") {
        return completion[i + "</think>".len()..].trim();
    }
    // Unterminated trace: no final answer was produced.
    if completion.trim_start().starts_with("<think>") {
        return "";
    }
    completion.trim()
}
```
Add a test: `grade_final_answer("<think>\n... so 42", "42") == false`.

### WR-02: `last_number` mis-parses comma lists and hyphenated tokens

**File:** `crates/vox-eval/src/qwen_reasoning.rs:250`
**Issue:** The digit loop accepts `,` between digits, so "the answers are 1,2,3" yields 123 (and "5,6" yields 56). A leading `-` directly before a digit is read as a sign, so "2024-05-01" yields -1 and "3-4" yields -4. For GSM8K-style grading this produces false negatives and, with a coincidental gold, false positives.
**Fix:** Accept `,` only as a thousands separator, i.e. followed by exactly three digits. Treat `-` as a sign only when it is not preceded by an alphanumeric character:
```rust
// inside the digit loop
if bytes[i] == b',' {
    let grp = bytes.get(i + 1..i + 4);
    let after = bytes.get(i + 4).copied();
    if grp.is_some_and(|g| g.iter().all(u8::is_ascii_digit))
        && !after.is_some_and(|b| b.is_ascii_digit())
    {
        i += 1;
        continue;
    }
    break;
}
```
Add tests for "1,2,3", "1,234" and "2024-05-01".

### WR-03: Non-canonical empty think block gets a second empty block prepended in training rows

**File:** `crates/vox-eval/src/qwen_reasoning.rs:53` (also `crates/vox-plugin-mens-candle-core/src/training_text.rs:145` and `crates/vox-populi/src/mens/tensor/training_text.rs:136`)
**Issue:** `has_reasoning_trace` is false for any whitespace-only think body, e.g. `<think>\n</think>\n\ncode`. `assistant_body` and `fuse_turns` only skip the prefix when the text starts with the exact `QWEN_EMPTY_THINK` (`<think>\n\n</think>\n\n`). A row whose assistant text starts with `<think>\n</think>` or `<think></think>` becomes `<think>\n\n</think>\n\n<think>\n</think>\n\ncode`. The second block lands in the supervised span and trains the model to emit a stray empty think block after the prefilled one. Only the exact-match case is tested.
**Fix:** Detect any leading empty think block, not only the canonical bytes. For example, add `fn starts_with_empty_think(s: &str) -> bool` that trims leading whitespace, strips `<think>`, trims, and strips `</think>`. Use it in `assistant_body` and `fuse_turns`. When it matches a non-canonical form, either normalize it to `QWEN_EMPTY_THINK` or leave it untouched. Add a test for `<think>\n</think>\n\ncode`.

## Info

### IN-01: Lane is validated twice; the two paths differ in error ordering

**File:** `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs:328` (chat) and `:103` (`do_generate`)
**Issue:** Chat parses the lane, then forwards `req.lane.clone()` as a raw string so `do_generate` parses it again; the comment at `:371` documents this. It is redundant, not incorrect. Two small consequences:
- `do_generate` checks the model mismatch (409, line 87) before the lane (400). Chat checks the lane first, so a request with both a bad lane and a wrong model returns 400 on `/v1/chat/completions` but 409 on `/generate`.
- The unreachable `do_generate` Err arm is not covered by the chat path, only by its own tests.

**Fix:** Optional. Extract an inner `generate_with_lane(state, req, lane)` that both handlers call, so the string is parsed once. If not refactoring, align the ordering by validating the lane before the model check in `do_generate`.

### IN-02: Hub/agent `/no_think` stripping is a raw substring replace

**File:** `crates/vox-eval/src/qwen_reasoning.rs:123`
**Issue:** `user.replace("/no_think", "")` also edits text like `/no_thinking` (leaving `ing`) or a path containing `/no_think_x`, and `trim_end()` runs on every hub/agent user turn even when no flag was present. In codegen, a user turn already containing `/think` keeps that flag while the assistant opener still prefills the empty block, so the two signals contradict each other. This mirrors training behavior and is documented in the tests, so it is intentional, but the contradiction reaches serving.
**Fix:** Optional. Strip only the flag as a standalone token (whitespace- or boundary-delimited), and only trim when a flag was removed.

### IN-03: Duplicated chatml helpers across `vox-populi` and `candle-core`; the populi copy has no multi-turn tests

**File:** `crates/vox-populi/src/mens/tensor/training_text.rs:124`, `crates/vox-plugin-mens-candle-core/src/training_text.rs:134`
**Issue:** `fuse_turns`, `chatml_turns_text`, `chatml_turns_prefix_open_assistant`, `chatml_supervised_text` and `chatml_prefix_open_assistant` are maintained in two copies, and the lane logic has now been added to both. The two already differ in config type (`training_config::ChatmlConfig` vs `config::ChatmlConfig`). WR-03 has to be fixed in both places. The populi copy gained tests only for single-turn direct and reasoning rows; `fuse_turns` and the messages-row path have no coverage there, while the `candle-core` copy tests the message-row mask.
**Fix:** Optional. Add the message-row mask test to the populi copy (mirror `message_rows_mask_the_empty_think_block_the_same_way`). Longer term, one shared implementation, with the config type split into a `-types` crate per the dependency-discipline rules.

---

_Reviewed: 2026-10-26T21:21:35Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
