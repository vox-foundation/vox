---
phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c
plan: 01
subsystem: infra
tags: [rust, cargo-shim, build-broker, fair-queue, coalescing, sccache, process-group, metrics]

requires: []
provides:
  - "Target-first FIFO FairQueue serialization per canonical CARGO_TARGET_DIR before global::acquire_slot_adaptive"
  - "Windows-safe 0-byte .lock + unlocked .meta/.stdout/.stderr/.exit leader-follower flight coalescing with source-mtime invalidation"
  - "Backward-compatible 13-field MetricRecord and 9-field Summary preserving legacy 8-field JSONL deserialization and daemon_recommended() verdict"
  - "Zero-config sccache auto-detection and opt-in workstream target isolation (VOX_ISOLATED_TARGET / VOX_WORKSTREAM / .vox/agent-target)"
  - "Post-queue VOX_BROKER_RUN_TIMEOUT_SECS active-compile process-tree supervision returning exit code 124"
  - "Read-only vox-broker stats/status split-wait and coalescing/timeout reporting"
affects:
  - 447-02
  - vox-cargo-shim
  - vox-build-queue
  - build-broker

actuals:
  tokens: 25809
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Target-first two-stage queueing: FairQueue::acquire(target_hash) strictly precedes global::acquire_slot_adaptive"
    - "Windows-safe flight coalescing under <broker_home>/inflight/flights/: 0-byte fs2-locked <key>.lock paired with unlocked atomic .meta/.exit JSON and .stdout/.stderr stream files"
    - "Post-queue active supervision: CommandExt::process_group(0) + kill_process_tree(pid) starting strictly after queue/slot acquisition"

key-files:
  created:
    - crates/vox-build-queue/src/coalesce.rs
    - crates/vox-cargo-shim/src/runner.rs
    - crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs
  modified:
    - crates/vox-build-queue/src/lib.rs
    - crates/vox-build-queue/src/queue.rs
    - crates/vox-build-queue/src/resolve.rs
    - crates/vox-build-queue/src/metrics.rs
    - crates/vox-cargo-shim/src/main.rs
    - crates/vox-cargo-shim/src/bin/vox-broker.rs
    - crates/vox-cargo-shim/tests/vox_broker_readonly.rs

key-decisions:
  - "Placed flight coalescing files under <broker_home>/inflight/flights/ so global::register_inflight (which scans top-level files in <broker_home>/inflight/ and unlinks unlocked regular files) never reaps active .meta/.stdout/.stderr/.exit sibling files while global.rs remains untouched at 483 LoC."
  - "Sampled arrival_ts_ms before Stage A (try_follow_inflight) and bounded ticket_ts_ms with ts.min(arrival_ts_ms) so a follower missing Stage A by sub-millisecond timing still qualifies for Stage B (try_replay_recent_leader) without sacrificing source-mtime causality."
  - "Exposed metrics::read_records(path) in vox-build-queue so integration tests in vox-cargo-shim parse MetricRecord JSONL without adding serde_json to crates/vox-cargo-shim/Cargo.toml."

patterns-established:
  - "Target-first admission ordering: resolve child env (sccache + workstream isolation) -> compute effective_target_dir -> Stage A follow -> Stage B FairQueue(targets/<target_hash>) + recent replay -> Stage C claim_leader + global::acquire_slot_adaptive -> Stage D post-queue runner -> cargo run exec_real tail."
  - "Strict zero-mutation read-only viewer invariant in vox-broker: inspect <broker_home>/targets and <broker_home>/inflight only when already present on disk."

requirements-completed:
  - BUILD-CONTENTION-01
  - BUILD-CONTENTION-03
  - BUILD-CONTENTION-04

