---
phase: 416
phase_name: "toestub-and-drift-check-signal-restoration"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 416 Learnings: toestub-and-drift-check-signal-restoration

## Decisions

### Updated crates/vox-code-audit/src/detectors/workspace_drift.rs to skip g...
Updated `crates/vox-code-audit/src/detectors/workspace_drift.rs` to skip generated/external manifests (`crates/workspace-hack/Cargo.toml`, `**/tests/fixtures/**`, `**/fuzz/Cargo.toml` / `cargo-fuzz = true`, `patches/`), exempt Cargo `default-features = false` path overrides, avoid matching `[package]` `rust-version = "..."`, and honor `members = ["crates/*"]` glob membership on root `Cargo.toml` orphan scanning.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 416-01-SUMMARY.md

---

### Updated crates/vox-code-audit/src/detectors/crypto_ban.rs to skip [patch...
Updated `crates/vox-code-audit/src/detectors/crypto_ban.rs` to skip `[patch.*]` TOML sections so `[patch.crates-io]` `aegis` pure-Rust MSVC override in root `Cargo.toml` is not misflagged as an application dependency.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 416-01-SUMMARY.md

---

### Registered 5 missing internal library paths (vox-term, vox-terminal-core...
Registered 5 missing internal library paths (`vox-term`, `vox-terminal-core`, `vox-runtime`, `vox-journal`, `vox-plugin-mens-candle-core`) and 9 locked external crates (`askama`, `objc2-metal`, `rcgen`, `rusqlite`, `sqlx`, `tauri-build`, `tauri-plugin-dialog`, `tauri-plugin-shell`, `uniffi`) in root `[workspace.dependencies]` and migrated 25 sub-crate `Cargo.toml` manifests to `{ workspace = true }` inheritance without changing `Cargo.lock` (`cargo build --locked` passed).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 416-01-SUMMARY.md

---

### crates/vox-code-audit/src/detectors/db_persistence_policy.rs
`crates/vox-code-audit/src/detectors/db_persistence_policy.rs`: added `!rust_byte_is_non_code(...) && !rust_byte_is_cfg_test(...)` guard so detector test string literals do not self-trigger.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 416-01-SUMMARY.md

---

### crates/vox-code-audit/src/detectors/unregistered_llm_env.rs
`crates/vox-code-audit/src/detectors/unregistered_llm_env.rs`: added `"HUGGINGFACE_HUB_CACHE"` alongside `"HF_HUB_CACHE" | "HF_HOME"`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 416-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

