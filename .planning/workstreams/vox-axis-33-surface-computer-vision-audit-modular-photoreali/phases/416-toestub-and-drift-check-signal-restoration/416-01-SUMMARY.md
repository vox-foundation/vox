# Phase 416 Plan 01 Summary: `toestub` & `vox-drift-check` Signal Restoration

**Completed:** 2026-11-13
**Requirements Satisfied:** `HYGIENE-AUDIT-01`, `HYGIENE-AUDIT-02`, `HYGIENE-AUDIT-03`, `HYGIENE-AUDIT-04`
**Status:** Complete & Verified (`exit 0`)

## Delivered Changes

1. **`arch/workspace_drift` Precision & Root `Cargo.toml` Inheritance (`HYGIENE-AUDIT-01`, `HYGIENE-AUDIT-02`):**
   - Updated `crates/vox-code-audit/src/detectors/workspace_drift.rs` to skip generated/external manifests (`crates/workspace-hack/Cargo.toml`, `**/tests/fixtures/**`, `**/fuzz/Cargo.toml` / `cargo-fuzz = true`, `patches/`), exempt Cargo `default-features = false` path overrides, avoid matching `[package]` `rust-version = "..."`, and honor `members = ["crates/*"]` glob membership on root `Cargo.toml` orphan scanning.
   - Updated `crates/vox-code-audit/src/detectors/crypto_ban.rs` to skip `[patch.*]` TOML sections so `[patch.crates-io]` `aegis` pure-Rust MSVC override in root `Cargo.toml` is not misflagged as an application dependency.
   - Registered 5 missing internal library paths (`vox-term`, `vox-terminal-core`, `vox-runtime`, `vox-journal`, `vox-plugin-mens-candle-core`) and 9 locked external crates (`askama`, `objc2-metal`, `rcgen`, `rusqlite`, `sqlx`, `tauri-build`, `tauri-plugin-dialog`, `tauri-plugin-shell`, `uniffi`) in root `[workspace.dependencies]` and migrated 25 sub-crate `Cargo.toml` manifests to `{ workspace = true }` inheritance without changing `Cargo.lock` (`cargo build --locked` passed).

2. **All Non-`god_object` `toestub` Errors Eliminated (`HYGIENE-AUDIT-03`):**
   - `crates/vox-code-audit/src/detectors/db_persistence_policy.rs`: added `!rust_byte_is_non_code(...) && !rust_byte_is_cfg_test(...)` guard so detector test string literals do not self-trigger.
   - `crates/vox-code-audit/src/detectors/unregistered_llm_env.rs`: added `"HUGGINGFACE_HUB_CACHE"` alongside `"HF_HUB_CACHE" | "HF_HOME"`.
   - `crates/vox-code-audit/src/detectors/stub.rs`: exempted PascalCase enum variants (`Stub,`) and qualified paths (`::Stub`) while preserving lowercase `let stub = ...` and `STUB` detection.
   - `crates/vox-ml-cli/src/commands/ai/train.rs`: interpolated `{TOGETHER_FINE_TUNES}/{job_id}` instead of repeating a raw provider hostname.
   - `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` and `crates/vox-populi/src/lib.rs`: replaced stale `TODO` comments with factual scope/architecture notes.
   - `crates/vox-orchestrator-mcp/src/chat_tools/chat/agent_loop/zero_paper_and_ratchet.rs`: aligned `"postgres://"` / `"sqlite://"` detector pattern strings with `concat!(...)` matching line 233.

3. **Zero `vox-drift-check` Errors & Warnings (`HYGIENE-AUDIT-04`):**
   - `crates/vox-drift-check/src/engine.rs`: added `resolve_workspace_root(&self.root)` so subdirectory runs (`vox-drift-check crates`) load `docs/src/architecture/layers.toml` (`sibling_of`) and root workspace metadata identically to `.`.
   - `crates/vox-code-audit/src/review/client.rs`: extracted `REVIEW_REQUEST_TIMEOUT` (`120s`) and documented standalone `_public.toml` `drift-allow(reqwest-bypass)` rationale.
   - `crates/vox-config/src/paths.rs`, `crates/vox-cli/src/commands/ci/pre_push.rs`, `crates/vox-compiler/tests/py_frame_report_run.rs`, `scripts/worktree-setup.vox`, `scripts/fmt.vox`: centralized `.vox/` path constants in `vox_config::paths` and annotated `.vox` scripts.
   - `crates/vox-orchestrator-mcp/src/gsd_tools.rs`, `crates/vox-gui/src/commands/mens_run_reports.rs`, `crates/vox-corpus/src/training/spoke_roundtrip/organic_types.rs`: consolidated `default_true` helpers onto `vox_config::serde_defaults::default_true`.
   - Extracted/routed `Duration` timeout constants across `builtins.rs`, `python.rs`, `workflow/run.rs`, `rust_bench.rs`, `daemon_steps/run.rs`, `tls.rs`, `barriers.rs`, `train_record.rs`, and `vox-python-bridge/src/{lib.rs,host.rs}`.

## Verification Receipts

- `./target/debug/vox-drift-check . --severity warning` → `✓ No drift findings at Warning level or above.` (`exit 0`, down from `37` findings).
- `./target/debug/vox-drift-check crates --severity warning` → `✓ No drift findings at Warning level or above.` (`exit 0`, down from `41` findings).
- `./target/debug/toestub Cargo.toml --min-severity error` → `✅ No issues found — code is clean!` (`exit 0`, down from `20` errors).
- `./target/debug/toestub crates --min-severity error --rules arch/workspace_drift` → `✅ No issues found — code is clean!` (`exit 0`, down from `69` errors).
- `./target/debug/toestub crates --min-severity error` → `277` total errors, `277/277` `arch/god_object`, **`0` non-`god_object` errors**.
- `./target/debug/vox-arch-check` → `exit 0`.
- `./target/debug/vox ci crate-edges` → `crate-edges: OK (699 live in-tree edges within baseline)` (`exit 0`).
- `cargo test --locked -p vox-code-audit --lib` → `486 passed; 0 failed` (`exit 0`).
- `cargo test --locked -p vox-drift-check --lib` → `73 passed; 0 failed` (`exit 0`).
- `cargo test --locked -p vox-test-harness -p vox-python-bridge --lib` → `15 passed` + `13 passed`; `0 failed` (`exit 0`).
