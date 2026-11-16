---
phase: 318-live-vram-selector-size-02
plan: 03
subsystem: models
tags: [qwen3, qlora, chat, gui, auto-select]

requires:
  - phase: 318-01
    provides: select_qwen3_code_qlora_for_free_mb and read_accelerator_free_mb
provides:
  - select_from_free_mb on the orchestrator runtime feature
  - Live local selection that fails closed
  - Chat auto-local and the GUI host mapping that Result
affects: []

actuals:
  tokens: 11296
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Injected free_mb selects; the live function only reads then calls that selector"
    - "Chat tests pass Some(Ok) or Some(Err) and do not probe"

key-files:
  created: []
  modified:
    - crates/vox-orchestrator/src/models/auto_select.rs
    - crates/vox-orchestrator/src/models/mod.rs
    - crates/vox-orchestrator/src/models/select.rs
    - crates/vox-cli/src/commands/chat.rs
    - crates/vox-gui/src/commands/models_host.rs

key-decisions:
  - "Directory tiers and vox-mens-8b-v0.6 are gone from auto_select"
  - "is_apple_silicon stays on the signature and is not used to choose a size"
  - "No Cargo.toml edit and no new crate edge"

patterns-established:
  - "resolve_chat_config_with_auto_pick takes an injected Result"

requirements-completed: [SIZE-02]

coverage:
  - id: D1
    description: 25000 MB resolves to the 14B QLoRA pin; 1500 MB is an error; 70000 MB is the 32B pin
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: cargo test -p vox-orchestrator --lib select_from_free_mb
        status: pass
    human_judgment: false
  - id: D2
    description: The directory-tier function and its re-exports are gone. The live function reads free memory then calls select_from_free_mb
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: cargo test -p vox-orchestrator --lib auto_select
        status: pass
    human_judgment: false
  - id: D3
    description: Injected auto-local chat uses the 14B pin. An injected error does not build a config. The GUI mapper passes Ok through and turns Err into String
    requirement: SIZE-02
    verification:
      - kind: unit
        ref: cargo test -p vox-cli --lib chat_routing_ ; cargo test -p vox-gui --bin vox-gui models_host
        status: pass
    human_judgment: false

duration: 45min
completed: 2026-10-29
status: complete
plan_head_before: 839bffed013bb3634203b60691ebe7f37c55023c
plan_head_after: 00d75032f700936e6061579221251db1e2bf986f
---

# Phase 318 Plan 03: Chat and GUI use the measured pin Summary

**Chat auto-local and the GUI host take the measured QLoRA pin from free memory, or an error when the reading cannot select one.**

## Accomplishments

- `select_from_free_mb` calls `select_qwen3_code_qlora_for_free_mb` under the existing `runtime` feature.
- `select_tier_for_vram` and the `mens/runs/qwen3_27b_metal_check/*` and `vox-mens-8b-v0.6` map are deleted, including the re-exports.
- `select_optimal_local_model` returns `Result`, calls `read_accelerator_free_mb`, then `select_from_free_mb`.
- `resolve_chat_config_with_auto_pick` uses an injected result for `auto-local`. `None` still calls the live function.
- The GUI command maps that `Result`. The unit test injects Ok and Err and does not call the live command.

## Tests

- `cargo test -p vox-orchestrator --lib select_from_free_mb -- --test-threads=8`: 1 passed.
- `cargo test -p vox-orchestrator --lib auto_select -- --test-threads=8`: 3 passed. None call `read_accelerator_free_mb`.
- `cargo test -p vox-cli --lib chat_routing_ -- --test-threads=8`: 2 passed.
- `cargo test -p vox-gui --lib models_host` failed: `vox-gui` has no library target (exit 101).
- `cargo test -p vox-gui --bin vox-gui models_host -- --test-threads=8`: 2 passed.
- `rustfmt --edition 2024 --check` on the five files: exit 0.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] vox-gui has no lib**
- **Found during:** Task 3
- **Issue:** `--lib` cannot run `models_host` tests.
- **Fix:** Ran the same filter on `--bin vox-gui`.
- **Files modified:** none
- **Commit:** n/a

**2. [Rule 1 - Bug] Tauri command attribute**
- **Found during:** Task 3
- **Issue:** The mapper insertion left `#[tauri::command]` on `recommendation_from_selection`, so the invoke handler could not see `get_auto_model_recommendation`.
- **Fix:** Moved the attribute back onto the command.
- **Files modified:** `crates/vox-gui/src/commands/models_host.rs`
- **Commit:** 00d75032f

**3. [process] Tests and implementation share each task commit**
- Same as plans 01 and 02. No separate red commit.

## Self-Check: PASSED

- Commits `74ddd3e13`, `9bd278410`, and `00d75032f` are ancestors of HEAD.