coverage:
  - id: D1
    description: "Target-first FairQueue serializes builds per canonical CARGO_TARGET_DIR before global::acquire_slot_adaptive, leaving global slots free for distinct target directories"
    requirement: BUILD-CONTENTION-01
    verification:
      - kind: unit
        ref: "crates/vox-build-queue/src/resolve.rs#resolve::tests::normalize_and_effective_target_dir_precedence"
        status: pass
      - kind: integration
        ref: "crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs#target_first_fair_queue_frees_global_slot_for_other_target"
        status: pass
    human_judgment: false
  - id: D2
    description: "Windows-safe 0-byte .lock + unlocked .meta/.stdout/.stderr/.exit leader-follower flight coalescing with source-mtime invalidation and cargo run build-step coalescing"
    requirement: BUILD-CONTENTION-01
    verification:
      - kind: unit
        ref: "crates/vox-build-queue/src/coalesce.rs#coalesce::tests::leader_follower_streams_with_zero_byte_lock_and_recent_replay"
        status: pass
      - kind: integration
        ref: "crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs#leader_follower_coalescing_run_build_and_mtime_invalidation"
        status: pass
    human_judgment: false
  - id: D3
    description: "Backward-compatible split metrics (target_wait_ms, slot_wait_ms, compile_ms, coalesced_hit, timed_out) and read-only vox-broker stats/status rendering"
    requirement: BUILD-CONTENTION-01
    verification:
      - kind: unit
        ref: "crates/vox-build-queue/src/metrics.rs#metrics::tests::legacy_8_field_jsonl_deserializes_and_renders_split_summary"
        status: pass
      - kind: integration
        ref: "crates/vox-cargo-shim/tests/vox_broker_readonly.rs#stats_reads_mixed_8_and_13_field_metrics_without_creating_dirs"
        status: pass
    human_judgment: false
  - id: D4
    description: "Zero-config sccache auto-detection and opt-in workstream target isolation (<worktree>/target/ws-<slug> + conditional CARGO_INCREMENTAL=0)"
    requirement: BUILD-CONTENTION-03
    verification:
      - kind: unit
        ref: "crates/vox-build-queue/src/resolve.rs#resolve::tests::sccache_and_workstream_isolation_pure_helpers"
        status: pass
      - kind: integration
        ref: "crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs#sccache_stub_and_workstream_isolation_env_injection_and_guards"
        status: pass
    human_judgment: false
  - id: D5
    description: "Post-queue VOX_BROKER_RUN_TIMEOUT_SECS / --broker-run-timeout supervision starting after queue/slot acquisition and reaping active hangs via kill_process_tree with exit 124"
    requirement: BUILD-CONTENTION-04
    verification:
      - kind: unit
        ref: "crates/vox-cargo-shim/src/runner.rs#runner::tests::strip_flags_preserves_post_dashdash_and_resolves_precedence"
        status: pass
      - kind: integration
        ref: "crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs#post_queue_timeout_survives_queue_wait_and_kills_active_hang_with_124"
        status: pass
    human_judgment: false

duration: 65min
completed: 2026-11-15
status: complete
---

# Phase 447 Plan 01: Target-Aware Coalescing Build Broker & Post-Queue Supervision Summary

**Target-first `FairQueue` per canonical `CARGO_TARGET_DIR`, Windows-safe 0-byte `.lock` leader-follower flight coalescing, backward-compatible split metrics, auto-`sccache` + opt-in workstream target isolation, and post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` (`exit 124`) supervision across `vox-build-queue` and `vox-cargo-shim`**

## Performance

- **Duration:** 65 min
- **Started:** 2026-11-15T04:35:00Z
- **Completed:** 2026-11-15T06:40:00Z
- **Tasks:** 2 completed (TDD RED + GREEN verified on both)
- **Files modified:** 10 (7 modified, 3 created; uncommitted in working tree per orchestrator `NO GIT MUTATIONS` directive)

## Accomplishments

- **Target-First `FairQueue` Admission (`BUILD-CONTENTION-01`):** `vox-cargo-shim` resolves the child environment first, computes `resolve::effective_target_dir`, and acquires `FairQueue::new(&root.join("targets"), &target_hash)` *before* calling `global::acquire_slot_adaptive(&root, n)`, preventing callers contending on the same `target/.cargo-lock` from burning machine-wide concurrency slots needed by isolated `--target-dir` / workstream builds.
- **Windows-Safe Leader-Follower Flight Coalescing (`BUILD-CONTENTION-01`):** Added `vox_build_queue::coalesce` using a 0-byte `fs2`-locked `<key>.lock` file alongside unlocked `<key>.meta`, `<key>.stdout`, `<key>.stderr`, and `<key>.exit` files under `<broker_home>/inflight/flights/` (avoiding Windows `LockFileEx` `os error 33` and avoiding interference with `global::register_inflight`'s top-level stale-marker sweep). Supports Stage A live stream tailing, Stage B post-`FairQueue` recent-leader replay, `cargo run` build-phase coalescing (`build_args_for_run`), `--message-format*` & snapshot-update opt-outs, and workspace `latest_source_mtime_ms` cache invalidation.
- **Backward-Compatible Split Metrics & Read-Only Viewer (`BUILD-CONTENTION-01`):** Extended `MetricRecord` with `#[serde(default)]` `target_wait_ms`, `slot_wait_ms`, `compile_ms`, `coalesced_hit`, and `timed_out` while preserving all 8 legacy fields (`queue_wait_ms = target_wait_ms + slot_wait_ms`, `ran_ms = compile_ms`) and `Summary::daemon_recommended()`. Updated `vox-broker stats` and `vox-broker status` to surface split waits, coalesced hits, timeouts, active flights, and per-target queues without ever creating directories or files.
- **Zero-Config `sccache` & Opt-In Workstream Target Isolation (`BUILD-CONTENTION-03`):** Added pure `BTreeMap`-based `should_inject_sccache`, `resolve_sccache_wrapper` (skipping shim self/sibling binaries), and `apply_workstream_target_isolation` (`VOX_ISOLATED_TARGET=1` / `VOX_WORKSTREAM=<slug>` / `<worktree>/.vox/agent-target` -> `<worktree>/target/ws-<slug>`, sanitizing slugs to `[A-Za-z0-9_-]{1,48}` and setting `CARGO_INCREMENTAL=0` only when both isolated target and `sccache` are active).
- **Post-Queue Active Run Timeout (`BUILD-CONTENTION-04`):** Added `crates/vox-cargo-shim/src/runner.rs` (`strip_broker_flags`, `resolve_run_timeout_secs`, `run_child_post_queue`) and `resolve::kill_process_tree(pid)` (`#[cfg(unix)] libc::killpg` with `process_group(0)` / `#[cfg(windows)] taskkill /T /F /PID`). Active compile/test hangs exceed `VOX_BROKER_RUN_TIMEOUT_SECS` only *after* queue and slot waits complete, reaping grandchildren and returning `124`.

