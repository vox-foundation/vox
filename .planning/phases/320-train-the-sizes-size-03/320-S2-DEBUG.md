---
status: awaiting_human_verify
trigger: "S2 0.6B Metal QLoRA smoke: ~374 GB peak_metal_allocated_bytes on step 0, silent exit 1 (seq_len 3072, batch 2, grad_accum 4, gradient_checkpointing=false, est. 1618 MB)"
created: 2026-10-27
updated: 2026-10-27
---

## Current Focus

hypothesis: (confirmed) one Metal row's eager autograd graph (28 layers of activations and [16,s,s] attention chains, plus a whole-row [s,151936] head) was resident at backward and pow2-pooled; checkpointing was never used on Metal and was off for 0.6B by policy.
test: 0.6B proof run at --seq-len 3072, --max-runtime-secs 240, default policy (no env var)
expecting: >= 3 optimizer steps, flat peak, well under live free memory
next_action: orchestrator re-runs S2 on the rebuilt binaries; S3/S4 sizing must account for the per-layer attention term (see Resolution: unresolved)

reasoning_checkpoint:
  hypothesis: "Peak Metal memory per row is eager graph retention: every layer's activations and attention chain, plus the whole-row head, stay referenced until backward, because the Metal loop never used the model's checkpointing and the policy kept it off below 2.9B."
  confirming_evidence:
    - "peaks flat across steps at 128/256/512 tokens (no leak), growing ~63 MB/token plus s^2"
    - "per-layer checkpointing alone: 41.33 -> 17.96 GB at s=512, same loss"
    - "chunking the head: 36.05 -> 19.96 GB at s=1024; proof at 3072 cap flat at 45.3-46.0 GB vs 374 GB"
  falsification_test: "if checkpointing did not lower the peak, or the peak still grew step over step, retention would not be the cause"
  fix_rationale: "checkpointing and head chunking bound what is on the tape at once to one layer and one 256-row head chunk, which is the retained set itself"
  blind_spots: "the per-layer attention backward still costs ~295 B per head per token pair (about 3x an analytic count); not explained, only calibrated"
  candidate_causes:
    - "code: Metal loop ignores gradient_checkpointing; whole-row head"
    - "config: 0.6B ladder rung raising batch/accum; checkpointing policy keyed only on >= 2.9B"
    - "environment: candle Metal pool keeps pow2 buffers and never sweeps private buffers"
  and_gate: "yes - the blowup needs eager retention (code) AND policy leaving checkpointing off (config); the pool's pow2 rounding (environment) inflates it ~1.5x but does not cause it"

## Symptoms

expected: 0.6B smoke at seq_len 3072 trains several optimizer steps under the ~59 GB free Metal budget
actual: step 0 logged (loss 3.1694), peak_metal_allocated_bytes 374417047552, process died ~46 s later with exit 1
errors: none printed; launcher maps signal death to exit 1
reproduction: target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --model Qwen3-0.6B --max-runtime-secs 480
started: first 320 S2 run (2026-10-27)

## Eliminated

- hypothesis: batch_size 2 doubles per-step activations on Metal
  evidence: the Metal loop never reads config.batch_size for compute; forward_masked_ce builds one [1, len] row per micro-step (training_loop/forward.rs, mod.rs). batch_size only feeds telemetry/OOM records. S2 step 0 had theoretical_tokens 2333 = one row's ids_len-1.
  timestamp: 2026-10-27
- hypothesis: step-over-step leak (buffers/graph retained across micro-steps, grads accumulated)
  evidence: peak_metal_allocated_bytes flat across 60 micro-steps at s=128 (16.72 -> 16.79 GB) and s=256 (24.78 -> 24.79 GB), and across 7 at s=512 (41.33 -> 41.37 GB). qlora-rs accumulate_and_maybe_step keeps only trainable-var grads in pending_grads.
  timestamp: 2026-10-27

## Evidence

- timestamp: 2026-10-27
  checked: preset resolution (vox-populi preset_schema.rs resolve_effective_profile L450-617, apply_qwen_size_ladder_policy L121-142; vox-ml-cli gpu.rs L103-145)
  found: size pick adopts qwen3_dev_cpu (batch 1, accum 1) and sets VOX_BASE_MODEL; resolve_effective_profile then detects S0p6 from VOX_BASE_MODEL and apply_qwen_size_ladder_policy raises batch to max(2) and grad_accum to max(4). Only an explicit --batch-size/--grad-accum flag escapes it.
  implication: Q1 answered: the generic Qwen size ladder overrides the size-specific preset. Not a memory cause on Metal (batch unused).
