---
phase: 318-live-vram-selector-size-02
reviewed: 2026-10-27T00:00:00Z
depth: deep
files_reviewed: 25
files_reviewed_list:
  - contracts/mens/training-presets.v1.yaml
  - crates/vox-cli/src/commands/chat.rs
  - crates/vox-gui/src/commands/models_host.rs
  - crates/vox-gui/ui/e2e/lib/tauriMock.families.test.ts
  - crates/vox-gui/ui/e2e/lib/tauriMock.ts
  - crates/vox-gui/ui/src/transport.ts
  - crates/vox-ml-cli/src/commands/ai/serve/mod.rs
  - crates/vox-ml-cli/src/commands/mens/pipeline.rs
  - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-ml-cli/src/commands/mens/training_selection.rs
  - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
  - crates/vox-ml-cli/src/commands/schola/train/run_train.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs
  - crates/vox-orchestrator/src/models/auto_select.rs
  - crates/vox-orchestrator/src/models/mod.rs
  - crates/vox-orchestrator/src/models/registry.rs
  - crates/vox-orchestrator/src/models/select.rs
  - crates/vox-populi/src/mens/tensor/mod.rs
  - crates/vox-populi/src/mens/tensor/preset_schema.rs
  - crates/vox-populi/src/mens/tensor/size_select.rs
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
  - docs/src/architecture/where-things-live.md
  - docs/src/how-to/how-to-train-mens-macos-metal.md
  - mens/config/domain-profiles.yaml
findings:
  critical: 3
  warning: 6
  info: 7
  total: 16
status: issues_found
---

# Phase 318: Code Review Report

**Reviewed:** 2026-10-27
**Depth:** deep (call chains traced into accel_budget, vram_autodetect, the candle checkpoint resume path, the adapter card, the serve handlers and the orchestrator VRAM probe)
**Files Reviewed:** 25 (range `47973e78f..HEAD`, `.planning/` excluded)
**Status:** issues_found

## Summary

The selector core (`size_select.rs`) is careful. The fit math is right: CUDA free MiB x0.88 is applied exactly once on every path, Metal gets no extra margin, `floor_mb` is compared in MiB on both sides, and the ladder is filtered to QLoRA rungs, so the 14B and 32B LoRA rows cannot become sizes. An explicit ladder size is never stepped down, either at pick time or at re-check (M4 and M7 hold). The TS `AutoModelRecommendation` matches the Rust serde shape field for field.

The defects sit at the edges, where the pick meets the rest of the system:

1. On macOS the "live" reading silently becomes the static driver ceiling whenever `vm_stat` fails. This is the D-01 fallback the phase forbids, hidden one call below the selector.
2. The auto pick is now non-deterministic between runs. The checkpoint resume path has no base-model guard, so a re-run or `--resume` can attach an adapter warmed on one size to a different base.
3. The pick never flows back to `train_arm`. Every local `--domain` run now writes an adapter card with `base_rung: "local"` and an empty `preset_version`. Before this phase the card got the spoke preset. vLLM provenance and the BFCL per-rung gate key on that field.

`vox chat --model auto-local` now names an HF base pin that no local server will accept. The wiring and its tests are also thin in the places that matter most (the `gpu.rs` step-down block).

## Critical Issues

### CR-01: Metal "live" reading silently falls back to the static `recommendedMaxWorkingSetSize` (D-01 violation)