## TDD Gate Compliance & Commits

Per parent coordinator `<strict_execution_rules>` (`NO GIT MUTATIONS` on shared working tree), zero git commits were created; all 10 files remain cleanly in place for orchestrator verification and atomic commit.

1. **Task 1 (`vox-build-queue`) TDD Verification:**
   - **RED (`task-64`):** Wrote unit tests across `coalesce.rs`, `queue.rs`, `resolve.rs`, and `metrics.rs` against unimplemented stubs -> `38 passed; 8 failed`.
   - **GREEN (`task-79` / `task-178`):** Implemented modules -> `43 passed; 0 failed` (`clippy --all-targets -- -D warnings` exit `0`).
2. **Task 2 (`vox-cargo-shim`) TDD Verification:**
   - **RED (`task-107`):** Wrote `runner.rs` unit test, `vox_broker_readonly.rs` mixed-schema test, and 4 integration tests in `tests/broker_coalesce_and_timeout.rs` -> `0 passed; 4 failed` in `broker_coalesce_and_timeout`.
   - **GREEN (`task-208` / `task-212`):** Implemented `runner.rs`, `main.rs`, and `src/bin/vox-broker.rs` -> all `27` unit + integration tests across all 6 test binaries passed (`0 failed`), `clippy --all-targets -- -D warnings` exit `0`.

## Files Created/Modified & Line Counts (`wc -l` < 500 Ceiling)

| File | Action | Total Lines (`wc -l`) | Role |
|---|---|---:|---|
| `crates/vox-build-queue/src/lib.rs` | Modified | 9 | Exports `pub mod coalesce;` |
| `crates/vox-build-queue/src/global.rs` | Untouched | 483 | Untouched per plan constraint |
| `crates/vox-build-queue/src/coalesce.rs` | Created | 482 | Windows-safe 0-byte `.lock` flight coalescing + `latest_source_mtime_ms` |
| `crates/vox-build-queue/src/queue.rs` | Modified | 243 | Added `FairQueue::ticket_timestamp_ms(seq)` |
| `crates/vox-build-queue/src/resolve.rs` | Modified | 494 | Target-dir normalization, `sccache` discovery, workstream isolation, `kill_process_tree` |
| `crates/vox-build-queue/src/metrics.rs` | Modified | 288 | Backward-compatible 13-field `MetricRecord`, `read_records`, 9-field `Summary` |
| `crates/vox-cargo-shim/src/runner.rs` | Created | 250 | Flag stripping, timeout precedence, post-queue `process_group(0)` tee + reaper |
| `crates/vox-cargo-shim/src/main.rs` | Modified | 380 | Target-first `FairQueue` -> coalescing -> global slot -> post-queue runner pipeline |
| `crates/vox-cargo-shim/src/bin/vox-broker.rs` | Modified | 305 | Read-only split metrics + active flights + target queue inspection |
| `crates/vox-cargo-shim/tests/vox_broker_readonly.rs` | Modified | 109 | Mixed 8/13-field JSONL read-only verification |
| `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs` | Created | 462 | End-to-end integration suite (FairQueue slot freeing, coalescing, `sccache`/isolation, timeout `124`) |

