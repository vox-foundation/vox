---
phase: 318-live-vram-selector-size-02
reviewed: 2026-10-29T21:30:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - crates/vox-cli/src/commands/chat.rs
  - crates/vox-gui/src/commands/models_host.rs
  - crates/vox-ml-cli/Cargo.toml
  - crates/vox-ml-cli/src/commands/mens/pipeline.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
  - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
  - crates/vox-ml-cli/src/commands/mens/training_selection.rs
  - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
  - crates/vox-orchestrator/src/models/auto_select.rs
  - crates/vox-orchestrator/src/models/mod.rs
  - crates/vox-orchestrator/src/models/select.rs
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
findings:
  critical: 0
  warning: 0
  info: 0
  total: 0
status: clean
re_reviewed: 2026-10-29T21:50:18Z
re_review_range: 9a41951bc..e3a27eda7
---

# Phase 318: Code Review Report

**Reviewed:** 2026-10-29T21:30:00Z
**Depth:** standard
**Files Reviewed:** 12
**Status:** clean

## Re-review

Re-reviewed the fix commits `9a41951bc` through `e3a27eda7` against `318-CONTEXT.md` on 2026-10-29T21:50:18Z. No new critical or warning findings. The original five findings below are fixed: the env pin wins over an injected reading, a same-size confirm keeps a non-empty caller preset, RagOnly and PromptOnly skip the accelerator read, the pre-load step-down uses the overlay already loaded for the pick, and `needs_reading` no longer substring-matches `Qwen3-`.


## Summary

Reviewed the source diff from `07f504dfb` (exclusive) through `670245c6e` (inclusive). Planning summaries were not treated as source. The working tree's unrelated dirty files were not reviewed.

The selector itself matches the locked decisions. `select_qwen3_code_qlora_for_free_mb` runs `pick_base` on measured `qwen3_code` QLoRA rows only, so 1.7B and 4B (`floor_measured: false`) cannot win, nothing-fits returns an error, and there is no `vox-mens-8b-v0.6` or `mens/runs/qwen3_27b_metal_check` ladder in `auto_select.rs`. CUDA free is `query_nvidia_smi_vram`'s `memory.free` field; Metal free is `monitor_metal`'s `memory_free_mb`. Chat `auto-local` and the GUI host fail closed on an injected error. Explicit size spellings are not stepped down. Capability tags stay on `resolve_base_model` / `pick_base`. No new workspace crate dependency was added. Tests pass free-memory readings in; the live probe is not on the default unit path.

The train entry points still drop a named model and a named preset on the way to that picker.

`crates/vox-orchestrator/src/models/select.rs` and `mod.rs` in this range also change router status demotion and `context_fill_ratio`. Unset models still default to operational, so that was not scored as a selector defect.

## Critical Issues

### CR-01: Unnamed training ignores `VOX_MENS_DEFAULT_MODEL`

**File:** `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:303-353`
**Issue:** A missing `--model` is not the same as an unnamed size. `resolve_default_model_id` still treats a non-empty `VOX_MENS_DEFAULT_MODEL` as the base id, and `maybe_resolve_metal_default_base` still documents that env pin as winning over the metal ladder. Both are now skipped whenever `automatic_qwen3 && model.is_none()`: that branch always calls `select_qwen3_code_qlora_for_free_mb` and assigns `model` and `preset`. Dogfood passes `None` and `automatic_qwen3: true`, so the env pin never runs. Pipeline and `resolve_local_spoke_base` hit the same hole one layer up: `resolve_training_selection` (`training_selection.rs:81-88`) selects a Qwen3 size as soon as `cli_model` is absent and a free-memory reading exists, then hands that id to `run_train` with `automatic_qwen3` still true. An operator pin is replaced by the live pick and can still be stepped down at load.
**Fix:**

```rust
let pinned = env_default_model
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_string);
if model.is_none() {
    if let Some(pinned) = pinned {
        model = Some(pinned);
        // A named pin is explicit: preload must refuse, not step down.
        automatic_qwen3 = false;
    }
}
if automatic_qwen3 && model.is_none() {
    let free_mb = read_accelerator_free_mb()?;
    let pick = select_qwen3_code_qlora_for_free_mb(free_mb)?;
    model = Some(pick.hf_id);
    preset = Some(pick.preset);
}
```

