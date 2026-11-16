---
phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c
fixed_at: 2026-11-15T10:02:00Z
review_path: .planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-REVIEW.md
iteration: 1
findings_in_scope: 10
fixed: 10
skipped: 0
status: all_fixed
---

# Phase 447: Code Review Fix Report

**Fixed at:** 2026-11-15T10:02:00Z
**Source review:** `.planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 10 (`2 Critical`, `6 Warning`, `2 Info`)
- Fixed: 10
- Skipped: 0

## Verification Environment & Results

All verification gates ran in the **main checkout** using the standalone isolated target directory `crates/vox-cargo-shim/target` (zero root `target/` contention, zero git mutations, zero `Cargo.toml`/`Cargo.lock` modifications; every `.rs` file verified `< 500` lines and formatted with `rustfmt --edition 2024`):

1. `RUSTC_WRAPPER="" timeout 300s cargo test -p vox-build-queue --target-dir crates/vox-cargo-shim/target` -> **PASS** (`43 passed; 0 failed`)
2. `RUSTC_WRAPPER="" timeout 300s cargo clippy -p vox-build-queue --target-dir crates/vox-cargo-shim/target --all-targets -- -D warnings` -> **PASS** (`0 warnings`)
3. `CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/crates/vox-cargo-shim/target VOX_BROKER_NO_SCCACHE=1 RUSTC_WRAPPER="" timeout 300s cargo test --offline --manifest-path crates/vox-cargo-shim/Cargo.toml --target-dir crates/vox-cargo-shim/target --all-targets` -> **PASS** (`30 passed; 0 failed` across unit + integration suites)
4. `RUSTC_WRAPPER="" timeout 300s cargo clippy --offline --manifest-path crates/vox-cargo-shim/Cargo.toml --target-dir crates/vox-cargo-shim/target --all-targets -- -D warnings` -> **PASS** (`0 warnings`)
5. `./target/debug/vox-doc-pipeline --lint-only --paths contributors/build-broker-usage.md` -> **PASS** (`no hard errors`)

## Fixed Issues

### CR-01: Coalescing key omits `cwd` and `+toolchain` / `RUSTUP_TOOLCHAIN`, causing false-green cross-crate and cross-toolchain coalescing

**Files modified:** `crates/vox-cargo-shim/src/main.rs`, `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed: requires human verification
**Applied fix:** Extracted `compute_coalesce_key` in `crates/vox-cargo-shim/src/main.rs` to hash canonical `cwd` (`resolve::normalize_target_path(cwd, Path::new("."))` -> `queue::hash_path`) and the effective toolchain (`+toolchain` CLI override falling back to `RUSTUP_TOOLCHAIN`). Added unit tests in `main.rs` (`coalesce_key_distinguishes_sibling_cwds_and_toolchains`) and end-to-end integration assertions in `broker_coalesce_and_timeout.rs` confirming `cargo check` in sibling workspace member directories (`crates/demo` vs `crates/other`) and `cargo +nightly check` never coalesce onto one another.

### CR-02: Unbounded post-exit `JoinHandle::join()` on tee threads deadlocks `FairQueue` and leaks global slots when grandchildren inherit stdio pipes

**Files modified:** `crates/vox-cargo-shim/src/runner.rs`, `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed: requires human verification
**Applied fix:** Replaced `Stdio::piped()` + blocking tee threads + unbounded `JoinHandle::join()` in `run_child_post_queue` with direct `Stdio::from(File)` appends to the leader's unlocked `.stdout`/`.stderr` files paired with a non-blocking `tail_stream` polling loop + final post-exit drain. Added unit test `background_grandchild_inheriting_stdio_never_blocks_runner` in `runner.rs` and e2e test `post_queue_timeout_and_grandchild_stdio_never_wedge_fair_queue` in `broker_coalesce_and_timeout.rs` proving a `sleep 10 &` daemon grandchild inheriting stdout/stderr completes in `< 2s` and frees `FairQueue` immediately.

### WR-01: `is_lock_held` uses `try_lock_exclusive()`, causing false-positive leader detection (stale receipt replay) and starving `claim_leader`

**Files modified:** `crates/vox-build-queue/src/coalesce.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed: requires human verification
**Applied fix:** Changed `is_lock_held` to open `<key>.lock` read-only and probe with `FileExt::try_lock_shared()` so concurrent followers never contend with each other; added a bounded 5x5ms retry on `try_lock_exclusive()` in `claim_leader`; enforced `RECEIPT_FRESH_WINDOW_MS` (`receipt_is_fresh`) in both `try_follow_inflight` and `try_replay_recent_leader`; and isolated flight artifacts by per-flight monotonic `run_id` (`<key>.<run_id>.{stdout,stderr,exit}`) while unlinking stale `.meta` before creating a new run so back-to-back runs never overwrite or replay duplicate streams.

### WR-02: `latest_source_mtime_ms` misses file deletions, non-`crates/` workspace inputs, and edits made while waiting in `FairQueue`