## Decisions Made

- **Subdirectory isolation for flight artifacts (`<broker_home>/inflight/flights/`):** `global::register_inflight` scans `std::fs::read_dir(root.join("inflight"))` and immediately deletes any top-level regular file (`p.is_file()`) that is not `fs2`-locked. Placing flight locks and unlocked `.meta`/`.stdout`/`.stderr`/`.exit` siblings inside `inflight/flights/` keeps them under `<broker_home>/inflight/` while preventing `register_inflight` from deleting unlocked metadata/streams, without modifying `global.rs`.
- **Sub-millisecond arrival timestamp floor for Stage B replay:** Sampling `arrival_ts_ms` prior to Stage A (`try_follow_inflight`) and passing `ticket_ts_ms = fq.ticket_timestamp_ms(seq).map(|ts| ts.min(arrival_ts_ms)).unwrap_or(arrival_ts_ms)` ensures a follower that misses Stage A microseconds before a leader acquires `<key>.lock` still replays that leader in Stage B.
- **Threat Model Mitigations Verified (`T-447-01` .. `T-447-05`):**
  - `T-447-01`: Workstream slugs restricted to `[A-Za-z0-9_-]{1,48}` (`../axis-v75!` -> `ws-axis-v75` verified in unit test).
  - `T-447-02`: `latest_source_mtime_ms` + `ticket_ts_ms` + `run_id` matching + `!timed_out` reject stale or timed-out leader receipts.
  - `T-447-03`: `--message-format*` and `INSTA_UPDATE`/`UPDATE_EXPECT`/`SNAPSHOT_UPDATE`/`TRYBUILD=overwrite` bypass coalescing.
  - `T-447-04`: `resolve_sccache_wrapper` skips `own_exe` and same-directory siblings; `VOX_BROKER_DEPTH >= 2` exits `70`.
  - `T-447-05`: `CommandExt::process_group(0)` isolates child PGID so `killpg` never signals caller's parent process group.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug Prevention] Stored flight files in `<broker_home>/inflight/flights/` to avoid `global::register_inflight` reaping**
- **Found during:** Task 1 (`coalesce.rs` design against untouched `global.rs`)
- **Issue:** `crates/vox-build-queue/src/global.rs:256-277` unlinks any unlocked top-level `p.is_file()` in `<broker_home>/inflight/`, which would delete unlocked `<key>.meta`, `<key>.stdout`, `<key>.stderr`, and `<key>.exit` files during concurrent builds while `global.rs` is off-limits.
- **Fix:** Stored flight locks and sibling files in `<broker_home>/inflight/flights/` (`!p.is_file()` is skipped by `register_inflight`), updated `vox-broker status` to count locked `.lock` flights in `inflight/flights/`.
- **Files modified:** `crates/vox-build-queue/src/coalesce.rs`, `crates/vox-cargo-shim/src/bin/vox-broker.rs`
- **Verification:** `leader_follower_coalescing_run_build_and_mtime_invalidation` passes alongside `global::register_inflight`.

**2. [Rule 3 - Blocking] Added `vox_build_queue::metrics::read_records` helper for integration test JSONL parsing**
- **Found during:** Task 2 (`tests/broker_coalesce_and_timeout.rs` compilation)
- **Issue:** `serde_json` is a dependency of `vox-build-queue` (`Cargo.toml`), not a direct dependency of `vox-cargo-shim/Cargo.toml`, and `Cargo.toml` files are outside `files_modified`.
- **Fix:** Added `pub fn read_records(path: &Path) -> Vec<MetricRecord>` in `crates/vox-build-queue/src/metrics.rs` and called it from `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs`.
- **Files modified:** `crates/vox-build-queue/src/metrics.rs`, `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs`
- **Verification:** `cargo test` and `cargo clippy --all-targets -- -D warnings` pass cleanly with zero manifest edits.

---

**Total deviations:** 2 auto-fixed (1 bug prevention, 1 blocking test helper)
**Impact on plan:** Preserved strict 10-file scope (`global.rs` and all `Cargo.toml`/`Cargo.lock` untouched). Zero scope creep.

## Verification Proof Outputs

