---
phase: 318-live-vram-selector-size-02
plan: phase
subsystem: training
tags: [qwen3, qlora, vram, SIZE-02]
status: complete
completed: 2026-10-29
requirements-completed: [SIZE-02]
---

# Phase 318: Live VRAM selector (SIZE-02)

**One measured qwen3_code QLoRA size is chosen from accelerator free memory. Explicit sizes that do not fit are refused. The old directory-tier map is gone.**

## Commits

| Hash | Plan | Subject |
| --- | --- | --- |
| `5077c9ea5` | 01 | pick the largest measured QLoRA size that fits |
| `59e037472` | 01 | refuse unmeasured and over-floor Qwen3 sizes |
| `ba5e86db0` | 01 | adapt CUDA free and Metal free into one reading |
| `26b058797` | 02 | use one measured QLoRA size for every spoke |
| `0e4a5517a` | 02 | read accelerator free memory instead of inventing a VRAM total |
| `839bffed0` | 02 | re-read free memory before schola loads a Qwen3 size |
| `74ddd3e13` | 03 | pick a measured QLoRA pin from an injected free-memory reading |
| `9bd278410` | 03 | drop the directory-tier ladder and fail closed on a live read |
| `00d75032f` | 03 | use the injected selector for chat auto-local and the GUI host |

## Tests

| Command | Result |
| --- | --- |
| `cargo test -p vox-populi --lib spoke_base_resolver` | 0 tests, exit 0. Not a pass. |
| `cargo test -p vox-populi --lib spoke_base_resolver --features mens -- --test-threads=8` | 36 passed |
| `cargo test -p vox-ml-cli --lib training_selection -- --test-threads=8` | 6 passed |
| `cargo test -p vox-ml-cli --lib resolve_local_spoke_base_does_not_invent_a_vram_reading` | 0 tests. Module is `cfg(feature = "gpu")`. |
| `cargo test -p vox-ml-cli --lib resolve_local_spoke_base_does_not_invent_a_vram_reading --features gpu -- --test-threads=8` | 1 passed |
| `cargo test -p vox-ml-cli --lib apply_preload_free_mb` | 0 tests. Schola train needs `gpu`. |
| `cargo test -p vox-ml-cli --lib apply_preload_free_mb --features gpu -- --test-threads=8` | 4 passed |
| `cargo test -p vox-ml-cli --lib run_train_checks_preload_before_select_mens_backend --features gpu -- --test-threads=8` | 1 passed |
| `cargo test -p vox-orchestrator --lib select_from_free_mb -- --test-threads=8` | 1 passed |
| `cargo test -p vox-orchestrator --lib auto_select -- --test-threads=8` | 3 passed |
| `cargo test -p vox-cli --lib chat_routing_ -- --test-threads=8` | 2 passed |
| `cargo test -p vox-gui --lib models_host` | exit 101, no library target |
| `cargo test -p vox-gui --bin vox-gui models_host -- --test-threads=8` | 2 passed |

## Deviations

- Selector unit tests compile only with `vox-populi` feature `mens`. Spoke and preload tests compile only with `vox-ml-cli` feature `gpu`.
- No separate red commits. Each task commit contains the test and the implementation (`tdd_mode` is false).
- Empty `hw-probe-live-test` feature on `vox-ml-cli` so the two existing live `train_arm` tests stay out of the default suite.
- `vox-gui` tests ran on the binary target.
- `1.7B` and `4B` floors were not invented. `small_code_default` and `strong_code_default` were not edited. No new crate edge. No weight download, Hugging Face call, GPU probe from the new unit tests, training run, or mesh change.
- STATE.md and ROADMAP.md were not edited. Phase was not marked complete (`--no-transition`).
