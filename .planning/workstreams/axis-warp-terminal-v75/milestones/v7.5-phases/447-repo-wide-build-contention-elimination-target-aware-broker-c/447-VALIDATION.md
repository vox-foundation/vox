---
phase: "447"
slug: "repo-wide-build-contention-elimination-target-aware-broker-c"
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-11-14"
---

# Phase 447 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (`vox-build-queue`, standalone `vox-cargo-shim` excluded workspace, `vox-orchestrator`, `vox-terminal-core`) + `vox-doc-pipeline` + `vox-arch-check` |
| **Config file** | `crates/vox-build-queue/Cargo.toml`, `crates/vox-cargo-shim/Cargo.toml` (standalone excluded crate), `crates/vox-orchestrator/Cargo.toml`, `crates/vox-terminal-core/Cargo.toml` |
| **Quick run command** | `cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml` |
| **Full suite command** | `cargo test -p vox-build-queue` + `cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml` + `cargo test -p vox-orchestrator --no-default-features --lib oversight` + `cargo test -p vox-terminal-core --lib` |
| **Estimated runtime** | ~20 seconds (`vox-cargo-shim` uses its own standalone `crates/vox-cargo-shim/target`!) |

---

## Sampling Rate

- **After every task:** Run the task's scoped `cargo test` command (`vox-cargo-shim` standalone manifest or `-p <crate>`)
- **After every plan wave:** Run wave-touched crate tests + `vox-doc-pipeline --lint-only` (when docs touched) + `vox-arch-check`
- **Before `/gsd-verify-work`:** Full Phase 447 suite + `vox ci dev-loop-guard` + `vox-arch-check` green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 447-01-01 | 01 | 1 | BUILD-CONTENTION-01 | T-447-01, T-447-02 | Target-first `FairQueue` ordering before global slot acquisition, Windows-safe zero-byte `.lock` leader-follower coalescing (`coalesce.rs`), and backward-compatible `target_wait_ms`/`slot_wait_ms`/`compile_ms` metrics | unit | `cargo test -p vox-build-queue` | ✅ | ✅ green |
| 447-01-02 | 01 | 1 | BUILD-CONTENTION-01, BUILD-CONTENTION-03, BUILD-CONTENTION-04 | T-447-02, T-447-03 | `vox-cargo-shim` target-first queue + coalescing + auto-`sccache` + `VOX_WORKSTREAM`/`VOX_ISOLATED_TARGET` + post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` (`exit 124` process-group kill) + `vox_broker_readonly` parity | unit + integration | `cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml` | ✅ | ✅ green |
| 447-02-01 | 02 | 1 | BUILD-CONTENTION-02 | T-447-04 | `vox-orchestrator` compiles & passes `--no-default-features` and `--no-default-features --features runtime` (`victory.rs`, `dei_dispatch.rs`, `route_policy.rs` fail-closed env fallback, `healing.rs`, integration test gates) | unit + compile | `cargo test -p vox-orchestrator --no-default-features --lib oversight` | ✅ | ✅ green |
| 447-02-02 | 02 | 1 | BUILD-CONTENTION-02 | T-447-04 | `vox-terminal-core` gates `vox-compiler` (`47k LoC`) behind `vox-lang` (preserving `vox_interp.rs` `caps.rs` audit contract) and uses `vox-orchestrator` `default-features = false` | unit + compile | `cargo test -p vox-terminal-core --lib` | ✅ | ✅ green |
| 447-03-01 | 03 | 2 | BUILD-CONTENTION-03, BUILD-CONTENTION-04 | T-447-03 | `dev-loop-guard` + `.agents/cargo-safety.md` + `docs/src/contributors/build-broker-usage.md` + `scripts/broker-install.vox` updated and doc-lint clean | contract + doc-lint | `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/contributors/build-broker-usage.md` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/vox-cargo-shim` is an excluded standalone crate (`crates/vox-cargo-shim/target` — zero lock contention with workspace `target/.cargo-lock`!), and all target crates already have unit/integration test harnesses in place.

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved

---

## Validation Audit 2026-11-15

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 5 |
| Escalated | 0 |

- **`447-01-01` (`vox-build-queue`):** `43 passed; 0 failed` + `clippy -D warnings` `0` warnings (`coalesce.rs`, `mtime.rs`, `queue.rs`, `resolve.rs`, `metrics.rs`).
- **`447-01-02` (`vox-cargo-shim` standalone):** `30 passed; 0 failed` (`4` bin `cargo`, `15` bin `vox-broker`, `4` integration `broker_coalesce_and_timeout`, `2` integration `passthrough`, `1` integration `queue_serializes`, `4` integration `vox_broker_readonly`) + `clippy -D warnings` `0` warnings.
- **`447-02-01` (`vox-orchestrator` minimal feature matrix):** `cargo test -p vox-orchestrator --no-default-features --lib --tests` (`exit 0`, 100% unit + integration binaries passed) + `--no-default-features --features runtime` compile-clean.
- **`447-02-02` (`vox-terminal-core` + `vox-compiler` caps contract):** `cargo test -p vox-terminal-core --lib --tests` (`67/67` unit + 10 integration binaries) + `--features vox-terminal-core/vox-lang --lib` (`exit 0`) + `cargo test -p vox-compiler --lib eval::caps::tests::production_embedders_assign_caps_explicitly` (`1 passed; 0 failed`).
- **`447-03-01` (`vox-cli-ci` + docs + `.vox` scripts + arch):** `cargo test -p vox-cli-ci --lib dev_loop_guard` (`9 passed; 0 failed`), `cargo test -p vox-cli-ci --lib crate_api_docs` (`4 passed; 0 failed`), `./target/debug/vox-doc-pipeline --lint-only --paths contributors/build-broker-usage.md` (`exit 0`), `./target/debug/vox check scripts/broker-install.vox` & `scripts/broker-ci.vox` (`exit 0`), and `./target/debug/vox-arch-check` (`exit 0`).