### 1. `vox-build-queue` Unit Tests (`task-178`)
Command: `RUSTC_WRAPPER="" timeout 300s cargo test -p vox-build-queue --target-dir crates/vox-cargo-shim/target`
```text
running 43 tests
test coalesce::tests::coalesce_eligibility_rules ... ok
test coalesce::tests::timed_out_leader_is_rejected_by_followers ... ok
test coalesce::tests::latest_source_mtime_tracks_source_edits_and_ignores_target ... ok
test coalesce::tests::leader_follower_streams_with_zero_byte_lock_and_recent_replay ... ok
test metrics::tests::legacy_8_field_jsonl_deserializes_and_renders_split_summary ... ok
test queue::tests::ticket_timestamp_ms_reads_issued_marker ... ok
test resolve::tests::sccache_and_workstream_isolation_pure_helpers ... ok
test resolve::tests::normalize_and_effective_target_dir_precedence ... ok
...
test result: ok. 43 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.27s
```

### 2. `vox-build-queue` Clippy (`task-183`)
Command: `RUSTC_WRAPPER="" timeout 300s cargo clippy -p vox-build-queue --target-dir crates/vox-cargo-shim/target --all-targets -- -D warnings`
```text
    Checking vox-build-queue v0.6.0 (/Users/brbrainerd/dev/vox/crates/vox-build-queue)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4m 07s
(exit code 0)
```

### 3. `vox-cargo-shim` Unit & Integration Tests (`task-208`)
Command: `CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/crates/vox-cargo-shim/target VOX_BROKER_NO_SCCACHE=1 RUSTC_WRAPPER="" timeout 300s cargo test --offline --manifest-path crates/vox-cargo-shim/Cargo.toml --target-dir crates/vox-cargo-shim/target --all-targets`
```text
     Running unittests src/main.rs
test runner::tests::strip_flags_preserves_post_dashdash_and_resolves_precedence ... ok
test result: ok. 1 passed; 0 failed

     Running unittests src/bin/vox-broker.rs
test result: ok. 15 passed; 0 failed

     Running tests/broker_coalesce_and_timeout.rs
test sccache_stub_and_workstream_isolation_env_injection_and_guards ... ok
test target_first_fair_queue_frees_global_slot_for_other_target ... ok
test leader_follower_coalescing_run_build_and_mtime_invalidation ... ok
test post_queue_timeout_survives_queue_wait_and_kills_active_hang_with_124 ... ok
test result: ok. 4 passed; 0 failed; finished in 5.04s

     Running tests/passthrough.rs
test result: ok. 2 passed; 0 failed

     Running tests/queue_serializes.rs
test two_builds_serialize_under_global_cap ... ok
test result: ok. 1 passed; 0 failed

     Running tests/vox_broker_readonly.rs
test stats_reads_mixed_8_and_13_field_metrics_without_creating_dirs ... ok
test log_reads_last_n_lines_without_disturbing_the_file ... ok
test unknown_subcommand_is_a_usage_error ... ok
test never_run_subcommands_exit_zero_and_say_so ... ok
test result: ok. 4 passed; 0 failed
```

### 4. `vox-cargo-shim` Clippy & `rustfmt --check` (`task-212`)
Commands:
- `RUSTC_WRAPPER="" timeout 300s cargo clippy --offline --manifest-path crates/vox-cargo-shim/Cargo.toml --target-dir crates/vox-cargo-shim/target --all-targets -- -D warnings` -> `Finished dev profile ... in 41.64s (exit 0)`
- `rustfmt --edition 2024 --check <all 10 files>` -> `exit 0`

## Issues Encountered

- Running `cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml` from `/Users/brbrainerd/dev/vox` without `CARGO_TARGET_DIR` set in the environment caused Cargo to inject `.cargo/config.toml`'s `[env] CARGO_TARGET_DIR = "target"` (`/Users/brbrainerd/dev/vox/target`) into `tests/queue_serializes.rs`, contending with a sibling `vox-cli` build on the host root target dir. Passing `CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/crates/vox-cargo-shim/target VOX_BROKER_NO_SCCACHE=1` isolated `queue_serializes.rs` completely from both `/Users/brbrainerd/dev/vox/target` and the host `sccache` daemon (`2.03s`).

## User Setup Required

None - `sccache` detection is automatic when installed on `PATH`, and all new broker controls have safe zero-config defaults.

## Next Phase Readiness

- Ready for Plan `447-02` (broker policy / `.vox` scripts / docs / installation rollout).
- Note: Reinstalling the updated `vox-cargo-shim` binary onto `~/.vox/bin/cargo` (or `CARGO_HOME/bin`) will activate target-first queueing, coalescing, auto-`sccache`, and `VOX_BROKER_RUN_TIMEOUT_SECS` for live host sessions.

---
*Phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c*
*Completed: 2026-11-15*
