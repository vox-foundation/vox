---
phase: 318-live-vram-selector-size-02
fixed_at: 2026-10-27T00:00:00Z
review_path: .planning/phases/318-live-vram-selector-size-02/318-REVIEW.md
iteration: 1
findings_in_scope: 16
fixed: 13
skipped: 3
status: partial
---

# Phase 318: Code Review Fix Report

**Fixed at:** 2026-10-27
**Source review:** `.planning/phases/318-live-vram-selector-size-02/318-REVIEW.md`
**Iteration:** 1
**Where verification ran:** the main checkout `/Users/brbrainerd/dev/vox-mens-compare`, on branch `gsd/mens-compare-316`. No worktree was used. Every check below can be reproduced from that tree.

**Summary:**
- Findings in scope: 16 (3 critical, 6 warning, 7 info; scope "all")
- Fixed: 13
- Skipped or deferred: 3 (IN-05 deferred by design, IN-06 deferred, IN-07 needs no action)

Each fix followed the same order. I wrote the test first and watched it fail against a stub that kept the old behavior. Then I made the fix, ran the crate's tests, and formatted with `rustfmt --edition 2024 --config-path <repo>`. Every BLOCKER and WR-05 also has a mutation proof. The proof records the `grep -c` of the guard before the change (1), after mutating it (0, and the named test fails), and after restoring it (1, with a matching `shasum`).

## Fixed Issues

### CR-01: Metal "live" reading silently falls back to `recommendedMaxWorkingSetSize`

**Files modified:** `crates/vox-populi/src/mens/tensor/accel_budget.rs`, `crates/vox-populi/src/mens/tensor/size_select.rs`, `crates/vox-populi/src/mens/tensor/memory_model.rs` (test fixture), `crates/vox-populi/src/mens/hardware/macos_metal.rs` (doc)
**Commit:** 81684ff95
**Applied fix:** `AccelBudget` has a new field, `working_set_is_live`. On Metal it is `live_pressure_budget_bytes().is_some()`. On CUDA it is `false`, because that reading can be an override or the total. The selector reads the budget through a new seam, `metal_reading(bytes, live)`, which returns `None` when the reading is not live. A missing or unparseable `vm_stat` is therefore "no live reading", and the selector refuses it (D-01). Other `accel_budget` consumers still get the same `working_set_bytes`.
**Test:** `size_select::tests::metal_working_set_without_vm_stat_is_no_reading`. It failed against the stub and passes now.
**Mutation proof** (guard `    if !live {` in `size_select.rs`):
- Before the change: `grep -c` = 1, sha `85edf78c0a2c57d87b710e9cf3f051f3c38d9f36`.
- Mutated (guard replaced by `let _ = live;`): `grep -c` = 0. Result: `1 test run: 0 passed, 1 failed`.
- Restored: `grep -c` = 1, same sha (match = yes).

### CR-02: Non-deterministic auto pick plus base-blind checkpoint resume

**Files modified:** `crates/vox-plugin-mens-candle-core/src/checkpoint_state.rs`, `.../candle_qlora_train/training_loop/checkpoint.rs`, `.../candle_qlora_train/epoch_boundary.rs`, and in both `vox-plugin-mens-candle-metal` and `vox-plugin-mens-candle-cuda`: `candle_qlora_train/checkpoint_mid.rs` and `candle_qlora_train/training_loop/mod.rs`
**Commit:** 43275581d
**Applied fix:**
- `CheckpointState` has a new `base_model` field (`#[serde(default)]`). It holds the `id@revision` from `LoraTrainingConfig::base_model`, and every writer sets it.
- `apply_checkpoint_resume` refuses when the recorded base differs from the run's base. The message names both pins and suggests `--model <recorded>` or `--force-restart`.
- An old checkpoint with no recorded base still resumes, with a warning.
- An adapter that exists but fails to load now refuses instead of warning. Before, the run kept the old epoch, step and pair offset with fresh weights.
- `cargo check` is clean for `vox-plugin-mens-candle-metal` and for `vox-plugin-mens-candle-cuda`.

**Tests:** `refuses_to_resume_onto_a_different_base`, `checkpoint_without_a_recorded_base_still_resumes` and `refuses_to_keep_counters_when_the_adapter_fails_to_load`. `roundtrip_save_load` and `load_pre_fingerprint_checkpoint_defaults_to_none` now also assert `base_model`. Before the fix, 2 of these failed.
**Mutation proofs** (file sha `53a8dcd4cf4e83b9a0caaf39553068da8742e23a` in both):
- Base guard `            && config.base_model.as_deref() != Some(was)`, mutated to `&& false`. Counts: 1, then 0, then 1. Result: `refuses_to_resume_onto_a_different_base` 0 passed, 1 failed. Restored sha matches.
- Adapter-load guard `refusing to continue from`; the `bail!` was replaced by the old `train_log::warn`. Counts: 1, then 0, then 1. Result: `refuses_to_keep_counters_when_the_adapter_fails_to_load` 0 passed, 1 failed. Restored sha matches.

