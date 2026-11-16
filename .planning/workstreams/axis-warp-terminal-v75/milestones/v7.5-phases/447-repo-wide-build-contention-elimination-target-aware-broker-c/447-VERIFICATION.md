---
phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c
verified: 2026-11-15T12:16:00Z
status: passed
score: 17/17 must-haves verified
covered_files:
  - .agents/cargo-safety.md
  - .cargo/config.toml
  - .planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-01-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-01-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-02-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-02-SUMMARY.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-03-PLAN.md
  - .planning/workstreams/axis-warp-terminal-v75/phases/447-repo-wide-build-contention-elimination-target-aware-broker-c/447-03-SUMMARY.md
  - crates/vox-build-queue/src/coalesce.rs
  - crates/vox-build-queue/src/lib.rs
  - crates/vox-build-queue/src/metrics.rs
  - crates/vox-build-queue/src/mtime.rs
  - crates/vox-build-queue/src/queue.rs
  - crates/vox-build-queue/src/resolve.rs
  - crates/vox-cargo-shim/src/bin/vox-broker.rs
  - crates/vox-cargo-shim/src/main.rs
  - crates/vox-cargo-shim/src/runner.rs
  - crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs
  - crates/vox-cargo-shim/tests/vox_broker_readonly.rs
  - crates/vox-cli-ci/src/dev_loop_guard.rs
  - crates/vox-gui/Cargo.toml
  - crates/vox-orchestrator-d/Cargo.toml
  - crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs
  - crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs
  - crates/vox-orchestrator/src/route_policy.rs
  - crates/vox-orchestrator/src/victory.rs
  - crates/vox-orchestrator/tests/context_compaction_wiring_test.rs
  - crates/vox-orchestrator/tests/scaling_test.rs
  - crates/vox-term/Cargo.toml
  - crates/vox-terminal-core/Cargo.toml
  - crates/vox-terminal-core/src/vox_interp.rs
  - crates/vox-terminal-core/tests/session.rs
  - crates/vox-terminal-core/tests/vox_interp.rs
  - docs/src/contributors/build-broker-usage.md
  - scripts/broker-ci.vox
  - scripts/broker-install.vox
covered_digest: "v1:sha256:6675efbce192d69dcdc3f86c8e0f1e40aeadad657d6ea861e7233711ac38a93e"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 447: Repo-Wide Build Contention Elimination Verification Report

**Phase Goal:** Multiple concurrent agent sessions, CLI tools, and subagent waves execute `cargo` builds and tests across the Vox workspace with zero 900s queue-timeout hangs, zero feature-unification artifact thrashing on terminal/MCP crates, shared `sccache` object reuse across isolated target directories, and target-aware broker coalescing.
**Verified:** 2026-11-15T12:16:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

#### ROADMAP Success Criteria (4/4)

| # | Success Criterion | Status | Evidence |
| --- | --- | --- | --- |
| SC-1 | Concurrent `cargo check`/`test`/`build` invocations on the same `CARGO_TARGET_DIR` enter a target-first `FairQueue` before holding global broker slots (freeing slots for other target dirs), and identical concurrent `cargo check`/`build`/`clippy`/`test --no-run` invocations coalesce onto a single leader build with replayed exit code and diagnostics. | ✓ VERIFIED | `crates/vox-cargo-shim/src/main.rs:300-359` acquires `FairQueue::new(&root.join("targets"), &target_hash)` (`take_ticket` + `acquire` at lines `301-309`) strictly before `global::acquire_slot_adaptive(&root, n)` at line `358`. `crates/vox-build-queue/src/coalesce.rs:154-438` implements Windows-safe 0-byte `<key>.lock` leader-follower coalescing (`try_follow_inflight`, `try_replay_recent_leader`, `claim_leader`) across `check`, `build`, `clippy`, `test`, and `run` build phases. Live subprocess tests `target_first_fair_queue_frees_global_slot_for_other_target` and `leader_follower_coalescing_run_build_and_mtime_invalidation` pass in `broker_coalesce_and_timeout.rs`. |
| SC-2 | `vox-terminal-core` and `vox-term` gate `vox-compiler` (`47k LoC`) behind an opt-in `vox-lang` feature and align `vox-orchestrator` feature flags with `vox-orchestrator-mcp` (`default-features = false`), and `vox-orchestrator` compiles cleanly under `--no-default-features` so switching between terminal, MCP, and CLI crate tests never invalidates `target/debug/deps` fingerprints. | ✓ VERIFIED | `crates/vox-terminal-core/Cargo.toml:8-25` sets `default = []`, `vox-lang = ["dep:vox-compiler"]`, `vox-compiler = { workspace = true, optional = true }`, and `vox-orchestrator = { path = "../vox-orchestrator", default-features = false }`; `crates/vox-term/Cargo.toml:8-10` sets `default = []` and `vox-lang = ["vox-terminal-core/vox-lang"]`; `vox-orchestrator-d` (`Cargo.toml:19`) and `vox-gui` (`Cargo.toml:62`) explicitly opt into `vox-terminal-core/vox-lang`. `vox-orchestrator` fixes feature-gated leaks in `victory.rs`, `dei_dispatch.rs`, `route_policy.rs`, `healing.rs`, `scaling_test.rs`, and `context_compaction_wiring_test.rs`, passing `cargo test -p vox-orchestrator --no-default-features --lib --tests` (`exit 0`). |
| SC-3 | `vox-cargo-shim` auto-detects `sccache` on `PATH` when `RUSTC_WRAPPER` is unset (without breaking machines lacking `sccache`), supports opt-in per-workstream target dir isolation (`VOX_WORKSTREAM` / `.vox/agent-target` → `target/ws-<slug>`) sharing a single `SCCACHE_DIR`, and `vox-broker stats` reports separate `target_wait_ms`, `slot_wait_ms`, `compile_ms`, and `coalesced_hit` metrics. | ✓ VERIFIED | `crates/vox-build-queue/src/resolve.rs:135-284` implements pure `BTreeMap` `should_inject_sccache`, `resolve_sccache_wrapper`, `sanitize_workstream_slug`, and `apply_workstream_target_isolation` (setting `CARGO_INCREMENTAL=0` only when both isolated target and `sccache` are active), wired in `main.rs:238-252`. `metrics.rs:11-96,150-176` adds `#[serde(default)]` `target_wait_ms`, `slot_wait_ms`, `compile_ms`, `coalesced_hit`, `timed_out` and renders `target_p95`/`slot_p95`/`compile_p50`/`coalesce_rate`/`timeouts` in `vox-broker stats`. Verified by `sccache_stub_and_workstream_isolation_env_injection_and_guards` and `stats_reads_mixed_8_and_13_field_metrics_without_creating_dirs`. |
| SC-4 | `vox-cargo-shim` supports post-queue execution timeouts (`VOX_BROKER_RUN_TIMEOUT_SECS` / `--vox-run-timeout`) so active compile/test hangs are terminated cleanly (`exit 124`, process-group kill) without penalizing queue wait time, and `.agents/cargo-safety.md` + `vox ci dev-loop-guard` enforce the updated broker contract. | ✓ VERIFIED | `crates/vox-cargo-shim/src/runner.rs:148-269` starts `Instant::now()` strictly post-queue (line `182`), uses file-backed `Stdio::from(File)` (`CR-02`), spawns in `process_group(0)` / `CREATE_NO_WINDOW`, and kills hung process trees via `resolve::kill_process_tree` (`libc::killpg(..., SIGKILL)` / `taskkill /T /F /PID`) returning `124`. `crates/vox-cli-ci/src/dev_loop_guard.rs:103-324` enforces `.cargo/config.toml`, `.vox/agent-target` slug safety, and `.agents/cargo-safety.md` + `docs/src/contributors/build-broker-usage.md` parity (`9 passed` including `the_repository_passes_the_dev_loop_guard`). |

