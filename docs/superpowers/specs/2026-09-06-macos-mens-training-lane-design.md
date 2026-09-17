---
title: "A portable macOS training lane for MENS"
description: "Auto-detect a Mac's live-available unified memory and select a fitting Qwen3 model plus qwen3_* preset as the CLI default, without touching the validated 4080 Super CUDA lane."
category: "Architecture SSOTs"
status: "proposed"
---

# A portable macOS training lane for MENS

## Problem

MENS training on Apple Silicon was validated end-to-end on this one 128 GB
Mac this session (see
[`mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md`](../../src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md)),
but the path to get there required hand-picking every flag. It does not yet
generalize to "any Mac" or default to the right **selection** automatically.
Three concrete gaps, found by tracing the real `vox mens train` code path
end to end:

1. **The CLI's actual hardware probe reports zero memory on every Mac.**
   `run_train.rs` calls `vox_populi::mens::probe_gpu()` → `hardware::probe()`
   → the registry's `MacosMetalProbe`, whose `probe_metal()`
   (`crates/vox-populi/src/mens/hardware/macos_metal.rs`) hardcodes
   `vram_mb: 0`. This is a stub that was never finished. Every
   hyperparameter decision downstream of it (`DeviceProfile::from_gpu_info`,
   `resolve_effective_profile`) treats every Mac as having no usable memory
   at all.
2. **A second, correct hardware probe already exists but isn't connected to
   the CLI path.** `vram_autodetect::query_apple_unified_memory()`
   (added 2026-09-05) reads `sysctl -n hw.memsize` and subtracts a GUI/OS
   reserve. It feeds `spoke_base_resolver::resolve_base_model` when a
   spoke's `base.model` is a capability tag. A bare `vox mens train`
   invocation never reaches this code: it goes through `probe_gpu()`
   instead. (`auto_preset_for` is unit-tested only; production
   `resolve_effective_profile` never calls it.)
3. **The CLI default ignores hardware entirely.**
   `preset_schema::DEFAULT_PRESET = "4080"` — a CUDA-16GB-tuned
   hyperparameter preset — is used whenever `--preset` is omitted, on the
   `resolve_effective_profile` path. An `"auto"` preset value already exists
   inside `resolve_effective_profile`, but it is (a) not the default, and
   (b) keyed against the CUDA-family `presets:` table in `gpu-specs.yaml`
   (`prosumer_16g` / `a100` / `h100`) via vendor-blind
   `best_for_vram`. Its failure fallback is the hardcoded `"4080_safe"`.
   Once the probe stub is fixed and a Mac reports real VRAM, `"auto"` will
   **match** those CUDA yaml rows by size (16 GB → `prosumer_16g`, 128 GB
   → `a100`/`h100`) and never reach the Metal `qwen3_*` ladder.

Separately, omitting `--model` on CandleQlora fills
`DEFAULT_MODEL_ID` (Qwen3-8B) at `run_train.rs` **before** any
hardware-aware resolve can run.

Net effect: today, on any Mac, `vox mens train` with no flags silently
selects CUDA-16GB-tuned hyperparameters against a memory budget of zero,
and the already-correct Apple-memory-aware pieces
(`query_apple_unified_memory`, `auto_preset_for`) never drive the CLI
default.

There is also a real prior incident this generalizes from: `31119b80c` →
`3d28548b5` defaulted training to Qwen2.5-Coder-7B, which did not fit the
4080 Super's 16 GB, and had to be recalibrated down to 3B after the fact.
The goal of this design is to make that class of mistake structurally
impossible — the catalogue should refuse to pick something that doesn't
fit, not get recalibrated after it fails.

## Goals

- `vox mens train` with **no `--model`/`--preset` flags** on Apple Silicon
  **selects** the largest `agentic_default` Qwen3 rung that fits
  live-available unified memory, plus a `qwen3_*` hyperparameter preset
  from `auto_preset_for(Metal, …)`. The 4080 Super CUDA lane is unchanged.
- Selection is the deliverable. Default `--device best` continues to use
  the existing `mens-candle-cuda` Metal device-select path. Explicit
  `--device metal` still hard-bails (no Metal Candle training backend).
  This design does not implement Metal-plugin training.
- "Fits" is judged against **live available memory**, not nameplate/total
  physical memory — a Mac with other applications already using RAM must
  not be sized as if all of it were free.
- Catalogue coverage is proven by injecting **live-available MB** into
  `pick_base("agentic_default", …)`, not by claiming every physical SKU
  maps to a rung. `agentic_default`'s minimum floor is 11000 MB: an 8 GB
  Mac fails closed. A ~48 GB live-available budget resolves 14B-**QLoRA**
  (20000), not 14B-LoRA (44000).
- Explicit flags (`--model`, `--preset`, `--device`) still override
  auto-detection exactly as today — this is a smarter default, not a
  removal of manual control. Precedence: `--model` >
  `VOX_MENS_DEFAULT_MODEL` > Metal auto-resolve > `DEFAULT_MODEL_ID`.
