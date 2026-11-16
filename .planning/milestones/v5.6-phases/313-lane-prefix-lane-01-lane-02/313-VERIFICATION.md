---
phase: 313-lane-prefix-lane-01-lane-02
verified: 2026-10-26T22:10:00Z
status: passed
score: 8/8 must-haves verified
covered_files:
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-01-PLAN.md"
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-01-SUMMARY.md"
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-02-PLAN.md"
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-02-SUMMARY.md"
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-03-PLAN.md"
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-03-SUMMARY.md"
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-04-PLAN.md"
  - ".planning/phases/313-lane-prefix-lane-01-lane-02/313-04-SUMMARY.md"
  - "crates/vox-eval/src/qwen_reasoning.rs"
  - "crates/vox-ml-cli/src/commands/ai/serve/handlers.rs"
  - "crates/vox-ml-cli/src/commands/ai/serve/schema.rs"
  - "crates/vox-ml-cli/src/commands/ai/serve/worker.rs"
  - "crates/vox-plugin-mens-candle-core/Cargo.toml"
  - "crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/encoding.rs"
  - "crates/vox-plugin-mens-candle-core/src/training_text.rs"
  - "crates/vox-plugin-mens-candle-cuda/src/inference.rs"
  - "crates/vox-plugin-mens-candle-metal/src/inference.rs"
  - "crates/vox-populi/src/mens/tensor/training_text.rs"
covered_digest: "v3:sha256:f0b8c09e6e970fa70b222e3fe47465e35a5f151e66c3d4a8d9c1f4090fcee348"
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
---

# Phase 313: Lane Prefix (LANE-01, LANE-02) Verification Report

**Phase Goal:** Codegen requests and codegen training rows use the non-thinking prefix, and hub and agent requests use the bare thinking-mode prefix.
**Verified:** 2026-10-26T22:10:00Z
**Status:** passed
**Re-verification:** No, initial verification

The behavior is in the working tree and every locked string is proven by unit tests that actually ran. The one gap is delivery integrity: part of what makes the committed code build is still uncommitted. No model is loaded, and that is not counted against the phase (it belongs to phase 316).

## Goal Achievement