- timestamp: 2026-10-27
  checked: Metal trainer (candle_qlora_train/mod.rs L342-350, training_loop/mod.rs, forward.rs) vs CUDA (training_loop/forward.rs, logic.rs)
  found: Metal mod.rs warns "gradient_checkpointing was requested but the Metal backend does not implement it"; Metal model.rs already has forward_checkpointed/recompute_segment and a passing eager-vs-checkpointed grad test, but the loop always calls model.forward + trainer.backward_step. gpu.rs enables checkpointing only for explicit env or params_b >= 2.9.
  implication: Q2 answered: the model-level pieces exist on Metal, the loop never uses them; off for 0.6B by policy.
- timestamp: 2026-10-27
  checked: candle-core 0.10.2 backprop.rs and metal_backend/device.rs
  found: (a) every op output stays referenced by the loss graph until backward returns; (b) matmul/binary backward computes and stores grads for untracked rhs (frozen weights, causal-mask broadcast, RoPE cos/sin, LoRA scaling) and never removes them; (c) Metal buffers are rounded up to the next power of two (buf_size) and op outputs come from private_buffers, which drop_unused_buffers never sweeps, so currentAllocatedSize is the pool high-water mark.
  implication: one row's eager graph is fully resident at backward, inflated by pow2 rounding (x1.54 at s=2333 for [16,s,s] F32).
- timestamp: 2026-10-27
  checked: cheap capped runs, 0.6B, eager (current binaries 900ceb5bf), rows at the seq cap
  found: peak 16.72 GB (s=128), 24.78 GB (s=256), 41.33 GB (s=512, stopped with SIGINT at >40 GB); fit F + a*s + c*s^2 gives F~8.8 GB, a~61 MB/token, c~4.4 KB/token^2 (c poorly determined at small s). S2: 374.4 GB at s=2333.
  implication: memory is per-row activation size, roughly linear at small s with a growing s^2 attention term; 61 MB/token x 2333 alone is ~143 GB before pow2 rounding and attention.
- timestamp: 2026-10-27
  checked: estimate_training_vram_mb_qlora (vox-populi device.rs L75-116)
  found: (v*d*4 + 12*d^2*4*L + b*s*d*L*16) * 0.35 = 1618 MiB for 0.6B, b=2, s=3072. No attention-score term (O(h*s^2*L)), no logits/log-softmax term (O(s*V)), activations at 16 B per s*d*L (measured ~61 MB/token ~ 2 KB per s*d*L), and the 0.35 QLoRA discount is applied to activations too. It only prints a warning; it never refuses.
  implication: Q4 answered.
- timestamp: 2026-10-27
  checked: checkpointed Metal loop (Fix A, plugin built from the working tree), VOX_MENS_GRADIENT_CHECKPOINTING=1, 0.6B, capped runs
  found: s=512 peak 16.47 -> 17.61 -> 17.96 GB (eager 41.33 GB), loss identical to eager (first 3.6088), 7 optimizer steps in 66 s. s=1024 peak 26.76 -> 32.73 -> 36.05 GB then flat to 36.05 GB over 15 micro-steps, 2+ optimizer steps. Both are ~35 MB/token with ~zero intercept, i.e. ~36 pow2 [s,V]-sized buffers (512 MB buckets at s=512, 1 GB at s=1024).
  implication: checkpointing removes the per-layer term but leaves the head: one [s,V] logits + log-softmax + its backward per row, pooled per row-length bucket. Extrapolated to s=3072 that is ~75 GB (2 GB buckets) - still over budget. The head must be chunked.
- timestamp: 2026-10-27
  checked: vox-populi memory_model.rs CalKey::new (L66-90)
  found: CalKey(candle-metal, gradient_checkpointing=true) is rejected with "candle-metal does not implement gradient checkpointing"; with checkpointing now on for every Metal run, every Metal run prints "VRAM check unavailable (...does not implement gradient checkpointing...)". The outcome is NoMeasurement either way (no candle-metal calibration rows exist), so only the message is stale.
  implication: plan_for/sweep give Metal no fit check; the heuristic estimate in gpu.rs is the only one, so Fix C makes it refuse on Metal past live free memory.