**File:** `crates/vox-populi/src/mens/tensor/size_select.rs:59-61`, via `crates/vox-populi/src/mens/tensor/accel_budget.rs:53-57,64-73` and `crates/vox-populi/src/mens/hardware/macos_metal.rs:143-149`
**Issue:** `read_free_memory()` on macOS returns `query_accel_budget().working_set_bytes`. That value is `combine_with_live_pressure(driver_advisory, live_pressure_budget_bytes())`, and when `live_pressure_budget_bytes()` is `None` it returns `driver_advisory` unchanged. That value is `recommendedMaxWorkingSetSize`, the static ceiling, about 75% of RAM. `live_pressure_budget_bytes()` is `None` whenever `vm_stat` cannot be spawned, exits non-zero, or `parse_vm_stat` rejects its output. CONTEXT already flags `vm_stat` output on macOS 26 as unreliable, and a sandboxed GUI process may not be able to exec it. The result is still labelled `"Metal working set"`, so the operator cannot tell.
**Failure scenario:** On a 128 GB Mac with 20 GB actually free and `vm_stat` output that fails to parse, `read_free_memory()` reports about 98 000 MB. The auto pick is then 32B (floor 60 000). The D-05 re-check reads the same ceiling again and keeps 32B, so the run downloads 64 GB of 32B weights before the post-download `plan_for` backstop can refuse. The same applies to `vox chat --model auto-local`, to the GUI recommendation and to `serve_fit_note`, which then waves through a base that does not fit. D-01 says "No live reading → refuse", and this path never refuses.
**Fix:** Read the live part directly in the selector and refuse when it is absent. `live_pressure_budget_bytes` is `pub(crate)` in the same crate:
```rust
#[cfg(target_os = "macos")]
let free = crate::mens::tensor::accel_budget::query_accel_budget().and_then(|b| {
    // No vm_stat reading = no live reading (D-01); never the driver ceiling alone.
    crate::mens::hardware::macos_metal::live_pressure_budget_bytes()
        .map(|live| FreeMemory::metal(b.total_bytes.min(live)))
});
```
Add a test that injects `live = None` and asserts `None`. This needs a small seam, for example a `fn metal_reading(driver: u64, live: Option<u64>) -> Option<FreeMemory>`.

### CR-02: Non-deterministic auto pick plus base-blind checkpoint resume lets an adapter cross bases

**File:** `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:257-275`; resume path `crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/checkpoint.rs:25-68`
**Issue:** Before this phase, an omitted `--model` meant `DEFAULT_MODEL_ID` on CUDA. On Metal it was a pick from the static `recommendedMaxWorkingSetSize`. Both were stable from run to run on one machine. Now `--model` is re-picked from fluctuating live free memory on every invocation, including `--resume <dir>` and the implicit resume, where `checkpoint_root = config.resume_from.unwrap_or(out)` resumes from the output dir unless `--force-restart` is given. `CheckpointState` records no base model. When `load_adapter_into_trainer` fails on the shape mismatch, `apply_checkpoint_resume` only `warn`s and then still restores `epoch`, `global_step` and `pair_offset` from the old run.
**Failure scenario:** Run 1 on a 24 GB card with about 21 GB free auto-picks 14B and is killed at epoch 2. Run 2 is the same command into the same `--output-dir`. A browser now holds 3 GB, so the pick is 8B. The 14B adapter fails to load (warning only), and training continues with fresh 8B LoRA weights from epoch 2 / step N of the old run's shuffle. The result is an under-trained 8B adapter that overwrites the 14B checkpoint in place. Nothing fails loudly, and the adapter card reports a normal run.
**Fix:** Make the base a resume invariant. Either (a) in `run_train`, when `resume.is_some()` or `output_dir` holds a checkpoint or `training_manifest.json`, read the recorded `base_model` and pass it to `select_size` as `explicit`, so it is kept or refused and never re-picked; or (b) add `base_model` to `CheckpointState` and bail in `apply_checkpoint_resume` on a mismatch (unless `force_restart`). (a) is the smaller diff and keeps D-04/D-05 semantics. Add a test that a resume with a recorded 14B base and 13 000 MB free is refused, not re-picked.

### CR-03: The pick never reaches `train_arm`, so local `--domain` adapter cards record `base_rung: "local"`

**File:** `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs:470-474, 527-528, 683, 689`
**Issue:** `resolve_local_spoke_base` now returns the CLI `(model, preset)`, normally `(None, None)`. `train_arm` copies `effective_preset` into `card_preset` before calling `run_train`. The size pick and any D-05 step-down happen inside `run_train`/`gpu.rs` and are never returned. `write_local_adapter_card` then writes `base_rung: meta.preset.unwrap_or("local")` and `preset_version: ""`. Before this phase, `--domain rust` wrote `qwen3_16g`. `base_rung` is load-bearing:
- `vllm_lora.rs:94` `card.is_compatible_with(serve_rung, …)` rejects the adapter for every real rung.
- `eval_gate/bfcl.rs:33-39` documents `per_rung_overrides` keyed by the card's `base_rung` (the preset name). `"local"` silently drops the override and applies the default threshold.