**Not changed:** if the adapter file is *missing*, resume still warns and continues with fresh weights while keeping the counters. That is the same hazard class, but the brief named only "fails to load", and the existing fingerprint tests depend on the missing-adapter path. I left it as is.

### CR-03: The pick never reaches `train_arm`; local cards say `base_rung: "local"`

**Files modified:** `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`, `crates/vox-ml-cli/src/commands/schola/train/gpu.rs`, `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`
**Commit:** 643226cd7
**Applied fix:**
- `run_train` and `run_gpu_training` now return `Result<Option<SizePick>>`, carrying the final pick after the D-05 re-check.
- `train_arm` writes the card's preset as `trained_preset(cli_preset, pick)`: an explicit `--preset` if one was given, otherwise the pick's preset. That value is both `base_rung` and `preset_version`.
- The `dispatch.rs` and `pipeline.rs` callers already discarded the value with `.await?`, so they needed no change.

**Test:** `local_card_tests::local_card_records_the_auto_pick_preset_as_base_rung`. It takes an auto pick at 21000 MB (14B), writes a card, and asserts `base_rung == preset_version == "qwen3_24g"`. It also checks that an explicit preset wins. It failed against the stub.
**Mutation proof** (guard `    cli_preset.or_else(|| pick.map(|p| p.rung.preset.to_string()))`):
- Before the change: 1, sha `c619ed74643dd7ad78cdd90fd58946486000efc9`.
- Mutated to `let _ = pick; cli_preset`: 0. Result: 0 passed, 1 failed.
- Restored: 1, sha matches.

**Coverage gap:** `Ok(final_pick)` at the end of `run_gpu_training` is not unit-tested, because reaching it needs a full training run.

### WR-01: `vox chat --model auto-local` sends an HF base pin no local server accepts

**Files modified:** `docs/src/how-to/how-to-train-mens-macos-metal.md`
**Commit:** 48275b098
**Applied fix:** I corrected the docs only. The how-to now says auto-local and the GUI recommendation are advice only. It explains why a chat turn cannot succeed: the base pin is sent to `VOX_LOCAL_ENDPOINT`, which defaults to Ollama's `:11434`, and serve answers 409 for any name but its checkpoint stem. It also shows the path that does work: set `VOX_LOCAL_ENDPOINT` to the serve port and run `vox chat --model mens/<id from /v1/models>`.
**Why not the serve change:** making serve accept its recorded base pin would not fix the default endpoint. auto-local would still reach Ollama, so that change would not have been a fix on its own.

### WR-02: Training and inference read different GPUs; NVML `0` blocks the fallback

**Files modified:** `crates/vox-populi/src/mens/tensor/size_select.rs`, `crates/vox-populi/src/mens/tensor/vram_autodetect.rs`, `crates/vox-orchestrator/src/models/auto_select.rs`, `crates/vox-orchestrator/src/models/vram.rs`
**Commit:** 40664bf7d
**Applied fix:** training and chat now share one function, `size_select::cuda_free_memory(&[(index, free_mib)])`.
- It keeps the devices that `CUDA_VISIBLE_DEVICES` names.
- It matches those entries by index only under `CUDA_DEVICE_ORDER=PCI_BUS_ID`, where the indices line up with nvidia-smi and NVML. Without that setting, or with UUID entries, it counts every device, which can never exceed the visible minimum.
- It takes the minimum, treats 0 as no reading, and applies the x0.88 margin.
- Training uses a new per-device nvidia-smi query, `query_nvidia_smi_free_mb`. One unreadable row such as `[N/A]` makes the whole reading `None`.
- The orchestrator parses NVML's `index` field (`parse_device_free_mb`). `free_vram_mb_from_probe` keeps returning the minimum over all devices for its other consumers.

**Tests:**
- `cuda_free_memory_is_the_visible_minimum`: failed against the stub.
- `nvidia_smi_free_reads_every_gpu`.
- `a_zero_probe_reading_falls_through_to_the_next_source` (orchestrator): fails if the old minimum-without-zero-filter path is put back. I checked this with a non-required mutation: 0 passed, 1 failed, then restored with a matching sha.
- The renamed `parse_device_free_mb_*` tests.