#### Plan Must-Have Truths (13/13)

| # | Plan | Truth | Status | Evidence |
| --- | --- | --- | --- | --- |
| 1 | 447-01 | Concurrent `cargo` builds targeting the same `CARGO_TARGET_DIR` enter a per-target `FairQueue` BEFORE acquiring a global broker slot, so waiting builds never hold global slots needed by builds targeting a different `CARGO_TARGET_DIR`. | ✓ VERIFIED | `crates/vox-cargo-shim/src/main.rs:301-309` executes `FairQueue::new(&root.join("targets"), &target_hash)`, `fq.take_ticket()`, and `fq.acquire(seq)` before `global::acquire_slot_adaptive(&root, n)` (`main.rs:358`). Subprocess test `target_first_fair_queue_frees_global_slot_for_other_target` (`broker_coalesce_and_timeout.rs:135-217`) proves Target-B completes in `<1.5s` while Target-A build #2 waits on Target-A's `FairQueue` under `VOX_BUILD_JOBS=2`. |
| 2 | 447-01 | Concurrent identical `cargo check`, `cargo build`, `cargo clippy`, `cargo test` (excluding snapshot-mutating env vars), and the build phase of `cargo run` coalesce onto a single leader invocation when source mtimes have not advanced, replaying exit code and captured stdout/stderr to followers (`coalesced_hit: true`). | ✓ VERIFIED | `coalesce::is_coalesceable` (`coalesce.rs:154-187`) gates subcommands, rejects `--message-format*` and `VOX_BROKER_COALESCE=0\|false`, blocks `INSTA_UPDATE`/`UPDATE_EXPECT`/`SNAPSHOT_UPDATE`/`TRYBUILD=overwrite` on `test`, and `main.rs:280-348` routes `run` through `resolve::build_args_for_run(&cargo_args)` followed by `exec_run_binary`. `mtime.rs` + `main.rs:311-315` re-sample `src_mtime_post_wait` after `fq.acquire(seq)` (`WR-02`). Tested in `leader_follower_coalescing_run_build_and_mtime_invalidation`. |
| 3 | 447-01 | `vox-cargo-shim` auto-injects `RUSTC_WRAPPER=<sccache_path>` into the child `Command` env when `RUSTC_WRAPPER` is unset and `sccache` is on `PATH` (disabled via `VOX_BROKER_NO_SCCACHE=1` or explicit `RUSTC_WRAPPER=""`), and routes `CARGO_TARGET_DIR` to `<worktree>/target/ws-<sanitized_slug>` when `VOX_ISOLATED_TARGET=1` with `VOX_WORKSTREAM` or `.vox/agent-target` (plus `CARGO_INCREMENTAL=0` when `sccache` is active). | ✓ VERIFIED | `resolve.rs:135-284` implements pure `BTreeMap` helpers (`should_inject_sccache`, `resolve_sccache_wrapper`, `sanitize_workstream_slug`, `apply_workstream_target_isolation`) with zero `std::env::set_var`, wired at `main.rs:238-252`. Tested in `sccache_stub_and_workstream_isolation_env_injection_and_guards`. |
| 4 | 447-01 | `vox-cargo-shim` enforces a post-queue active execution timeout via `--vox-run-timeout <SECS>` or `VOX_BROKER_RUN_TIMEOUT_SECS` whose timer starts strictly AFTER `FairQueue` and global slot acquisition, killing the entire child process group (`SIGKILL` to `-pgid` on Unix, `taskkill /T /F /PID` on Windows) and exiting `124` (`timed_out: true`) on timeout. | ✓ VERIFIED | `runner::run_child_post_queue` (`runner.rs:148-269`) starts `Instant::now()` at line `182` after `FairQueue` (`main.rs:309`) and `acquire_slot_adaptive` (`main.rs:358`), spawns in `process_group(0)` / `CREATE_NO_WINDOW`, and calls `resolve::kill_process_tree` (`resolve.rs:313-332`), returning `124`. Tested in `post_queue_timeout_and_grandchild_stdio_never_wedge_fair_queue`. |
| 5 | 447-01 | `MetricRecord` appends backward-compatible `#[serde(default)]` fields (`target_wait_ms`, `slot_wait_ms`, `compile_ms`, `coalesced_hit`, `timed_out`) while keeping all 8 existing fields intact, and `vox-broker stats` remains strictly read-only while displaying `target_p95`, `slot_p95`, `compile_p50`, `coalesce_rate`, and `timeouts`. | ✓ VERIFIED | `metrics.rs:11-39` retains all 8 original `MetricRecord` fields and appends 5 `#[serde(default)]` fields; `Summary::render` (`metrics.rs:150-176`) formats `target_p95`, `slot_p95`, `compile_p50`, `coalesce_rate`, `coalesced`, and `timeouts` with legacy `ran_ms` fallback. Read-only guarantee verified in `vox_broker_readonly.rs` (`4 passed`). |
| 6 | 447-02 | `cargo check -p vox-orchestrator --no-default-features --lib --tests` and `cargo check -p vox-orchestrator --no-default-features --features runtime --lib --tests` both succeed with zero compile errors, and `cargo test -p vox-orchestrator --no-default-features --lib` passes. | ✓ VERIFIED | `victory.rs:2,49`, `dei_dispatch.rs:216,259`, `route_policy.rs:6,24-73`, `healing.rs:12,78`, `scaling_test.rs:1`, and `context_compaction_wiring_test.rs:2` properly gate `toestub-gate` / `runtime` paths while keeping `VictoryEvaluator::new`, `ai.check` parameter validation, and `route_policy` env checks intact. `cargo test -p vox-orchestrator --no-default-features --lib --tests` passed `100%` (`exit 0`). |
| 7 | 447-02 | `route_policy.rs` without feature `runtime` still enforces `VOX_ROUTE_POLICY_PROFILE`, `VOX_ROUTE_ALLOW_NET`, `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP`, and `VOX_ROUTE_ALLOW_PROVIDER_NETWORK` fail-closed instead of silently allowing remote/network routing. | ✓ VERIFIED | `exclusion_reason_without_runtime_with` (`route_policy.rs:24-73`) parses `airgapped`/`strict-local` profiles and `false`/`0`/`no`/`off` allow flags fail-closed without depending on `vox-actor-runtime`. Closure-injected unit tests (`route_policy.rs:195-310`) verify every env guard. |
| 8 | 447-02 | `vox-terminal-core` compiles and passes `cargo test -p vox-terminal-core --lib --tests` WITHOUT compiling `vox-compiler` (`47k LoC`) by default (`default = []`), while `vox-lang = ["dep:vox-compiler"]` preserves full embedded Vox REPL evaluation and satisfies `vox-compiler` `eval::caps::tests::production_embedders_assign_caps_explicitly`. | ✓ VERIFIED | `crates/vox-terminal-core/Cargo.toml:8-25` sets `default = []`, `vox-lang = ["dep:vox-compiler"]`, `vox-compiler = { workspace = true, optional = true }` (absent from `[dev-dependencies]`). `vox_interp.rs:13-47` keeps exact `Interpreter::new(100_000)` and `CapabilitySet::parse("").unwrap()` lines in `#[cfg(feature = "vox-lang")]` and returns a clean disabled diagnostic in `#[cfg(not(feature = "vox-lang"))]`. Both `vox-terminal-core` (`67/67` unit + 10 integration binaries) and `vox-compiler` `production_embedders_assign_caps_explicitly` (`1/1`) pass. |
| 9 | 447-02 | `vox-terminal-core/Cargo.toml` depends on `vox-orchestrator` with `default-features = false` (matching `vox-orchestrator-mcp`), eliminating `target/debug/deps` feature-unification thrashing between terminal and MCP builds. | ✓ VERIFIED | `crates/vox-terminal-core/Cargo.toml:24` sets `vox-orchestrator = { path = "../vox-orchestrator", default-features = false }`, dropping default `toestub-gate` (`vox-code-audit` + `vox-lsp`), `jj`, `corpus-db`, and `http-server` edges from terminal builds so `vox-terminal-core` and `vox-orchestrator-mcp` (`Cargo.toml:47`, `default-features = false, features = ["runtime"]`) never pull `default` `vox-orchestrator` features. |
| 10 | 447-02 | `vox-orchestrator-d` and `vox-gui` enable `vox-terminal-core/vox-lang` so production daemon and GUI terminal sessions retain embedded Vox REPL support, while `vox-term` exposes a pass-through `vox-lang = ["vox-terminal-core/vox-lang"]` feature with lean default `[]`. | ✓ VERIFIED | `crates/vox-orchestrator-d/Cargo.toml:19` (`features = ["vox-lang"]`), `crates/vox-gui/Cargo.toml:62` (`features = ["vox-lang"]`), and `crates/vox-term/Cargo.toml:8-10` (`default = []`, `vox-lang = ["vox-terminal-core/vox-lang"]`). |
| 11 | 447-03 | `vox ci dev-loop-guard` (`crates/vox-cli-ci/src/dev_loop_guard.rs`) fails closed if `.cargo/config.toml` hardcodes `rustc-wrapper` or `RUSTC_WRAPPER` (which breaks contributors lacking `sccache`), if `.vox/agent-target` contains an unsafe slug outside `[A-Za-z0-9_-]{1,48}`, or if `.agents/cargo-safety.md` / `docs/src/contributors/build-broker-usage.md` drifts from the Phase 447 broker contract, and emits a non-fatal advisory warning when `sccache` is missing on interactive developer `PATH`. | ✓ VERIFIED | `crates/vox-cli-ci/src/dev_loop_guard.rs:103-324` keeps `pub fn run(root: &Path) -> Result<()>` synchronous (`#[inline(never)]`) and enforces `hardcoded_rustc_wrapper`, `shared_sccache_or_target_missing`, `invalid_agent_target_slug` (via `root.join(".vox").join("agent-target")`), `broker_safety_docs_missing_guidance`, and non-fatal `sccache_on_path` stderr advisory when `CI` is unset. `dev_loop_guard` (`9/9`) and `crate_api_docs` (`4/4`) pass. |
| 12 | 447-03 | `.agents/cargo-safety.md` and `docs/src/contributors/build-broker-usage.md` accurately document target-first `FairQueue`, flight coalescing, batched `cargo test -p crate-a -p crate-b --lib --tests`, post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` vs outer shell `timeout 900s`, auto-`sccache` (`VOX_BROKER_NO_SCCACHE=1`), `VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET=1`, and split `vox-broker stats` fields (`target_p95`, `slot_p95`, `compile_p50`, `coalesce_rate`, `timeouts`), passing `vox-doc-pipeline --lint-only`. | ✓ VERIFIED | `.agents/cargo-safety.md` (lines `1-74`) and `docs/src/contributors/build-broker-usage.md` (lines `1-165`) document every Phase 447 mechanism, exact env/CLI knobs, and exact `vox-broker stats` / `broker.log` output tokens (`IN-01` fixed). `./target/debug/vox-doc-pipeline --lint-only --paths contributors/build-broker-usage.md` exited `0` with zero hard errors. |
| 13 | 447-03 | `scripts/broker-install.vox` and `scripts/broker-ci.vox` document and smoke-check the new Phase 447 env knobs and split `vox-broker stats` output while passing `vox check`. | ✓ VERIFIED | `scripts/broker-install.vox` (lines `46-75, 422-426`) and `scripts/broker-ci.vox` (lines `124-158`) use `path.join` exclusively, locate binaries in `crates/vox-cargo-shim/target/release` with `<root>/target/release` fallback, and pass `./target/debug/vox check` (`exit 0`). |

**Score:** 17/17 must-haves verified (4/4 ROADMAP Success Criteria + 13/13 Plan Must-Have Truths)

### Prohibitions Verification (12/12 Upheld)

Every `must_haves.prohibitions` contract across Plans `447-01`, `447-02`, and `447-03` was adversarially verified directly against source on disk:

| # | Plan | Prohibition | Status | Codebase Proof |
| --- | --- | --- | --- | --- |
| P-01 | 447-01 | Do not acquire global slot (`global::acquire_slot_adaptive`) before target `FairQueue::acquire`. | ✓ UPHELD | `crates/vox-cargo-shim/src/main.rs:301-309` calls `FairQueue::new` + `take_ticket` + `acquire` 49 lines before `global::acquire_slot_adaptive(&root, n)` at `main.rs:358`. |
| P-02 | 447-01 | Do not hold `fs2` locks on `.stdout`, `.stderr`, or `.meta` files — lock ONLY a zero-byte `<key>.lock` file so followers can read on Windows without `ERROR_LOCK_VIOLATION` (`os error 33`). | ✓ UPHELD | `crates/vox-build-queue/src/coalesce.rs:305-322` (`claim_leader`) calls `try_lock_exclusive()` solely on 0-byte `paths.lock`, and `is_lock_held` (`coalesce.rs:93-104`) probes `paths.lock` via `try_lock_shared()` (`WR-01`). `.meta`, `.stdout`, `.stderr`, and `.exit` are never locked. |
| P-03 | 447-01 | Do not start the post-queue timeout timer before `FairQueue::acquire` and `global::acquire_slot_adaptive` have both succeeded. | ✓ UPHELD | `runner::run_child_post_queue` (`runner.rs:182`) initializes `let started = Instant::now();` strictly inside `run_child_post_queue`, which `main.rs:365` calls only after both `fq.acquire(seq)` (`main.rs:309`) and `global::acquire_slot_adaptive` (`main.rs:358`) return `Ok`. |
| P-04 | 447-01 | Do not break backward compatibility of existing 8-field `metrics.jsonl` lines, do not modify `crates/vox-build-queue/src/global.rs` (483 LoC), do not call `std::env::set_var` in Rust 2024 code or tests, and do not add crate dependencies or touch `Cargo.lock`. | ✓ UPHELD | `metrics.rs:11-39` keeps all 8 original fields and tags the 5 new fields `#[serde(default)]`; `global.rs` has zero modifications (`483` LoC); `rg "set_var|remove_var"` across all Phase 447 files returns `0` matches; `vox-build-queue/Cargo.toml` and `vox-cargo-shim/Cargo.toml` add zero dependencies. |
| P-05 | 447-02 | Do not silently disable `VOX_ROUTE_POLICY_PROFILE`, `VOX_ROUTE_ALLOW_NET`, `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP`, or `VOX_ROUTE_ALLOW_PROVIDER_NETWORK` when `vox-orchestrator` is compiled without feature `runtime`. | ✓ UPHELD | `route_policy.rs:24-73` (`exclusion_reason_without_runtime_with`) enforces all 4 env vars fail-closed under `#[cfg(not(feature = "runtime"))]`. |
| P-06 | 447-02 | Do not alter the public signature `pub fn eval_line(src: &str) -> Result<String>` in `vox_interp.rs`, and do not remove `Interpreter::new` or `CapabilitySet::parse("")` from `vox_interp.rs` (enforced by `vox-compiler` `eval::caps::tests::production_embedders_assign_caps_explicitly`). | ✓ UPHELD | `vox_interp.rs:13,41` exposes `pub fn eval_line(src: &str) -> Result<String>` in both `cfg` arms, and lines `23-24` retain `Interpreter::new(100_000)` and `CapabilitySet::parse("").unwrap()`. `production_embedders_assign_caps_explicitly` passes (`1/1`). |
| P-07 | 447-02 | Do not leave `vox-compiler` in `[dev-dependencies]` or default `features` of `vox-terminal-core` or `vox-term`. | ✓ UPHELD | `crates/vox-terminal-core/Cargo.toml:8-29` sets `default = []`, `vox-compiler = { workspace = true, optional = true }`, and `[dev-dependencies]` contains only `tempfile = { workspace = true }`. `crates/vox-term/Cargo.toml:8-10` sets `default = []`. |
| P-08 | 447-02 | Do not touch `docs/src/architecture/layers.toml`, `Cargo.lock`, or add any new crate dependencies. | ✓ UPHELD | Phase 447 added zero crate dependencies and made zero edits to `layers.toml` or `Cargo.lock` (note: staged `M  ` entries in the shared git index belong to Phase 405 `vox-repository` rules, verified via `git diff --cached -- docs/src/architecture/layers.toml`; `./target/debug/vox-arch-check` exited `0`). |
| P-09 | 447-03 | Do not make `dev_loop_guard::run(root: &Path) -> Result<()>` `async`, and do not remove `#[inline(never)]` (enforced by `vox-cli-ci` `crate_api_docs` tests). | ✓ UPHELD | `crates/vox-cli-ci/src/dev_loop_guard.rs:103-104` preserves `#[inline(never)] pub fn run(root: &Path) -> Result<()>` synchronously; `crate_api_docs` passes (`4/4`). |
| P-10 | 447-03 | Do not hardcode `rustc-wrapper = "sccache"` in `.cargo/config.toml`, and do not write literal `".vox/..."` string path segments in Rust source (construct via `.join(".vox").join("agent-target")` so repo path-policy detectors stay clean). | ✓ UPHELD | `.cargo/config.toml` has no `rustc-wrapper` or `RUSTC_WRAPPER`, and both `resolve.rs:265` (`wt.join(".vox").join("agent-target")`) and `dev_loop_guard.rs:267` (`root.join(".vox").join("agent-target")`) assemble path segments without literal `".vox/..."`. |
| P-11 | 447-03 | Do not invent fake env vars, do not add `training_eligible` or `last_updated` to `docs/src/contributors/build-broker-usage.md` frontmatter, and do not touch `contracts/documentation/kb-facet-backlog.v1.yaml`. | ✓ UPHELD | `docs/src/contributors/build-broker-usage.md:1-10` contains only valid governance frontmatter keys (`title`, `description`, `category`, `status`, `topic`, `domain`, `authorship`, `authorship_model`), documents only real code-backed env vars, and `kb-facet-backlog.v1.yaml` is clean (`git status --short` empty). |
| P-12 | 447-03 | Do not use `.sorted()` or `join_path(...)` in `.vox` scripts (use `path.join(...)` per `scripts/broker-install.vox`), and never run `scripts/broker-install.vox -- --apply` during verification. | ✓ UPHELD | `scripts/broker-install.vox` and `scripts/broker-ci.vox` use `path.join(...)` exclusively (`0` `.sorted()` / `join_path` calls), pass `./target/debug/vox check`, and `--apply` was never invoked. |

