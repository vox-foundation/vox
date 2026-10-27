---
phase: 318-live-vram-selector-size-02
verified: 2026-10-27T00:00:00Z
status: passed
# Orchestrator 2026-10-27: human item 2 resolved (cargo check -p vox-cli -p vox-gui --tests exit 0 at 229b3ce72);
# human item 1 (live pick + step-down on this Mac) carried to Phase 320 S0 --plan-only and S2 smoke (320-CONTEXT D-02, D-05).
score: 4/4 roadmap success criteria verified (8/8 decisions D-01..D-08 hold)
covered_files:
  - ".planning/phases/318-live-vram-selector-size-02/318-01-PLAN.md"
  - ".planning/phases/318-live-vram-selector-size-02/318-01-SUMMARY.md"
  - ".planning/phases/318-live-vram-selector-size-02/318-02-PLAN.md"
  - ".planning/phases/318-live-vram-selector-size-02/318-02-SUMMARY.md"
  - ".planning/phases/318-live-vram-selector-size-02/318-03-PLAN.md"
  - ".planning/phases/318-live-vram-selector-size-02/318-03-SUMMARY.md"
  - ".planning/phases/318-live-vram-selector-size-02/318-04-PLAN.md"
  - ".planning/phases/318-live-vram-selector-size-02/318-04-SUMMARY.md"
  - "crates/vox-cli/src/commands/chat.rs"
  - "crates/vox-gui/src/commands/models_host.rs"
  - "crates/vox-gui/ui/src/transport.ts"
  - "crates/vox-ml-cli/src/commands/ai/serve/mod.rs"
  - "crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs"
  - "crates/vox-ml-cli/src/commands/mens/training_selection.rs"
  - "crates/vox-ml-cli/src/commands/schola/train/gpu.rs"
  - "crates/vox-ml-cli/src/commands/schola/train/run_train.rs"
  - "crates/vox-orchestrator/src/models/auto_select.rs"
  - "crates/vox-orchestrator/src/models/registry.rs"
  - "crates/vox-plugin-mens-candle-core/src/candle_qlora_train/training_loop/checkpoint.rs"
  - "crates/vox-plugin-mens-candle-core/src/checkpoint_state.rs"
  - "crates/vox-populi/src/mens/tensor/accel_budget.rs"
  - "crates/vox-populi/src/mens/tensor/preset_schema.rs"
  - "crates/vox-populi/src/mens/tensor/size_select.rs"
  - "crates/vox-populi/src/mens/tensor/vram_autodetect.rs"
covered_digest: "v3:sha256:e1478f2ce8a26a78f72dd60b2b90cdc84d5770f4ccde47014f50be4ad8d1bfa3"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "On this Mac run a real `vox mens train --backend qlora --tokenizer hf --device metal --data-dir examples/mens/metal-e2e --output-dir mens/runs/size-select-check --epochs 1 --max-runtime-secs 60 --fast-corpus` (VALIDATION manual-only row) and confirm `size: picked <size>` prints before the HF download. Then start a memory-heavy process between pick and download."
    expected: "The pick is the largest Qwen3 size whose floor fits current free memory; with memory taken, `size: stepped down from X to Y: free memory fell to ...` prints and the smaller size's pin and preset are what download and train."
    why_human: "A real run downloads weights and trains. Unit tests cover the selector, the re-check and the model/preset adoption with injected readings. No test drives the live vm_stat/nvidia-smi read through `run_train` -> `gpu.rs` to the download."
  - test: "Let hosted CI compile `vox-cli` and `vox-gui` (`cargo check -p vox-cli`, `cargo check -p vox-gui --tests`, then `chat_routing` and `auto_model_recommendation` tests)."
    expected: "Both crates compile and their tests pass."
    why_human: "Per CONTEXT these two crates did not compile locally on this branch (an unrelated vox-orchestrator-mcp error). `chat.rs` and `models_host.rs` (the chat and GUI consumers) were therefore never compiled or tested here. I only confirmed by reading that they call `select_optimal_local_model_for_host`."
---

# Phase 318: Live VRAM Selector (SIZE-02) Verification Report

**Phase Goal:** One selector reads current free memory, titrates to the largest Qwen3 size that fits, and picks the hub and its spokes together. Training, local serve and chat all use it. The fixed `auto_select` tiers are gone.
**Verified:** 2026-10-27
**Status:** human_needed (no gaps found; two items need a person or hosted CI)
**Re-verification:** No, initial verification