**Not compiled locally:** the non-macOS branch of `read_free_memory`. This Mac has no Linux target installed, so hosted CI will compile that branch first.

### WR-03: Bare concrete ids without `/` are a hard error in training

**Files modified:** `crates/vox-populi/src/mens/tensor/size_select.rs`
**Commit:** c470d1426
**Applied fix:** when an explicit id does not resolve, it now passes through unless it is shaped like a ladder size name: `Qwen3-<n>B`, with an optional `owner/` and `@rev`. An existing path always passes through. So `gpt2`, `my-local-dir`, an existing relative dir and `Qwen/Qwen3-Coder-30B-A3B-Instruct` all pass through.

Size-shaped typos (`Qwen3-1234B`, `Qwen3-8b`, `Qwen3-8B@main`, `Qwen/Qwen3-8B@main`) are still refused before the download. This narrows the brief's "reuse serve's special case". Serve passes *every* unresolvable name through, and copying that literally would have deleted the typo guard and its test. Serve's own pre-check is unchanged.
**Tests:** `concrete_ids_off_the_ladder_pass_through` gained `gpt2`, `my-local-dir`, `src` and the Coder repo, and it failed before the fix. `unknown_bare_size_name_is_an_error` gained the size-shaped typos.

### WR-04: Serve refuses runs that previously served, with no override

**Files modified:** `crates/vox-ml-cli/src/commands/ai/serve/mod.rs`, `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`, `crates/vox-ml-cli/src/commands/schola/train/mod.rs`, `crates/vox-populi/src/mens/tensor/size_select.rs` (refusal text), `docs/src/how-to/how-to-train-mens-macos-metal.md`
**Commit:** ace4fe409
**Applied fix:** `serve_fit_note` takes a `force` argument. Serve passes training's own `force_train_env()`, which is now re-exported as `pub(crate)`. That is the existing `VOX_MENS_FORCE_TRAIN` override; no new flag or env var was added. The refusal text and the how-to now say the override applies to both training and serve.
**Tests:** `serve_fit_note_forced_serves_a_base_that_does_not_fit` covers the forced arm, and it failed against the stub. `serve_fit_note_refuses_a_ladder_base_that_does_not_fit` covers the refusing arm.
**Left alone:** the registry description in `contracts/config/registry.v1.yaml` still says "training". Changing it would regenerate `config-keys.generated.md`.

### WR-05: Step-down wiring in `gpu.rs` and `--model` precedence were untested

**Files modified:** `crates/vox-ml-cli/src/commands/schola/train/gpu.rs`, `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`
**Commit:** d3e60db79
**Applied fix:** two extractions.
- `adopt_size_pick(pick, &mut model, &mut preset)`: the pick's pin becomes the model, and its preset fills an unset `--preset`. `VOX_BASE_MODEL` is now set from the adopted model.
- `explicit_model(cli, env)`: `--model` beats `VOX_MENS_DEFAULT_MODEL`, and a blank env value counts as unset.

**Tests:**
- `size_pick_tests::a_stepped_down_pick_replaces_the_model_and_fills_the_preset`: a 14B auto pick re-checked at 13000 MB gives the 8B pin and `qwen3_16g`.
- `size_pick_tests::an_explicit_preset_survives_the_pick`.
- `run_train::tests::cli_model_beats_the_default_model_env`.
- All three failed against stubs.

**Mutation proofs:**
- `gpu.rs` guard `    *model = Some(pick.rung.hf_id.clone());`, mutated to `let _ = model;`. Counts: 1, then 0, then 1. Result: 0 passed, 1 failed. sha `c1e7288ae291ff0a51f1ba956b32db71bf4ae280` matches after restore.
- `run_train.rs` guard `    cli.or_else(|| env.map(|s| s.trim().to_string()).filter(|s| !s.is_empty()))`, mutated to `env.or(cli)`. Counts: 1, then 0, then 1. Result: 0 passed, 1 failed. sha `92039e38ecbfdc0db305a958c96fffe0dc60d88e` matches after restore.

### WR-06: Chat and the GUI judge against the embedded ladder

**Files modified:** `crates/vox-orchestrator/src/models/auto_select.rs`
**Commit:** 0f0744642
**Applied fix:** `select_for_free_memory` now loads the overlay through the existing `load_ladder_overlay`. It passes `vox_repository::find_cargo_workspace_root()`, the same function training's `find_workspace_root` wraps. `load_overlay` keeps the embedded fallback when the file is absent. The GUI goes through `select_optimal_local_model_for_host`, so it is covered too. No crate edge was added: vox-orchestrator already depends on vox-repository.
**Test:** `auto_selection_reads_the_workspace_ladder`. A temp workspace raises the 8B floor to 14000, so 13000 MB picks 4B. A missing file falls back to embedded, which picks 8B. The test failed against the stub.

