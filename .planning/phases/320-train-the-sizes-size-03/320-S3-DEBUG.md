---
status: resolved
trigger: "S3 gap closure: peak Metal memory rises with each accumulated micro-step inside one optimizer step (1.7B accum 8: +12.2 GB row 1 -> row 2, peak 54.6 GB vs 37.7 GB at accum 1; 4B accum 8: 59.7 -> 86.3 GB at row 2, killed)"
created: 2026-10-27
updated: 2026-10-29
---

# Phase 320 S3 debug: memory growth under gradient accumulation (Metal QLoRA)

## Current Focus

hypothesis: (confirmed) the trainer's pending LoRA grads, held from micro-step k to k+1 when accum > 1, sit in oversized pooled Metal buffers and pin them; the next micro-step allocates its working set again.
bug_class: Bohrbug (deterministic; same rows, same placement every run)
test: fixed plugin + qlora-rs; full S4 runs (0.6B accum 1, 1.7B accum 8) and the 4B qwen3_12g accum 8 re-probe
expecting: no row-2 jump at accum 8; peaks below the pre-fix accum-1 peaks
next_action: none (resolved; live measurements under "Live measurements (after the fix)")

reasoning_checkpoint:
  hypothesis: "Per-row peak grows at accum > 1 because the LoRA grads the trainer holds between micro-steps occupy oversized buffers in candle's never-shrinking private Metal pool (best fit = any free buffer >= the request), so holding ~25 MB of grads pins 6-13 GB of buffers that the next micro-step must allocate again."
  confirming_evidence:
    - "instrumented 0.6B accum 8: 394 grads, 25 MB logical, 6.0 GB of pooled buffers at micro-step 1, 12.9 GB at 2, 9.3 GB after"
    - "0.6B accum 1 vs 8 on the same rows: 19.1 -> 19.95 GB flat vs 19.1 -> 30.1 -> 31.8 GB flat; growth tracks the LoRA-grad count (394 at 0.6B and 1.7B: +11.0 / +12.2 GB; 506 at 4B: +26.6 GB), not the parameter count"
  falsification_test: "if LoRA grads held on the host (no Metal buffer at all) still leave accum 8 ~11 GB above accum 1, something else is held"
  fix_rationale: "fold each segment's / head chunk's LoRA grads into a host GradStore as soon as they are computed, so no LoRA grad outlives the segment that produced it in the Metal pool; the trainer then holds host grads across micro-steps and moves them to each var's device only for clip + AdamW"
  blind_spots: "per-segment device->host copies add syncs (s/row to be measured); the step-time host->device copies land in the shared pool (exact size, swept); the 4B fit depends on how much of its row-1 peak is the same placement"
  candidate_causes:
    - "code: trainer holds device grads across micro-steps; checkpointed_grads holds them across segments"
    - "environment: candle 0.10.2 Metal private pool best-fit-any-size and never released"
  and_gate: "yes - needs long-lived small tensors (code) AND a pool that hands them large buffers and never shrinks (environment); either alone is harmless"

## Symptoms

expected: accumulating rank-16 LoRA grads across micro-steps costs megabytes; per-row peak at accum 8 equals accum 1
actual: 1.7B qwen3_6g accum 8 per-row peak 35.53 / 47.77 / 50.32 / 50.70 GB (accum 1: 35.5 then flat 37.70 GB); 4B qwen3_12g accum 8 59.70 / 86.33 / 88.65 GB, watchdog SIGINT
errors: none (watchdog kill)
reproduction: target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --model Qwen3-1.7B (preset accum 8) vs --grad-accum 1
started: visible since 320-S2 gap closure (chunked attention); 1.7B accum 8 50.5 GB vs accum 1 37.7 GB

## Eliminated

- hypothesis: something sized like the base parameters (frozen-weight grads, a second BF16 dequant cache, grads for every var) is retained per micro-step
  evidence: 0.6B (0.6 B params) grows +11.0 GB at row 2, 1.7B (2.0 B params) +12.2 GB: same layer count (28, 394 LoRA vars), 3x the parameters, about the same growth. 4B (36 layers, 506 LoRA vars) +26.6 GB. The trainer's GradStore keeps only `varmap.all_vars()` (LoRA A/B) in `pending_grads`; base weights come from `vb_mmap`, not the trainer's VarMap; checkpointed_grads folds only LoRA vars and drops each segment's GradStore (frozen-weight grads included) before the next.
  timestamp: 2026-10-27
- hypothesis: the autograd graph of micro-step k stays alive into k+1
  evidence: run_training_loop keeps nothing from a micro-step (forward result is moved into checkpointed_backward_step, CheckpointedCe is consumed); grads are detached in candle's backward and `or_insert` zeros + detached adds record no op; peaks go flat from row 3-4 instead of growing per micro-step.
  timestamp: 2026-10-27

## Evidence

