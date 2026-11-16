---
phase: 313-lane-prefix-lane-01-lane-02
plan: 03
subsystem: serving
tags: [lane, qwen3, thinking-mode, chatml, axum, chat-completions]
status: complete

requires:
  - phase: 313-lane-prefix-lane-01-lane-02
    provides: "Lane, user_for_lane, assistant_opener_for_lane, GenerateRequest.lane, generate-route 400, counting/capturing fake worker"
provides:
  - "ChatCompletionRequest.lane (Option<String>, serde default)"
  - "flatten_chat_messages(messages, tools, lane): last user turn via user_for_lane, assistant opener via assistant_opener_for_lane"
  - "do_chat_completions validates lane first; unknown lane is a JSON ChatCompletionResponse 400 before the worker; lane forwarded to GenerateRequest"
affects: [313-04]

actuals:
  tokens: 3600
  tasks: 2
  commits: 2
plan_head_before: 9bbd1c9a54d571c96b7666b605f73d31b2a67cd3
plan_head_after: df367d4e0f75a048cb028cc512924ce5d12ddd24

tech-stack:
  added: []
  patterns:
    - "Chat lane handling lives entirely in flatten_chat_messages; only the last user turn and the assistant opener depend on lane"
    - "Lane is validated at the top of each HTTP handler, before any other work"

key-files:
  created: []
  modified:
    - crates/vox-ml-cli/src/commands/ai/serve/schema.rs
    - crates/vox-ml-cli/src/commands/ai/serve/handlers.rs

key-decisions:
  - "No system prompt added for hub/agent; earlier turns and the tool catalog are untouched"
  - "prompt_for_output_mode (tools suffix) left unchanged, as scoped"
  - "do_chat_completions keeps its own lane check even though do_generate re-validates the forwarded lane: the chat 400 is a ChatCompletionResponse body, not a GenerateResponse mapped through the chat wrapper"

requirements-completed: [LANE-01]

coverage:
  - id: D1
    description: "Codegen chat bytes unchanged; hub/agent chat ends at bare assistant turn with no /no_think"
    requirement: LANE-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-ml-cli --features gpu --lib -- flatten_chat_messages"
        status: pass
  - id: D2
    description: "Unknown chat lane is a JSON 400 before the worker; hub chat reaches worker as Lane::Hub bare thinking prompt"
    requirement: LANE-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-ml-cli --features gpu --lib -- unknown_lane do_chat_completions"
        status: pass
---

# Phase 313 Plan 03: Chat lane Summary

`/v1/chat/completions` accepts `lane`, rejects unknown values with a JSON 400 before inference, and builds codegen or bare-thinking ChatML by lane inside `flatten_chat_messages`.

## Accomplishments

- `ChatCompletionRequest.lane` added after `tools` (doc points at `GenerateRequest::lane`).
- `flatten_chat_messages` takes a final `Lane`; hub/agent output for `hi` is `<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n`, codegen is byte-identical to before.
- `do_chat_completions` parses the lane first (None -> Codegen), returns a JSON `ChatCompletionResponse` 400 (content = parse error, `finish_reason` stop, model = `state.model_name`) and passes the lane to flatten and to `GenerateRequest` (`lane: req.lane.clone()`, one occurrence).

## Task Commits

1. Task 1: `a83d9c14e` feat(313-03): lane-aware flatten_chat_messages and ChatCompletionRequest.lane
2. Task 2: `df367d4e0` feat(313-03): do_chat_completions validates lane first, 400s, forwards lane

## Verification

Features: `--features gpu` is sufficient (it pulls in `execution-api`); no extra feature needed.

- `cargo test -p vox-ml-cli --features gpu --lib -- flatten_chat_messages chat_completion_request_` -> 8 passed, 0 failed
- `cargo test -p vox-ml-cli --features gpu --lib -- unknown_lane do_chat_completions` -> 7 passed, 0 failed
- Combined run of all four filters -> 15 passed, 0 failed
- RED first: both tasks' tests failed to compile / failed before the implementation.
- Mutation (Task 1, guarded): hub pushing `QWEN_EMPTY_THINK` -> 2 flatten tests FAILED; restored -> 5 passed.
- Mutation (Task 2, plan's version: chat Err arm falls back to Codegen) -> chat test **still passed**. Reason: `lane: req.lane.clone()` forwards the bad lane and `do_generate` returns its own 400, whose text/status/model map to the same chat body. Only removing both the Err arm and the forwarding (`lane: None`) made `unknown_lane_on_chat_is_400_before_the_worker` fail. Restored (byte-compared) -> green. So the chat-side 400 is covered by observable-behavior tests but its early check is redundant with `do_generate`'s; it still guarantees the lane is validated before `flatten_chat_messages`.
- `rg user_for_mode` in handlers.rs -> no match. `rg -c -F "lane: req.lane.clone()"` -> 1.

## Deviations from Plan

**1. [Rule 3 - Blocking] `lane: None` on the three existing `ChatCompletionRequest` test literals moved into Task 1.**
Adding the field breaks those literals at compile time, so Task 1 could not compile its tests without it. Same files, committed in `a83d9c14e`.

**2. Task 2 mutation check could not be satisfied as written** (see Verification); noted rather than weakened.

`cargo clippy -p vox-ml-cli --features gpu --lib --tests -- -D warnings` did not complete: it fails compiling `vox-eval` (3 `collapsible_if` errors in `vox-eval`, a crate this plan does not edit). Nothing in the edited files was reported, but clippy never reached `vox-ml-cli`.

## Known Stubs

None.

## Self-Check: PASSED

- Files modified exist; commits `a83d9c14e` and `df367d4e0` are ancestors of HEAD.
