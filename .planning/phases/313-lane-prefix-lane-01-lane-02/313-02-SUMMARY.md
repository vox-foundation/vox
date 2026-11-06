---
phase: 313-lane-prefix-lane-01-lane-02
plan: 02
subsystem: serving
tags: [lane, qwen3, thinking-mode, candle, chatml, cuda, metal]
status: complete

requires:
  - phase: 313-lane-prefix-lane-01-lane-02
    provides: "vox_eval::qwen_reasoning::{Lane, user_for_lane, assistant_opener_for_lane}; host payload carries `lane`"
provides:
  - "chatml_prefix_for_lane(system, user, lane, cfg) in candle-core training_text (single raw-wrap builder)"
  - "CUDA and Metal PromptRequest.lane (Option<String>) + private resolve_lane"
  - "assemble_prompt(system, prompt, lane) on both plugins; hub/agent raw generate opens a bare thinking turn"
affects: [313-03, 313-04]

actuals:
  tokens: 14000
  tasks: 2
  commits: 2
plan_head_before: 85e0e7efb
plan_head_after: dfd6ce41add7ce2cb3469d39b37c7aebada3ecbb

tech-stack:
  added: []
  patterns:
    - "Codegen delegates to chatml_prefix_open_assistant(.., \"\", cfg) so it is byte-identical; hub/agent reuse the same format template with lane helpers"
    - "Plugins hold no lane strings: Lane::parse + chatml_prefix_for_lane are the only sources"

key-files:
  created: []
  modified:
    - crates/vox-plugin-mens-candle-core/src/training_text.rs
    - crates/vox-plugin-mens-candle-cuda/src/inference.rs
    - crates/vox-plugin-mens-candle-metal/src/inference.rs

key-decisions:
  - "Hub/agent do not call chatml_prefix_open_assistant with a fake assistant string (avoids coupling to has_reasoning_trace); they reuse its template inline"
  - "resolve_lane runs right after check_output_mode, so an unknown lane errors before any token is generated (defense in depth; host already validates)"
  - "Early <|im_start|> pre-wrapped branch untouched for every lane"

patterns-established:
  - "Lane-aware raw wrap lives in candle-core; plugin twins are edited identically"

requirements-completed: [LANE-01]

coverage:
  - id: D1
    description: "chatml_prefix_for_lane: codegen byte-identical to chatml_prefix_open_assistant; hub/agent bare thinking turn with /no_think stripped"
    requirement: LANE-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-mens-candle-core chatml_prefix_for_lane"
        status: pass
    human_judgment: false
  - id: D2
    description: "CUDA and Metal raw generate honor lane; pre-wrapped prompts and codegen output unchanged; unknown lane rejected"
    requirement: LANE-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-plugin-mens-candle-cuda assemble_prompt / resolve_lane; same for metal"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-26
---

# Phase 313 Plan 02: Lane-aware raw generate in the Candle plugins Summary

**CUDA and Metal plugins read `lane` from the payload and wrap raw `/generate` prompts through one shared `chatml_prefix_for_lane`, so hub/agent get a bare thinking turn while codegen stays byte-identical.**

## Accomplishments

- `chatml_prefix_for_lane` (`#[must_use]`, documented) in candle-core `training_text.rs`; `chatml_prefix_open_assistant`, `fuse_turns` and training paths unchanged (diff vs pre-copy removes only the import line).
- Both plugins: `#[serde(default)] lane: Option<String>`, private `resolve_lane` (None -> Codegen, else `Lane::parse`, Err text lists codegen, hub, agent), called right after `check_output_mode`; `assemble_prompt` gains `lane: Lane`.
- Three pre-existing `assemble_prompt_*` tests per plugin keep their literal expectations; only the call gained `Lane::Codegen` (verified by diff of removed lines).

## TDD

- **RED (core):** with a stub that ignored `lane`, `chatml_prefix_for_lane_hub_and_agent_are_bare_thinking` failed on its planned assertion (left carried ` /no_think` and the empty think block); the codegen test passed. Semantic assessment: target ran and failed for the intended reason. Rust is not a format the `tdd-red-evidence` classifier supports, so evidence is the captured `cargo test` output, not a classifier verdict.
- **RED (plugins):** with `resolve_lane`/`assemble_prompt` stubbed (lane ignored), `assemble_prompt_hub_and_agent_open_a_bare_thinking_turn` failed on the planned assertion (CUDA run; Metal twin has the identical test and stub).
- **GREEN:** real `resolve_lane` + `chatml_prefix_for_lane` wiring; all filters pass.
- Per the dispatch instruction, one atomic commit per task, so RED and GREEN landed together. No separate `test(313-02)` commit exists.

## Verification

| Command | Result |
|---|---|
| `cargo test -p vox-plugin-mens-candle-core chatml_prefix_for_lane` | 2 passed, 0 failed |
| `cargo test -p vox-plugin-mens-candle-core training_text` | 10 passed, 0 failed |
| `cargo test -p vox-plugin-mens-candle-cuda assemble_prompt` | 5 passed, 0 failed |
| `cargo test -p vox-plugin-mens-candle-cuda resolve_lane` | 1 passed, 0 failed |
| `cargo test -p vox-plugin-mens-candle-metal assemble_prompt` | 5 passed, 0 failed |
| `cargo test -p vox-plugin-mens-candle-metal resolve_lane` | 1 passed, 0 failed |
| `cargo check -p vox-plugin-mens-candle-cuda -p vox-plugin-mens-candle-metal --tests` | exit 0 |
| `rg -c chatml_prefix_for_lane` / `lane: Option<String>` on both plugin files | 2 / 1 each |

**Mutation check (guarded):** edited `chatml_prefix_for_lane` to use `QWEN_EMPTY_THINK` as the hub/agent tail (presence confirmed with `rg -c`), the CUDA `assemble_prompt_hub_and_agent_open_a_bare_thinking_turn` test failed (4 passed, 1 failed); restored (presence confirmed) and 5 passed.

## Deviations from Plan

None to code. Notes:

- `cargo clippy -p ... -- -D warnings` fails in `vox-eval` (3 `collapsible_if` errors in benchmark code at ~line 784), a dependency outside this plan's files. Out of scope, not fixed. Plain `cargo clippy` on the three crates reports nothing in the touched files.
- `plan_head_before` is `85e0e7efb` (the HEAD named in the dispatch); the per-plan ledger file was not created, so `commits: 2` is `git rev-list --count 85e0e7efb..HEAD`.
- The three source files were already dirty with prior fusion work; explicit-path staging included it, as the plan accepted.

## Commits

- `5994c9524` feat(313-02): add chatml_prefix_for_lane shared raw-wrap builder
- `dfd6ce41a` feat(313-02): honor lane in CUDA and Metal raw generate

## Known Stubs

None.

## Threat Flags

None. T-313-04 (closed-set parse in both plugins) and T-313-05 (pre-wrapped branch locked by `assemble_prompt_keeps_prewrapped_chatml_for_every_lane`) are mitigated.

## Self-Check: PASSED

- Files modified exist and are the real size (core 339 lines, CUDA 1464, Metal 1898; headers intact).
- Commits `5994c9524` and `dfd6ce41a` are ancestors of HEAD.
