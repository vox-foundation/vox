---
phase: 318-live-vram-selector-size-02
fixed_at: 2026-10-29T21:50:18Z
review_path: .planning/phases/318-live-vram-selector-size-02/318-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 318: Code Review Fix Report

**Fixed at:** 2026-10-29T21:50:18Z
**Source review:** `.planning/phases/318-live-vram-selector-size-02/318-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 5
- Fixed: 5
- Skipped: 0

Verification ran in the main checkout (`workflow.use_worktrees` is false). Cargo went through PATH `cargo`. rustfmt used `--edition 2024` on the touched files only.

## Fixed Issues

### CR-01: env pin wins over the live pick

**Files modified:** `crates/vox-ml-cli/src/commands/mens/training_selection.rs`, `crates/vox-ml-cli/src/commands/mens/pipeline.rs`, `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`, `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`
**Commit:** `9a41951bc`
**Applied fix:** `resolve_training_selection` takes the env pin as an argument. A non-empty pin is used when `--model` is absent, and `automatic_qwen3` is false so preload refuses instead of stepping down. The live pick runs only when both the CLI model and the pin are absent. `env_pin_wins_over_an_injected_free_memory_reading` injects `org/Pinned-Not-Qwen` and 25000 MB and shows the pin wins.

### WR-01: keep caller preset and record the preload revision

**Files modified:** `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`, `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`, `crates/vox-populi/src/mens/tensor/adapter_card.rs`, `crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs`
**Commit:** `94ac0341d`
**Applied fix:** When `confirm_before_load` keeps the same size and the caller preset is non-empty, that preset stays. A real step-down keeps the smaller size's preset. `run_train` returns `Option<PreloadRevision>`, and the local adapter card records `loaded_size` and `loaded_preset`.

### WR-02: read free memory only for a trainable Qwen3 QLoRA size

**Files modified:** `crates/vox-ml-cli/src/commands/mens/training_selection.rs`, `crates/vox-ml-cli/src/commands/mens/pipeline.rs`, `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`
**Commit:** `1574a2bce`
**Applied fix:** `selection_requires_free_reading` is false for RagOnly and PromptOnly. Pipeline and `resolve_local_spoke_base` call `read_accelerator_free_mb` only after that predicate, and only for a QLoRA train whose size is unnamed or an explicit Qwen3 spelling.

### WR-03: step down on the same overlay as the live pick

**Files modified:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`, `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`
**Commit:** `96e057bcc`
**Applied fix:** `select_qwen3_code_qlora_for_free_mb` loads the workspace overlay when a root exists. `confirm_before_load` takes that overlay and steps down with `select_qwen3_code_qlora_from_overlay`. A synthetic overlay test at 2000 MB loads the synthetic 8B row, not the embedded 0.6B row.

### WR-04: require a live reading only for automatic, spelling, or measured rows

**Files modified:** `crates/vox-ml-cli/src/commands/schola/train/run_train.rs`
**Commit:** `e3a27eda7`
**Applied fix:** `model_needs_accelerator_reading` is true only for `automatic_qwen3`, `is_qwen3_size_spelling`, or `measured_qlora_row` on the exact hf id. `org/Qwen3-8B-finetune` does not require a reading.

## Tests

- `cargo test -p vox-ml-cli --lib commands::mens::training_selection::tests` — 9 passed (default `mens-base`; includes the pin and skip-gate tests). An earlier `--exact` filter reported 0 tests and was not treated as a pass.
- `cargo test -p vox-populi --features mens --lib mens::tensor::spoke_base_resolver::tests` — 37 passed. `--features mens` is required; the module is `cfg`'d on it.
- `cargo test -p vox-ml-cli --features gpu --lib commands::schola::train::run_train` — 11 passed. `--features gpu` is required; `run_train.rs` does not compile on the default features.
- `cargo test -p vox-ml-cli --features gpu --lib local_card_tests` — 3 passed, including `local_card_records_loaded_size_and_preset`. `--exact` on the short name reported 0 tests; the module filter is the pass.

## Re-review

Re-reviewed `9a41951bc..e3a27eda7` against `318-CONTEXT.md`. No new critical or warning findings. `318-REVIEW.md` status is `clean`. Iteration cap was 3; this is iteration 1.

---

_Fixed: 2026-10-29T21:50:18Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