- Zero behavior change on the CUDA lane: `DEFAULT_PRESET` stays `"4080"`;
  omitted-preset `"auto"` is **Metal-only**. CPU/unknown keep `"4080"` so
  Linux CI without a GPU does not silently switch to `"auto"` →
  `4080_safe`.
- New `VOX_*` knobs are registered in `contracts/config/env-vars.v1.yaml`
  and `contracts/config/registry.v1.yaml` (including a backfill of the
  already-coded `VOX_MENS_UNIFIED_MEM_RESERVE_GIB`).

## Non-goals

- Intel Mac / discrete-GPU-on-Mac training. `query_apple_unified_memory`
  is `#[cfg(target_os = "macos")]` only — `sysctl hw.memsize` succeeds on
  Intel Macs too and will keep returning a budget. Special-casing Intel
  to `None`/CPU is out of scope.
- Changing `--device metal` training itself. The CLI hard-bails today;
  the `mens-candle-metal` plugin's train/eval steps are still
  unimplemented. That is a separate, already-tracked project.
- A GUI/interactive picker. The approved CLI shape is "bare command
  auto-detects selection," not a wizard.
- Re-opening the hub-model debate (Qwen3 vs. other families). Out of
  scope; this design works within the existing Qwen3 ladder. No new
  `floor_mb` rows (including no 0.6B rung on `agentic_default`).
- Expanding `gpu.rs`'s CUDA-only `auto_preset` inject to Metal. After
  the Metal `"auto"` fix, `--device best` on a Mac goes through
  `resolve_effective_profile(None, …)` and is enough.

## Approach

### 1. Fix the stub, don't build a parallel system

