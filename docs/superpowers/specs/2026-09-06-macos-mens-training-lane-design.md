---
title: "A portable macOS training lane for MENS"
description: "Auto-detect any Mac's real available unified memory, size training to it from a real hardware catalogue, and make that the CLI default — without touching the validated 4080 Super CUDA lane."
category: "Architecture SSOTs"
status: "proposed"
---

# A portable macOS training lane for MENS

## Problem

MENS training on Apple Silicon was validated end-to-end on this one 128 GB
Mac this session (see
[`mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md`](../../src/architecture/mac-hub-qwen3-quantization-pipeline-decision-2026-09-05.md)),
but the path to get there required hand-picking every flag. It does not yet
generalize to "any Mac" or default to the right behavior automatically. Three
concrete gaps, found by tracing the real `vox mens train` code path end to
end:

1. **The CLI's actual hardware probe reports zero memory on every Mac.**
   `run_train.rs` (the real entry point) calls
   `vox_populi::mens::probe_gpu()` → `hardware::probe()` → the registry's
   `MacosMetalProbe`, whose `probe_metal()`
   (`crates/vox-populi/src/mens/hardware/macos_metal.rs:22-34`) hardcodes
   `vram_mb: 0`. This is a stub that was never finished. Every hyperparameter
   decision downstream of it (`DeviceProfile::from_gpu_info`,
   `resolve_effective_profile`) treats every Mac as having no usable memory
   at all.
2. **A second, correct hardware probe already exists but isn't connected to
   the CLI path.** `vram_autodetect::query_apple_unified_memory()`
   (added 2026-09-05, `crates/vox-populi/src/mens/tensor/vram_autodetect.rs:83-99`)
   correctly reads `sysctl -n hw.memsize` and subtracts a GUI/OS reserve. It
   feeds `spoke_base_resolver::resolve_base_model` and
   `auto_preset_for(AcceleratorKind::Metal, ...)` — used only when a spoke's
   `base.model` is a capability tag (e.g. `strong_code_default`). A bare
   `vox mens train --model <hf-id>` invocation never reaches this code at
   all.
3. **The CLI default ignores hardware entirely.**
   `preset_schema::DEFAULT_PRESET = "4080"` — a CUDA-16GB-tuned
   hyperparameter preset — is used whenever `--preset` is omitted, on any
   platform. An `"auto"` preset value already exists inside
   `resolve_effective_profile` (`preset_schema.rs:355-378`), but it is (a)
   not the default, and (b) keyed against a *different* GPU-family preset
   table (`presets:` in `gpu-specs.yaml`, e.g. `4080`/`a100`/`h100`) than the
   Metal-aware `qwen3_*` ladder `auto_preset_for` already knows how to walk.
   Its failure fallback is the hardcoded `"4080_safe"`.

Net effect: today, on any Mac, `vox mens train` with no flags silently
trains with CUDA-16GB-tuned hyperparameters against a memory budget of zero,
and the two already-correct pieces of Apple-memory-aware code
(`query_apple_unified_memory`, `auto_preset_for`) never run unless a caller
goes through the spoke/capability-tag path.

There is also a real prior incident this generalizes from: `31119b80c` →
`3d28548b5` defaulted training to Qwen2.5-Coder-7B, which did not fit the
4080 Super's 16 GB, and had to be recalibrated down to 3B after the fact.
The goal of this design is to make that class of mistake structurally
impossible — the catalogue should refuse to pick something that doesn't
fit, not get recalibrated after it fails.

## Goals

- `vox mens train` with **no `--model`/`--preset`/`--device` flags** detects
  the machine, picks the largest model that safely fits, and trains — on
  any Mac (Apple Silicon; Intel Macs fall through to the existing CPU path,
  see Non-goals) and unchanged on the existing 4080 Super CUDA lane.
- "Fits" is judged against **live available memory**, not nameplate/total
  physical memory — a Mac with other applications already using RAM must
  not be sized as if all of it were free.
- A real, current (Sept 2026) catalogue of common Apple Silicon memory
  tiers, each mapped to the largest Qwen3 rung that leaves headroom for
  collisions/peaks, replacing today's four-tier ladder (16/24/48/96 GB
  only).
- Explicit flags (`--model`, `--preset`, `--device`) still override
  auto-detection exactly as today — this is a smarter default, not a
  removal of manual control.
- Zero behavior change on the CUDA lane: the 4080 Super's existing presets,
  boundaries, and defaults are covered by regression tests that must keep
  passing unmodified.

## Non-goals

- Intel Mac / non-unified-memory Mac support. `query_apple_unified_memory`
  is already gated to real unified-memory reads (`hw.memsize`); Intel Macs
  will get `None` from it and fall through to the CPU tier, same as today.
  Nobody has asked for AMD/Intel-GPU-on-Mac training and it is out of scope.