- timestamp: 2026-10-27
  checked: chunked head (256-row chunks, head re-run per chunk in the backward; plugin e1eb8287), s=1024, VOX_MENS_GRADIENT_CHECKPOINTING=1, mens/runs/320-debug-chunk-s1024
  found: peak 14.75 -> 18.63 -> 19.44 -> 19.94 -> 19.96 GB, flat from micro-step 5 to 14; 4 optimizer steps in 96 s (capped). Losses match the whole-head run to 1e-6 until the first optimizer update (3.29429 / 3.40100 / 1.95360 / 1.96177) and stay close after (3.1447 vs 3.1533 at micro 6).
  implication: confirmed - the head was ~16 GB of the 36 GB at s=1024; chunking it leaves ~20 GB. The hypothesis holds; next is the s=3072 proof.
- timestamp: 2026-10-27
  checked: proof run on rebuilt binaries (vox 55a338c53; vox-ml-cli built from the same working tree; plugin e1eb8287 installed and byte-checked), no env var, `vox mens train --backend qlora --tokenizer hf --device metal --require-gpu --model Qwen/Qwen3-0.6B@c1899de2... --data-dir target/dogfood --seq-len 3072 --epochs 1 --max-runtime-secs 240 --fast-corpus --no-auto-heal --output-dir mens/runs/320-debug-proof-s3072 --force-restart`
  found: log shows "checkpointing ENABLED ... explicit=false", batch_size=1, grad_accum=1, "est. ~29325 MB" (pre-recalibration constants), 67,211 MB free (Metal working set). 13 optimizer steps in 243.6 s, run cap reached, exit 0. Rows 2333/2277/2280/2330/2381/2295/2499/2298/2455/2283/2303/2280/2308 tokens. Losses 3.1694 (identical to S2's step 0) / 1.2010 / 1.8798 / 1.6647 / 1.1623 / 1.9805 / 0.2607 / 1.8896 / 2.6303 / 1.2809 / 2.0531 / 1.3002 / 3.6657. Peak per telemetry record (GB): 45.29, 45.55, 45.55, 45.83 x6, 45.99 x5; run_summary peak 45,993,525,248 B. ~18 s per 2300-token step.
  implication: fixed: flat, bounded, 68% of live free memory, versus 374 GB and a SIGKILL. The 0.35x estimate was 37% low here; the per-layer attention backward (~29 GB of the 46) is the dominant term left.
- timestamp: 2026-10-27
  checked: two-point fit of the checkpointed peaks (19.96 GB at 1024, 45.99 GB at a 2499-token max row) with 1 KB per token per hidden unit for one layer's activations
  found: ~295 B per head per token pair for one layer during its backward (95 B kept on the tape in eager mode, so ~200 B extra while the attention backward runs), fixed part ~14 GB incl. the 256-row head chunks. candle's composed softmax (max, sub, exp, sum, div) and broadcast mask-add backward each allocate several [16,s,s] F32 tensors; GradStore::or_insert adds zeros+add per node. That does not fully account for ~74 [16,s,s]-equivalents.
  implication: at every row = 3072 tokens the recalibrated estimate is 59,150 MiB (~62 GB) for 0.6B, ~69 GB for 1.7B (16 heads, d 2048), ~120 GB for 4B (32 heads, d 2560, 36 layers). Recalibrated the estimate to these terms so Metal refuses instead of being killed.

## Resolution

root_cause: Two contributing causes (AND-gate). (1) code: the Metal QLoRA loop always ran the eager forward and a single backward, ignoring `gradient_checkpointing` (the model's `forward_checkpointed`/`recompute_segment` existed but were unused; the plugin warned it was unimplemented), and computed the head + masked CE over the whole row, so one row's 28 layers of activations and [16,s,s] attention chains plus ~12 [s,151936] F32 head buffers stayed referenced until backward; candle's Metal pool keeps each pow2 buffer (x1.5 at s=2333). (2) config: gpu.rs turned checkpointing on only for >= 2.9B models. Contributing, not causal: the 0.6B ladder rung raised the size pick's qwen3_dev_cpu (1/1) to batch 2 / accum 4 (batch is unused for Metal compute); the QLoRA estimate (0.35 x a FP formula with no attention or head term, 16 B per token per layer unit) said 1618 MB and only warned; the launcher mapped the child's signal death to exit 1. No step-over-step leak.
fix: (a6e959e1a) a named preset keeps its batch/accum on the 0.6B rung. (7febb6bde) Metal forward_masked_ce honors gradient_checkpointing: one layer per segment, the head and masked CE in 256-row chunks, each chunk backpropagated and freed in checkpointed_grads, then the boundary cotangent threaded through the last segment and each earlier one recomputed; validation reuses forward_masked_ce. (55a338c53) vox reports "vox-ml-cli was killed by signal N" and exits 128+N. (Fix C commit) checkpointing is always on for Metal (and Best on macOS); estimate_training_vram_mb_qlora models resident weights, per-layer activations, the attention chain kept and in backward, the head (whole row or one chunk), checkpointing-aware, calibrated on these runs; on Metal, gpu.rs refuses when the estimate exceeds live free memory unless VOX_MENS_FORCE_TRAIN=1.
verification: tests - vox-plugin-mens-candle-metal --lib 68 passed (checkpointing_splits_per_layer_and_keeps_the_eager_loss, head_chunks_are_bounded_and_skip_unsupervised_rows, checkpointed_grads_match_the_eager_backward over a 9-position row with a skipped chunk and a partial chunk, sample weight 2, scale 0.25); vox-populi preset_schema 34 + a_named_preset_keeps_its_batch_and_accum_on_the_0_6b_rung, vram_estimate_tests::qlora_estimate_tracks_measured_metal_peaks; vox-cli-core ml_cli_handshake 5 incl. a_signal_death_is_reported_not_mapped_to_exit_1; vox-ml-cli checkpointing_policy_tests::metal_always_checkpoints. Each written RED first (compile errors for new API; assertion failure for the policy and preset tests). Mutation: forward.rs guard `if config.gradient_checkpointing {` -> `if false {` (grep count 1 -> 0) failed 2 of 3 forward tests; restored from cp backup, diff -q identical, shasum 702f6610..., count 1. Proof run above: 13 optimizer steps, flat 45.3-46.0 GB, exit 0. Signal path verified by unit test only (no real SIGKILL reproduced).
oracle_type: derived (eager full-graph backward is the reference for checkpointed grads and loss)
files_changed: [crates/vox-populi/src/mens/tensor/preset_schema.rs, crates/vox-plugin-mens-candle-metal/src/model.rs, crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs, crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/forward.rs, crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/types.rs, crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/mod.rs, crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/training_loop/validation.rs, crates/vox-cli-core/src/ml_cli_handshake.rs, crates/vox-cli/src/main.rs, crates/vox-ml-cli/src/commands/schola/train/gpu.rs, crates/vox-populi/src/mens/tensor/device.rs]
unresolved:
  - The per-layer attention backward (~295 B per head per token pair, ~29 GB of the 46 GB proof peak) is now the dominant term and is only calibrated, not explained. With every row at 3072 tokens the estimate is ~62 GB for 0.6B, ~69 GB for 1.7B, ~120 GB for 4B, so S3/S4 at --seq-len 3072 will be refused on a ~59-67 GB free host (and 0.6B itself is refused below ~59.2 GB free). A fused softmax with backward or query-chunked/flash attention in the Metal model is the next fix; until then lower --seq-len or set VOX_MENS_FORCE_TRAIN=1 knowingly.
  - The estimate assumes every row reaches --seq-len; the dogfood rows are ~2300-2500 tokens, so it can refuse runs that would fit. Calibrated on 0.6B only; the d, L and head scaling for larger sizes is extrapolated.
  - vox-populi memory_model.rs CalKey still rejects (candle-metal, gradient_checkpointing=true) with "candle-metal does not implement gradient checkpointing"; every Metal run now prints that stale reason. The outcome is NoMeasurement either way.
  - The 14B qwen3_24g preset's accum raise and other rungs are untouched; only the 0.6B rung was the S2 path.
  - The CUDA plugin's validation still runs an eager forward with whole-row logits.
  - The BF16 dequant cache keeps ~2 B/param resident (~64+ GB for 32B).
  - Throughput: ~18 s per 2300-token row on 0.6B with checkpointing (ETA ~6 h for the 1108-row epoch).
  - The real SIGKILL path of the launcher was not reproduced end to end.