## Goal Achievement

### Roadmap success criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Reads current free memory (Metal live budget, CUDA free VRAM), not total, and picks the largest size whose footprint fits | VERIFIED | `size_select.rs:69-79` `read_free_memory`: macOS uses `query_accel_budget` and `metal_reading(working_set_bytes, working_set_is_live)`; elsewhere per-device nvidia-smi `memory.free`. `largest_fit` (`:371`) scans the QLoRA-only ladder in floor order. Tests `auto_pick_is_the_largest_size_that_fits`, `cuda_reading_keeps_the_safety_margin`, `metal_working_set_without_vm_stat_is_no_reading` pass (vox-populi, 96/96 in my filter). |
| 2 | If free memory drops between pick and load it steps down a size and says why | VERIFIED | `recheck_before_load` (`size_select.rs:191`) keeps a fitting auto pick (never steps up), steps down with a note naming both sizes and the MB, refuses with no reading, never steps an explicit size. Wired at `gpu.rs:94-109` (re-read, then `adopt_size_pick` sets model, preset and `VOX_BASE_MODEL`) before the download at `gpu.rs:216`. Serve re-reads before `spawn_inference_worker` (`serve/mod.rs:~125-140`). Tests `recheck_steps_an_auto_pick_down_and_says_why`, `size_pick_tests::a_stepped_down_pick_replaces_the_model_and_fills_the_preset` pass. Live end to end is a human item. |
| 3 | Every spoke's base and preset follow the picked size; an explicit size overrides; a size that does not fit is refused, not shrunk | VERIFIED | `training_selection.rs:56-62`: live mode (`fixed_tier_mb: None`) ignores the spoke `base.model` tag and `base.preset`. `pipeline.rs:466` and `train_arm.rs:739` pass `None`; `run_train` does the one pick (`run_train.rs:261-277`). `explicit_pick` (`size_select.rs:309`) refuses unless `VOX_MENS_FORCE_TRAIN`. Tests `live_mode_leaves_size_and_preset_to_the_selector`, `explicit_size_that_does_not_fit_is_refused_not_shrunk` pass. |
| 4 | The inference auto-selector and the training pick both use the selector; the fake tiers are gone | VERIFIED | Orchestrator `auto_select.rs` is 190 lines calling `size_select::select_size`; no `select_tier_for_vram`, `detect_hardware_vram_gb`, `register_mens_local_candidates` or `qwen3_27b_metal_check` candidate remains (grep over `crates`). `chat.rs:574`, `models_host.rs:37` and serve call into it. Registry test for the six fake ids is deleted. Orchestrator `auto_select`: 5/5 pass. |

### Decisions D-01..D-08

| ID | Status | Evidence |
|----|--------|----------|
| D-01 free memory | HELD | Metal budget with no extra margin (`FreeMemory::metal`), CUDA x`SAFETY_FRACTION` 0.88 (`FreeMemory::cuda`). No reading refuses (`NO_READING`, `size_select.rs:172`). CR-01: `AccelBudget.working_set_is_live = live.is_some()` (`accel_budget.rs:84`) so a missing/unparseable vm_stat gives `None`. `vram_autodetect` total-as-free and `probe_gpu().vram_mb` are not used. |
| D-02 ladder + location | HELD | `size_ladder` filters `qwen3_code` to rungs with `qlora`, requires a preset per size (fails closed), sorts by `floor_mb`. Module is `vox-populi::mens::tensor::size_select`. `git diff --stat` shows no `Cargo.toml` change, so no new crate edge. |
| D-03 presets | HELD | `SIZE_PRESETS` 0.6B qwen3_dev_cpu, 1.7B qwen3_6g, 4B qwen3_12g, 8B qwen3_16g, 14B qwen3_24g, 32B qwen3_96g. `qwen3_6g`/`qwen3_12g` are in `training-presets.v1.yaml`, `KNOWN_PRESETS` (`spoke_base_resolver.rs:34-35`) and `preset_schema.rs` (`S1p7`, `S4`). populi preset contract tests pass. |
| D-04 explicit size | HELD | `explicit_model(--model, VOX_MENS_DEFAULT_MODEL)` (`run_train.rs:111`); `resolve_base_pin` moved into populi and re-exported at `reasoning_compare/detect.rs:83`; no new flag; off-ladder ids pass through; force still applies. See "@main" below for one edge. |
| D-05 step-down | HELD | See SC2. Explicit never stepped down; `plan_for` post-download refusal retained. |
| D-06 hub + spokes | HELD | One pick per `run_train`; the pipeline trains one profile per invocation and spokes inherit via live mode. Not persisted (IN-05 deferred by decision). |
| D-07 inference consumers | HELD | `floor_mb` is the footprint; fake tiers and registry entries deleted; `AutoModelSelection { selected_model_id, size, free_mb, floor_mb, reason }` matches the TS `AutoModelRecommendation` and the e2e mock; no stale `detected_vram_gb`/`tier_reason` left. |
| D-08 cloud | HELD | `train_arm.rs:778` `CLOUD_VRAM_MB = 24_000` untouched, left for Phase 319. |

