---
phase: 320-train-the-sizes-size-03
plan: 05
subsystem: mens-training
tags: [mens, qlora, metal, smoke, d-05, d-06, size-03]
status: complete
requires: [320-04]
provides:
  - "mens/runs/320-smoke-0.6b (gitignored): capped 0.6B adapter, 29 optimizer steps, pin in manifest and summary"
  - "320-S2-SMOKE.md attempt 2 with the six D-05 checks; Stage S2: FAIL — held-out answer"
  - "evidence/s2-* (attempt 2) and evidence/s2-attempt1/ (attempt 1 history)"
affects: [320-06, 320-07, 320-08, 321]
tech-stack:
  added: []
  patterns: ["loss falling judged on run-scoped telemetry rows (after the last train_start), quarter and 5-step windows"]
key-files:
  created:
    - .planning/phases/320-train-the-sizes-size-03/evidence/s2-attempt1/
    - .planning/phases/320-train-the-sizes-size-03/evidence/s2-*
  modified:
    - .planning/phases/320-train-the-sizes-size-03/320-S2-SMOKE.md
decisions:
  - "D-05 serve decision stays accept-eval-local (user, recorded in attempt 1); serve refused at the Phase 315 gate as expected"
  - "Run C forced with VOX_MENS_FORCE_TRAIN=1 after the memory estimate refused it before the base check; a watcher guarded P-320-10"
  - "Stage S2: FAIL — eval-local refuses the adapter (tied-head LoRA delta model.embed_tokens.weight never applied); code fix needed before S3"
metrics:
  duration: "~15 min (02:56-03:11 UTC 2026-10-28)"
  completed: 2026-10-28
actuals:
  tokens: 9000
  tasks: 3
  commits: 1
plan_head_before: 2c98828a6b0282c10592f073c4184f7575e282b0
plan_head_after: 49173587ecdd357a671c665a6e06031435b4aeb8
---

# Phase 320 Plan 05: S2 0.6B smoke run (attempt 2) Summary

The second attempt ran on the fixed binaries (`vox 0.6.0+build.7428 (55a338c53)`, plugin sha1 `e1eb8287…`). The 0.6B smoke trained 29 steps under the 480 s cap with a flat ~44 GiB peak, and its loss fell. The adapter names the pin, and resume refuses the 1.7B base. Serve refused at the Phase 315 gate as expected. eval-local refuses to load the adapter: the tied-head LoRA delta, keyed `model.embed_tokens.weight`, is never applied by Metal inference. **Stage S2: FAIL — held-out answer.**

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1-2 | `49173587e` | docs(320-05): S2 smoke run evidence, attempt 2 |

## Results

| D-05 check | Result | Key evidence |
|---|---|---|
| 1. Loss falls | PASS | n = 29. Quarter windows 1.6225 → 1.2988; 5-step windows 1.8224 → 1.6266. Final loss EMA 1.4059. |
| 2. Manifest names the pin | PASS | `run_summary.base_model` is `Qwen/Qwen3-0.6B@c1899de…`; `adapter_manifest.base_model` is the `…/snapshots/c1899de…` directory. |
| 3. Resume refuses a different base | PASS | `was trained on base Qwen/Qwen3-0.6B@c1899de…, but this run's base is Qwen/Qwen3-1.7B@70d244cc…`. The checkpoint's bytes are unchanged. |
| 4. Serve | REFUSED AT THE GATE (expected) | `eval_collateral_damage check not found!`, exit 1. No gate files exist. |
| 5. Held-out answer | **FAIL** | `1 trained LoRA delta(s) were never applied to any weight: ["model.embed_tokens.weight"]`, exit 1, no JSON written. |
| 6. Peak recorded | PASS | 45,994,475,520 B (43,864 MiB) in `run_summary`, `train-record` and step telemetry. The estimate was 59,149 MB and the host had 64,599 MB free. |

Run record:

| field | value |
|---|---|
| status | `capped` |
| steps | 29 |
| train wall | 491.5 s |
| total wall | 511.1 s |
| truncated / trimmed rows | 0 / 0 |

## Deviations from Plan

1. **Runs went through `target/release/vox mens …`, not `$ML`.** The shell allowlist blocks calling `vox-ml-cli` directly. `vox` forwards argv unchanged. The SIGINT for Run B went to the `vox-ml-cli` PID.
2. **Run C was refused twice.** The first refusal came from the new Metal memory estimate (65,896 MB against 65,502 MB free), after the 1.7B download and before the base check. The run was repeated once with `VOX_MENS_FORCE_TRAIN=1` so the base check could run. A watcher would have sent SIGINT on `Masked CE numeric preflight` or `Training started` (P-320-10); it never fired. Both outcomes are in the evidence.
3. **Loss was measured on run-scoped telemetry.** `--force-restart` does not truncate `telemetry.jsonl`, which still held attempt 1's `train_start` and its 374 GB step-0 row. Only rows after the last `train_start` were used.
4. **The acceptance check "no `Downloading base model` line" fails as written.** The line also prints on a cache hit. Blob listings before and after the run are identical, so nothing was fetched.
5. **Task 3 (`checkpoint:decision`, blocking-human) was not re-asked.** The user's attempt 1 decision `accept-eval-local` stands. It does not turn S2 into a PASS, because eval-local itself failed.
6. **Concurrent commits.** `bc034f7e9` (query-chunked attention) and `2c98828a6` (calibration cell) landed on the branch during this run from another session. The binaries used here predate them: `vox`, `vox-ml-cli` and the plugin have timestamps of 19:26-19:53, and the plugin sha1 `e1eb8287…` was unchanged before and after.

## Deferred Issues (code, not patched per instructions)

- **Blocks S3.** The Metal trainer's tied-head LoRA (`base_key_map["lm_head"] = "model.embed_tokens.weight"`, `candle_qlora_train/mod.rs:1006-1020`) is never requested by Metal inference, and `assert_adapter_fully_applied` (`inference.rs:830-850`) refuses it. Every tied-embedding base is affected: 0.6B, and 1.7B, which was confirmed tied.
- Hygiene: `--force-restart` should truncate `telemetry.jsonl`.
- Ordering: a mismatched `--resume` should be refused before the memory estimate and the base download.

## Self-Check: PASSED

- `320-S2-SMOKE.md` ends with `Stage S2: FAIL — …`.
- Commit `49173587e` is an ancestor of HEAD.
- No `*.safetensors` file was committed.
- No `collateral_damage_report.json` or `gate_receipt.json` exists under the run dir.
