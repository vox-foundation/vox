---
phase: 320-train-the-sizes-size-03
plan: 05
subsystem: mens-training
tags: [mens, qlora, metal, smoke, d-05, d-06, size-03]
status: complete
requires: [320-04]
provides:
  - "mens/runs/320-smoke-0.6b-a3 (gitignored): capped 0.6B adapter, 60 optimizer steps, pin in manifest and summary, resume state kept"
  - "320-S2-SMOKE.md attempt 3 with the six D-05 checks; Stage S2: PASS (attempts 1-2 kept as history)"
  - "evidence/s2-* (attempt 3), evidence/s2-attempt2/ and evidence/s2-attempt1/ (history)"
affects: [320-06, 320-07, 320-08, 321]
tech-stack:
  added: []
  patterns:
    - "resume refusal tested against the capped run's own kept checkpoint instead of a SIGINT'd Run B"
    - "eval-local subset via a --bench directory holding a trimmed manifest.json"
key-files:
  created:
    - .planning/phases/320-train-the-sizes-size-03/evidence/s2-attempt2/
    - .planning/phases/320-train-the-sizes-size-03/evidence/s2-*
  modified:
    - .planning/phases/320-train-the-sizes-size-03/320-S2-SMOKE.md
    - .planning/phases/320-train-the-sizes-size-03/320-05-SUMMARY.md
decisions:
  - "D-05 serve decision stays accept-eval-local (user, attempt 1); serve refused at the Phase 315 gate as expected"
  - "eval-local scored on the first 8 held-out tasks at --max-tokens 1024: the full 52-task bench projected to ~2.7 h"
  - "Stage S2: PASS on the plan's criteria, with two caveats: 5-step loss window flat, and no complete answer text quotable (eval-local stores a 240-char preview)"
metrics:
  duration: "~35 min (04:44-05:19 UTC 2026-10-28)"
  completed: 2026-10-28
actuals:
  tokens: 12000
  tasks: 3
  commits: 1
plan_head_before: 6d326bf745a627e81eb7cd4a35a62616b6a86113
---

# Phase 320 Plan 05: S2 0.6B smoke run (attempt 3) Summary

Attempt 3 ran on `vox 0.6.0+build.7442 (cdc8fdc3a)`. The installed Metal plugin is byte-identical to `target/release` (sha256 `73e98f3d…`). The 0.6B smoke trained 60 steps under the 480 s cap at seq_len 3072. Its peak was 24.8 GB, flat from the first full-length row, and its quarter-window loss fell. The capped run kept its resume state. A resume onto 1.7B was refused in 0.43 s with no download, and the checkpoint was unchanged afterwards. Serve was refused at the Phase 315 gate, as expected. eval-local loaded the adapter and scored pass@1 3/8 on a subset of the held-out tasks. **Stage S2: PASS.**

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1-2 | see git log | docs(320-05): S2 smoke run evidence, attempt 3 |

## Results

| D-05 check | Result | Key evidence |
|---|---|---|
| 1. Loss falls | PASS (quarter) | n = 60. Quarter windows 1.7491 → 1.1632. The 5-step windows went 1.8251 → 1.8373 and did **not** fall (batch-1 row noise; per-row loss ranged 0.005-3.65). Final loss EMA 1.3123. |
| 2. Pin and checkpoint kept | PASS | `run_summary.base_model` is `Qwen/Qwen3-0.6B@c1899de…` and `capped` is true. The record's status is `capped`. `checkpoint_state.json` (global_step 60) and `checkpoint_step_60.safetensors` remain. |
| 3. Resume refuses a different base | PASS | Exit 1 in 0.43 s, no `Downloading` line, `was trained on base …0.6B@c1899de…, but this run's base is …1.7B@70d244cc…`. Checkpoint sha1s are unchanged. |
| 4. Serve | REFUSED AT THE GATE (expected) | `eval_collateral_damage check not found!`, exit 1. No gate files exist. W1 is carried. |
| 5. Held-out answer | PASS | 8-task subset at 1024 tokens: pass@1 3/8 (action_delete, test_smoke, page_index), semantic 0/8, 5/8 still inside `<think>`. No errors. The full text is not persisted: the preview is 240 chars. |
| 6. Peak recorded | PASS | 24,824,217,600 B (23,675 MiB) in `run_summary`, `train-record` and telemetry. It rose 19.2 → 20.8 GB, then stayed flat at 24.8 GB from step 33, the first 3071-token row. The estimate of 20,725 MiB is 12% low. |

Run record: `capped`, 60/60 steps, train wall 486.3 s, total wall 491.3 s, 2 truncated and 2 trimmed rows, 145,261 tokens.

## Deviations from Plan

1. **Runs went through `target/release/vox mens …`, not `$ML`.** The shell allowlist blocks running `vox-ml-cli` directly.
2. **No separate Run B.** The capped smoke run now keeps `checkpoint_state.json` (`afb5ef543`), so the 1.7B refusal was run against that checkpoint (`--resume mens/runs/320-smoke-0.6b-a3 --output-dir mens/runs/320-resume-1.7b-a3`). No `Training paused` evidence exists for this attempt.
3. **eval-local ran on a subset at `--max-tokens 1024`**, per the orchestrator: the first 8 of the 52 tasks, through a trimmed `manifest.json` passed as `--bench`. The plan's bench path and `--max-tokens 128` were not used. A 2-task probe and a single `test_smoke` run at 4096 tokens are also kept.
4. **The plan's acceptance check "no `Downloading base model` line" still fails as written.** The line also prints on a cache hit.

## Deferred Issues (not patched, per instructions)

- eval-local stores only `completion.chars().take(240)` (`eval_local.rs:288`), so complete answers cannot be quoted.
- The memory estimate is 12% low at a full 3072-token row. 4B and larger estimates are extrapolations from the same fit.
- At 1024 tokens, 5 of 8 answers stay inside `<think>`.

## Self-Check: PASSED

- `320-S2-SMOKE.md` ends with `Stage S2: PASS`.
- Task 1's automated verify, adapted to the `-a3` dir and `capped == true`, printed `TASK1-VERIFY-PASS`. The Task 2 checks that apply printed `TASK2-CHECKS-PASS`.
- No `*.safetensors` file was staged.
- No `collateral_damage_report.json` or `gate_receipt.json` exists under the run dir.
