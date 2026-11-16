# Phase 416: `toestub` & `vox-drift-check` Signal Restoration — Context

**Gathered:** 2026-11-13
**Status:** Ready for execution
**Mode:** Autonomous

<domain>
## Phase Boundary

Restore high-signal static analysis across `toestub` (`vox-code-audit`) and `vox-drift-check` (`HYGIENE-AUDIT-01..04`):
1. Eliminate all `624` `arch/workspace_drift` errors across `crates/` and all `20` root `Cargo.toml` `toestub` errors (`19` false-positive orphan alerts under glob `members = ["crates/*"]` + `1` `[patch.crates-io]` `aegis` match) without altering `Cargo.lock`.
2. Resolve all `10` non-`arch/god_object` `toestub` errors across `crates/` (`db/non-turso-persistence`, `vox/llm/direct-provider-call`, `vox/llm/unregistered-env`, `magic-value/db-conn`, `stub/todo-comment`, `stub/placeholder`).
3. Fix `vox-drift-check` workspace-root discovery when invoked on a subdirectory (`vox-drift-check crates`) and resolve all `33` live `vox-drift-check` warnings across `.` and `crates/`.

</domain>

<decisions>
## Implementation Decisions

- **Workspace-Drift Accuracy + Manifest Cleanup (`HYGIENE-AUDIT-01`, `HYGIENE-AUDIT-02`):**
  - Update `crates/vox-code-audit/src/detectors/workspace_drift.rs` to skip generated `crates/workspace-hack/Cargo.toml` (`cargo hakari`), `**/tests/fixtures/**/Cargo.toml`, standalone `cargo-fuzz` workspaces (`fuzz/Cargo.toml`), and vendored `patches/`. Honor glob workspace membership (`members = ["crates/*"`), and avoid misclassifying `rust-version = "..."` as a dependency version.
  - Update `crates/vox-code-audit/src/detectors/crypto_ban.rs` so `[patch.*]` sections in `Cargo.toml` (transitive pure-Rust MSVC build overrides) are not misflagged as direct `[dependencies]`.
  - Register missing shared workspace crates and existing pinned external dependencies in root `Cargo.toml` `[workspace.dependencies]` and migrate the 27 member `Cargo.toml` files to `{ workspace = true }` / `.workspace = true` with zero `Cargo.lock` changes.
- **Zero Non-`god_object` `toestub` Errors (`HYGIENE-AUDIT-03`):**
  - Teach `DbPersistencePolicyDetector` to respect `rust_byte_is_non_code` / `rust_byte_is_cfg_test` when `rust_ctx` is present.
  - Recognize `HUGGINGFACE_HUB_CACHE` alongside `HF_HUB_CACHE` and `HF_HOME` as standard Hugging Face cache directory layout variables in `UnregisteredLlmEnvDetector`.
  - Exempt PascalCase enum variant declarations (`Stub,`) and qualified variant paths (`::Stub`) in `StubDetector::bare_stub_word_not_stub_check`.
  - Clean up `train.rs` (`TOGETHER_FINE_TUNES`), `train_arm.rs` / `vox-populi/src/lib.rs` comments, and `zero_paper_and_ratchet.rs` detector literal patterns.
- **Zero `vox-drift-check` Warnings (`HYGIENE-AUDIT-04`):**
  - Walk upward from `self.root` in `DriftEngine` (`crates/vox-drift-check/src/engine.rs`) to find the enclosing workspace root (`Cargo.toml` with `[workspace]`) so `LayersManifest` (`sibling_of`), `FeatureCache`, and `workspace_version` work identically for `vox-drift-check .` and `vox-drift-check crates`.
  - Add `REPO_SALVAGE_DIR_PREFIX`, `REPO_PYTHON_PYPROJECT_FILE`, and `REPO_PYTHON_VENV_BIN` in `crates/vox-config/src/paths.rs`, consolidate duplicate `serde` defaults, and replace inline timeout literals with `vox_config::timeouts::*` or module `const`s.

</decisions>