**Files modified:** `crates/vox-build-queue/src/mtime.rs` (new submodule, 124 lines), `crates/vox-build-queue/src/lib.rs`, `crates/vox-build-queue/src/coalesce.rs`, `crates/vox-cargo-shim/src/main.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed: requires human verification
**Applied fix:** Extracted `crates/vox-build-queue/src/mtime.rs` (re-exported via `coalesce::latest_source_mtime_ms` to keep `coalesce.rs < 500` lines), recording directory mtimes in `max_mtime_walk` (catching file deletions/renames) and scanning `contracts/`, `.agents/`, `rust-toolchain.toml`, `rust-toolchain`, and `lefthook.yml`. In `crates/vox-cargo-shim/src/main.rs`, re-sampled `src_mtime_post_wait` after `fq.acquire(seq)` before Stage B `try_replay_recent_leader`.

### WR-03: `strip_broker_flags` swallows the following Cargo flag when `--broker-run-timeout` lacks a numeric argument, and `VOX_BROKER_RUN_TIMEOUT_SECS=0` fails to override `VOX_BROKER_BUILD_TIMEOUT_SECS`

**Files modified:** `crates/vox-cargo-shim/src/runner.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed: requires human verification
**Applied fix:** Updated `strip_broker_cli_args` to consume `args[i + 1]` after `--broker-run-timeout` only when it parses as `u64` (preserving flags like `-p`), and updated `resolve_timeout_secs_from` to parse `VOX_BROKER_RUN_TIMEOUT_SECS` as `Option<u64>` prior to filtering `> 0` so `VOX_BROKER_RUN_TIMEOUT_SECS=0` explicitly disables the timeout (`None`) without falling back to `VOX_BROKER_BUILD_TIMEOUT_SECS`. Added regression assertions in `runner::tests`.

### WR-04: `effective_target_dir` resolves `<cwd>/target` instead of `<worktree>/target` when `--manifest-path` points into a worktree from an external `cwd`

**Files modified:** `crates/vox-build-queue/src/resolve.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed: requires human verification
**Applied fix:** Updated `effective_target_dir` in `crates/vox-build-queue/src/resolve.rs` to check `if let Some(wt) = worktree_root_of(manifest_dir)` whenever `--manifest-path` is present, returning `<wt>/target` even when `cwd` is outside any worktree. Added regression test assertion in `normalize_and_effective_target_dir_precedence`.

### WR-05: `run_child_post_queue` unconditionally forces `CARGO_TERM_COLOR=always`, overriding caller `CARGO_TERM_COLOR=never` and `NO_COLOR`

**Files modified:** `crates/vox-cargo-shim/src/runner.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed
**Applied fix:** Added `should_force_cargo_color` in `crates/vox-cargo-shim/src/runner.rs` so `CARGO_TERM_COLOR=always` is injected only when `stderr` is a terminal (`IsTerminal::is_terminal`) and neither `CARGO_TERM_COLOR` nor `NO_COLOR` is set. Added unit test `color_policy_respects_caller_overrides_no_color_and_tty`.

### WR-06: `broker_coalesce_and_timeout.rs` inherits ambient `VOX_BROKER_*` and `CARGO_TARGET_DIR` from parent `cargo test`, failing with exit 70 under nested broker runs

**Files modified:** `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed
**Applied fix:** Routed every `shim_cargo_bin()` invocation in `broker_coalesce_and_timeout.rs` through `clean_shim_cmd()`, scrubbing `VOX_BROKER_DEPTH`, `VOX_BROKER_*`, `VOX_ISOLATED_TARGET`, `VOX_WORKSTREAM`, `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL`, `RUSTC_WRAPPER`, and `RUSTUP_TOOLCHAIN`.

### IN-01: `docs/src/contributors/build-broker-usage.md` sample `vox-broker stats` verdict contradicts `Summary::render()`

**Files modified:** `docs/src/contributors/build-broker-usage.md`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed
**Applied fix:** Updated the sample `vox-broker stats` output line in `docs/src/contributors/build-broker-usage.md` to `-> BUILD-DAEMON: recommended (coalesce>=10% and queue waits present)` to match `Summary::render()`, and updated the Stage A coalesce-key tuple description to include `cwd_hash` and `toolchain`.

### IN-02: Completed flight artifacts under `<broker_home>/inflight/flights/` lack stale-file reaping

**Files modified:** `crates/vox-build-queue/src/coalesce.rs`
**Commit:** uncommitted (orchestrator-managed working tree per strict rules)
**Status:** fixed
**Applied fix:** Added `FLIGHT_ARTIFACT_TTL_MS` (`3_600_000` ms / 1 hour) and best-effort opportunistic `reap_stale_flights` in `claim_leader` that unlinks expired (>1h) files under `inflight/flights/` whose `<key>.lock` is not actively held, covered by unit test `shared_probe_back_to_back_isolation_and_ttl_reaping`.

---

_Fixed: 2026-11-15T10:02:00Z_
_Fixer: Antigravity (gsd-code-fixer)_
_Iteration: 1_
