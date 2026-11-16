# Phase 417 Context: Test Suite Hardening & Open `WINDOWS.md` Closure

**Phase:** 417 (`HYGIENE-TEST-01..03`)
**Goal:** Close both open `.planning/WINDOWS.md` items (`detect.rs` and feature-independent `spoke_base_resolver.rs` / `training_presets_yaml_contract` tests) so `.planning/WINDOWS.md` reaches `open_count: 0`, and resolve all `#[ignore]` governance violations so `vox ci ignored-test-age --mode enforce` exits `0` with zero bare `#[ignore]` attributes across `crates/`.

## Empirical Findings

1. **Window `#1` (`crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs`, `HYGIENE-TEST-01`):**
   - `Detected::Ready` already carries `base: BaseLabel` (verified in `snapshot_with_single_safetensors_is_present`) and `snapshot_complete` rejects non-`[Component::Normal(_)]` shard paths (`../escape.safetensors` tested on lines 359–364).
   - Adding explicit shard-escape assertions for leading `/` (`"/escape.safetensors"`) and nested subdirectory (`"sub/escape.safetensors"`) plus `Detected::Ready` `base` inspection on indexed multi-shard snapshots seals every branch of `snapshot_complete` and `detect_inputs`.

2. **Window `#2` (`crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` & `tests/training_presets_yaml_contract.rs`, `HYGIENE-TEST-02`):**
   - Two harness gaps caused zero tests to run without `--features mens,mens-train`:
     1. Running default `cargo test -p vox-populi --lib spoke_base` ran `0` tests because `pub mod mens` in `crates/vox-populi/src/lib.rs` is gated behind `#[cfg(feature = "mens")]`. Adding `anyhow = { workspace = true }` to `[dev-dependencies]` (already present in `vox-populi`'s `Cargo.lock` entry via optional `[dependencies]`), gating the live `vram_autodetect` fallback branch in `spoke_base_resolver::resolve_base_model` on `#[cfg(feature = "mens")]`, and mounting a `#[cfg(all(test, not(feature = "mens")))]` `#[path = "mens/tensor/spoke_base_resolver.rs"]` test shim in `crates/vox-populi/src/lib.rs` compiles and executes all `19` `spoke_base_resolver` unit tests under plain `cargo test -p vox-populi --lib spoke_base`!
     2. `crates/vox-populi/Cargo.toml` declared `required-features = ["mens"]` for `[[test]] name = "training_presets_yaml_contract"`, yet `crates/vox-populi/tests/training_presets_yaml_contract.rs` had `#![cfg(feature = "mens-train")]` (and `crates/vox-populi/src/mens/mod.rs` only re-exported `KNOWN_PRESETS` under `mens-train`), causing `--features mens` runs to execute `0` contract tests. Re-exporting `KNOWN_PRESETS` directly from `tensor::spoke_base_resolver` under `mens` and switching `training_presets_yaml_contract.rs` to `#![cfg(feature = "mens")]` makes all `4` contract tests execute without `mens-train`.

3. **`#[ignore]` Audit & Governance (`HYGIENE-TEST-03`):**
   - `./target/debug/vox ci ignored-test-age --mode warn` scanned `102` ignored tests across `crates/` and identified `8` governance violations (`1` bare `#[ignore]` in `crates/vox-corpus/src/corpus/rust_bench.rs:502` and `7` `#[ignore = "..."]` attributes missing `owner:`/`sunset:` metadata across `vox-cli`, `vox-diagram`, and `vox-populi`).
   - Additionally, `crates/vox-actor-runtime/src/feedback.rs:195` (`test_feedback_log_and_thumbs_up_round_trip`) was historically ignored for `"requires libsql local feature"` prior to the Turso migration and can be tested/un-ignored under `--features database` (`VoxDb::connect(DbConfig::Memory)`).