- Changing `--device metal` training itself. The Metal plugin's
  `run_train_step`/`run_eval_step` still `unimplemented!()` — that is a
  separate, already-tracked project. This design only fixes *which*
  model/preset gets selected before training starts; it does not implement
  Metal-backend training.
- A GUI/interactive picker. The approved CLI shape is "bare command
  auto-detects everything," not a wizard.
- Re-opening the hub-model debate (Qwen3 vs. other families). Out of scope;
  this design works within the existing Qwen3 ladder.

## Approach

### 1. Fix the stub, don't build a parallel system

`MacosMetalProbe::probe_metal()` will call
`vram_autodetect::query_apple_unified_memory()` (new: a live-available
variant of it, see §2) instead of hardcoding `vram_mb: 0`. This is the
highest-leverage change in this design: it makes the *already-correct*
`auto_preset_for`/`qwen3_*` ladder reachable from the real CLI path for the
first time, instead of adding a third hardware-detection mechanism. No new
probe is invented; the existing one is finally wired in.

### 2. Live available memory, not nameplate total

Today, `query_apple_unified_memory()` computes `physical_total - 12 GiB`
flat reserve. This over- or under-reserves depending on what else is
running. Add `query_apple_available_memory()` alongside it:

- Shell out to `vm_stat` (same idiom as the existing `sysctl`/`nvidia-smi`
  shell-outs — no new FFI/unsafe surface).
- Compute reclaimable-available = `(free + inactive + speculative) *
  page_size`. This is the standard macOS memory-pressure definition —
  `page_free_count` alone is misleading (observed ~830 MB "free" on this
  128 GB machine with tens of GB of real headroom, because macOS parks
  unused RAM in reclaimable pages rather than reporting it free).
- Apply a smaller proportional safety margin (default 15%) to *that*
  number, floored at a minimum absolute reserve (2 GiB) so a nearly-idle
  small Mac doesn't get a margin of a few hundred MB.
- On any parse/shell failure, fall back to the existing static
  `physical_total - 12 GiB` heuristic — never hard-fail hardware detection
  because a live read didn't work.