### Required Artifacts

Verified via `gsd-tools.cjs query verify.artifacts --ws axis-warp-terminal-v75` across all 3 plans (`15/15` passed, plus extracted `mtime.rs` submodule):

| Artifact | Plan | Expected Provides | Exists | Substantive | Wired | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `crates/vox-build-queue/src/coalesce.rs` (+ `src/mtime.rs`) | 447-01 | Windows-safe `<broker_home>/inflight/flights/` 0-byte `.lock` leader-follower coalescing, `try_lock_shared` probe, `run_id` artifact isolation, 1h stale flight reaper, `workspace_source_mtime_ns` (`mtime.rs`) | ✓ | ✓ (`497` + `128` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-build-queue/src/resolve.rs` | 447-01 | Pure `BTreeMap` `should_inject_sccache`, `resolve_sccache_wrapper`, `sanitize_workstream_slug`, `apply_workstream_target_isolation`, `build_args_for_run`, `kill_process_tree`, `effective_target_dir` (`--manifest-path` aware) | ✓ | ✓ (`477` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-build-queue/src/metrics.rs` | 447-01 | Backward-compatible `MetricRecord` (+5 `#[serde(default)]` fields) & `Summary::render` (`target_p95`, `slot_p95`, `compile_p50`, `coalesce_rate`, `timeouts`) | ✓ | ✓ (`278` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-cargo-shim/src/runner.rs` | 447-01 | `strip_broker_flags`, `run_child_post_queue` (post-queue timeout, `Stdio::from(File)` non-blocking tail, `process_group(0)`, `kill_process_tree` -> `124`), `exec_run_binary` | ✓ | ✓ (`335` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-cargo-shim/src/main.rs` | 447-01 | Target-first `FairQueue` -> flight coalescing (`compute_coalesce_key` with `cwd` + toolchain) -> `acquire_slot_adaptive` -> `run_child_post_queue` pipeline | ✓ | ✓ (`449` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs` | 447-01 | Subprocess integration tests for target-first `FairQueue`, coalescing + `cargo run` build phase + mtime invalidation, `sccache`/workstream env guards, and post-queue timeout `124` + grandchild stdio safety | ✓ | ✓ (`483` lines, 4 end-to-end tests) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator/src/route_policy.rs` | 447-02 | `#[cfg(not(feature = "runtime"))]` `exclusion_reason_without_runtime_with` fail-closed `VOX_ROUTE_*` enforcement + unit tests | ✓ | ✓ (`312` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs` | 447-02 | `#[cfg(all(feature = "runtime", feature = "toestub-gate"))]` LSP diagnostic gate (+ `victory.rs`, `dei_dispatch.rs`, `scaling_test.rs`, `context_compaction_wiring_test.rs`) | ✓ | ✓ (`140` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/Cargo.toml` | 447-02 | `default = []`, `vox-lang = ["dep:vox-compiler"]`, `vox-orchestrator = { path = "../vox-orchestrator", default-features = false }` (+ `vox-term`, `vox-orchestrator-d`, `vox-gui` Cargo manifests) | ✓ | ✓ (`30` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-terminal-core/src/vox_interp.rs` | 447-02 | Feature-gated `eval_line(src: &str) -> Result<String>` preserving `Interpreter::new(100_000)` + `CapabilitySet::parse("")` and lean disabled diagnostic | ✓ | ✓ (`48` lines) | ✓ | ✓ VERIFIED |
| `crates/vox-cli-ci/src/dev_loop_guard.rs` | 447-03 | Synchronous `#[inline(never)] pub fn run(root: &Path)` enforcing `.cargo/config.toml`, `.vox/agent-target` slug safety, doc parity, and `sccache` advisory | ✓ | ✓ (`432` lines, 9 tests) | ✓ | ✓ VERIFIED |
| `.agents/cargo-safety.md` | 447-03 | Agent cargo-safety guide covering `FairQueue` + coalescing, batched `-p`, `VOX_BROKER_RUN_TIMEOUT_SECS` vs outer `timeout 900s`, `VOX_ISOLATED_TARGET=1` + `VOX_WORKSTREAM` + auto-`sccache` | ✓ | ✓ (`74` lines) | ✓ | ✓ VERIFIED |
| `docs/src/contributors/build-broker-usage.md` | 447-03 | Full KB-compliant contributor architecture & operations guide for `vox-cargo-shim` and `vox-broker` | ✓ | ✓ (`165` lines) | ✓ | ✓ VERIFIED |
| `scripts/broker-install.vox` | 447-03 | Updated `--help` + install next-steps guidance for Phase 447 tunables and split `vox-broker stats` | ✓ | ✓ (`428` lines) | ✓ | ✓ VERIFIED |
| `scripts/broker-ci.vox` | 447-03 | Standalone `crates/vox-cargo-shim/target/release` + `<root>/target/release` fallback lookup & `vox-broker stats` smoke assertion | ✓ | ✓ (`160` lines) | ✓ | ✓ VERIFIED |

### Key Link Verification

Verified via `gsd-tools.cjs query verify.key-links --ws axis-warp-terminal-v75` across all 3 plans (`10/10` verified):

| From | To | Via | Status | Details |
| --- | --- | --- | --- | --- |
| `crates/vox-cargo-shim/src/main.rs` | `crates/vox-build-queue/src/queue.rs` | `FairQueue::new(&root.join("targets"), &target_hash)` + `take_ticket` + `acquire` BEFORE `global::acquire_slot_adaptive(&root, n)` | ✓ WIRED | `main.rs:301-309` acquires per-target `FairQueue` before `main.rs:358` acquires global slot. |
| `crates/vox-cargo-shim/src/main.rs` | `crates/vox-build-queue/src/coalesce.rs` | `coalesce::{is_coalesceable, try_follow_inflight, try_replay_recent_leader, claim_leader}` | ✓ WIRED | `main.rs:280-348, 375-397` wires pre-queue follow, post-wait replay with `src_mtime_post_wait` recheck, and leader artifact publication. |
| `crates/vox-cargo-shim/src/main.rs` | `crates/vox-cargo-shim/src/runner.rs` | `runner::run_child_post_queue` invoked strictly after `FairQueue::acquire` and `global::acquire_slot_adaptive` | ✓ WIRED | `main.rs:365-372` invokes `run_child_post_queue` after both queue stages complete. |
| `crates/vox-cargo-shim/src/runner.rs` | `crates/vox-build-queue/src/resolve.rs` | `vox_build_queue::resolve::kill_process_tree` called when active compile/test exceeds post-queue timeout | ✓ WIRED | `runner.rs:235` calls `resolve::kill_process_tree(child.id())` and returns exit `124`. |
| `crates/vox-terminal-core/Cargo.toml` | `crates/vox-orchestrator/Cargo.toml` | `vox-orchestrator = { path = "../vox-orchestrator", default-features = false }` | ✓ WIRED | `vox-terminal-core/Cargo.toml:24` disables default `vox-orchestrator` features (`toestub-gate`, `jj`, `corpus-db`, `http-server`), matching `vox-orchestrator-mcp/Cargo.toml:47`'s `path = "../vox-orchestrator", default-features = false` convention (root `[workspace.dependencies]` does not declare `vox-orchestrator`). |
| `crates/vox-terminal-core/Cargo.toml` | `crates/vox-terminal-core/src/vox_interp.rs` | Optional `dep:vox-compiler` gated by `vox-lang` | ✓ WIRED | `vox-terminal-core/Cargo.toml:9,13` wires `vox-lang = ["dep:vox-compiler"]` to `vox_interp.rs:13,40`. |
| `crates/vox-compiler/src/eval/caps.rs` | `crates/vox-terminal-core/src/vox_interp.rs` | `production_embedders_assign_caps_explicitly` source-text assertion on `Interpreter::new` and `CapabilitySet::parse` | ✓ WIRED | `caps.rs:700-728` scans `crates/vox-terminal-core/src/vox_interp.rs` and verifies both `Interpreter::new` and `CapabilitySet::parse` appear (`1 passed`). |
| `crates/vox-cli-ci/src/dev_loop_guard.rs` | `.cargo/config.toml` | `hardcoded_rustc_wrapper` and `shared_sccache_or_target_missing` called from synchronous `pub fn run(root)` | ✓ WIRED | `dev_loop_guard.rs:116-123, 220-264` audits `.cargo/config.toml` against live repo in `the_repository_passes_the_dev_loop_guard`. |
| `crates/vox-cli-ci/src/dev_loop_guard.rs` | `.agents/cargo-safety.md` | `broker_safety_docs_missing_guidance` parity check | ✓ WIRED | `dev_loop_guard.rs:287-304` verifies `FairQueue`, `VOX_BROKER_RUN_TIMEOUT_SECS`, `VOX_ISOLATED_TARGET`, `VOX_WORKSTREAM`, and `-p` batching markers. |
| `crates/vox-cli-ci/src/dev_loop_guard.rs` | `docs/src/contributors/build-broker-usage.md` | `broker_safety_docs_missing_guidance` parity check | ✓ WIRED | `dev_loop_guard.rs:306-324` verifies `VOX_BROKER_RUN_TIMEOUT_SECS`, `VOX_WORKSTREAM`, `VOX_BROKER_NO_SCCACHE`, `target_p95`, and `coalesce_rate` markers. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| --- | --- | --- | --- | --- |
| `resolve.rs` -> `main.rs` -> `runner.rs` | `child_env: BTreeMap<String, String>` (`RUSTC_WRAPPER`, `CARGO_TARGET_DIR`, `CARGO_INCREMENTAL`) + `target_hash` | `std::env::vars()` + `.vox/agent-target` + `PATH` `sccache` lookup -> `effective_target_dir` SHA-256 | Yes — verified end-to-end with executable `sccache` stub and `.vox/agent-target` worktrees in `broker_coalesce_and_timeout.rs` | ✓ FLOWING |
| `mtime.rs` + `main.rs` -> `coalesce.rs` | `CoalesceKey` + `FlightMeta { pid, run_id, started_ms, src_mtime_ns }` -> `<key>.<run_id>.{stdout,stderr,exit}` | SHA-256 of `(effective_sub, argv_hash, env_hash, target_hash, cwd_hash, tc_tag)` + recursive workspace mtime walk + child file-backed stdio | Yes — followers replay exact leader stdout/stderr/exit code, and post-build source mtime bump triggers fresh rebuild | ✓ FLOWING |
| `main.rs` -> `metrics.rs` -> `bin/vox-broker.rs` | `MetricRecord { wait_ms, target_wait_ms, slot_wait_ms, compile_ms, coalesced_hit, timed_out }` -> `Summary::render()` | Monotonic `Instant` deltas across `FairQueue`, `acquire_slot_adaptive`, and `run_child_post_queue` appended to `<broker_home>/metrics.jsonl` | Yes — `vox-broker stats` aggregates mixed 8-field and 13-field JSONL records accurately | ✓ FLOWING |

### Behavioral Spot-Checks

#### Directly Executed in Verification Session (Zero Workspace `target/` Contention)

| Behavior | Command | Result | Status |
| --- | --- | --- | --- |
| Full standalone `vox-cargo-shim` unit + subprocess integration suite (`cargo` bin, `vox-broker` bin, `broker_coalesce_and_timeout`, `passthrough`, `queue_serializes`, `vox_broker_readonly`) | `env CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/crates/vox-cargo-shim/target VOX_BROKER_NO_SCCACHE=1 RUSTC_WRAPPER="" timeout 120s cargo test --offline --manifest-path crates/vox-cargo-shim/Cargo.toml --target-dir crates/vox-cargo-shim/target --all-targets` | `30 passed; 0 failed` across 6 test binaries (7.62s test execution) | ✓ PASS |
| Workspace architectural layer, fan-in, and forbidden-dependency invariants (`layers.toml`) | `timeout 60s ./target/debug/vox-arch-check` | `exit 0` (`0` errors) | ✓ PASS |
| KB taxonomy, frontmatter, and summary lint on `docs/src/contributors/build-broker-usage.md` | `timeout 60s ./target/debug/vox-doc-pipeline --lint-only --paths contributors/build-broker-usage.md` | `exit 0` (`vox-doc-pipeline lint complete — no hard errors`; 4 stale-excuse `WARN`s confirming the doc now satisfies all 4 previously-excused KB facets) | ✓ PASS |
| Vox compiler type-check on `scripts/broker-install.vox` and `scripts/broker-ci.vox` | `timeout 60s ./target/debug/vox check scripts/broker-install.vox` & `timeout 60s ./target/debug/vox check scripts/broker-ci.vox` | Both `exit 0` (`Check passed`) | ✓ PASS |

#### Session-Verified Workspace Cargo Matrix (Cited Without Re-Locking Active Sibling `target/`)

| Suite | Command | Result | Status |
| --- | --- | --- | --- |
| `vox-build-queue` unit + clippy | `cargo test -p vox-build-queue` + `cargo clippy -p vox-build-queue -- -D warnings` | `43 passed; 0 failed`, `0` clippy warnings | ✓ PASS |
| `vox-orchestrator` minimal feature matrix | `cargo test -p vox-orchestrator --no-default-features --lib --tests` (`task-1004`) | `exit 0` (`100%` passed across unit & integration binaries) | ✓ PASS |
| `vox-terminal-core` lean default + `vox-lang` feature | `cargo test -p vox-terminal-core --lib --tests` (`task-1009`) + `--features vox-terminal-core/vox-lang --lib` (`task-1081`) | `exit 0` (`67/67` unit + 10 integration binaries) | ✓ PASS |
| `vox-compiler` production embedder capability audit | `cargo test -p vox-compiler --lib eval::caps::tests::production_embedders_assign_caps_explicitly` (`task-1107`) | `1 passed; 0 failed` | ✓ PASS |
| `vox-cli-ci` live-repo `dev_loop_guard` + `crate_api_docs` | `cargo test -p vox-cli-ci --lib dev_loop_guard` (`task-1088`) + `cargo test -p vox-cli-ci --lib crate_api_docs` (`task-1100`) | `9 passed; 0 failed` + `4 passed; 0 failed` | ✓ PASS |

### Code Review Remediation Verification (`447-REVIEW.md` → `447-REVIEW-FIX.md`)

All 10 findings (`2` Critical, `6` Warning, `2` Info) raised during `/gsd-code-review` were verified directly in source:
- **`CR-01` (`main.rs:206-224`):** `compute_coalesce_key` hashes `cwd` and `RUSTUP_TOOLCHAIN` / `RUSTC` alongside subcommand, args, env, and `target_hash` (`coalesce_key_distinguishes_sibling_cwds_and_toolchains` passes).
- **`CR-02` (`runner.rs:148-269`):** `run_child_post_queue` writes child stdout/stderr directly to `Stdio::from(File)` and tails non-blockingly so long-lived background grandchildren (`sccache` daemon) never block `FairQueue` release (`background_grandchild_inheriting_stdio_never_blocks_runner` passes).
- **`WR-01` (`coalesce.rs:93-104`):** `is_lock_held` uses `FileExt::try_lock_shared()` so Windows followers never steal or block leader lock acquisition.
- **`WR-02` (`main.rs:311-315`):** `src_mtime_post_wait` is re-sampled after `fq.acquire(seq)` before `try_replay_recent_leader`.
- **`WR-03` (`runner.rs:19-68`):** `strip_broker_flags` preserves non-numeric next tokens and treats `VOX_BROKER_RUN_TIMEOUT_SECS=0` as an explicit disable.
- **`WR-04` (`resolve.rs:36-74`):** `effective_target_dir` resolves `<wt>/target` even when `--manifest-path` is invoked from outside the worktree.
- **`WR-05` (`runner.rs:92-104`):** `should_force_cargo_color` honors `CARGO_TERM_COLOR`, `NO_COLOR`, `--color`, and `std::io::stderr().is_terminal()`.
- **`WR-06` (`mtime.rs:1-128`):** Recursive mtime scanner extracted into `crates/vox-build-queue/src/mtime.rs`, keeping `coalesce.rs` (`497` LoC) under the 500-line Soft-500 ceiling.
- **`IN-01` (`docs/src/contributors/build-broker-usage.md:145`):** `vox-broker stats` sample line matches exact `-> BUILD-DAEMON: recommended ...` verdict format.
- **`IN-02` (`coalesce.rs:107-133`):** `reap_stale_flights` prunes unlocked flight files older than 1 hour in `claim_leader`.

### Probe Execution

No `probe-*.sh` scripts were declared in `447-01-PLAN.md`..`447-03-PLAN.md` (per repo VoxLang/Rust policy, executable TDD verification was authored as Rust unit/integration suites in `crates/vox-build-queue`, `crates/vox-cargo-shim/tests/`, `crates/vox-orchestrator`, `crates/vox-terminal-core/tests/`, and `crates/vox-cli-ci/src/dev_loop_guard.rs`, plus `.vox` scripts in `scripts/broker-*.vox`).

### Requirements Coverage

Checked against `.planning/workstreams/axis-warp-terminal-v75/REQUIREMENTS.md` (`Phase 447` mapping):

| Requirement | Source Plan(s) | Description | Status | Supporting Evidence |
| --- | --- | --- | --- | --- |
| `BUILD-CONTENTION-01` | `447-01` | Target-first broker queueing & in-flight build coalescing in `vox-build-queue` + `vox-cargo-shim` (`FairQueue` per `CARGO_TARGET_DIR` BEFORE global slot acquisition; leader-follower deduplication for identical concurrent invocations; split `target_wait_ms` / `slot_wait_ms` / `compile_ms` metrics in `vox-broker stats`). | ✓ SATISFIED | `coalesce.rs`, `mtime.rs`, `queue.rs`, `metrics.rs`, `main.rs`, `bin/vox-broker.rs`; `broker_coalesce_and_timeout.rs` + `vox_broker_readonly.rs`. |
| `BUILD-CONTENTION-02` | `447-02` | Feature-unification & heavy-dependency decoupling for terminal & MCP crates (`vox-compiler` gated behind opt-in `vox-lang` in `vox-terminal-core` & `vox-term`; `vox-orchestrator` `default-features = false` alignment; `vox-orchestrator --no-default-features` compile & test fixes). | ✓ SATISFIED | `vox-terminal-core/Cargo.toml`, `vox-term/Cargo.toml`, `vox-orchestrator-d/Cargo.toml`, `vox-gui/Cargo.toml`, `vox_interp.rs`, `victory.rs`, `dei_dispatch.rs`, `route_policy.rs`, `healing.rs`, `scaling_test.rs`, `context_compaction_wiring_test.rs`. |
| `BUILD-CONTENTION-03` | `447-01`, `447-03` | Automatic `sccache` object-cache sharing & isolated per-workstream target directory routing (`RUSTC_WRAPPER=sccache` auto-injection when on `PATH`; `VOX_WORKSTREAM` / `.vox/agent-target` -> `target/ws-<slug>` with `CARGO_INCREMENTAL=0` under `sccache`; `dev-loop-guard` enforcement). | ✓ SATISFIED | `resolve.rs` (`should_inject_sccache`, `resolve_sccache_wrapper`, `sanitize_workstream_slug`, `apply_workstream_target_isolation`), `main.rs:238-252`, `dev_loop_guard.rs` (`hardcoded_rustc_wrapper`, `shared_sccache_or_target_missing`, `invalid_agent_target_slug`). |
| `BUILD-CONTENTION-04` | `447-01`, `447-03` | Post-queue execution timeout & batched test-harness safety contract (`VOX_BROKER_RUN_TIMEOUT_SECS` / `--vox-run-timeout` starting strictly post-queue, exiting `124` with process-group kill; `.agents/cargo-safety.md` & `build-broker-usage.md` & `dev-loop-guard` parity). | ✓ SATISFIED | `runner.rs` (`strip_broker_flags`, `run_child_post_queue`), `resolve.rs` (`kill_process_tree`), `.agents/cargo-safety.md`, `docs/src/contributors/build-broker-usage.md`, `scripts/broker-install.vox`, `scripts/broker-ci.vox`, `dev_loop_guard.rs`. |

**Orphaned requirements check:** No additional requirements in `REQUIREMENTS.md` are mapped to Phase 447 beyond `BUILD-CONTENTION-01..04`. Zero orphaned requirements (`Note on Architectural Scope`: `vox-terminal-core` + `vox-orchestrator-mcp` built together unify `vox-orchestrator` to `["runtime"]` without `toestub-gate`/`jj`/`corpus-db`/`http-server`, while standalone `vox-terminal-core`/`vox-term` builds compile `vox-orchestrator` with zero features — completely eliminating `default`-feature thrashing).

### Anti-Patterns Found

Scanned all 25 modified/created Rust, `.vox`, and Markdown files with `rg -n "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER|unimplemented!|todo!|set_var|remove_var"`:

| File | Line | Pattern | Severity | Impact |
| --- | --- | --- | --- | --- |
| *(all 25 Phase 447 source, test, `.vox`, and doc files)* | — | Zero `TODO` / `FIXME` / `HACK` / `PLACEHOLDER` / `todo!` / `unimplemented!` / `set_var` / `remove_var` matches | — | Clean. |

### Human Verification Required

None — zero `<human-check>` blocks were declared in `447-01-PLAN.md`..`447-03-PLAN.md`, and `447-VALIDATION.md` confirms all Phase 447 behaviors have automated unit, subprocess integration, doc-lint, and architecture-gate coverage.

### Gaps Summary

No gaps found. All 4 ROADMAP Success Criteria, 13 plan must-have truths, 12 plan prohibitions, 15 required artifacts (+ `mtime.rs`), 10 key links, 10 code-review fixes, and requirements `BUILD-CONTENTION-01` through `BUILD-CONTENTION-04` are implemented, wired end-to-end, and verified.
