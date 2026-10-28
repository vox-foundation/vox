# Phase 320 — Stage S2: 0.6B smoke run (D-05)

**Run:** 2026-10-27 18:30-18:36 local (2026-10-28T01:30Z), binaries `900ceb5bf` (S0), Metal plugin sha256 `68b80d73…` (S0).
**Command:** `target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --model Qwen3-0.6B --output-dir mens/runs/320-smoke-0.6b --max-runtime-secs 480 --force-restart`, wrapped in `timeout --signal=INT --kill-after=300s 3600s`. `vox` forwards to the sibling release `vox-ml-cli`. The shell allowlist blocks calling `vox-ml-cli` directly (320-04 deviation 1).

**Result: FAIL.** The trainer died after its first micro-step, 354 s after launch and before the 480 s cap. It exited 1 with no error line and wrote no adapter, `run_summary.json` or `train-record.json`. Per D-05 and Plan 05 Task 1 step 4, the phase stops here. Code must be fixed before any further GPU run, and this plan does not patch code.

## Inputs (recorded as instructed)

- **Corpus.** `target/dogfood/train_mixed.jsonl` holds 1166 rows: 0 think and 1166 direct-answer (S1). The smoke run exercises only the lane prefix's direct-answer path.
- **Preset.** 0.6B resolves to `qwen3_dev_cpu` (the smoke tier). The run's `training_manifest.json` reports `rank: 8`, `alpha: 16`, `seq_len: 3072`, `epochs: 1` and **`grad_accum: 4`**, and the log reports `batch_size=2` and `gradient_checkpointing=false`. The orchestrator's brief said grad_accum 1, but the run used 4: telemetry `train_start` plans 1108 micro-steps and 277 optimizer steps.
- **User decision (relayed by the orchestrator, 2026-10-27).** The full S4 0.6B run uses `--preset qwen3_6g`. The smoke run keeps `qwen3_dev_cpu`.

## The six D-05 checks

### 1. Loss recorded and falling — FAIL

- `evidence/s2-telemetry.jsonl` has one `step` row: `optimizer_step 0`, `loss 3.1694371700286865`, `valid_tokens 63`, `theoretical_tokens 2333`.
- `evidence/s2-loss.json`: `n = 1`, `q = 0`, `any_non_finite: false`. The plan needs `n >= 8` with the first-quarter mean above the last-quarter mean, so a falling loss cannot be shown.
- The `Masked CE numeric preflight passed` line shows the loss is finite.
- Log (`evidence/s2-smoke-log.txt`), last lines:
  ```text
  [mens] Masked CE numeric preflight passed (finite loss on first eligible batch).
  [mens] Training started: first backward pass done after 235.6s (277 optimizer steps planned, grad_accum=4).
  [mens] E1/1 step 0/277 (0.0%) micro 1 | loss 3.1694 (last 3.1694) | lr 1.00e-5 | 0 tok/s | ETA -- | skipped curric=0 short=0 no_sup=0 nonfinite=0 oob=0
  ```
  Nothing follows; the process was gone about 46 s later. `evidence/s2-smoke.exit` is `1`.

### 2. Manifest names the pinned base — PARTIAL (no adapter)

- No `adapter_manifest.json` or `run_summary.json` was written.
- The pre-training `training_manifest.json` (`evidence/s2-training-manifest.json`) does name the pin: `"base_model":"Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca"`.
- Cache: no new blob under `models--Qwen--Qwen3-0.6B` (`evidence/s2-smoke-timing-and-cache.txt`: every blob is dated 2026-09-05 or 2026-09-11), so nothing was downloaded. The log prints `📥 Downloading base model from Hugging Face: …` and then `✓ Cached at …` even on this cache hit. The line is misleading but harmless, and it makes the acceptance check "no `Downloading base model` line" fail literally.

### 3. Resume refuses a different base — NOT RUN

It needs a paused 0.6B checkpoint (Run B), and Run B would train the same 0.6B config that just died. It was not attempted.

### 4. `vox mens serve` loads the adapter — NOT RUN (no adapter)

### 5. One held-out prompt answered (eval-local) — NOT RUN (no adapter)

### 6. Peak memory recorded and above zero — RECORDED, value is the failure signal

- The step row reports `peak_metal_allocated_bytes: 374417047552`, about **374,417 MB (~349 GiB)**. The reading is Metal `currentAllocatedSize`, sampled by `PeakSampler` in `vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs:104-108`.
- Context:
  - the host's Metal `recommendedMaxWorkingSetSize` is 110101 MB
  - the live free reading was 58869 MB
  - the pre-run estimate was `VRAM: est. ~1618 MB`
  - the 0.6B floor is 2000 MB

  The measured allocation is about 230 times the estimate and about 3.4 times the working set.
- There is no `run_summary.json` or `train-record.json`, so no peak exists there.

## Diagnosis (observed, not fixed)

- **Memory runaway on the first step.** One micro-step (batch 2, padded length about 2333, 28 layers, `gradient_checkpointing=false`) allocated about 374 GB of Metal memory. The 235.6 s first backward pass is consistent with heavy swapping.
- **Likely a silent memory kill.** The process died 46 s after the first step with no Rust error, no panic and no crash report (`~/Library/Logs/DiagnosticReports` and `/Library/Logs/DiagnosticReports` have nothing for `vox-ml-cli`). It was not the timeout: 354 s is under 3600 s, and the 480 s cap was not reached. This plan's shell cannot read `log show`, `vm_stat` or `top`, so the killing signal is not confirmed.
- **The launcher hides signal deaths.** `crates/vox-cli/src/main.rs` forwards with `std::process::exit(status.code().unwrap_or(1))`, so a child killed by a signal shows up as a plain exit 1 with no message. Calling `vox-ml-cli` directly would show the signal, but the allowlist blocks it.
- **Signal deaths leave no record.** Plan 03's record writer runs in-process, so a run killed by a signal leaves no `train-record.json`.
- **Where to look in the fix plan.** The trainer's seq_len 3072 memory behavior on Metal: attention materialization or retained buffers without gradient checkpointing, and candle's Metal buffer pool. The memory estimate (1618 MB) misses it by two orders of magnitude, so the D-04 `--seq-len 3072` choice needs that fix, or a measured fallback, before S3.

## Carried items

- W1: serve load of a 320 adapter is verified in Phase 321 once the gate files exist.

D-05 serve decision: accept-eval-local (2026-10-27, user, relayed by the orchestrator)
Stage S2: FAIL — loss (trainer killed after optimizer step 0 with a ~374 GB Metal peak; no adapter)