- `VOX_MENS_UNIFIED_MEM_RESERVE_GIB` keeps working as the override for the
  static fallback; add `VOX_MENS_LIVE_MEM_MARGIN_PCT` for the new
  proportional margin, and `VOX_MENS_DISABLE_LIVE_MEM=1` to force the old
  static behavior (useful for reproducible CI/test runs, which must not
  depend on the test runner's live memory state).

### 3. Real hardware catalogue

Checked the existing five-rung ladder (0.6B/2000MB,
8B-QLoRA/12000MB, 14B-QLoRA/20000MB, 14B-LoRA/44000MB, 32B-QLoRA/60000MB,
32B-LoRA/100000MB — `strong_code_default`/`agentic_default` in
`gpu-specs.yaml`) against every real 2026 Apple Silicon SKU tier (8, 16, 24,
32, 36, 48, 64, 96, 128, 192, 256, 512 GB) run through the new
live-available formula (§2): **no tier produces a gap** (a tier where
nothing fits, or where fail-closed under-picks by more than one rung) —
32/36 GB resolve 14B-QLoRA, 64 GB resolves 14B-LoRA (32B-QLoRA's 60 GB floor
isn't met by 64 GB's live-available budget — correctly conservative, not a
bug), 192/256/512 GB all resolve the existing 32B-LoRA top rung (there is no
larger model in the ladder to reach for, which is fine — adding a bigger
base model is the out-of-scope hub-model debate, not this design). So no new
`floor_mb` rows are needed: the deliverable here is a **test per real tier**
proving `pick_base` resolves a safe, correctly-sized rung under the new
live-available formula, not new catalogue entries. `pick_base`'s existing
fail-closed "largest that fits, error if nothing fits" semantics are
unchanged. Below the 8 GB tier (or when live-available memory is too low),
resolution fails closed with a clear error rather than silently picking
something — matching `qwen3_code_fail_closed_below_floor`'s existing
behavior at the low end.

### 4. `auto` becomes the real default, and stops being two systems

- `preset_schema::DEFAULT_PRESET` (`"4080"`) itself is **not** changed —
  changing it globally would risk altering hyperparameters for existing bare
  `vox mens train` invocations on real CUDA hardware, which is exactly the
  "zero behavior change on the CUDA lane" goal this design must not violate,
  and nothing in the codebase today proves the `auto` path reproduces
  `"4080"`'s exact numbers at 16 GiB. Instead, `resolve_effective_profile`
  picks the fallback-when-omitted **conditionally on detected accelerator
  kind**: CUDA devices keep using `DEFAULT_PRESET` ("4080") exactly as
  today; every other kind (Metal, CPU, unknown) uses `"auto"` instead. This
  is the minimal change that fixes "ignores hardware on Mac" without
  touching a single code path a CUDA user's bare invocation goes through.
- `resolve_effective_profile`'s `"auto"` branch's failure fallback changes
  from the hardcoded `"4080_safe"` to consulting
  `vram_autodetect::auto_preset_for` when the GPU-family `presets:` table
  has no match. `DeviceProfile` (`preset_schema.rs:24-27`) currently only
  carries `model_name`/`vram_mb`, not vendor — `GpuInfo.vendor` (already
  populated by `probe_gpu()`) is dropped before it reaches
  `resolve_effective_profile`. This design adds `pub vendor: String` to
  `DeviceProfile`, threaded through from `GpuInfo::vendor` at the
  `from_gpu_info` call site in `run_train.rs`, and maps it to
  `AcceleratorKind` (`"apple"` → `Metal`, `"nvidia"` → `Cuda`, anything else
  → `Cpu`) at the point `auto_preset_for` is called. This reconciles the two
  "auto" mechanisms instead of leaving them to silently diverge further.
- When `--model` is also omitted, `run_train.rs` resolves it via
  `spoke_base_resolver::pick_base(&overlay, "agentic_default", vram_mb)`
  using the same live-detected VRAM, before falling through to whatever
  today's no-model default does. This is what makes "bare `vox mens
  train`" pick an actual model, not just hyperparameters.

### 5. CUDA lane: untouched, and pinned by tests

`probe_gpu()`'s nvidia-smi-backed path, `auto_preset`'s CUDA arm, and every
existing preset boundary test (6/10/16/24 GiB boundaries,
`qwen_4080_16g`/`4080`/`a100`) are not modified. A new regression test
asserts `DEFAULT_PRESET` changing to `"auto"` still resolves to
`"qwen_4080_16g"` on a simulated 16 GiB CUDA device end-to-end through
`resolve_effective_profile`, closing the loop on "the 4080 Super lane must
keep working exactly as validated."

## Data flow (after this change)

```
vox mens train  (no flags)
  -> probe_gpu() -> hardware::probe() -> MacosMetalProbe::probe_metal()
       -> vram_autodetect::query_apple_unified_memory()  [was: hardcoded 0]
  -> DeviceProfile { vram_mb: <live, real> }
  -> resolve_effective_profile(preset=None, device{vendor:"apple"}, ...)
       -> kind = AcceleratorKind::from_vendor("apple") = Metal (not Cuda)
       -> default-when-omitted = "auto" (DEFAULT_PRESET stays CUDA-only)
       -> gpu-specs presets: no CUDA-family match on Apple vendor
       -> falls back to auto_preset_for(AcceleratorKind::Metal, vram_gb)
       -> walks the extended qwen3_* ladder -> e.g. "qwen3_48g"
  -> model omitted -> spoke_base_resolver::pick_base("agentic_default", vram_mb)
       -> e.g. Qwen3-14B (LoRA, unquantized) at the 48 GB tier
  -> training starts with a model+preset pair that is guaranteed, by
     construction, to fit under live-available memory with margin.
```

## Testing

- Unit tests for `query_apple_available_memory`'s `vm_stat` parser (happy
  path, malformed output, zero page size) — pure functions, no live
  syscalls in CI, matching the existing `parse_hw_memsize`/
  `parse_nvidia_smi_output` test style.
- Unit tests proving no catalogue gap: one test per real SKU tier not
  already covered (8/32/36/64/96/192/256/512 GB) asserting `pick_base`
  resolves the expected model+method under the new live-available formula,
  mirroring the existing `mac_128g_prefers_unquantized_32b_lora`-style
  tests.
- Regression test: `resolve_effective_profile(None, <cuda device>, ...)`
  still resolves through `DEFAULT_PRESET` ("4080") byte-for-byte identically
  to today, for every existing CUDA boundary case — this is the test that
  guards "don't break the validated 4080 Super lane."
- New test: `resolve_effective_profile(None, <metal device>, ...)` now
  resolves through `"auto"` instead of `"4080"` — proving the CLI-default
  fix actually reaches a bare `vox mens train` on a Mac.
- Regression test: `MacosMetalProbe::probe_metal()` no longer returns
  `vram_mb: 0` unconditionally; on a live macOS test runner it returns a
  positive number consistent with `query_apple_unified_memory`.
- Env-var tests for `VOX_MENS_LIVE_MEM_MARGIN_PCT` and
  `VOX_MENS_DISABLE_LIVE_MEM`, following the existing `#[serial]` pattern
  used for other env-mutating tests in this file (`preset_schema.rs`
  already does this for `VOX_BASE_MODEL`).

## Open questions resolved during brainstorming

- Live vs. nameplate memory: **live**, via `vm_stat`'s reclaimable-page
  definition, with a static fallback (§2).
- CLI shape: bare `vox mens train` auto-detects everything by default;
  explicit flags still override (confirmed with the user).
- Scope: Apple Silicon only, Qwen3 ladder only, training-preset selection
  only (not Metal training execution itself, not a GUI picker).