Must-haves are the LANE-01 and LANE-02 requirement text and the CONTEXT decisions, plus the PLAN `must_haves.truths`. The ROADMAP success criteria restate the same behavior. Plan truths were not allowed to narrow scope.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Codegen request (chat and raw) is served with ` /no_think` and the empty `<think>\n\n</think>\n\n` block, byte-identical to before | ✓ VERIFIED | `user_for_lane(_, Codegen)` delegates to `user_for_mode(_, false)`; `assistant_opener_for_lane(Codegen) == QWEN_EMPTY_THINK` (`qwen_reasoning.rs`). `flatten_chat_messages` uses both (`handlers.rs:287,303`). `chatml_prefix_for_lane(Codegen)` returns `chatml_prefix_open_assistant(.., "", ..)` (`training_text.rs:43-44`). Tests ran: `flatten_chat_messages_codegen_keeps_user_think_and_the_empty_think_opener`, `chatml_prefix_for_lane_codegen_matches_open_assistant`, CUDA/Metal `assemble_prompt_wraps_in_training_chatml` |
| 2 | Hub and agent requests get the bare thinking prefix: ends at `assistant\n`, no `/no_think`, no empty think block, same string for both | ✓ VERIFIED | `Lane::is_thinking` gives `""` opener and strips every `/no_think` then `trim_end`. Chat: `flatten_chat_messages_hub_and_agent_open_a_bare_thinking_turn`, `..._hub_strips_no_think_from_the_last_user_turn_only`, `do_chat_completions_hub_lane_reaches_the_worker_as_bare_thinking`. Raw: CUDA and Metal `assemble_prompt_hub_and_agent_open_a_bare_thinking_turn` (5 passed each, filter `assemble_prompt`). Lane wins over user `/no_think` (P-313-07 holds), and no system turn is injected |
| 3 | `lane` travels `/generate` JSON, then `GenerateRequest`, `do_generate`, `InferenceRequest`, and the plugin payload `"lane"` | ✓ VERIFIED | `schema.rs:50`, `handlers.rs:103-110,164`, `worker.rs:34,151`; plugin `PromptRequest.lane` plus `resolve_lane` at CUDA `inference.rs:483,496` and the Metal twin. Tests: `do_generate_hub_lane_reaches_the_worker_payload` (asserts `v["lane"] == "hub"`), `the_payload_carries_the_lane`, `do_generate_without_a_lane_serializes_codegen`. Chat forwards `lane: req.lane.clone()` (`handlers.rs:372`) |
| 4 | Unknown, mis-cased, or empty lane is rejected before any worker or model call, on `/generate`, chat completions and the plugin, naming `codegen, hub, agent`; never coerced | ✓ VERIFIED | `Lane::parse` is exact lowercase, `Err` text built from `Lane::ALLOWED`. Handler `Err` arms return a JSON 400 before `tx.send`. Tests: `unknown_lane_on_generate_is_400_before_the_worker` (covers "bogus", "Hub", ""; worker receive counter 0), `unknown_lane_on_chat_is_400_before_the_worker`, CUDA/Metal `resolve_lane_rejects_unknown_values`. P-313-01 holds |
| 5 | Callers that omit `lane` (or send `null`) see no prompt change | ✓ VERIFIED | `None` maps to `Lane::Codegen`. Tests: `generate_request_lane_defaults_to_none_and_keeps_the_raw_string`, `chat_completion_request_lane_defaults_to_none`, `do_chat_completions_without_a_lane_stays_codegen`. P-313-03 holds; a pre-wrapped `<|im_start|>` prompt is returned unchanged for every lane |
| 6 | LANE-02: a codegen direct-answer training row uses the same non-thinking prefix and the empty think block is outside the loss | ✓ VERIFIED | Prefix ends with `QWEN_EMPTY_THINK`, contains ` /no_think`, and `full[prefix.len()..] == "code" + im_end`. The loss mask is the prefix string (`encoding.rs` `aligned_prefix_token_len`). Tests ran: candle-core `direct_answer_puts_empty_think_in_the_prompt_not_the_loss` (12 passed), populi copy of the same test (5 passed). Plan 04 recorded a mutation check on the populi builder. Caveat: the live training call site is the uncommitted file listed in truth 8 |
| 7 | LANE-02: a row that already has a non-empty think body keeps the thinking-mode entry and is not rewritten with `/no_think` | ✓ VERIFIED | `has_reasoning_trace` selects the empty tail `""` and leaves the user turn as sent. Tests: candle-core and populi `reasoning_trace_keeps_the_thinking_mode_entry_point`, candle-core `message_rows_leave_a_reasoning_turn_unrewritten`, vox-eval `assistant_body_does_not_double_the_empty_think_block`, `codegen_user_with_an_existing_flag_gets_no_second_flag`, candle-core `supervised_prefix_adds_no_second_flag`. P-313-08 and P-313-09 hold: plan 04 added tests only |
| 8 | The committed tree builds and delivers the above (nothing it needs is left uncommitted) | ✓ VERIFIED | Commit `5e0dccbb9` adds the `vox-eval` dependency, the Cargo.lock line, and the 4-arg call sites in `encoding.rs` and both `validation.rs` files. `cargo check -p vox-plugin-mens-candle-core` and `cargo check -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal` exited 0 after that commit. `eval_local_prompt.rs` stays uncommitted; HEAD's copy does not call the 4-arg helper, so the build does not need it. |