- timestamp: 2026-10-27
  checked: code path micro-step -> forward_masked_ce (checkpointed) -> checkpointed_backward_step -> checkpointed_grads -> QLoraTrainer::optimizer_step_with_grads -> accumulate_and_maybe_step (patches/qlora-rs-1.0.5/src/training.rs L760-808)
  found: the only state that crosses a micro-step at accum > 1 is `pending_grads: HashMap<TensorId, Tensor>` holding `g.clone()` of each LoRA var's grad (L782-786), i.e. the exact tensors computed during the backward. At accum 1 those tensors are dropped right after the AdamW step.
  implication: if those few-hundred-KB tensors occupy big buffers, holding them pins the buffers.
- timestamp: 2026-10-27
  checked: candle-core 0.10.2 metal_backend/device.rs `new_buffer` / `find_available_buffer` / `drop_unused_buffers`
  found: every op output (binary add included, i.e. the final `sum_grad.add(&grad)` that produces a LoRA grad in backward) comes from `private_buffers` via `find_available_buffer(size)`: the smallest bucket >= size that has a free (strong_count == 1) buffer, of ANY larger size. New buffers are next_power_of_two sized. `drop_unused_buffers` sweeps only the shared pool, so private buffers are never released; `currentAllocatedSize` is the private pool's high-water mark plus live shared buffers.
  implication: a small tensor allocated while only large buffers are free takes a large one, and that buffer is unusable for as long as the small tensor lives.
- timestamp: 2026-10-27
  checked: baseline, installed binaries (plugin 73e98f3d), 0.6B `--seq-len 3072 --grad-accum 8 --max-runtime-secs 120`, mens/runs/320-debug-accum-06b-a8-before, watchdog limit 68,273,831,936 B (69,207 MB free - 4 GiB)
  found: per-row peak 19.09 / 30.10 / 31.76 GB, then flat 31.76 GB through row 16 (2 optimizer steps, capped at 147 s, exit 0). Same rows as before (2333/2277/2280/... tokens). Earlier 0.6B accum 1 on the chunked build: 19.28 then flat 19.95 GB.
  implication: reproduces at 0.6B; +11.0 GB at row 2 with 0.6 B params, so not proportional to parameters.
- timestamp: 2026-10-27
  checked: baseline, same binaries, 0.6B `--grad-accum 1 --max-runtime-secs 60`, mens/runs/320-debug-accum-06b-a1-before
  found: per-row peak 19.12 / 19.34 / 19.34 / 19.36 / 19.43 / 19.52 / 19.52 / 19.95 / 19.95 GB, 9 optimizer steps in 62 s, exit 0.
  implication: the same rows cost ~19-20 GB when nothing is held between micro-steps; accum 8 adds ~11.8 GB.
- timestamp: 2026-10-27
  checked: temporary instrumentation (not committed; forward.rs restored from a cp backup, diff -q identical, sha256 d6a8ca20...), plugin built from it and installed (b96ebc64...), `VOX_MENS_DEBUG_GRAD_BUFFERS=1`, 0.6B `--grad-accum 8 --max-runtime-secs 60`, mens/runs/320-debug-accum-06b-a8-instr. Per micro-step: Metal buffer length behind each LoRA grad handed to the trainer, vs its logical size.
  found: 394 LoRA grads, 25,079,808 B logical. Micro-step 1: they occupy 6,011,387,904 B of pooled buffers (141 of them >= 4x their size; 14 sit in 256 MB buffers, 91 in 16 MB). Micro-step 2: 12,900,663,296 B (368 oversized, 4 in 512 MB buffers). Micro-steps 3-8: 9,312,174,080 B each (255 x 16 MB, 102 x 32 MB, 8 x 64 MB, 4 x 256 MB). `currentAllocatedSize` at the end of micro-step 1: 19.24 GB; micro-step 2: 30.50 GB. Per-row peak 19.28 / 30.34 / 30.69 / 30.84 GB.
  implication: confirmed (see the fix and tests under Resolution). A ~64 KB LoRA grad occupies a 16-256 MB pooled buffer; the trainer holds micro-step 1's grads (6.0 GB of buffers) through micro-step 2, which then needs its own working set plus its own oversized grads (12.9 GB), so the pool grows ~11 GB once and stays there (the private pool never shrinks). The same placement also holds the later layers' grads through the rest of every backward (the 6.0 GB at micro-step 1 are live while earlier segments recompute), so the accum-1 peak carries part of it too.

## Resolution

