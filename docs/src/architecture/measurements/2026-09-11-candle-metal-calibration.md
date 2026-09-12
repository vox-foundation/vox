---
title: "candle-metal calibration: one real point, no fit yet"
description: "Records the single real candle-metal peak-memory measurement obtained for Qwen3-0.6B and states plainly that a_candle_metal cannot be fit from it."
category: "Architecture SSOTs"
status: "current"
---

## What this is

One real, on-disk, live-training measurement of the `candle-metal` lane's peak
Metal allocation, captured via `vox-plugin-mens-candle-metal`'s `PeakSampler`
(added in an earlier task of the memory-SSOT plan this measurement belongs
to). The raw record lives at
[`2026-09-11-candle-metal-calibration.json`](./2026-09-11-candle-metal-calibration.json)
in the `CalibrationRecord` shape defined in
`crates/vox-populi/src/mens/tensor/calibration.rs`.

- **Host:** Apple M5 Max, `working_set_bytes = 115448725504` (this machine,
  queried live by Task 1's `query_accel_budget`).
- **Model:** `Qwen/Qwen3-0.6B` (layers=28, hidden=1024).
- **Measured shape:** batch=2 × seq=512 (`tokens_per_step = 1024`) — **not**
  the requested batch=1 × seq=512. See the bug writeup below for why.
- **Result:** `peak_metal_allocated_bytes = 53,439,004,672`, outcome `Fits`.

## Why `a_candle_metal` is not fit

`fit_a_lane` (`crates/vox-populi/src/mens/tensor/calibration.rs`) computes a
difference quotient between **two** `Fits` observations of the same model
shape at different `tokens_per_step` values. One point is one equation with
one unknown slope and zero unknowns pinned down — fitting it would invent a
degree of freedom, so the function correctly returns `None`. This is
exercised directly by the
`single_real_candle_metal_point_refuses_to_fit` test in `calibration.rs`,
which loads this exact JSON record and asserts `fit_a_lane` returns `None`
on it.

Three attempts across this task to obtain a second point (same model,
different `tokens_per_step`, or a cross-model anchor on Qwen3-8B) did not
complete within a reasonable foreground timeout, and a fourth attempt was
ruled not worth the cost relative to the plan's remaining tasks. **This is
not a blocker.** A second measurement — same host, same lane, a different
`seq_len` or a different model — is a follow-up to schedule on its own,
unhurried, without a strict foreground timeout.

## What downstream tasks should do meanwhile

Until a second point lands, `a_candle_metal` has no fitted value. Downstream
code selecting a memory-budget coefficient for the `candle-metal` lane should
treat it the same way the CUDA lane is already treated when its live budget
cannot be queried: an honest absence, not a fabricated number. The precedent
in this codebase is `crates/vox-populi/src/mens/tensor/accel_budget.rs`'s
`query_accel_budget()` on non-macOS, which returns `None` rather than
guessing at a CUDA budget (see that function's doc comment for the full
crate-layering rationale). The `candle-metal` lane's `a` coefficient should
follow the same pattern: `Seeded`/`Uncalibrated`, never a value silently
presented as measured.

## Separate finding: `--batch-size 1` is silently floored to 2 for Qwen3-0.6B

Discovered while trying to obtain this measurement, not part of the
measurement itself, and **not fixed here** — flagged for Task 5's rank/clamp
work.

`crates/vox-populi/src/mens/tensor/preset_schema.rs`:

- `resolve_effective_profile` applies the CLI's explicit `--batch-size`
  override at line 477: `p.batch_size = b;`.
- It then calls `apply_qwen_size_ladder_policy(p, class, device.vram_mb, model_hint)`
  at line 493 — **after** the override has already been applied.
- For `QwenSizeClass::S0p6` (Qwen3-0.6B), that function unconditionally does,
  at line 120: `p.batch_size = p.batch_size.max(2);` — with no check for
  whether the value came from an explicit CLI override or an unset default.

Net effect: `vox mens train --model Qwen/Qwen3-0.6B ... --batch-size 1`
silently trains at batch 2. The startup banner logs the *effective* value
(`Batch/Accum: 2/4`) with no warning that it differs from what was
requested — which is exactly how this measurement ended up at batch=2×seq=512
instead of the requested batch=1×seq=512. Reproduced twice, identically, in
this task. Other `QwenSizeClass` branches in the same function have similar
unconditional floors/caps (e.g. `S8` at line 125 `.max(1)`), so Qwen3-0.6B is
just the rung this task's grid happened to hit — worth checking the others
when this is fixed.