The base itself is recorded correctly, because `lineage.base_model` comes from the training manifest. Only the rung and preset go stale. This is the "step-down leaves stale preset state behind" class: even with no step-down, the card never learns the preset that was actually used.
**Failure scenario:** `vox mens train --domain agentic` on a 24 GB card trains 14B with `qwen3_24g`. The card says `base_rung: "local"`, `preset_version: ""`. A vLLM LoRA load with `serve_rung = "qwen3_24g"` fails with "provenance mismatch". A BFCL gate with `per_rung_overrides: {qwen3_24g: 0.7}` evaluates against the default `min_accuracy` instead.
**Fix:** Have `run_train` (and `run_gpu_training`) return the final `(hf_id, preset)` after the re-check, for example `Result<TrainOutcome>`, or write the effective preset into `training_manifest.json` and have `write_local_adapter_card` read it from `read_training_lineage` the way it already reads `base_model`. Add a test asserting the card's `base_rung` equals `SizeRung.preset` for an auto pick.

## Warnings

### WR-01: `vox chat --model auto-local` sends an HF base pin no local server will accept

**File:** `crates/vox-cli/src/commands/chat.rs:571-586`; `crates/vox-ml-cli/src/commands/ai/model_id.rs:5-25`; `crates/vox-ml-cli/src/commands/ai/serve/handlers.rs:87,362,375`
**Issue:** auto-local now sets `LlmConfig.model = "Qwen/Qwen3-8B@b968…"`. `vox mens serve` names its model by the checkpoint file stem (`serve/mod.rs:112-117`, for example `candle_qlora_adapter`). `do_chat_completions` forwards `req.model` into `do_generate`, where `mismatched_model_error` compares `stem("Qwen/Qwen3-8B@b968…") = "Qwen3-8B@b968…"` with the loaded stem and returns 409. The default endpoint `127.0.0.1:11434` is Ollama's port, and Ollama has no model by that name. So auto-local can never complete a turn. The removed fake ids were equally unservable, so this is not a regression in kind. However, the new how-to text (lines 145-148) says auto-local "uses the same pick as training" as if it were a working path.
**Fix:** Decide what auto-local should resolve to: a served run whose recorded base matches the pick, or the pick used only as an admission check against the server's `/v1/models`. Alternatively, send no `model` field, since `mismatched_model_error` accepts `None`, and print the pick as advice. At minimum, do not advertise it in the how-to until a request can succeed.

### WR-02: Training and inference read different GPUs; NVML `0` blocks the fallback

**File:** `crates/vox-populi/src/mens/tensor/size_select.rs:64-67`, `crates/vox-populi/src/mens/tensor/vram_autodetect.rs:12-25`, `crates/vox-orchestrator/src/models/auto_select.rs:265-269`, `crates/vox-orchestrator/src/models/vram.rs:209-216`
**Issue:** Training uses `parse_nvidia_smi_output`, which reads only the **first line**, physical GPU 0 in nvidia-smi enumeration order. That order ignores `CUDA_VISIBLE_DEVICES` and is not CUDA's order. Chat and the GUI use the NVML probe's **minimum** free memory across all devices. The how-to says both make "the same pick". The probe path also skips the `mb > 0` filter that `read_free_memory` applies: `Some(FreeMemory::cuda(0))` short-circuits `.or_else(read_free_memory)`.
**Failure scenario:** On a 2-GPU box, GPU 0 is an idle 48 GB card and GPU 1 is a 24 GB card at 90% use. `CUDA_VISIBLE_DEVICES=1 vox mens train` reads GPU 0, picks 32B, and trains on GPU 1, where the HF download and `plan_for` backstop are the first things to object. On the same box, auto-local reads min = 2.4 GB and refuses. With a MIG or display device reporting `memory_free_mb: 0`, auto-local refuses with "0 MB free" even though nvidia-smi would have given a reading.
**Fix:** In training, read the device that will actually be used. Query `nvidia-smi --id=<ordinal>`, mapping `CUDA_VISIBLE_DEVICES[0]` through `--query-gpu=uuid`, or take the min across visible devices to match the orchestrator. In `host_free_memory`, filter `free_vram_mb_from_probe().filter(|&mb| mb > 0)` before mapping.

