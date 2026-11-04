---
phase: 321-harness-before-after-harn-01
plan: 01
subsystem: mens
tags: [mens, ladder, harness, vox-eval, vox-ml-cli, voxscript]

requires:
  - phase: 320-train-the-sizes-size-03
    provides: ladder.json trained 0.6B/1.7B adapters and not_trained rungs
provides:
  - mens_size_ladder_before_after.v1 aggregate builder in vox-eval
  - resolve_serve_run_dir HARN-01 guard wired into serve dispatch
  - scripts/mens-ladder-before-after.vox plan-only orchestration skeleton
affects: [321-03, 321-04, BENCH-04, HARN-01]

actuals:
  tokens: 28000
  tasks: 3
  commits: 2

plan_head_before: 0db5319e37aa6453bd61b593adc5e7c35ee14632
plan_head_after: 06816f4fd2365a5ff94233989df83823b48310d9
commits: 2

tech-stack:
  added: []
  patterns:
    - "Aggregate rollup schema with gate_input false and not_trained score-key prohibition"
    - "Serve --model must be run directory; adapter file refused before collateral gate"

key-files:
  created:
    - crates/vox-eval/src/ladder_before_after.rs
    - crates/vox-ml-cli/src/commands/mens/ladder_serve.rs
    - scripts/mens-ladder-before-after.vox
    - crates/vox-eval/tests/fixtures/ladder_before_after/
  modified:
    - crates/vox-eval/src/lib.rs
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs

key-decisions:
  - "Intelligence leg reads pass_rate_at_1 and total from eval-local JSON fixtures"
  - "Task 3 script shipped in the same commit as ladder_serve (single staged wave)"

patterns-established:
  - "Recursive forbidden-key walk for not_trained ladder rows"
  - "VoxScript plan-only lists trained pipeline steps without vox mens invocation"

requirements-completed: [BENCH-04, HARN-01]

coverage:
  - id: D1
    description: mens_size_ladder_before_after.v1 builder and tests
    requirement: BENCH-04
    verification:
      - kind: unit
        ref: "cargo test -p vox-eval --lib ladder_before_after"
        status: pass
    human_judgment: false
  - id: D2
    description: resolve_serve_run_dir refuses adapter file paths
    requirement: HARN-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-ml-cli --features gpu --lib ladder_serve"
        status: pass
    human_judgment: false
  - id: D3
    description: plan-only orchestration script lists 0.6B, 1.7B, not_trained rungs
    requirement: BENCH-04
    verification:
      - kind: other
        ref: "vox run --interp scripts/mens-ladder-before-after.vox -- --plan-only"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-28
status: complete
---

# Phase 321 Plan 01: Harness foundations Summary

**Ladder rollup schema, serve-directory guard, and VoxScript plan-only orchestration for Phase 321 Metal scoring waves.**

## Performance

- **Duration:** ~25 min
- **Tasks:** 3/3
- **Commits:** 2 (plan ledger `0db5319e..06816f4fd`)

## Accomplishments

- `vox-eval::ladder_before_after` emits `mens_size_ladder_before_after.v1` with trained reasoning/intelligence blocks and status-only not_trained rungs.
- `resolve_serve_run_dir` blocks `candle_qlora_adapter.safetensors` file paths and symlinks; wired into local `PopuliAction::Serve`.
- `scripts/mens-ladder-before-after.vox --plan-only` reads Phase 320 `ladder.json` and prints ordered steps for 0.6B/1.7B without score-like tokens.

## Deviations from Plan

### Auto-fixed Issues

None — behavior matches plan intent.

### Commit grouping

**Task 3 script** landed in commit `06816f4fd` together with Task 2 (staged in one wave). All three tasks verified; commit count is 2 not 3.

## Self-Check: PASSED

- FOUND: crates/vox-eval/src/ladder_before_after.rs
- FOUND: crates/vox-ml-cli/src/commands/mens/ladder_serve.rs
- FOUND: scripts/mens-ladder-before-after.vox
- FOUND: commit 4e6c9c3c8
- FOUND: commit 06816f4fd
