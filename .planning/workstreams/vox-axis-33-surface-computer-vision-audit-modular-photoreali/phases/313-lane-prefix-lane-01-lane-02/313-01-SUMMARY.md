---
phase: 313-lane-prefix-lane-01-lane-02
plan: 01
subsystem: serving
tags: [lane, qwen3, thinking-mode, axum, generate, serde]
status: complete

requires: []
provides:
  - "vox_eval::qwen_reasoning::Lane (Codegen default, Hub, Agent) with ALLOWED, parse, as_str, is_thinking"
  - "user_for_lane and assistant_opener_for_lane helpers (single source of lane strings)"
  - "GenerateRequest.lane (Option<String>) and InferenceRequest.lane (Lane)"
  - "inference_payload always emits \"lane\""
  - "JSON 400 on unknown lane in do_generate, before any worker send"
affects: [313-02, 313-03, 313-04]

actuals:
  tokens: 7600
  tasks: 2
  commits: 2
plan_head_before: 1a0667364
plan_head_after: faaf67b09

tech-stack:
  added: []
  patterns:
    - "Wire lane as String, parsed in the handler, so a bad value gets the JSON error shape (not axum's plain-text 422)"
    - "Lane is Copy and is set in the single InferenceRequest literal inside the retry loop"

key-files:
  created: []
  modified:
    - crates/vox-eval/src/qwen_reasoning.rs
    - crates/vox-ml-cli/src/commands/ai/serve/schema.rs
    - crates/vox-ml-cli/src/commands/ai/serve/handlers.rs
    - crates/vox-ml-cli/src/commands/ai/serve/worker.rs

key-decisions:
  - "Lane::parse is exact lowercase, no trim or case folding; error text quotes the value cut to 32 chars"
  - "Hub/agent user_for_lane strips /no_think then trim_end, leaves /think; codegen delegates to user_for_mode(user, false)"
  - "do_chat_completions GenerateRequest literal gets lane: None here; plan 313-03 forwards req.lane"

patterns-established:
  - "Capturing fake worker (Arc<Mutex<Vec<InferenceRequest>>>) for handler tests that need the request and its payload"

requirements-completed: [LANE-01]
---

# Phase 313 Plan 01: Lane through /generate Summary

A raw `/generate` request with `lane: "hub"` now reaches the plugin JSON as `"lane":"hub"`; unknown lanes are a JSON 400 before the worker, and repair retries keep the lane.

## Accomplishments

- `Lane` enum plus `user_for_lane` / `assistant_opener_for_lane` in `vox-eval` (11 tests in `qwen_reasoning`, 6 new).
- `GenerateRequest.lane`, `InferenceRequest.lane`, `"lane"` in `inference_payload`; the five existing worker test literals gained `lane: Lane::Codegen`.
- `do_generate` resolves the lane right after the 409 block. Missing lane is codegen; an unknown, mis-cased or empty lane returns a JSON `GenerateResponse` 400 naming `codegen, hub, agent`.
- Tracer test `do_generate_hub_lane_reaches_the_worker_payload` proves HTTP JSON to `inference_payload` end to end. `inference_payload` is now `pub(crate)`.

## Task Commits

1. Task 1 (tracer: Lane, wire fields, payload, interim Err arm): `00375744d`
2. Task 2 (unknown-lane 400 and retry tests): `faaf67b09`

## Verification

- `cargo test -p vox-eval qwen_reasoning`: 11 passed, 0 failed.
- `cargo test -p vox-ml-cli --features gpu --lib -- serve::`: 40 passed, 0 failed (includes `do_generate_hub_lane_reaches_the_worker_payload`, `do_generate_without_a_lane_serializes_codegen`, `the_payload_carries_the_lane`, `unknown_lane_on_generate_is_400_before_the_worker`, `do_generate_repair_retries_keep_the_lane`, schema lane test).
- `cargo check -p vox-ml-cli --features gpu --tests`: clean.
- Mutation checks (guarded, restored, re-run green): dropping the `"lane"` key fails `the_payload_carries_the_lane` and the tracer test; making the unknown-lane Err arm fall back to codegen fails `unknown_lane_on_generate_is_400_before_the_worker`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan test commands run zero tests without `--features gpu`**
- **Found during:** Task 1 verification
- **Issue:** `commands::ai::serve` is gated by the `gpu` feature, so `cargo test -p vox-ml-cli <filter>` matched 0 tests (exit 0, "0 passed").
- **Fix:** ran every ml-cli filter with `--features gpu --lib`. Later plans' `<automated>` lines for `vox-ml-cli` need the same flag.
- **Commit:** n/a (command change only)

**2. [Process] ml-cli tests and implementation applied together**
- Red was shown for `vox-eval` (compile failure, then green). For `vox-ml-cli` the tests and implementation were written in one edit; the mutation checks above substitute for the red run.

**3. [Process] Editing tools corrupt files in this environment**
- The Cursor `StrReplace` tool wrote lean-ctx's compressed view into `schema.rs`. It was restored with `git checkout -- schema.rs` (clean at HEAD, no other edits lost) and all later edits were done by script against the on-disk file.

## Notes for the Orchestrator

- `qwen_reasoning.rs` was untracked and `handlers.rs` carried uncommitted fusion edits (`user_for_mode`, `QWEN_EMPTY_THINK`). Both were committed with this plan, so commit `00375744d` includes that prior fusion work (432 added lines in `qwen_reasoning.rs`).
- `crates/vox-eval/src/lib.rs` (which declares `pub mod qwen_reasoning`) is still modified and uncommitted; it was outside this plan's file list. A checkout of the commit alone will not compile `vox-eval` until it is committed.
- `.planning/` is tracked; STATE.md and ROADMAP.md were not touched.

## Known Stubs

None.

## Threat Flags

None. Lane arrives as a string and maps only to fixed strings; it is never interpolated into the prompt.

## Self-Check: PASSED

- Files modified exist; commits `00375744d` and `faaf67b09` are ancestors of HEAD.