root_cause: Two contributing causes (AND-gate). (1) code: LoRA grads lived on the Metal device far longer than the op that produced them - the trainer (`qlora-rs` `accumulate_and_maybe_step`) held each micro-step's grads in `pending_grads` until the accumulation window closed, and `checkpointed_grads` held every later layer's grads through the remaining segment recomputes. (2) environment: candle 0.10.2's Metal private pool serves an op output from the smallest free buffer of ANY size >= the request and never releases buffers. Mid-backward, mostly activation-sized buffers are free, so a 32-512 KB LoRA grad lands in a 16-512 MB buffer (0.6B: 394 grads, 25 MB logical, in 6.0-12.9 GB of buffers). At accum 1 they are dropped after each AdamW step; at accum > 1 micro-step 1's set stays pinned while micro-step 2 needs a full working set again, so the pool grows by roughly that set once (+11.0 GB 0.6B, +12.2 GB 1.7B, +26.6 GB 4B - it scales with the LoRA-grad count and the activation bucket size, not with parameters).
fix: (c56e13a33) `checkpointed_grads` folds each head chunk's and each segment's LoRA grads into a host GradStore (`fold_on_host`: copy to CPU, sum there in the same order), so a device grad is freed with the segment that produced it; the trainer therefore holds host grads across micro-steps. `accumulate_and_maybe_step` moves each grad to its var's device just before clip + AdamW (a no-op when the grads are already there, e.g. CUDA).
verification: tests (Metal, in training_loop/forward.rs), both RED first: `checkpointed_grads_leave_the_metal_pool_and_match_the_eager_backward` (tiny 3-layer model on Metal; every LoRA grad handed over is host-resident and equals 0.25 x the eager Metal backward within 1e-4; RED: "a LoRA grad is still held in a Metal buffer") and `host_held_micro_step_grads_step_exactly_like_their_device_sum` (accum-2 trainer on Metal fed host grads g1, g2 vs an accum-1 trainer stepped once on g2 + g1 summed on Metal: weights bitwise equal and changed; RED: "device mismatch in mul, lhs: Cpu, rhs: Metal"). Plugin lib 77 passed (75 + 2), 3 ignored; `cargo clippy -p vox-plugin-mens-candle-metal --all-targets --no-deps -- -D warnings` clean (without --no-deps clippy stops on 3 pre-existing vox-eval lints). `cargo test -p qlora-rs` is refused by cargo (patched dep, not a workspace member); the trainer change is covered by the second Metal test. Mutation 1: `let g = g.to_device(&Device::Cpu)?;` -> `let g = g.clone();` (grep -c 1 -> 0) failed the first test; restored from cp backup, diff -q identical, sha256 a9c74e22..., count 1. Mutation 2: `g.to_device(t.device())` -> `g.to_device(g.device())` (count 1 -> 0) failed the second test (device mismatch); restored, diff -q identical, sha256 da9bbe74..., count 1. Live measurements below.
oracle_type: derived (eager full-graph backward for the grads; one step on the device-summed grads for the accumulated update)
why_not_caught: no gate existed for this class. The S2 step-over-step leak check ran at accum 1 only (flat peaks there are real); the 1.7B accum-8 gap (50.5 vs 37.7 GB) was logged as unexplained in 320-S2 and the estimate has no accum input; unit tests run on CPU, where none of this is visible.
recurrence_guard: forward.rs `checkpointed_grads_leave_the_metal_pool_and_match_the_eager_backward` (fails if any LoRA grad leaves checkpointed_grads on the device) and `host_held_micro_step_grads_step_exactly_like_their_device_sum` (fails if the trainer cannot step on host-held grads, or steps differently); both Metal-only, skipped on hosts without a Metal device.
files_changed: [crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/forward.rs, patches/qlora-rs-1.0.5/src/training.rs]

## Live measurements (after the fix)

Binaries: `vox 0.6.0+build.7447 (c56e13a33)`, installed plugin sha256 `cdaf90fa…` (`diff -q` identical to `target/release`). All runs used seq_len 3072 on `target/dogfood` (1166 rows), and every one reached the first 3071-token row.

| size | preset / accum | run | peak MiB (run_summary) | before the fix | s/row |
|---|---|---|---|---|---|
| 0.6B | qwen3_dev_cpu / 1 | S4 full epoch, `mens/runs/qwen3-0.6b` (1108 opt steps, `trained`) | 15,319 | 23,675 (S2 smoke, accum 1) | 8.70 (10,146 s / 1166) |
| 1.7B | qwen3_6g / 8 | S4 full epoch, `mens/runs/qwen3-1.7b` (138 opt steps, `trained`) | 27,007 | 54,583 (S3 probe, accum 8) | 12.86 (14,998 s / 1166) |
| 4B | qwen3_12g / 8 | S3 re-probe, `mens/runs/320-probe-4B-postfix` (capped at row 40) | 33,037 | ≥ 84,541 (S3 probe, killed at row 2) | 38.29 |

- The row-2 jump is gone. At 4B the per-row peak is 33.77 GB for rows 1-4, creeps to 34.38 GB by row 13, and reaches 34.64 GB at the 3071-token row 33 (`320-S3-4B-REPROBE.md`). Before the fix it went 59.70 → 86.33 GB at row 2.
- The fix also lowered the accum-1 peak (0.6B: 23,675 → 15,319), as predicted: at accum 1 the later layers' grads were held through the rest of every backward.
- These peaks are the measured floors in `mens/config/gpu-specs.yaml` (D-08).