### WR-03: Bare concrete model ids without `/` are now a hard error in training (D-04 says pass through)

**File:** `crates/vox-populi/src/mens/tensor/size_select.rs:104-113`; caller `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:261-267`
**Issue:** `select_size` treats only inputs containing `/` or `\` as concrete ids. Anything else that `resolve_base_pin` cannot resolve bails with "size X does not resolve". `--model gpt2`, `--model my-local-dir` (a relative path in the CWD) and `--model Qwen3-8B@main` all used to pass through to the HF downloader or a local path, and now refuse before training starts. Plan 03 noticed exactly this for serve and special-cased it (`serve/mod.rs:206-208`), but the training path was not fixed. D-04: "A concrete id not on the ladder passes through unchanged."
**Fix:** Pass through anything that is not a ladder size name. Error only when the input looks like a ladder size (`Qwen3-<n>B` / `Qwen/Qwen3-<n>B` without a valid pin), or when `Path::new(req).exists()` is false and the input matches the size pattern. Add `gpt2` and an existing relative dir to `concrete_ids_off_the_ladder_pass_through`.

### WR-04: Serve now refuses runs that previously served, with no override

**File:** `crates/vox-ml-cli/src/commands/ai/serve/mod.rs:125-134, 210`
**Issue:** `serve_fit_note` calls `select_size(..., false)`. `force` is hard-wired off, and `VOX_MENS_FORCE_TRAIN` is not consulted. The footprint is the QLoRA **training** floor (D-07 accepts this as conservative). Together they turn an advisory into a hard refusal with no escape hatch. The only remedy the docs offer is "serve on a machine with more free memory or train a smaller size".
**Failure scenario:** A 32B run (floor 60 000) on a 64 GB Mac whose live working set is 45 000 MB. That memory is enough to run inference on the 4-bit base plus adapter, but `vox mens serve` now exits before the worker loads. The run cannot be served on the machine that trained it after a reboot, if training ran with more free memory or with `VOX_MENS_FORCE_TRAIN=1`.
**Fix:** Honor an operator override, either the existing `VOX_MENS_FORCE_TRAIN` or a serve-specific `VOX_MENS_FORCE_SERVE`, by passing it as `force`. `explicit_pick` already words the forced case. Test both arms.

### WR-05: The step-down wiring in `gpu.rs` and the explicit-precedence wiring in `run_train` are untested

**File:** `crates/vox-ml-cli/src/commands/schola/train/gpu.rs:96-107`; `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:257-275`
**Issue:** M9 and M10 mutate only the helpers (`live_free_memory`, `recheck_size_pick`). Nothing covers the three statements that make a step-down effective: `set_var("VOX_BASE_MODEL")`, `model = Some(pick.rung.hf_id)` and `preset = preset.or(...)`. Nothing covers the `--model` over `VOX_MENS_DEFAULT_MODEL` precedence or the `Size` vs `PassThrough` arms in `run_train`. Deleting `model = Some(pick.rung.hf_id.clone());` passes every test: after a step-down the run would download the *original*, non-fitting size with the *smaller* size's preset, and the plan summary's own deviation #1 shows this class was already hit once.
**Fix:** Extract the block into a pure `fn apply_recheck(pick, model, preset) -> (String, String, &str /*env*/)`, or have `recheck_size_pick` return the triple and test it: auto 14B at 13 000 MB gives model 8B pin, preset `qwen3_16g` and env 8B pin. An explicit `--preset` must survive. Add a `run_train`-level test (or extract `resolve_explicit_model(cli, env)`) for CLI over env precedence and empty or whitespace env values.

### WR-06: Chat and the GUI judge against the embedded ladder; training and serve use the workspace file

**File:** `crates/vox-orchestrator/src/models/auto_select.rs:350`
**Issue:** `select_for_free_memory` always calls `load_ladder_overlay(None)`, the compile-time copy. `run_train` and serve use `load_ladder_overlay(workspace_root)`. `embedded_ladder_matches_the_repo_file` only holds at build time. Phase 320 exists to replace estimated `floor_mb` values with measured ones in `gpu-specs.yaml`, and once that file is edited in a checkout, chat and the GUI keep the old floors until a rebuild. That contradicts the how-to's "use the same pick as training".
**Fix:** Pass `vox_repository::find_cargo_workspace_root()` (or the orchestrator's existing repo-root helper, so no new crate edge is needed) to `load_ladder_overlay`. `load_overlay` already falls back to the embedded copy on `NotFound`.

## Info

### IN-01: A re-check that keeps the pick prints the stale first note

**File:** `crates/vox-populi/src/mens/tensor/size_select.rs:146-148`; `crates/vox-ml-cli/src/commands/schola/train/run_train.rs:142`
**Issue:** When the auto pick still fits, `recheck_before_load` returns the original `SizePick`, and `recheck_size_pick` prints its note again: "picked 8B … fits 13000 MB free" with the *first* reading. The operator sees the same line twice and never sees the fresh number.
**Fix:** On the keep path, replace the note with a re-check line such as `"{size} still fits: floor {floor} MB, {free} MB free now ({source})"`.

### IN-02: The selector module still exports a default-size path

**File:** `crates/vox-populi/src/mens/tensor/size_select.rs:172-174`
**Issue:** `resolve_base_pin(None, _)` returns `DEFAULT_MODEL_ID`, which is 8B. Today only `reasoning_compare` uses it, as the default for `--base`. `select_size` and `serve_fit_note` never pass `None`. Still, a public function in the "no default size" module that returns a default size invites a future D-01 regression.
**Fix:** Change the signature to `requested: &str` and move the `None => DEFAULT_MODEL_ID` default into `reasoning_compare.rs:340`.

### IN-03: Another revision of a ladder size skips the floor check

**File:** `crates/vox-populi/src/mens/tensor/size_select.rs:105-109`
**Issue:** `Qwen/Qwen3-32B@<another 40-hex>` passes `resolve_base_pin` unchanged, is not found by exact `hf_id`, and becomes `PassThrough`, with no fit check and no preset. Only the post-download `plan_for` backstop remains.
**Fix:** Match ladder rungs by repo id (`split_pin(..).0`) for the floor and preset, and keep the requested revision.

### IN-04: The e2e mock's `reason` does not match the real note format

**File:** `crates/vox-gui/ui/e2e/lib/tauriMock.ts:301`
**Issue:** The real note is `"picked 8B: largest size whose floor 12000 MB fits 13926 MB free (Metal working set)"`. The mock's text has a different shape. That is harmless today, since no UI renders it, but a future renderer that parses or highlights it would be tested against fiction.
**Fix:** Copy the exact `select_size` note format into the mock.

### IN-05: Hub and spokes trained in separate invocations can land on different sizes

**File:** `crates/vox-ml-cli/src/commands/mens/training_selection.rs:56-62`
**Issue:** D-06 scopes inheritance to one invocation, and persisting the hub size is deferred. In practice each `vox mens train --domain X` or pipeline run is its own invocation, so the hub and each spoke get independent live picks and may end up on different bases. This compounds CR-03, because the card cannot even say which rung was used.
**Fix:** Deferred by design. Print a warning when a `--domain` pick differs from the hub's recorded base, once one exists.

### IN-06: The hardware-dependent tests only exercise the refusal branch on CI

**File:** `crates/vox-cli/src/commands/chat.rs:675-686`; `crates/vox-gui/src/commands/models_host.rs:95-105`; `crates/vox-populi/src/mens/tensor/size_select.rs:376-385`
**Issue:** Each test accepts either `Ok` or `Err`, or `Some` or `None`. On a GPU-less hosted runner only the `Err`/`None` arm runs, so the success shape (pin format, `floor_mb <= free_mb`) is never asserted in CI.
**Fix:** Test the success shape through `select_for_free_memory(Some(&FreeMemory{..}))`, which already exists, and keep the live tests as smoke tests.

### IN-07: An out-of-phase commit is in the reviewed range

**File:** `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs` (commit `371c198fe`)
**Issue:** This commit came from another session (a 655-line revert to restore compilation). It is not phase 318 work. No dangling references to the removed functions (`chat_regenerate`, `chat_stream_resume`, `chat_queue_steer`, `chat_compact_up_to`, …) remain under `crates/`, `contracts/` or `docs/src/reference`.
**Fix:** None for this phase. Keep it out of the phase PR, or call it out in the PR description.

---

_Reviewed: 2026-10-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