**Score:** 8/8 truths verified. Truth 8 was closed by commit `5e0dccbb9`.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/vox-eval/src/qwen_reasoning.rs` | `Lane`, `parse`, `ALLOWED`, `as_str`, `is_thinking`, `user_for_lane`, `assistant_opener_for_lane`, tests | ✓ VERIFIED | Substantive and wired (serve and both plugins consume it). Tracked. 19 tests pass |
| `.../serve/schema.rs` | `lane: Option<String>` on `GenerateRequest` and `ChatCompletionRequest`, `#[serde(default)]` | ✓ VERIFIED | Lines 50 and 150. A `String` is used, not a serde enum, so a bad value gets the JSON 400 instead of axum's plain-text 422 |
| `.../serve/worker.rs` | `InferenceRequest.lane: Lane`, `"lane": req.lane.as_str()` | ✓ VERIFIED | Lines 34 and 151. All five older test literals updated |
| `.../serve/handlers.rs` | Lane parse and 400 in `do_generate` and `do_chat_completions`; `flatten_chat_messages(.., lane)` | ✓ VERIFIED | Single non-test `InferenceRequest {` literal sets `lane`. No literal `<think>` or `/no_think` in non-test handler code |
| `candle-core/src/training_text.rs` | `chatml_prefix_for_lane` plus tests | ✓ VERIFIED | Builds after `5e0dccbb9` (`vox-eval` dependency committed). argo.toml` edit (gap) |
| CUDA and Metal `inference.rs` | `PromptRequest.lane`, `resolve_lane`, `assemble_prompt(.., lane)` | ✓ VERIFIED | Twin edits are line-for-line identical in the lane functions. No per-crate lane strings (P-313-04 holds) |
| `vox-populi/.../training_text.rs` | Fusion lock tests | ✓ VERIFIED | Committed whole, including production fusion hunks that predate the plan (see Anti-Patterns) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `do_generate` | `InferenceRequest` | `lane` in the single literal inside the retry loop | ✓ WIRED | `do_generate_repair_retries_keep_the_lane` asserts at least 2 captured requests, all `Hub`. This closes the edge-probe LANE-01 concurrency row the plan left flagged |
| `inference_payload` | plugin `PromptRequest.lane` | JSON key `"lane"` | ✓ WIRED | Host payload test and plugin `lane: Option<String>` field |
| `do_chat_completions` | `flatten_chat_messages` and `do_generate` | parsed `Lane` plus `lane: req.lane.clone()` | ✓ WIRED | Hub chat test asserts `Lane::Hub` reaches the worker and the prompt ends at `assistant\n` |
| plugin `generate_from_json` | `assemble_prompt` | `resolve_lane(req.lane.as_deref())?` | ✓ WIRED | Runs after `check_output_mode`, before any token |
| training prefix string | loss mask | `aligned_prefix_token_len` | ✓ WIRED | `encoding.rs` 4-arg call committed in `5e0dccbb9`. itted (gap) |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `inference_payload` | `"lane"` | `req.lane.as_str()` from the parsed request, not a constant | Yes | ✓ FLOWING |
| `flatten_chat_messages` | opener and last user turn | `Lane` parsed from the request | Yes | ✓ FLOWING |
| plugin `assemble_prompt` | `lane` | `resolve_lane(payload.lane)` | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

Each filter was confirmed to run tests (no "0 passed"). All ran against the working tree.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Lane helpers, grader fixes (WR-01/02) | `cargo test -p vox-eval --lib qwen_reasoning` | 19 passed, 0 failed | ✓ PASS |
| candle-core prefix and fusion locks | `cargo test -p vox-plugin-mens-candle-core --lib training_text` | 12 passed, 0 failed | ✓ PASS |
| populi fusion locks | `cargo test -p vox-populi --features mens-train --lib training_text` | 5 passed, 0 failed (344 filtered) | ✓ PASS |
| serve: chat, generate, 400s, retries, payload, schema | `cargo test -p vox-ml-cli --features gpu --lib -- serve` | 51 passed, 0 failed | ✓ PASS |
| Metal raw wrap | `cargo test -p vox-plugin-mens-candle-metal --lib assemble_prompt` | 5 passed, 0 failed | ✓ PASS |
| CUDA raw wrap | `cargo test -p vox-plugin-mens-candle-cuda --lib assemble_prompt` | 5 passed, 0 failed | ✓ PASS |

### Probe Execution

Step 7c: SKIPPED (the phase declares no probe scripts).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| LANE-01 | 313-01, 313-02, 313-03 | Codegen served non-thinking; hub/agent served bare thinking prefix; a test locks both strings | ✓ SATISFIED in the working tree | Truths 1 to 5. Delivery caveat in truth 8 (plan 313-02 files) |
| LANE-02 | 313-04 | Codegen training row uses the same prefix with the empty think block outside the loss; a row with a real think body is not rewritten | ✓ SATISFIED in the working tree | Truths 6 and 7. Delivery caveat in truth 8 (live call site uncommitted) |

Both IDs appear in plan frontmatter and in REQUIREMENTS.md (v5.6, lines 11-12, traceability lines 43-44). No orphaned requirements for phase 313. REQUIREMENTS.md still shows both as `Pending`. I did not edit it.

### CONTEXT Decisions Honored

| Decision | Honored |
|----------|---------|
| Explicit `lane` of `codegen`/`hub`/`agent`; omitted means codegen | Yes |
| Hub and agent share one string; lane beats user `/no_think` | Yes (test-locked) |
| Unknown lane fails before model load, names allowed values, same on chat and generate | Yes |
| Only the last user turn and the assistant opener change; no new system prompt | Yes (multi-turn byte-equality test across the three lanes) |
| Pre-wrapped `<|im_start|>` prompt left unchanged | Yes (every lane) |
| No lane column on training rows; fusion builders not edited | Yes. Plan 04 added test lines only |
| Codegen text with `/think` or `/no_think` gets no second flag | Yes |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `crates/vox-populi/src/mens/tensor/training_text.rs` | commit `013bd55c2` | Plan 04 committed 84 insertions and 11 deletions, including production fusion hunks it did not author | ⚠️ Warning | Not a goal gap. Those hunks are what make the LANE-02 populi tests pass, and they are now committed. Provenance of the production change is not attributed to a plan |
| `crates/vox-eval/src/qwen_reasoning.rs` | `Lane::parse` | No `#[must_use]`, unlike the other new pub fns | ℹ️ Info | Returns `Result`, which is already `#[must_use]` |
| `handlers.rs` | lane parse in two handlers | Lane validated twice (REVIEW IN-01) | ℹ️ Info | Skipped as triaged |