### Review fixes

| Fix | Status | Evidence |
|-----|--------|----------|
| CR-01 Metal without vm_stat is no reading | VERIFIED | `metal_reading(_, false) == None`, test passes. Field populated from the real `vm_stat` read in `query_accel_budget`; CUDA budget sets it `false`. |
| CR-02 checkpoint records base; resume refuses mismatch | VERIFIED | `CheckpointState.base_model` (`#[serde(default)]`); all writers set it (core `epoch_boundary.rs:67`, metal and cuda `checkpoint_mid.rs:49` and `training_loop/mod.rs`). `apply_checkpoint_resume` bails on a different base naming both pins plus `--model`/`--force-restart`; an old checkpoint without a base resumes with a warning; an adapter that fails to load now bails. `config.base_model` is set from the final model at `gpu.rs:460`. vox-plugin-mens-candle-core checkpoint tests: 14/14 pass, including `refuses_to_resume_onto_a_different_base`. |
| CR-03 cards record the picked preset | VERIFIED | `run_train`/`run_gpu_training` return `Option<SizePick>` (final pick after re-check); `train_arm.rs:611-618` passes `trained_preset(card_preset, final_pick)` into the card meta, and `base_rung`/`preset_version` come from it. `local_card_records_the_auto_pick_preset_as_base_rung` passes. |

I did not repeat the mutation proofs (no rebuild of mutated trees). I relied on reading the guards plus the tests passing; the REVIEW-FIX records sha-matched before/mutated/restored values.

### The `@main` question: contradiction or not

Both statements are true; they describe different inputs.
- IN-03 (`another_revision_of_a_ladder_size_is_still_a_size`) covers a revision that is a full 40-lowercase-hex SHA. `split_pin` sees a valid pin, `resolve_base_pin` returns it, `select_size` matches the ladder by repo id, and the size's floor and preset apply.
- `Qwen/Qwen3-8B@main` is not a valid pin (`split_pin` needs exactly 40 hex). `resolve_base_pin` errors, and `names_a_ladder_size` is true (it strips `@rev`), so the pass-through arm is skipped and `select_size` returns the error. `unknown_bare_size_name_is_an_error` asserts exactly this for `Qwen3-8B@main` and `Qwen/Qwen3-8B@main`; `base_resolution_unknown_or_ambiguous_is_an_error` asserts it at the resolver.

Is it a regression? Yes, a small, deliberate one for training. At `47973e78f` an omitted-size flow passed `--model Qwen/Qwen3-8B@main` straight to the downloader, and `hub.rs:50` accepts any revision string. Now training refuses it before the download with a message listing known sizes and "pass owner/name@<40-hex sha>". It fits the milestone's pin policy (Phase 317, "no fallback revision"; a floating ref cannot be recorded reproducibly in the manifest), and the `resolve_base_pin` it came from already rejected it. It is not a SIZE-02 gap. Worth a decision (Advisory 1).

## Test evidence (scoped, run by me)

| Crate | Filter | Result |
|-------|--------|--------|
| vox-populi `--features mens-train --lib` | `size_select`, `spoke_base_resolver`, `preset_schema`, `accel_budget` | 96 run, 96 passed (2 leaky) |
| vox-ml-cli `--features cloud --lib` | `schola::train`, `ai::serve`, `training_selection`, `train_arm`, `reasoning_compare` | 122 run, 122 passed |
| vox-orchestrator `--lib` | `auto_select`, `premium_alias`, `models::registry` | 29 run, 27 passed, 2 failed (the known two) |
| vox-plugin-mens-candle-core `--lib` | `checkpoint` | 14 run, 14 passed |

