---
phase: 417
plan: 01
subsystem: test-suite-hardening
tags: [windows-closure, spoke-base-resolver, detect-rs, ignored-test-age, feedback-collector]
requires:
  - HYGIENE-AUDIT-04
provides:
  - HYGIENE-TEST-01
  - HYGIENE-TEST-02
  - HYGIENE-TEST-03
affects:
  - crates/vox-ml-cli/src/commands/mens/mod.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-populi/Cargo.toml
  - crates/vox-populi/src/lib.rs
  - crates/vox-populi/src/mens/mod.rs
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
  - crates/vox-populi/tests/training_presets_yaml_contract.rs
  - crates/vox-actor-runtime/src/lib.rs
  - crates/vox-actor-runtime/src/feedback.rs
  - .planning/WINDOWS.md
tech-stack:
  added: []
  patterns:
    - "Feature-independent contract test shims (`#[cfg(all(test, not(feature = ...)))] #[path = ...]`) for model-free filesystem and YAML validation"
    - "Structured `owner:<domain> sunset:<date|never> <category>: <reason>` metadata on every `#[ignore]` attribute"
key-files:
  created:
    - .planning/phases/417-test-suite-hardening-and-windows-closure/417-01-SUMMARY.md
    - .planning/phases/417-test-suite-hardening-and-windows-closure/417-REVIEW.md
    - .planning/phases/417-test-suite-hardening-and-windows-closure/417-VERIFICATION.md
  modified:
    - crates/vox-ml-cli/src/commands/mens/mod.rs
    - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
    - crates/vox-populi/Cargo.toml
    - crates/vox-populi/src/lib.rs
    - crates/vox-populi/src/mens/mod.rs
    - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
    - crates/vox-populi/tests/training_presets_yaml_contract.rs
    - crates/vox-actor-runtime/src/lib.rs
    - crates/vox-actor-runtime/src/feedback.rs
    - crates/vox-corpus/src/corpus/rust_bench.rs
    - crates/vox-cli/tests/it/eval_diagram_live.rs
    - crates/vox-diagram/tests/freeze_corpus.rs
    - crates/vox-diagram/tests/migration_fidelity.rs
    - crates/vox-diagram/tests/sequence_corpus.rs
    - crates/vox-diagram/tests/svg_corpus.rs
    - crates/vox-populi/src/mens/hub_card/lints_tests.rs
    - crates/vox-populi/src/mens/hub_sync.rs
    - .planning/WINDOWS.md
key-decisions:
  - "Exposed `reasoning_compare/detect.rs` unit tests under default `mens-base` via `#[cfg(all(test, not(feature = \"gpu\"), feature = \"mens-base\"))]` shim in `crates/vox-ml-cli/src/commands/mens/mod.rs` so model-free snapshot/escape tests execute without GPU backends."
  - "Exposed `spoke_base_resolver.rs` unit tests under zero features in `vox-populi` and narrowed `training_presets_yaml_contract.rs` from `mens-train` to `mens`."
  - "Re-wired orphaned `feedback` module (`#[cfg(feature = \"database\")] pub mod feedback;`) in `vox-actor-runtime`, fixed 8-argument `log_interaction` bit-rot, and un-ignored `test_feedback_log_and_thumbs_up_round_trip`."
requirements-completed:
  - HYGIENE-TEST-01
  - HYGIENE-TEST-02
  - HYGIENE-TEST-03
completed: 2026-11-13
---

# Phase 417 Plan 01 Summary: Test Suite Hardening & Open `WINDOWS.md` Closure

## Delivered Outcomes
1. **Closed Window `#1` (`HYGIENE-TEST-01`):** Expanded `snapshot_with_all_indexed_shards_is_present` in `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs` to assert `Detected::Ready { base, snapshot, adapter }` carries `BaseLabel::from_pin(PIN_8B)` and rejects `../escape.safetensors`, `sub/escape.safetensors`, and absolute shard paths as `AbsenceReason::BaseSnapshotIncomplete`. Added a default-feature test shim in `crates/vox-ml-cli/src/commands/mens/mod.rs` (`11 passed; 0 failed` under default features).
2. **Closed Window `#2` (`HYGIENE-TEST-02`):** Decoupled pure `spoke_base_resolver.rs` contract logic from `mens`/`mens-train` tensor backends in `crates/vox-populi`, enabling `29/29` unit tests under `cargo test --locked -p vox-populi --lib spoke_base` (zero features) and `4/4` YAML contract tests under `--features mens`. Updated `.planning/WINDOWS.md` to `open_count: 0`, `fixed_count: 2`.
3. **Zero-Violation `#[ignore]` Governance & Orphan Recovery (`HYGIENE-TEST-03`):** Wired the orphaned `feedback` module in `crates/vox-actor-runtime/src/lib.rs`, repaired its `VoxDb::log_interaction` 8-arg callsite, and un-ignored `test_feedback_log_and_thumbs_up_round_trip` using `tempfile::tempdir()` (`1 passed; 0 failed`). Annotated all remaining bare/unowned `#[ignore]` attributes (`101` scanned, `0` violations under `vox ci ignored-test-age --mode enforce`).