No `TODO`, `FIXME`, `XXX` or `TBD` markers were found in the lane code.

### Triaged Items (not goal gaps)

- WR-01 and WR-02: fixed in `9cec48abd`. The tests pass (19/19 in `qwen_reasoning`).
- WR-03 is deferred. LANE-02 forbids editing the fusion builders, and non-canonical empty think blocks are outside the locked contract.
- IN-01, IN-02 and IN-03 are skipped info.

### Prohibitions

| Id | Statement | Disposition |
|----|-----------|-------------|
| P-313-01 | Unknown lane never coerced | Verified by test (`unknown_lane*` tests) |
| P-313-02 | Lane value never interpolated into prompt text | Verified by code read: the only consumers are `Lane::parse` and fixed strings from `qwen_reasoning`. No test asserts it. Non-authoritative judgment, human review recommended |
| P-313-03 | Omitted lane leaves prompt bytes unchanged | Verified by test |
| P-313-04 | No lane strings in the plugins | Verified by code read |
| P-313-05 | No empty think block or injected system turn for hub/agent raw | Verified by test |
| P-313-06, P-313-07 | No new system prompt; user `/no_think` cannot flip hub | Verified by test |
| P-313-08, P-313-09 | No lane column or `fuse_turns` change; reasoning rows never rewritten | Verified by test and diff (plan 04 added tests only) |

### Human Verification Required

None. The phase proof is unit tests. Real-model behavior with Qwen3-8B is phase 316.

### Gap closure

Truth 8 failed because the `vox-eval` dependency and the 4-arg training call sites were uncommitted. Commit `5e0dccbb9` adds only those five paths (8 insertions, 6 deletions). `cargo check -p vox-plugin-mens-candle-core` and `cargo check -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal` both exited 0 afterward. `crates/vox-ml-cli/src/commands/mens/eval_local_prompt.rs` was left uncommitted on purpose: HEAD still formats the eval prompt inline and does not need the new helper to compile.

### Gaps Summary

The lane behavior is complete and well tested, but one delivery problem remains.

1. **Plan 313-02 and the LANE-02 call site depend on uncommitted edits.** Commits `5994c9524` and `fa70fab00` make `vox-plugin-mens-candle-core` import `vox_eval`, but the matching `Cargo.toml` dependency is not committed. The 4-argument `chatml_prefix_open_assistant` call sites that carry the non-thinking prefix onto real training rows (`encoding.rs`, CUDA and Metal `validation.rs`) are also still uncommitted. Every green test above is therefore a working-tree result.

Closing it is mechanical: commit those five files, then run a clean-checkout `cargo check` on `vox-plugin-mens-candle-core`. A test failure is not involved, and no new implementation is needed. The Cargo.lock edit and `eval_local_prompt.rs` also sit uncommitted in the same tree. Check who owns them before bundling them into the commit.

---

_Verified: 2026-10-26T22:10:00Z_
_Verifier: Claude (gsd-verifier)_