Apply the same order in `resolve_training_selection`: only the live selector when both the CLI model and `VOX_MENS_DEFAULT_MODEL` are absent. Thread the env value in as an argument so the function stays free of process-env reads in unit tests.

## Warnings

### WR-01: Caller preset does not survive the pre-load revision

**File:** `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:365-379`
**Issue:** `resolve_training_selection` keeps a CLI `--preset` (`training_selection.rs:91-95`), and its test expects `a100` to stay with the 8B pick. `apply_preload_free_mb` ignores that argument whenever it returns a revision, including when `confirm_before_load` keeps the same size (`revision_keeping` copies `pick.preset`). `run_train` then assigns `preset = Some(loaded_preset)`. Training therefore uses `qwen3_16g` / `qwen3_24g` / `qwen3_96g` instead of the flag the operator passed. `train_arm` also snapshots `effective_preset` into the adapter card before this call (`train_arm.rs:533-536`, `691`), so `base_rung` can name the pre-revision preset while the run loaded a stepped size.
**Fix:** When the loaded size equals the original size and the caller passed a non-empty preset, keep that preset. On a real step-down, use the smaller size's preset and return the `PreloadRevision` to `train_arm` so the adapter card records `loaded_size` and `loaded_preset`.

### WR-02: RagOnly and PromptOnly still require a live free-memory read

**File:** `crates/vox-ml-cli/src/commands/mens/pipeline.rs:465-494`
**Issue:** `resolve_training_selection` returns `Skip` for `RagOnly` and `PromptOnly` before it needs a reading (`training_selection.rs:38-41`). Pipeline reads `read_accelerator_free_mb()` first whenever `--model` is absent, and `?` aborts the stage. `resolve_local_spoke_base` (`train_arm.rs:741-748`) does the same, then turns `Skip` into a bail. A skippable spoke on a host with no `nvidia-smi` and no Metal device fails closed instead of skipping. The old resolver returned `Skip` without a VRAM probe.
**Fix:** Resolve the profile method first. Call `read_accelerator_free_mb` only when the selection is `Train` and the size is unnamed or an explicit Qwen3 spelling.

### WR-03: Automatic pick and the pre-load check use different overlays

**File:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs:513-516`
**Issue:** `select_qwen3_code_qlora_for_free_mb` always parses the compile-time `include_str` of `gpu-specs.yaml`. `apply_preload_free_mb` and `explicit_qwen3_size_for_free_mb` use `load_overlay(workspace)`, which prefers the on-disk file. `confirm_before_load` steps down by calling the embed picker again (`spoke_base_resolver.rs:583`). If the checkout yaml and the binary embed disagree, the automatic pick can choose a size whose workspace `floor_mb` does not fit, and the step-down can select that same embed row again.
**Fix:** Thread the overlay `apply_preload_free_mb` already loaded into the step-down (`select_qwen3_code_qlora_from_overlay`). For the initial pick, use `load_overlay(root)` when a workspace root exists; `load_overlay` already falls back to the embed when the file is missing.

### WR-04: Any repo substring `Qwen3-` forces a live read

**File:** `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:355-364`
**Issue:** `needs_reading` is true when `repo.contains("Qwen3-")`, not only for a measured `qwen3_code` pin or a `resolve_qwen3_size` spelling. A concrete id such as `org/Qwen3-8B-finetune` is not a size spelling and is not a measured row, so `apply_preload_free_mb` would pass it through — but only after `read_accelerator_free_mb()?`. A probe failure aborts that train. The same arm also treats every official `Qwen/Qwen3-*@sha` pin as a floor-checked explicit size even when the caller used the concrete-id passthrough (`resolve_base_model` returns those ids without a VRAM check).
**Fix:** Set `needs_reading` from `automatic_qwen3`, `is_qwen3_size_spelling`, or `measured_qlora_row` on the exact hf id. Do not substring-match the repo.

---

_Reviewed: 2026-10-29T21:30:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