`MacosMetalProbe::probe_metal()` will call
`vram_autodetect::query_apple_available_memory()` (see §2) instead of
hardcoding `vram_mb: 0`. It must call that **leaf** function, never
`get_system_vram_info()` / `get_system_vram_gb()`: Priority 4 of
`get_system_vram_info` calls `hardware::probe()`, which re-enters
`probe_metal()` (latent stack overflow). No new probe is invented; the
existing one is finally wired in. On query `None`, keep returning
`Some(HardwareSummary)` (today's macOS contract) with `vram_mb: 0` and
`probe_failures` set.

### 2. Live available memory, not nameplate total

Today, `query_apple_unified_memory()` computes `physical_total - 12 GiB`
flat reserve. This over- or under-reserves depending on what else is
running. Add `query_apple_available_memory()` alongside it:

- Shell out to `vm_stat` (same idiom as the existing `sysctl`/`nvidia-smi`
  shell-outs — no new FFI/unsafe surface). English prefixes only.
- Compute reclaimable-available = `(free + inactive + speculative) *
  page_size`. Purgeable pages are omitted (documented; not part of this
  definition). `page_free_count` alone is misleading (observed ~830 MB
  "free" on this 128 GB machine with tens of GB of real headroom).
- Apply a smaller proportional safety margin (default 15%) to *that*
  number, floored at a minimum absolute reserve (2 GiB).
- On any parse/shell failure, fall back to the existing static
  `physical_total - 12 GiB` heuristic — never hard-fail hardware detection
  because a live read didn't work.
- `VOX_MENS_UNIFIED_MEM_RESERVE_GIB` keeps working as the override for the
  static fallback; add `VOX_MENS_LIVE_MEM_MARGIN_PCT` for the new
  proportional margin, and `VOX_MENS_DISABLE_LIVE_MEM=1` to force the old
  static behavior (CI/tests). All three are registered in the env-var
  contracts.

### 3. Catalogue: inject live-available MB, fail closed at 8 GB

No new `floor_mb` rows. Tests inject live-available MB into
`pick_base("agentic_default", …)` (not physical-GB × 0.85):

| live-available MB | expected |
|---|---|
| 6144 | Err (below 11000 floor; 8 GB Mac) |
| 13926 | Qwen3-8B QLoRA |
| 20890 / 27853 / 31334 / 41779 | Qwen3-14B QLoRA |
| 55706 | Qwen3-14B LoRA (64 GB; conservative — 32B-QLoRA's 60000 floor is not met) |
| 83558 | Qwen3-32B QLoRA |
| 111411+ | Qwen3-32B LoRA |

The 0.6B / 2000 MB rung lives only on `qwen3_code`, not `agentic_default`.
Adding it is the out-of-scope hub-model debate.

### 4. Metal-only `"auto"` default, and `"auto"` on Metal skips yaml `presets:`

- `preset_schema::DEFAULT_PRESET` (`"4080"`) itself is **not** changed.
- `resolve_effective_profile` picks the fallback-when-omitted
  **conditionally**: Metal → `"auto"`; CUDA / CPU / unknown →
  `DEFAULT_PRESET` (`"4080"`).
- `DeviceProfile` gains `pub vendor: String`, threaded from
  `GpuInfo::vendor` (`format!("{:?}", GpuVendor).to_lowercase()` →
  `"apple"` / `"nvidia"` / `"cpu"`). `AcceleratorKind::from_vendor`
  maps those strings.
- Inside `if name == "auto"`: if the device is Metal, **do not** call
  `TrainingPreset::best_for_vram` on the CUDA yaml table. Call
  `auto_preset_for(Metal, Some(vram_gb))` then `base_for_name`. If that
  returns `None`, use `qwen3_dev_cpu` only in the documented 6–16 GiB
  smoke band (matching `auto_preset_for`'s existing arms); otherwise
  fail closed. CUDA `"auto"` (explicit `--preset auto` or
  `VOX_TRAIN_PROFILE=auto`) keeps today's yaml walk.
- When `--model` is omitted on Metal and `VOX_MENS_DEFAULT_MODEL` is
  unset, `spoke_base_resolver::resolve_metal_default_base` runs
  **before** the CandleQlora `DEFAULT_MODEL_ID` fill, fail-closed via
  `pick_base("agentic_default", …)`. The helper lives in `vox-populi`;
  `run_train.rs` stays thin. One `probe_gpu()` per invocation.

### 5. CUDA lane: untouched, and pinned by tests

`probe_gpu()`'s nvidia-smi-backed path, `auto_preset`'s CUDA arm,
`gpu.rs`'s CUDA-only inject, and every existing preset boundary test
are not modified. New regression tests assert
`resolve_effective_profile(None, <cuda device>, …)` still matches
explicit `"4080"` byte-for-byte, and that a CPU/unknown device with
`vram_mb == 0` still gets `"4080"` (not `"auto"`).

## Data flow (after this change)

```text
vox mens train  (no --model/--preset; --device best)
  -> probe_gpu() -> hardware::probe() -> MacosMetalProbe::probe_metal()
       -> vram_autodetect::query_apple_available_memory()  [was: hardcoded 0]
       -> MUST NOT call get_system_vram_info() (Priority 4 re-enters probe)
  -> DeviceProfile { vram_mb: <live>, vendor: "apple" }
  -> resolve_effective_profile(preset=None, device{vendor:"apple"}, ...)
       -> kind = AcceleratorKind::from_vendor("apple") = Metal
       -> default-when-omitted = "auto" (DEFAULT_PRESET stays "4080")
       -> Metal "auto": skip gpu-specs.yaml presets: table
       -> auto_preset_for(AcceleratorKind::Metal, vram_gb)
       -> base_for_name("qwen3_16g"|"qwen3_24g"|"qwen3_48g"|"qwen3_96g"|...)
  -> model omitted + Metal + VOX_MENS_DEFAULT_MODEL unset
       -> resolve_metal_default_base(root, vram_mb)  [before CandleQlora fill]
       -> pick_base("agentic_default", vram_mb) fail-closed
       -> e.g. Qwen3-14B QLoRA at ~42 GB live-available (not 14B LoRA)
  -> selection is done. Training proceeds on the existing --device best path.
```

## Testing

- Unit tests for `parse_vm_stat` (16k and 4096 page sizes, missing
  fields) and `apply_live_margin` (40 / 5 / 1 GiB). No live syscalls in
  the parser tests.
- `#[serial]` env tests for `VOX_MENS_LIVE_MEM_MARGIN_PCT` (default +
  clamp) and `VOX_MENS_DISABLE_LIVE_MEM` (equals static heuristic).
- `vram_mb_from_apple_info(Some(116.0)) == 118784` and `None → 0`.
  Optional live macOS test: if the query returns Some, `probe_metal`
  matches the conversion; skip if None (sandboxed CI).
- Catalogue table: inject the live-MB rows in §3; assert exact `hf_id`
  substring + methods. 6144 must be `Err` — do not weaken that
  assertion.
- CUDA: `resolve_effective_profile(None, nvidia/16384)` matches
  explicit `"4080"` on rank/alpha/seq_len/batch_size/grad_accum/lr.
  CPU/unknown/`vram_mb == 0` still uses `"4080"`.
- Metal: 16384 MB matches `qwen3_16g` (not yaml `prosumer_16g`);
  116 GiB matches `qwen3_96g` (not yaml `a100`).

## Open questions resolved during brainstorming and code audit

- Live vs. nameplate memory: **live**, via `vm_stat`'s reclaimable-page
  definition, with a static fallback (§2).
- CLI shape: bare command auto-detects **selection**; explicit flags
  still override.
- Scope: Apple Silicon selection only, Qwen3 ladder only, no Metal
  training execution, no GUI picker, no new catalogue rungs.
- `"auto"` on Metal must skip the CUDA yaml table (code-audit finding;
  the first draft's "no yaml match on Apple vendor" claim was false
  once VRAM is non-zero).
- Omitted-preset `"auto"` is Metal-only (CPU/unknown → `"auto"` would
  change Linux CI).