Not run: vox-cli, vox-gui, GUI vitest/typecheck, metal/cuda plugin builds (the fix report states a clean `cargo check`).

### Known failures: pre-existing, not caused by phase 318

`default_premium_alias_targets_exist_in_models_list` and `select_with_premium_alias_honors_alias_when_intelligence_high` fail on HEAD. Their messages are about cloud alias targets missing from the default models list (`~openai/gpt-pro-latest`, `~google/gemini-flash-latest`, `~deepseek/deepseek-pro-latest`); the first missing alias differs per retry. I did not build `47973e78f` in a worktree (cold orchestrator build). The evidence:
- The tests live in `models/tests.rs` and `models/select.rs`; the alias data is `contracts/orchestration/model-defaults.v1.yaml`. `tests.rs` and the yaml are untouched in `47973e78f..HEAD`; their last commits are older (`6d88f215d`, `7c3cefceb`).
- In `select.rs` and `registry.rs` the phase only changed re-exports and removed the six `voxlocal` candidates, unrelated to cloud aliases.
- 318-VALIDATION recorded the planner baseline before any phase code: "unfiltered `test(models::)`: 317 run, 2 failed", the same two tests.

## Advisory (not blocking, not counted in the score)

1. `Qwen/Qwen3-<n>B@<branch>` is now refused in training (above); serve passes it through (`serve_fit_note` returns `None` when the pin does not resolve). Decide whether to accept non-hex revisions of ladder repos with the size's floor check, or document the refusal in the how-to (no `@main` mention exists there).
2. WR-01 stays open by design: `vox chat --model auto-local` selects a pin, but the default endpoint is Ollama's port and a `vox mens serve` answers 409 for any name but its checkpoint stem, so a turn cannot complete without setting `VOX_LOCAL_ENDPOINT` and using `mens/<id>`. Only the how-to was corrected. SIZE-02 asks that chat use the selector, which it does; a working chat turn is a follow-up.
3. A CPU run with no explicit model now refuses (no accelerator reading, "no default size"); before, it defaulted. Tested (`cpu_auto_pick_refuses_and_names_the_way_out`) and intentional per D-01.
4. CR-02 residual (stated in the fix report): a missing adapter file on resume still warns and keeps counters.
5. Commit `371c198fe` (restores `vox-orchestrator-mcp/.../history.rs`, 656 lines) is in the range but outside this phase. Keep it out of the phase PR or call it out.
6. Bookkeeping: `ROADMAP.md` still shows the four plans unchecked and `0/4 Planned`, and `REQUIREMENTS.md` still lists SIZE-02 as Pending. Update after the human items clear.
7. The roadmap's plan line names presets `qwen3_6g`/`qwen3_12g` (matches the code); CONTEXT D-03 says only "new presets for 1.7B and 4B". No conflict.

## Human Verification Required

### 1. Real training run: pick and live step-down
**Test:** Run the manual-only command in the frontmatter on this Mac, once with idle memory and once with a memory-heavy process started between the pick and the download.
**Expected:** `size: picked <size>` before the download; under pressure `size: stepped down from X to Y`, and the smaller pin and preset are downloaded and trained.
**Why human:** The unit tests inject readings; nothing drives the real vm_stat/nvidia-smi path through `run_train` to the download.

### 2. vox-cli and vox-gui compile and consumer tests
**Test:** Hosted CI (or a local build once the `vox-orchestrator-mcp` error is out of the way): `cargo check -p vox-cli`, `cargo check -p vox-gui --tests`, `chat_routing`, `auto_model_recommendation`; `pnpm --dir crates/vox-gui/ui typecheck` and `vitest run e2e/lib/tauriMock.families.test.ts`.
**Expected:** Compile clean; tests pass.
**Why human:** Not compilable locally on this branch, so the chat and GUI consumers are verified by reading only.

## Gaps Summary

None. All four roadmap criteria and D-01..D-08 hold in the code, the three critical review fixes are present and tested, and the two orchestrator failures predate the phase. Status is `human_needed` only because the live-run check and the vox-cli/vox-gui compile cannot be done here.

---

_Verified: 2026-10-27_
_Verifier: Claude (gsd-verifier)_
