# Phase 320: Train the Sizes - Context

**Gathered:** 2026-10-27
**Status:** Ready for planning
**Order:** Runs ahead of Phase 319 (user decision 2026-10-27: real runs first; mesh selection does not affect training on this Mac).

<domain>
## Phase Boundary

Train Metal QLoRA adapters with the lane prefix for the Qwen3 ladder sizes that fit this Mac, using the Phase 318 live selector, and record sizes that don't fit as not trained with measured memory. Spend GPU time only after cheap checks pass: every stage proves the one before it worked (selection, corpus, smoke, probe) before a longer run starts.

Inputs audit: `.planning/research/v5.6-training-inputs-audit.md` (blockers 1-7). Phase 317 already fixed blocker 5; Phase 318 replaced blocker 6 (no `--model` now picks live, so an explicit pin is still passed for each size).

</domain>

<decisions>
## Implementation Decisions (user-accepted 2026-10-27)

- **D-01 Staged, cheapest first.** The stages are:
  - S0 plan-only dry run, no GPU.
  - S1 corpus build and QA.
  - S2 0.6B smoke run, minutes long.
  - S3 one memory probe per size, short.
  - S4 full runs.

  A stage starts only when the previous stage's check passed. Each stage's evidence is written to the phase directory.
- **D-02 Plan-only dry run.** `vox mens train --plan-only` resolves everything a run would and prints it, then exits 0. It does not download, load weights or train. It prints:
  - the free-memory reading and its source
  - the size pick (auto or explicit) and its reason
  - the pin, preset, seq_len and epochs
  - whether the base snapshot is cached
  - the corpus path and row count
  - the contamination check result

  This verifies model selection on the real vm_stat reading at zero GPU cost, and closes Phase 318's carried human item (the live pick on this Mac). Expose it to scripts as JSON (`--json`). A run that would refuse also refuses under `--plan-only`, with the same error.
- **D-03 Corpus.** Build `target/dogfood/train.jsonl` with the existing `vox mens corpus extract` and `vox mens corpus pairs`, plus the mix step.
  - QA: no empty rows and no exact duplicates. Count the direct-answer and think rows.
  - Contamination guard: training must refuse any row whose normalized question matches a held-out `mens/data/*_think` slice question. Use an existing preflight surface in vox-corpus, not a new crate, and test it first.
  - The audit says e2e `metal-e2e` rows are double-wrapped ChatML; they are not used.
- **D-04 seq_len.** Fit the system prompt plus fine-tune addendum (about 2K tokens) without middle truncation. Pass `--seq-len 3072` per run, or set the new size presets' seq_len, whichever is the smaller change. A row that is still truncated is counted and reported.
- **D-05 Smoke run (S2).** Train 0.6B at pin `c1899de…` with the run time capped at about 5-10 minutes. It must show all of the following:
  - loss is recorded and falls
  - the adapter and `adapter_manifest.json` name the pinned base
  - resume refuses a different base (318 CR-02)
  - `vox mens serve` loads the adapter
  - one held-out prompt gets an answer
  - peak memory is recorded

  If any check fails, fix the code before any larger run.
- **D-06 Probe per size (S3).** Go up the ladder: 1.7B, 4B, 8B, 14B, 32B. Each probe is a short run, a few optimizer steps or a run cap of a couple of minutes.
  - Measure peak memory: Metal `currentAllocatedSize` or the process peak, whichever the trainer can read.
  - A size whose probe OOMs, or whose measured peak exceeds the live budget, is recorded as not trained, with the measurement, and gets no adapter.
  - Climbing stops at the first size that doesn't fit. Larger sizes are recorded as not attempted (above a non-fitting size).
- **D-07 Full runs (S4), GPU budget.**
  - 0.6B, 1.7B, 4B and 8B get full runs: one pass over the corpus at the size's preset, with the lane prefix.
  - 14B and 32B, if their probes fit, get time-capped runs of about 4 h each (`--max-runtime-secs 14400`). The record marks the run as capped, with steps completed.
  - Long runs go in the background, and each is checked by its log at intervals, not by polling loops. A run that diverges (NaN or rising loss) is stopped early.
- **D-08 Records.** Each size's measured `peak_mb` and `measured_on` go into `mens/config/gpu-specs.yaml`. A trained rung gets `floor_measured: true` (meeting Phase 317's rule `0 < peak_mb <= floor_mb`; raise `floor_mb` when the measured peak exceeds it). Update the IN-02 test accordingly.

  A per-size record (`mens/runs/<size>/train-record.json`, or one ladder report) holds:
  - status: trained, capped, or not trained
  - pin and preset
  - steps and final loss
  - peak memory and wall time
- **D-09 Binaries.** Rebuild the release `vox` from this branch with `--features gpu,execution-api` (or whatever `vox doctor` names), and reinstall the mens-candle-metal plugin before S0. Each run uses that binary.

### Claude's Discretion
- The exact probe step count and run caps.
- How peak memory is read on Metal.
- Plan split. Suggested: one plan for code (S0 flag, contamination guard, peak-memory recording, record file), then plans for operating the runs.

</decisions>

<code_context>
## Existing Code Insights

- **Training call chain.** `vox mens train` → `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` → `gpu.rs` (`adopt_size_pick`, re-check before download) → `vox_populi::mens::run_mens_training` → `vox-plugin-mens-candle-metal/src/candle_qlora_train/`.
- **Selector.** It lives in `crates/vox-populi/src/mens/tensor/size_select.rs`. `read_free_memory` uses `working_set_is_live` (318 CR-01).
- **Corpus gate.** `check_corpus_threshold` requires at least 100 lines in `<data-dir>/train.jsonl` (`run_train.rs`).
- **Checkpoint.** `CheckpointState.base_model` exists (318 CR-02).
- **Metal fit check.** Metal has no memory-model calibration row, so the `plan_for` fit check only warns ("UNVERIFIED"). The measured peaks from this phase are what calibrate it.
- **Host.** The Metal recommendedMaxWorkingSetSize is 110101 MB (likely 128 GB).

</code_context>

<deferred>
## Deferred Ideas

- Full untimed 14B and 32B runs on Metal.
- A CUDA or mesh node for the larger sizes (Phase 319 and later).
- Calibrating the memory model's Metal row from these peaks, beyond `floor_mb`.

</deferred>