### IN-01: A re-check that keeps the pick prints the stale first note

**Commit:** 6e5dea8d3 (`size_select.rs`)
**Applied fix:** the keep path now writes a fresh note: `"{size} still fits: floor {floor} MB, {free} MB free now ({source})"`. `recheck_keeps_an_auto_pick_that_still_fits` asserts it and failed first.

### IN-02: The selector module still exports a default-size path

**Commit:** 40d84b57c (`size_select.rs`, `ai/serve/mod.rs`, `mens/reasoning_compare.rs`, `reasoning_compare/detect.rs`)
**Applied fix:** `resolve_base_pin` now takes `&str`. `reasoning_compare` applies the `DEFAULT_MODEL_ID` default for an omitted `--base` itself. The tests were adapted. This is a pure refactor, covered by the existing resolution tests.

### IN-03: Another revision of a ladder size skips the floor check

**Commit:** 22c3493ae (`size_select.rs`)
**Applied fix:** rungs are matched by repo id. The requested revision is kept, and the size's floor and preset apply. `another_revision_of_a_ladder_size_is_still_a_size` failed first.

### IN-04: The e2e mock's `reason` does not match the real note format

**Commit:** 03aad2702 (`crates/vox-gui/ui/e2e/lib/tauriMock.ts`)
**Applied fix:** the mock now uses the exact `select_size` note. `vitest run e2e/lib/tauriMock.families.test.ts` gives 4 passed.

## Skipped Issues

### IN-05: Hub and spokes in separate invocations can land on different sizes

**File:** `crates/vox-ml-cli/src/commands/mens/training_selection.rs:56-62`
**Reason:** deferred by design. Persisting the hub size is deferred in CONTEXT. The suggested warning needs a recorded hub base, and none exists yet. CR-03 now makes each card record its own rung, so a size mismatch is at least visible.
**Original issue:** each invocation makes its own live pick, so a hub and its spokes can train on different bases.

### IN-06: The hardware-dependent tests only exercise the refusal branch on CI

**File:** `crates/vox-cli/src/commands/chat.rs:675-686`, `crates/vox-gui/src/commands/models_host.rs:95-105`, `crates/vox-populi/src/mens/tensor/size_select.rs`
**Reason:** deferred. The success shape is already asserted through the injectable path at every layer: `select_size` (`auto_pick_is_the_largest_size_that_fits`) and the orchestrator (`auto_selection_picks_the_largest_fitting_pin`, which checks pin, size, floor and free). The vox-cli and vox-gui wrappers only forward that value. Duplicating the assertion there would mean building vox-cli (CONTEXT notes it does not compile on this branch) and vox-gui (which needs a sidecar). The live tests stay as smoke tests.
**Original issue:** on GPU-less runners only the `Err` and `None` arms run.

### IN-07: An out-of-phase commit is in the reviewed range

**File:** `crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs` (commit `371c198fe`)
**Reason:** no action for this phase, as the review states. Keep that commit out of the phase PR, or call it out in the PR description.

## Verification

Per-crate lib suites, run with `cargo nextest` in the main checkout after the last fix:

| Crate | Command | Result |
|---|---|---|
| vox-populi | `--package vox-populi --features mens-train --lib` | 381 passed |
| vox-plugin-mens-candle-core | `--package vox-plugin-mens-candle-core --lib` | 79 passed |
| vox-ml-cli | `--package vox-ml-cli --features cloud --lib` | 390 passed |
| vox-orchestrator | `--package vox-orchestrator --lib` | 1409 passed, 2 failed (see below) |
| vox-plugin-mens-candle-metal / -cuda | `cargo check --lib` | clean |
| vox-ml-cli (default features) | `cargo check --lib` | clean |
| vox-gui ui | `vitest run e2e/lib/tauriMock.families.test.ts` | 4 passed |

`cargo clippy` on vox-orchestrator and on vox-populi (`--features mens-train`) found nothing in the touched files.

**The two vox-orchestrator failures are not caused by these fixes:**
- `models::tests::premium_alias_tests::default_premium_alias_targets_exist_in_models_list` fails with "premium_alias planning -> ~openai/gpt-pro-latest not in default models list".
- `models::select::tests::select_with_premium_alias_honors_alias_when_intelligence_high` fails with "expected PremiumAlias, got Scored".

Both concern cloud premium-alias routing config. No fix commit touches that code; these fixes only change `auto_select.rs` and `vram.rs` in that crate.

---

_Fixed: 2026-10-27_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
