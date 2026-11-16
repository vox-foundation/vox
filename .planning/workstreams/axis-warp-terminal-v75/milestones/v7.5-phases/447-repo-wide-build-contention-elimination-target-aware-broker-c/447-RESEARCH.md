# Phase 447: Repo-Wide Build Contention Elimination & Broker Coalescing - Research

**Researched:** 2026-11-14
**Domain:** Daemonless Cargo Build Broker (`vox-cargo-shim` + `vox-build-queue`), Cargo Feature Graph Decoupling (`vox-orchestrator` + `vox-terminal-core`), `sccache` & Workstream Target Isolation, Queue-Aware Process Timeouts
**Confidence:** HIGH

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Target-First FairQueue Ordering, True In-Flight Coalescing & Honest Telemetry (`BUILD-CONTENTION-01`)
- Wire `vox_build_queue::queue::FairQueue` (keyed by canonicalized effective `CARGO_TARGET_DIR`, honoring `--target-dir`, `CARGO_TARGET_DIR`, and `.cargo/config.toml`) into `crates/vox-cargo-shim/src/main.rs` **before** calling `global::acquire_slot_adaptive`. Waiters on the same `target/` directory must queue on the per-target lock without consuming one of the machine-wide `cap` slots (fixing the bug where 6 sleeping `Blocking waiting for file lock on build directory` processes drop `load_average_1m()` near `0`, grab all 6 global slots at `wait=0ms`, and hide 30–107 minutes of lock wait inside `ran_ms`).
- Upgrade `register_inflight` / `FairQueue` from passive `would_coalesce` logging (`5.2%+` global, `~50%` during parallel agent waves) to **real leader-follower coalescing** for idempotent verification commands (`cargo check`, `cargo build`, and identical `cargo test` invocations when source mtimes have not advanced since leader start): followers attach to the active leader's output log + exit code and return in `0 ms` post-leader instead of re-running Cargo.
- Split `MetricRecord` (`metrics.jsonl`, `broker.log`, and `vox-broker stats`) into explicit `target_wait_ms`, `slot_wait_ms`, and `compile_ms` (backward-compatible `#[serde(default)]` on existing `metrics.jsonl` rows) so `vox-broker stats` never misreports `p95_wait=0ms` when builds spend 30 minutes queued on `target/.cargo-lock`.

#### Lean `vox-orchestrator` `--no-default-features` & `vox-terminal-core` Dependency Decoupling (`BUILD-CONTENTION-02`)
- Gate unguarded `vox_code_audit` / `crate::validation` references in `crates/vox-orchestrator/src/victory.rs` and `crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs` behind `#[cfg(feature = "toestub-gate")]` so `cargo check/test -p vox-orchestrator --no-default-features` and `--no-default-features --features runtime` compile and pass cleanly.
- Activate the planned `vox-lang` feature in `crates/vox-terminal-core/Cargo.toml` (`vox-compiler` optional behind `vox-lang`, enabled only by crates needing inline `.vox` compilation) and switch `vox-terminal-core`'s `vox-orchestrator` dependency to `default-features = false` (aligning its feature fingerprint with `vox-orchestrator-mcp` so concurrent MENS edits in `vox-corpus` / `vox-populi` / `jj-lib` no longer trigger 180k-LoC cascade recompiles or `target/debug/deps` feature ping-pong).

#### Workstream Isolation, Zero-Config `sccache` Auto-Discovery & Command Batching Guard (`BUILD-CONTENTION-03`)
- In `vox-cargo-shim`, automatically set `RUSTC_WRAPPER=sccache` for child Cargo invocations whenever `RUSTC_WRAPPER` is unset AND `sccache` is present and executable on `PATH` (preserving `.cargo/config.toml`'s CI safety when `sccache` is absent while sharing object caches across worktrees/targets when installed).
- Support optional per-workstream target isolation (`VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET=1` routing `CARGO_TARGET_DIR` to `target/ws-<slug>` when `use_worktrees: false` forces multiple GSD workstreams into one checkout) plus a `vox ci dev-loop-guard` / `.agents/rules/00-core.md` update encouraging batched multi-package `cargo test -p a -p b --lib --tests` verification per wave instead of N sequential single-test lock acquisitions.

#### Queue-Aware Build Timeouts Eliminating `98%` `exit 124` Kills (`BUILD-CONTENTION-04`)
- Add `VOX_BROKER_RUN_TIMEOUT_SECS` (and `--broker-run-timeout` / env support in `vox-cargo-shim`) that starts the timeout clock **only after** both the per-target `FairQueue` lock and global slot are acquired — preventing outer `timeout 300s`/`900s` wrappers from killing `rustc`/`ld` at `Building 964/965` simply because a build spent 14 minutes waiting in line, and document the canonical pattern in `.agents/rules/00-core.md` and `docs/src/contributors/build-broker-usage.md`.

### the agent's Discretion
- Exact file-lock protocol for leader-follower output replay (`<broker_home>/inflight/<key>.out` + `.exit`) and exact `MetricRecord` field names so long as Windows + macOS + Linux file-locking rules (`fs2` single-handle rule on Windows `os error 33`) and `< 500` LoC `god_object` limits are strictly respected.

### Deferred Ideas (OUT OF SCOPE)
- Stateful persistent background build daemon (`sccache-dist` or remote Bazel/Buck2 migration) — explicitly out of scope; fixing `vox-cargo-shim` + `vox-build-queue` + feature decoupling solves contention daemonlessly within Cargo.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| `BUILD-CONTENTION-01` | `vox-cargo-shim` + `vox-build-queue` serialize builds per effective `CARGO_TARGET_DIR` via `FairQueue` **before** claiming a global machine slot, coalesce identical in-flight `(argv, env, target_dir)` `check`/`build`/`test` invocations by sharing the leader's exit status + output stream, and separate target lock wait vs global slot wait vs active compile time in `vox-broker stats` / `broker.log`. | Verified root cause in `crates/vox-cargo-shim/src/main.rs:150-189` (`FairQueue` is never imported or called). Designed canonical `effective_target_dir` resolver, Windows `os error 33`-safe separate-lockfile leader-follower coalescing (`crates/vox-build-queue/src/coalesce.rs`), and `#[serde(default)]` `MetricRecord` extension (`target_wait_ms`, `slot_wait_ms`, `compile_ms`, `coalesced_hit`). |
| `BUILD-CONTENTION-02` | `vox-orchestrator` compiles and passes `--no-default-features` (`#[cfg(feature = "toestub-gate")]` guards on `victory.rs` and `orch_daemon/dei_dispatch.rs`), and `vox-terminal-core` gates `vox-compiler` (`47k LoC`) behind `vox-lang` while depending on `vox-orchestrator` with `default-features = false`. | Exhaustive static audit of `vox-orchestrator` uncovered **3 additional hidden `--no-default-features` / `--features runtime` blockers** beyond `victory.rs` and `dei_dispatch.rs` (`route_policy.rs:6`, `healing.rs:12-25`, and integration tests `scaling_test.rs` + `context_compaction_wiring_test.rs`), plus a cross-crate source-text audit in `vox-compiler/src/eval/caps.rs:700-728` that constrains how `vox_interp.rs` is gated. |
| `BUILD-CONTENTION-03` | Multi-session / multi-workstream agent harnesses isolate `CARGO_TARGET_DIR` (`VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET=1` / `.vox/agent-target`) + automatic `sccache` wrapper detection when `sccache` is installed on `PATH`, and enforce `dev-loop-guard` rules against unbatched per-test Cargo lock storms. | Verified `sccache 0.19.0` on host `PATH`, `.cargo/config.toml:15-17, 78-95` rules, `scripts/broker-ci.vox:119-125` standalone manifest target nuance, and `crates/vox-cli-ci/src/dev_loop_guard.rs:1-248` fast-tier architecture. |
| `BUILD-CONTENTION-04` | Queue-aware build timeout (`VOX_BROKER_RUN_TIMEOUT_SECS` / `VOX_BROKER_BUILD_TIMEOUT_SECS` / `--broker-run-timeout`) starts ticking **only after** both `FairQueue` and global `Slot` acquisition, returning exit code `124` on active-runtime overrun without premature kills at `98%`. | Designed post-slot `Child::try_wait` polling runner (`crates/vox-cargo-shim/src/runner.rs`) with Unix process-group (`process_group(0)` + `killpg`) and Windows `taskkill /T /F` / `Child::kill` cleanup, returning exit `124`, plus documentation/installer updates (`scripts/broker-install.vox`, `docs/src/contributors/build-broker-usage.md`, `.agents/cargo-safety.md`, `.agents/rules/00-core.md`). |
</phase_requirements>

---

## Project Constraints (from CLAUDE.md, GEMINI.md & AGENTS.md)

1. **`god_object` `< 500` LoC Ceiling:** Every Rust source file must stay strictly under 500 lines (`vox audit code`). **Critical watchpoint:** `crates/vox-build-queue/src/global.rs` is already **483 lines** — new coalescing and target-resolution code MUST go into dedicated modules (`crates/vox-build-queue/src/coalesce.rs` and `crates/vox-build-queue/src/resolve.rs` / `crates/vox-cargo-shim/src/runner.rs`), NOT appended to `global.rs`.
2. **Standalone Excluded Workspace for `vox-cargo-shim`:** `crates/vox-cargo-shim/Cargo.toml` declares its own `[workspace]` (`[VERIFIED: crates/vox-cargo-shim/Cargo.toml:19]`: `"[workspace]"`) and is excluded from the root workspace so a `cargo`-named binary never lands in `<repo>/target/debug/cargo`. All build/test/clippy commands for `vox-cargo-shim` MUST pass `--manifest-path crates/vox-cargo-shim/Cargo.toml` (and `scripts/broker-ci.vox` must remain green).
3. **Zero New External Crate Dependencies / `Cargo.lock` Churn:** Per `.agents/rules/00-core.md` Rule 5, tasks must not introduce new external crates, `layers.toml` exceptions, or `Cargo.lock` churn. `vox-build-queue` already has `anyhow`, `serde`, `serde_json`, `fs2 = "0.4"`, and `[target.'cfg(unix)'.dependencies] libc` — 100% sufficient for Phase 447.
4. **Read-Only `vox-broker` Viewer Invariant:** `crates/vox-cargo-shim/tests/vox_broker_readonly.rs` enforces that `vox-broker stats|log|status` never creates or mutates any file or directory inside `VOX_BROKER_HOME` (`[VERIFIED: crates/vox-cargo-shim/tests/vox_broker_readonly.rs:43-45]`: `"assert!(entries.is_empty(), \"vox-broker must never write into the broker home: found {entries:?}\");"`).
5. **Windows Mandatory File-Lock Rule (`os error 33`):** `fs2::FileExt::lock_exclusive` on Windows uses `LockFileEx` over the entire file, causing `ReadFile`/`WriteFile` via any second file handle to fail with `ERROR_LOCK_VIOLATION` (`os error 33`, `[VERIFIED: crates/vox-build-queue/src/queue.rs:66-67]`: `"// Hold a single locked handle and read/write through it; opening a second\n// handle to the same file while locked fails on Windows (os error 33)."`).
6. **VoxLang-First Glue (`AGENTS.md`):** Automation under `scripts/` MUST remain `.vox` (`scripts/broker-install.vox`, `scripts/broker-ci.vox`, `scripts/broker-bypass-lint.vox`); never add `.sh`, `.ps1`, or `.py` scripts.
7. **Formatting & Verification Discipline:** Format changed `.rs` files individually with `rustfmt --edition 2024 <file.rs>` (never `cargo fmt`). Authored docs under `docs/src/` require valid YAML frontmatter verified with `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/contributors/build-broker-usage.md`.

---

## Summary

Phase 447 eliminates four compounding bottlenecks that currently turn parallel multi-agent development in Vox into 20–107 minute build stalls and wasted `exit 124` timeouts:

1. **Inverted Lock Ordering & Passive Coalescing (`BUILD-CONTENTION-01`):** Although `vox-build-queue` already contains a unit-tested `FairQueue` (`crates/vox-build-queue/src/queue.rs`), `crates/vox-cargo-shim/src/main.rs` never imports or invokes it (`[VERIFIED: crates/vox-cargo-shim/src/main.rs:15]`: `"use vox_build_queue::{env_filter, global, metrics, resolve};"`). Instead, `run_global` immediately calls `global::acquire_slot_adaptive(&root, n)` (`main.rs:153`) and spawns `cargo`. When 6 agents share one `target/` directory (`"use_worktrees": false` in `.planning/config.json`), 5 of them block inside Cargo's own `target/.cargo-lock` in kernel sleep (`0%` CPU), driving `load_average_1m()` down so `should_admit` hands out all `cap` global slots at `queue_wait_ms = 0` while burying 30–107 minutes of lock wait inside `ran_ms`. Acquiring a per-target `FairQueue` lock (keyed by canonicalized effective `CARGO_TARGET_DIR`) **before** `acquire_slot_adaptive`, paired with real leader-follower in-flight coalescing on `<broker_home>/inflight/`, frees global slots for other targets/workstreams and collapses duplicate `check`/`build`/`test` waves to `0 ms` post-leader.
2. **Broken `--no-default-features` in `vox-orchestrator` & Heavy `vox-terminal-core` Edges (`BUILD-CONTENTION-02`):** `vox-terminal-core` unconditionally compiles `vox-compiler` (`47k LoC`) and `vox-orchestrator` with default features (`["runtime", "toestub-gate", "corpus-db", "jj"]`), whereas `vox-orchestrator-mcp` compiles `vox-orchestrator` with `default-features = false, features = ["runtime", "toestub-gate", "json-schema"]` (leaving `corpus-db` and `jj` OFF). Every alternating terminal vs MCP build ping-pongs `vox-corpus` (`+database` vs `-database`) and `vox-vcs` (`+jj` vs `-jj`). Static inspection of `vox-orchestrator` revealed **5 total `--no-default-features` / `--features runtime` compile/test gaps** (`victory.rs:51`, `dei_dispatch.rs:226`, `route_policy.rs:6`, `healing.rs:12-25`, and integration tests `scaling_test.rs` + `context_compaction_wiring_test.rs`) plus a critical source-text test in `vox-compiler/src/eval/caps.rs:700-728` that inspects `crates/vox-terminal-core/src/vox_interp.rs`.
3. **Missing Dynamic `sccache` & Shared-Checkout Target Collisions (`BUILD-CONTENTION-03`):** `.cargo/config.toml` sets `SCCACHE_DIR = ".sccache"` (`relative = true`) but intentionally omits `rustc-wrapper = "sccache"` so CI runners without `sccache` do not fail. Having `vox-cargo-shim` dynamically set `RUSTC_WRAPPER=<sccache_path>` when unset and `sccache` is on `PATH` (verified `sccache 0.19.0` installed on host), combined with opt-in `VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET=1` routing `CARGO_TARGET_DIR` to `<worktree_root>/target/ws-<slug>`, unlocks concurrent Cargo execution across workstreams in one checkout while sharing `.sccache` objects.
4. **Queue-Blind Outer Timeouts (`BUILD-CONTENTION-04`):** Outer `timeout 300s`/`900s cargo ...` wrappers count `FairQueue` + `Slot` wait against compile budget, killing `rustc`/`ld` at `Building 964/965` with `exit 124`. Enforcing `VOX_BROKER_RUN_TIMEOUT_SECS` (and `--broker-run-timeout`) inside `vox-cargo-shim` **only after** both locks are held guarantees timeouts measure active child execution only.

**Primary recommendation:** Structure Phase 447 into four focused, non-overlapping plans:
- **Plan 01 (`BUILD-CONTENTION-02`):** Fix all 5 `vox-orchestrator` feature-gate gaps (`victory.rs`, `dei_dispatch.rs`, `route_policy.rs`, `healing.rs`, integration tests) and decouple `vox-terminal-core` (`vox-lang = ["dep:vox-compiler"]`, `vox-orchestrator` `default-features = false`, preserving `vox_interp.rs` strings for `caps.rs:710`, enabling `vox-terminal-core/vox-lang` in `vox-orchestrator-d` and `vox-gui`).
- **Plan 02 (`BUILD-CONTENTION-01`):** Implement canonical `effective_target_dir` resolution, `coalesce.rs` Windows-safe leader-follower coalescing, target-first `FairQueue` -> `acquire_slot_adaptive` ordering in `vox-cargo-shim`, and 3-stage `MetricRecord` (`target_wait_ms`, `slot_wait_ms`, `compile_ms`, `coalesced_hit`).
- **Plan 03 (`BUILD-CONTENTION-03` + `BUILD-CONTENTION-04`):** Add dynamic `sccache` wrapper injection, `VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET=1` target isolation, and post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` / `--broker-run-timeout` child-tree timeout (`exit 124`) in `vox-cargo-shim` + `vox-build-queue`.
- **Plan 04 (`BUILD-CONTENTION-03` + `BUILD-CONTENTION-04` Guardrails & Reinstall):** Extend `vox ci dev-loop-guard` (`crates/vox-cli-ci/src/dev_loop_guard.rs`), update `scripts/broker-ci.vox` / `scripts/broker-install.vox`, update `docs/src/contributors/build-broker-usage.md` + `.agents/cargo-safety.md` + `.agents/rules/00-core.md`, and verify the full `broker-ci` + `broker-bypass-lint` + `vox-doc-pipeline` gate suite.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Effective `CARGO_TARGET_DIR` resolution, `sccache` discovery, `VOX_WORKSTREAM` slug sanitization | `vox-build-queue` (`src/resolve.rs`) | `vox-cargo-shim` (`src/main.rs`) | Pure, unit-testable path/env logic shared across platform targets without spawning Cargo. |
| Per-target FIFO queuing & Windows-safe leader-follower coalescing | `vox-build-queue` (`src/queue.rs`, new `src/coalesce.rs`) | `vox-cargo-shim` (`src/main.rs`) | Keeps `global.rs` (483 LoC) under the 500-LoC `god_object` limit while isolating `fs2` locking invariants. |
| Three-stage wait/compile telemetry & summary rendering | `vox-build-queue` (`src/metrics.rs`) | `vox-cargo-shim` (`src/bin/vox-broker.rs`) | Backward-compatible `#[serde(default)]` JSONL schema consumed by read-only `vox-broker stats/log`. |
| Post-queue child execution, output teeing & active-runtime timeout (`exit 124`) | `vox-cargo-shim` (new `src/runner.rs`, `src/main.rs`) | OS process groups (`libc::killpg` / Windows `taskkill`) | Ensures timeout clock starts strictly after `FairQueue` + `Slot` acquisition and reaps child `rustc`/test trees cleanly. |
| Lean orchestrator & terminal feature boundaries | `vox-orchestrator` & `vox-terminal-core` | `vox-orchestrator-d`, `vox-gui`, `vox-term` | Eliminates `corpus-db` / `jj` feature ping-pong and gates `47k-LoC` `vox-compiler` behind `vox-lang`. |
| Config & batching CI enforcement | `vox-cli-ci` (`src/dev_loop_guard.rs`) | `docs/src/contributors/build-broker-usage.md`, `.agents/cargo-safety.md` | Fast, build-free CI/pre-push enforcement without adding new CLI subcommands or regenerating command catalogs. |

---

## Standard Stack

### Core (Already Present — Zero New Crates Required)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `fs2` | `0.4` | Cross-platform advisory/mandatory file locks (`lock_exclusive`, `try_lock_exclusive`, `unlock`) | Already in `crates/vox-build-queue/Cargo.toml:14`; battle-tested in `global.rs` and `queue.rs` `[VERIFIED: crates/vox-build-queue/src/queue.rs:15]`. |
| `serde` / `serde_json` | `1` | `MetricRecord` (`metrics.jsonl`) and `.meta`/`.exit` coalescing state | Already in `crates/vox-build-queue/Cargo.toml:12-13` `[VERIFIED: crates/vox-build-queue/src/metrics.rs:7]`. |
| `anyhow` | `1` | Fallible broker coordination with graceful fallback to `exec_real` on any error | Already in both `vox-build-queue` and `vox-cargo-shim/Cargo.toml:44` `[VERIFIED: crates/vox-cargo-shim/Cargo.toml:44]`. |
| `libc` (`cfg(unix)`) | workspace | `getloadavg` (`global.rs:193`) and `killpg(pid, SIGKILL)` for timeout process-group cleanup | Already declared under `[target.'cfg(unix)'.dependencies]` in `vox-build-queue`. |

### Supporting Host Tools
| Tool | Version on Host | Purpose | When to Use |
|------|-----------------|---------|-------------|
| `sccache` | `0.19.0` | Content-addressed `rustc` object cache across `target/ws-*` and worktrees | Auto-injected as `RUSTC_WRAPPER` by `vox-cargo-shim` when unset and found on `PATH`. |
| `vox-broker` | `0.6.0` | Read-only status/stats/log inspector for `~/.vox/build-broker` | Inspecting `target_wait_ms`, `slot_wait_ms`, `compile_ms`, and `coalesce_rate`. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| File-backed leader-follower coalescing (`coalesce.rs`) | Background persistent socket daemon | Rejected (`447-CONTEXT.md` `<deferred>`): daemon lifecycle/crash recovery adds complexity; lockfile + atomic `.exit` file is daemonless and crash-safe via OS kernel handle cleanup. |
| Separate `.lock` + `.meta`/`.out`/`.exit` files | Reading/writing the locked `fs2` file directly | Rejected: on Windows, `LockFileEx` causes `ReadFile` on any second handle to fail with `os error 33` (`ERROR_LOCK_VIOLATION`). Separating zero-byte `.lock` from unlocked `.meta`/`.out`/`.exit` eliminates `os error 33` completely. |
| Extending `vox ci dev-loop-guard` | Adding a brand-new `vox ci` subcommand | Extending `dev-loop-guard.rs` runs automatically in pre-push and unit tests (`the_repository_passes_the_dev_loop_guard`) with zero `command-registry.yaml` / `cli-command-surface.generated.md` SSOT churn. |

---

## Architecture Patterns

### System Architecture Diagram

```text
Agent / CLI invokes `cargo <args>`
        │
        ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ vox-cargo-shim (`src/main.rs`)                                            │
│  1. Recursion guard (`VOX_BROKER_DEPTH >= 2` -> exit 70)                  │
│  2. Strip shim-only flags (`--broker-run-timeout[=]<secs>`)               │
│  3. Resolve `real_cargo` (`~/.cargo/bin/cargo` or `rustup which cargo`)   │
│  4. Non-build subcommand? -> `exec_real` immediately (zero overhead)      │
│  5. Inject child env overrides BEFORE fingerprinting:                     │
│     - `VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET=1` -> `CARGO_TARGET_DIR`    │
│     - Auto-detect `sccache` on `PATH` if `RUSTC_WRAPPER` unset            │
│  6. Resolve canonical `effective_target_dir` (`--target-dir` > env > wt)  │
│  7. Compute `coalesce_key = H(subcmd, run_cmd_args, env_hash, target)`    │
└───────┬───────────────────────────────────────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ Stage A: Leader-Follower Coalescing Check (`<broker_home>/inflight/`)     │
│  Eligible (`check` | `build` | `clippy` | pure `test` | `run`'s build)    │
│  AND active `<key>.lock` held with `leader_start_ms >= latest_src_mtime`? │
│   ├── YES (FOLLOWER):                                                     │
│   │     Hold ZERO `FairQueue` run locks & ZERO global `Slot`s!            │
│   │     Stream `<key>.out` bytes while polling `<key>.lock` release ->    │
│   │     Read atomic `<key>.exit` -> record `coalesced_hit=true`,          │
│   │     `target_wait_ms=waited`, `slot_wait_ms=0`, `compile_ms=0` -> DONE │
│   └── NO: Proceed to Stage B                                              │
└───────┬───────────────────────────────────────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ Stage B: Per-Target `FairQueue` (`<broker_home>/targets/<target_hash>/`)  │
│  1. `take_ticket(coalesce_key)` -> `acquire(seq)` (`run.lock`)            │
│  2. Measure `target_wait_ms` (waiters sleep HERE without holding a Slot!) │
│  3. Re-check `<key>.exit`: if an identical Leader started AFTER we took   │
│     our ticket (`leader_start_ms >= ticket_ts_ms` & `>= latest_src_mtime`)│
│     and succeeded (`exit == 0`), replay `.out` + `.exit` (`compile_ms=0`)!│
│  4. Otherwise claim `<key>.lock` as LEADER (truncate `.out`, write `.meta`)│
└───────┬───────────────────────────────────────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ Stage C: Machine-Wide Adaptive Semaphore (`<broker_home>/slots/slot_*`)   │
│  `global::acquire_slot_adaptive(&root, cap)` -> measure `slot_wait_ms`    │
└───────┬───────────────────────────────────────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────────────────────────────────────────────┐
│ Stage D: Active Compile/Test Execution + Post-Queue Timeout (`runner.rs`) │
│  `t_run = Instant::now()` STARTS NOW (queue wait excluded from deadline!) │
│  Spawn `real_cargo` (in own process group on Unix); tee stdout/stderr to  │
│  caller terminal AND `<key>.out` (if Leader). Poll `try_wait` (100ms):    │
│   - Exceeds `VOX_BROKER_RUN_TIMEOUT_SECS` -> kill process tree, exit `124`│
│   - Finishes -> write `<key>.exit` atomically, unlock `<key>.lock`,       │
│     drop `Slot`, drop `FairQueue::Ticket`, append `MetricRecord`          │
│   - If `cargo run` & build succeeded -> `exec_real` binary outside locks  │
└───────────────────────────────────────────────────────────────────────────┘
```

### Pattern 1: Canonical Effective `CARGO_TARGET_DIR` Resolution (`BUILD-CONTENTION-01`)

Cargo resolves its build directory in a strict precedence order. `vox_build_queue::resolve::effective_target_dir(cwd, args, child_env)` must match Cargo's precedence so two invocations locking the same `target/.cargo-lock` always map to the same `FairQueue` hash:

1. **CLI `--target-dir` flag (before `--`):** Scan `args.iter().take_while(|a| a.as_str() != "--")` for `--target-dir <p>` or `--target-dir=<p>`.
2. **Effective `CARGO_TARGET_DIR` environment variable:** Look up `CARGO_TARGET_DIR` in `child_env` (after applying any `VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET` override from Pattern 5), ignoring empty strings.
3. **Standalone `--manifest-path` workspace fallback:** If `--manifest-path <p>` points to a `Cargo.toml` that declares its own `[workspace]` table and has no `.cargo/config.toml` alongside it — specifically when `cwd` is outside a Vox worktree root — resolve `<manifest_dir>/target`. Note: when `cwd` IS inside `<repo_root>`, Cargo still reads `<repo_root>/.cargo/config.toml` (`[VERIFIED: .cargo/config.toml:95]`: `CARGO_TARGET_DIR = { value = "target", relative = true }`), which sets `CARGO_TARGET_DIR` to `<repo_root>/target`!
4. **Worktree `.cargo/config.toml` default:** Via `resolve::worktree_root_of(cwd)` (`[VERIFIED: crates/vox-build-queue/src/resolve.rs:24-29]`: `"pub fn worktree_root_of(start: &Path) -> Option<PathBuf>"`), resolve `<worktree_root>/target`.
5. **Fallback:** `<cwd>/target`.

**Non-existent directory canonicalization (critical for fresh `target/ws-<slug>`):** `std::fs::canonicalize` fails with `NotFound` if `target/` has not been created yet. Implement `normalize_lexical_and_canonical(base_cwd, raw_path)`:
- Resolve relative paths against `base_cwd`.
- Walk `.ancestors()` until finding the deepest existing directory, call `.canonicalize()` on that ancestor, and re-append the non-existent relative suffix components (stripping `.` and resolving `..`).
- Store `FairQueue` state under `<broker_home>/targets/<hash_path(&canonical_target)>/` (`root.join("targets")`), **never** inside `<worktree>/.vox/build-queue/`, so `git clean -fdx` cannot delete live queue locks and `vox_broker_readonly.rs` stays untouched. Also update `broker_has_run` in `crates/vox-cargo-shim/src/bin/vox-broker.rs:112-116` to recognize `root.join("targets").is_dir()`.

### Pattern 2: Windows `os error 33`-Safe Leader-Follower Coalescing (`BUILD-CONTENTION-01`)

#### Why Existing `register_inflight` Had a Subtle Windows Hazard & Passive-Only Behavior
In `crates/vox-build-queue/src/global.rs:252-278` (`[VERIFIED: crates/vox-build-queue/src/global.rs:269-277]`):
```rust
Err(_) => {
    // Entry is locked by an active process -> still in-flight.
    drop(f);
    if std::fs::read_to_string(&p)
        .map(|s| s.trim() == key)
        .unwrap_or(false)
    {
        coalesce = true;
    }
}
```
On Windows, `f.lock_exclusive()` locks bytes `0..u64::MAX` of `mine` (`fs2` calls Win32 `LockFileEx`), which causes `std::fs::read_to_string(&p)` from **another process** to fail with `ERROR_LOCK_VIOLATION` (`os error 33`) while the lock is held!

#### Zero-`os error 33` File Protocol (`<broker_home>/inflight/<coalesce_hash>.*`)
Separate the **lock handle** from **all readable/writable data files**:
- `<hash>.lock` — **0-byte lock-only file**. Leaders call `try_lock_exclusive()` and hold the handle until `.exit` is atomically renamed into place. **No process ever calls `read` or `write` on `<hash>.lock`.**
- `<hash>.meta` — Unlocked JSON file written atomically (`<hash>.meta.tmp` -> `fs::rename`) by Leader right after acquiring `<hash>.lock`: `{ "run_id": "<pid>-<ts_ms>", "started_ms": 1763..., "subcmd": "check", "target_dir": "..." }`.
- `<hash>.out` — Unlocked combined output log opened by Leader with `OpenOptions::new().create(true).write(true).truncate(true)` (on Windows `std::fs::OpenOptions` grants `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` by default, and because `fs2::lock_exclusive` is NEVER called on `.out`, followers can open `.out` read-only and stream appended bytes concurrently without `os error 33`!).
- `<hash>.exit` — Unlocked JSON completion receipt written atomically (`<hash>.exit.tmp` -> `fs::rename`) **before** Leader unlocks `<hash>.lock`: `{ "run_id": "<pid>-<ts_ms>", "started_ms": ..., "finished_ms": ..., "exit_code": 0 }`.

#### Coalescing Eligibility & Source-Mtime Freshness Guard
1. **Command eligibility (`is_coalesceable(sub, args, env)`):**
   - `check`, `build`, `clippy`: **Eligible**, EXCEPT when `--message-format` is passed (e.g. `scripts/broker-install.vox` parses structured JSON compiler messages on stdout; keeping `--message-format` uncoalesced avoids mixing stdout/stderr JSON streams, or store `.stdout` and `.stderr` separately — storing `<hash>.stdout` and `<hash>.stderr` separately is trivial with two reader threads and preserves exact stream fidelity even for stdout consumers!).
   - `test`: **Eligible** ONLY when:
     - Args after `--` contain no snapshot/fixture-mutating flags (`--nocapture` and test name filters are already part of `argv_hash` and safe; skip coalescing if env contains `INSTA_UPDATE`, `UPDATE_EXPECT`, `TRYBUILD=overwrite`, or `SNAPSHOT_UPDATE`).
     - No workspace source file modified after `leader.started_ms` (`latest_workspace_mtime_ms(&worktree) <= leader.started_ms`).
   - `run`: `resolve::build_args_for_run(args)` (`[VERIFIED: crates/vox-build-queue/src/resolve.rs:76-83]`) already extracts `["build", ...flags_before_dashdash]`. Coalesce **only** that `build` phase (under key `("build", build_args, env_hash, effective_target)`), then execute `exec_real` for the binary outside the broker as `main.rs:220-226` already does!
   - `bench`: **Never coalesce** (benchmarks measure timing).
2. **Fast Workspace Mtime Check (`latest_workspace_mtime_ms`):**
   - Walk `crates/` (and root `Cargo.toml` / `.cargo/config.toml`), skipping `target`, `.git`, `node_modules`, `.sccache`, `.planning`, `docs` — or scan `Cargo.toml` + `src/`/`tests/`/`benches/`/`build.rs` mtimes (~3–8 ms for ~90 crates on macOS/Windows SSD). If any file has `mtime_ms > leader.started_ms`, the follower has newer edits than the leader saw and **must not** coalesce onto the stale leader!
3. **Both In-Flight Attachment AND Queued-Batch Coalescing:**
   - **Case 1 (Direct In-Flight Follower — Stage A):** Leader is currently holding `<hash>.lock` and `leader.started_ms >= latest_src_mtime`. Follower tails `<hash>.stdout` / `<hash>.stderr` every `50ms` until `<hash>.lock` can be probed (`try_lock_exclusive` -> immediately `unlock`), flushes remaining bytes, reads `<hash>.exit` matching `meta.run_id`, and returns `exit_code`. If Leader crashed without writing matching `<hash>.exit`, Follower cleanly falls through to Stage B and runs Cargo itself!
   - **Case 2 (Queued Follower Behind Earlier Build — Stage B):** Build X (different crate) holds `FairQueue` while Agents B and C both queue identical `cargo test -p vox-terminal-core` tickets (`ticket_ts_ms`). When X finishes, B wakes first, becomes Leader (`b.started_ms >= c.ticket_ts_ms`), compiles, writes `<hash>.exit`, and releases `FairQueue`. C wakes 1ms later: seeing `<hash>.exit` with `b.started_ms >= c.ticket_ts_ms` AND `b.started_ms >= latest_src_mtime`, C replays `<hash>.stdout`/`.stderr` and returns `b.exit_code` in `0 ms` without acquiring a global slot or invoking Cargo!

### Pattern 3: Backward-Compatible `MetricRecord` & Honest `vox-broker` Telemetry (`BUILD-CONTENTION-01`)

Extend `MetricRecord` in `crates/vox-build-queue/src/metrics.rs:11-21` using `#[serde(default)]` so all `10,021+` historical rows in `~/.vox/build-broker/metrics.jsonl` deserialize cleanly:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricRecord {
    pub ts_ms: u128,
    pub worktree: String,
    pub subcmd: String,
    /// Total wait (`target_wait_ms + slot_wait_ms`) preserved for backward compatibility.
    pub queue_wait_ms: u64,
    /// Total post-queue runtime (`compile_ms` for new rows; historical total `ran_ms` for old rows).
    pub ran_ms: u64,
    pub argv_hash: u64,
    pub env_hash: u64,
    pub would_coalesce: bool,
    #[serde(default)]
    pub target_wait_ms: u64,
    #[serde(default)]
    pub slot_wait_ms: u64,
    #[serde(default)]
    pub compile_ms: u64,
    #[serde(default)]
    pub coalesced_hit: bool,
    #[serde(default)]
    pub timed_out: bool,
}
```

- **Preserving `vox-broker` test contracts:** `crates/vox-cargo-shim/src/bin/vox-broker.rs:249` tests `assert!(out.contains("builds=1"), "got: {out}")`. Keep `Summary::render()` starting with `"builds={} p50_wait={}ms p95_wait={}ms target_p95={}ms slot_p95={}ms coalesce_rate={:.1}% coalesced={} -> {}"` so existing substring assertions pass while surfacing separate `target_p95` and `slot_p95` percentiles!
- **`broker.log` format:** Include `wait={queue_wait_ms}ms target_wait={target_wait_ms}ms slot_wait={slot_wait_ms}ms compile={compile_ms}ms ahead={busy} cap={n} coalesce={would_coalesce} coalesced={coalesced_hit} exit={code}`.

### Pattern 4: Complete `vox-orchestrator` Feature Hygiene & `vox-terminal-core` Decoupling (`BUILD-CONTENTION-02`)

#### Exhaustive Audit of `vox-orchestrator` (`5` Exact Locations Found)
Static inspection across all files in `crates/vox-orchestrator/src/` and `crates/vox-orchestrator/tests/` found **5** compile blockers under `--no-default-features` and `--no-default-features --features runtime` (2 named in `447-CONTEXT.md` + **3 hidden blockers discovered during research**):

1. **`crates/vox-orchestrator/src/victory.rs:51` (`toestub-gate` blocker):**
   - Current code (`[VERIFIED: crates/vox-orchestrator/src/victory.rs:48-55]`):
     ```rust
     pub fn evaluate(
         &self,
         task_id: &str,
         validation: &crate::validation::ValidationResult,
         observation: Option<&ObservationReport>,
         socrates_score: Option<f32>,
     ) -> VictoryVerdict {
     ```
   - `pub mod validation;` in `src/lib.rs:251` is gated on `#[cfg(feature = "toestub-gate")]`. Workspace-wide `rg` confirmed `VictoryEvaluator` has **zero** callers outside `victory.rs`.
   - **Fix:** Gate `VictoryEvaluator::evaluate` (or `impl VictoryEvaluator` / module usage of `crate::validation::ValidationResult`) behind `#[cfg(feature = "toestub-gate")]`.

2. **`crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs:216-256` (`toestub-gate` blocker):**
   - Current code (`[VERIFIED: crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs:226-234]`):
     ```rust
     let cfg = vox_code_audit::ToestubConfig {
         roots: vec![root.to_path_buf()],
         min_severity: vox_code_audit::Severity::Info,
         format: vox_code_audit::OutputFormat::Json,
         run_mode: vox_code_audit::ToestubRunMode::Audit,
         ..Default::default()
     };
     let engine = vox_code_audit::ToestubEngine::new(cfg);
     ```
   - **Fix:** Gate the `vox_code_audit` block in `handle_ai_check` with `#[cfg(feature = "toestub-gate")]`, and under `#[cfg(not(feature = "toestub-gate"))]` return `response_err(id, "ai.check requires feature `toestub-gate`")` (after validating `params.path` or immediately so `root` does not warn unused).

3. **HIDDEN BLOCKER #1 — `crates/vox-orchestrator/src/route_policy.rs:6-8, 25-28` (`runtime` blocker under `--no-default-features`):**
   - Current code (`[VERIFIED: crates/vox-orchestrator/src/route_policy.rs:6-8, 25-28]`):
     ```rust
     use vox_actor_runtime::route_capability_policy::{
         RouteCapabilityPolicySnapshot, exclusion_reason_for_llm_lane,
     };
     ...
     pub fn route_policy_exclusion_reason(model: &ModelSpec) -> Option<&'static str> {
         let snap = RouteCapabilityPolicySnapshot::from_env();
         exclusion_reason_for_llm_lane(is_local_http_provider(&model.provider_type), &snap)
     }
     ```
   - `vox-actor-runtime` is `optional = true` in `[dependencies]` (`[VERIFIED: crates/vox-orchestrator/Cargo.toml:89]`: `"vox-actor-runtime = { workspace = true, optional = true }"`), enabled only by `runtime`! Under `cargo check -p vox-orchestrator --no-default-features`, `route_policy.rs:6` fails to compile!
   - **Fix:** Gate `use vox_actor_runtime::...` on `#[cfg(feature = "runtime")]`, and under `#[cfg(not(feature = "runtime"))]` evaluate the exact same 4 env vars (`VOX_ROUTE_POLICY_PROFILE`, `VOX_ROUTE_ALLOW_NET`, `VOX_ROUTE_ALLOW_PROVIDER_NETWORK`, `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP` — verified in `crates/vox-actor-runtime/src/route_capability_policy.rs:18-64` as 35 lines of pure `std::env::var` logic) so `route_policy_exclusion_reason` never fails-open even without `runtime`!

4. **HIDDEN BLOCKER #2 — `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs:12, 25` (`--no-default-features --features runtime` blocker):**
   - Current code (`[VERIFIED: crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs:12, 25]`):
     ```rust
     #[cfg(feature = "runtime")]
     {
         ...
         let diagnostics = vox_lsp::validate_document(src);
     ```
   - In `Cargo.toml:51-52` (`[VERIFIED: crates/vox-orchestrator/Cargo.toml:51-52]`):
     ```toml
     runtime = ["vox-actor-runtime", "vox-populi/mens", "lsp"]
     toestub-gate = ["vox-code-audit", "vox-lsp", "lsp"]
     ```
   - `runtime` enables `lsp` (`tower-lsp-server`), **NOT `vox-lsp`**! Thus `cargo check -p vox-orchestrator --no-default-features --features runtime` (which is also how `vox-orchestrator-mcp --no-default-features` compiles `vox-orchestrator`!) fails with `undeclared crate vox_lsp`!
   - **Fix:** Change `healing.rs:12` from `#[cfg(feature = "runtime")]` to `#[cfg(all(feature = "runtime", feature = "toestub-gate"))]` and bind `let _ = (agent_id, task_desc, write_files);` under `#[cfg(not(all(feature = "runtime", feature = "toestub-gate")))]` to prevent unused-variable warnings.

5. **HIDDEN BLOCKER #3 — Integration Tests Missing `#![cfg(feature = "runtime")]` under `cargo test -p vox-orchestrator --no-default-features`:**
   - `crates/vox-orchestrator/tests/scaling_test.rs:5, 22` (`[VERIFIED: crates/vox-orchestrator/tests/scaling_test.rs:5]`: `"use vox_orchestrator::runtime::AgentFleet;"`) uses `vox_orchestrator::runtime`, which is gated on `#[cfg(feature = "runtime")]` in `src/lib.rs`.
   - `crates/vox-orchestrator/tests/context_compaction_wiring_test.rs:101` (`[VERIFIED: crates/vox-orchestrator/tests/context_compaction_wiring_test.rs:101]`: `".assemble_llm_messages(&session_id, &engine)"`) calls `SessionManager::assemble_llm_messages`, which is gated on `#[cfg(feature = "runtime")]`.
   - **Fix:** Add `#![cfg(feature = "runtime")]` at line 1 of both `tests/scaling_test.rs` and `tests/context_compaction_wiring_test.rs` (matching the existing convention in `tests/chat_round_trip.rs`).

#### `vox-terminal-core` Feature Decoupling & `caps.rs:710` Invariant
1. **`crates/vox-terminal-core/Cargo.toml` (`[VERIFIED: crates/vox-terminal-core/Cargo.toml:8-26]`):**
   - Change `vox-lang = []` to `vox-lang = ["dep:vox-compiler"]`.
   - Change `vox-compiler = { workspace = true }` to `vox-compiler = { workspace = true, optional = true }` (do **NOT** add `vox-compiler` to `[dev-dependencies]`, otherwise `cargo test -p vox-terminal-core` still compiles `47k-LoC` `vox-compiler` on every test run!).
   - Change `vox-orchestrator = { workspace = true }` to `vox-orchestrator = { workspace = true, default-features = false }`.
   - Verified every `vox_orchestrator` import in `vox-terminal-core`:
     - `src/agent.rs:10-15`: `events::{AgentEvent, AgentEventKind}`, `models::{SelectionAxes, SelectionIntent, TaskCategory, infer_prompt_intents, select_with_default_registry}`, `types::AgentId` — all unconditional in `vox-orchestrator`.
     - `src/daemon_host.rs:11` (`[VERIFIED: crates/vox-terminal-core/src/daemon_host.rs:11]`): `use vox_orchestrator::orch_daemon::OrchDaemonClient;` — unconditional in `vox-orchestrator`.
   - **Why this stops feature-fingerprint ping-pong:** `vox-orchestrator-mcp` (`[VERIFIED: crates/vox-orchestrator-mcp/Cargo.toml:47]`: `vox-orchestrator = { path = "../vox-orchestrator", default-features = false, features = ["runtime"] }`) compiles `vox-corpus` with `[]` (no `database`) and `vox-vcs` with `[]` (no `jj`). Switching `vox-terminal-core` to `default-features = false` aligns `vox-corpus` (`[]`) and `vox-vcs` (`[]`) across both crates AND eliminates `vox-populi`, `vox-code-audit`, `vox-lsp`, `vox-actor-runtime`, and `jj-lib` from `vox-terminal-core` and `vox-term` builds!
2. **CRITICAL Cross-Crate Test Guard in `vox-compiler/src/eval/caps.rs:700-728`:**
   - `[VERIFIED: crates/vox-compiler/src/eval/caps.rs:709-726]`:
     ```rust
     (
         "crates/vox-terminal-core/src/vox_interp.rs",
         "CapabilitySet::parse",
     ),
     ...
     assert!(
         src.contains("Interpreter::new") && src.contains(needle),
         "{path} missing explicit caps ({needle})"
     );
     ```
   - `vox-compiler`'s unit test reads `crates/vox-terminal-core/src/vox_interp.rs` as raw text and asserts both `"Interpreter::new"` and `"CapabilitySet::parse"` appear in that file!
   - **Fix:** Keep `pub mod vox_interp;` always compiled in `crates/vox-terminal-core/src/lib.rs`, keep the `#[cfg(feature = "vox-lang")] pub fn eval_line(src: &str) -> Result<String>` implementation in `src/vox_interp.rs` (preserving both literal substrings!), and add a `#[cfg(not(feature = "vox-lang"))] pub fn eval_line(_src: &str) -> Result<String>` stub returning `Err(anyhow!("inline .vox evaluation requires vox-terminal-core feature `vox-lang`"))`.
   - Because `Session::submit_with_origin` (`src/session.rs:323-339`) and `LocalHost::submit_with_origin` (`[VERIFIED: crates/vox-terminal-core/src/host.rs:813-838]`) already catch `Err(_)` from `eval_line`:
     - Non-explicit inputs (like `ls -la` or `echo 42`) automatically fall back to shell PTY execution!
     - Explicit `.vox` inputs (`fn `, `let `, `type `, `import `, `@`) cleanly emit a `VoxNative` stderr block with exit code `1`!
   - Gate `crates/vox-terminal-core/tests/vox_interp.rs` with `#![cfg(feature = "vox-lang")]` and gate the `"40 + 2"` `VoxNative` assertion in `crates/vox-terminal-core/tests/session.rs` on `#[cfg(feature = "vox-lang")]` (adding a `#[cfg(not(feature = "vox-lang"))]` test asserting explicit `let x = 1` returns exit `1` cleanly).
3. **Downstream `vox-terminal-core` Consumers (All 3 Verified):**
   - `crates/vox-term/Cargo.toml:17`: Leave `vox-terminal-core = { workspace = true }` (lean default; optionally add `vox-lang = ["vox-terminal-core/vox-lang"]` feature to `vox-term`).
   - `crates/vox-orchestrator-d/Cargo.toml:19`: Enable `vox-terminal-core = { workspace = true, features = ["vox-lang"] }` (`vox-orchestrator-d` already compiles `vox-compiler` via `vox-orchestrator-mcp` -> `vox-skills`, so `vox-lang` adds 0 extra crates while keeping daemon-hosted `Console` `.vox` evaluation live!).
   - `crates/vox-gui/Cargo.toml:62`: Enable `vox-terminal-core = { workspace = true, features = ["vox-lang"] }` (`vox-gui` already links `vox-cli` -> `vox-compiler`, so 0 extra crates).

### Pattern 5: Dynamic `sccache` Auto-Discovery & `VOX_WORKSTREAM` Target Isolation (`BUILD-CONTENTION-03`)

1. **Zero-Config `sccache` Wrapper Injection (`resolve::resolve_sccache_wrapper`):**
   - Check child `env` / caller `std::env`:
     - If `VOX_BROKER_NO_SCCACHE` is truthy (`1`/`true`/`yes`) -> do nothing.
     - If `RUSTC_WRAPPER` is already set in caller env (even to `""`, which is standard Rust convention for explicit opt-out!) -> respect caller's `RUSTC_WRAPPER` untouched.
     - Otherwise scan `PATH` via `std::env::split_paths` for `sccache` (`sccache.exe` on Windows; verify regular file + executable mode `0o111` on Unix, and skip `own_dir` for symmetry with `resolve_real_cargo`).
     - If found, insert `RUSTC_WRAPPER = <sccache_path>` into `env` **before** calling `env_filter::env_hash(&env)` so `env_hash` reflects the true compiler wrapper (`RUSTC_WRAPPER` is in `FINGERPRINT_ALLOW`, `[VERIFIED: crates/vox-build-queue/src/env_filter.rs:39]`: `"\"RUSTC_WRAPPER\","`).
2. **Optional Per-Workstream Target Isolation (`resolve::apply_workstream_target_isolation`):**
   - Triggered ONLY when caller did NOT pass `--target-dir` in `args` AND caller did NOT explicitly set `CARGO_TARGET_DIR` in process env:
     - Check `VOX_WORKSTREAM` (when `VOX_ISOLATED_TARGET=1`, OR whenever `VOX_WORKSTREAM` is non-empty and `VOX_ISOLATED_TARGET != "0"` — **Important:** guard against breaking `scripts/broker-ci.vox`, see Pitfall 3 below: skip target-dir injection when `--manifest-path` points outside the root workspace members, e.g. `crates/vox-cargo-shim/Cargo.toml`!) OR read `<worktree_root>/.vox/agent-target` if present.
     - Sanitize slug strictly to ASCII `[A-Za-z0-9_-]` (max 48 chars; reject empty after trim/sanitize to prevent path traversal).
     - Set `CARGO_TARGET_DIR = <worktree_root>/target/ws-<slug>` in child `env` **before** `effective_target_dir` and `env_hash`.
   - **Why `<worktree_root>/target/ws-<slug>` is the ideal path:**
     - Already gitignored by `/target/` (`.gitignore:2`).
     - Automatically pruned by `vox-arch-check` (`walk_prune` skips `target`) and TOESTUB.
     - Shares the checkout's absolute source paths, allowing `sccache` (`SCCACHE_DIR = ".sccache"`) to share compiled `.rlib`/`.rmeta` artifacts across `target/ws-a` and `target/ws-b` (`CARGO_INCREMENTAL=0` can also be injected when `CARGO_INCREMENTAL` is unset AND both isolated target + `sccache` are active, because Cargo's `incremental = true` dev default bypasses `sccache` for workspace crates!).
3. **`vox ci dev-loop-guard` Extension (`crates/vox-cli-ci/src/dev_loop_guard.rs`):**
   - Add pure, build-free checks in `dev_loop_guard.rs`:
     1. `hardcoded_rustc_wrapper(cargo_config)`: reject committed `rustc-wrapper = ...` under `[build]` or `RUSTC_WRAPPER` under `[env]` in `.cargo/config.toml` (enforcing `.cargo/config.toml:15-17` in code).
     2. `missing_shared_cache_or_target_env(cargo_config)`: verify `.cargo/config.toml` retains `SCCACHE_DIR` and `CARGO_TARGET_DIR = { value = "target", relative = true }`.
     3. `broker_timeout_and_batching_docs_present(root)`: verify `docs/src/contributors/build-broker-usage.md` and `.agents/cargo-safety.md` document `VOX_BROKER_RUN_TIMEOUT_SECS` and batched multi-package `cargo test -p <a > -p <b>` verification.

### Pattern 6: Post-Queue Active-Runtime Timeout (`BUILD-CONTENTION-04`)

1. **Configuration Surface (Shim CLI + Env):**
   - CLI flag stripped by `vox-cargo-shim` before invoking real `cargo` (and before `argv_hash`): `--broker-run-timeout <secs>` or `--broker-run-timeout=<secs>`.
   - Env vars (in precedence order): `VOX_BROKER_RUN_TIMEOUT_SECS` -> `VOX_BROKER_BUILD_TIMEOUT_SECS` (supporting both the `447-CONTEXT.md` name and the `REQUIREMENTS.md` alias!). Value `0` or unset disables the timeout. Strip `VOX_BROKER_RUN_TIMEOUT_SECS` / `VOX_BROKER_BUILD_TIMEOUT_SECS` from `DENYLIST` or leave out of `FINGERPRINT_ALLOW` so changing timeout does not invalidate coalescing hashes!
2. **Clock Boundary & Process-Tree Termination (`crates/vox-cargo-shim/src/runner.rs`):**
   - Start `let t_run = Instant::now();` **strictly after** `FairQueue::acquire` AND `global::acquire_slot_adaptive` have both succeeded!
   - Spawn child `cargo` in its own process group:
     - Unix (`#[cfg(unix)]`): `use std::os::unix::process::CommandExt; cmd.process_group(0);`
     - Windows (`#[cfg(windows)]`): keep `CREATE_NO_WINDOW` (`0x0800_0000`).
   - Poll `child.try_wait()?` every `100ms`:
     - If `t_run.elapsed() >= deadline`:
       - On Unix: `unsafe { libc::killpg(child.id() as libc::pid_t, libc::SIGKILL); }` followed by `let _ = child.kill(); let _ = child.wait();` so child `rustc`, `lld`, and test binaries immediately release `target/.cargo-lock` and output pipe FDs!
       - On Windows: run `Command::new("taskkill").args(["/T", "/F", "/PID", &child.id().to_string()]).status()` (best-effort) + `let _ = child.kill(); let _ = child.wait();` so grand-child `.exe` test binaries never linger holding locked files in `target/`!
       - Emit clear diagnostic to stderr: `eprintln!("vox-broker: build timed out after {secs}s of active runtime (queue wait {queue_wait_ms}ms excluded); exiting 124");`
       - Do **not** publish a coalescing `.exit` receipt for a timed-out build (or write `exit_code: 124, timed_out: true` and prevent post-completion reuse).
       - Record `MetricRecord` with `timed_out: true`, `compile_ms`, `target_wait_ms`, `slot_wait_ms`, and return exit code **`124`**.

### Anti-Patterns to Avoid
- **Appending code to `crates/vox-build-queue/src/global.rs`:** `global.rs` is already **483 lines** (`17` lines below the 500-LoC `god_object` audit gate). Keep `global.rs` untouched or slimmer; place coalescing in `crates/vox-build-queue/src/coalesce.rs`.
- **Reading or writing a file while another handle holds `fs2::lock_exclusive` on that same path:** Triggers `os error 33` on Windows. Always lock a dedicated 0-byte `.lock` file and read/write unlocked sibling `.meta`/`.stdout`/`.stderr`/`.exit` files.
- **Putting `FairQueue` directories inside `<worktree>/target/` or `<worktree>/.vox/`:** `cargo clean` wipes `target/` and `git clean -fdx` wipes untracked `.vox/` subdirectories. Keep all broker runtime locks under `<broker_home>/targets/<target_hash>/`.
- **Computing `env_hash` BEFORE injecting `RUSTC_WRAPPER` and `CARGO_TARGET_DIR`:** Both keys are in `env_filter::FINGERPRINT_ALLOW` (`env_filter.rs:39, 43`). Mutate child `env` first, then compute `env_hash(&env)` and `effective_target_dir`.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Per-target FIFO lock + stale marker reaping | Custom spinlock or PID-file parser | `vox_build_queue::queue::FairQueue` (`crates/vox-build-queue/src/queue.rs:34-160`) | Already implements `counter`, `t<seq>` arrival markers, 4-hour stale marker reaping, and Windows single-handle lock safety, with passing unit tests. |
| Build environment fingerprinting | Full-env hashing or custom regex denylist | `vox_build_queue::env_filter::{passthrough_env, argv_hash, env_hash}` (`env_filter.rs:34-90`) | Uses a strict `FINGERPRINT_ALLOW` allowlist so volatile IDE vars (`WT_SESSION`, `VSCODE_*`, `TEMP`) never fragment coalescing keys. |
| `cargo run` build-vs-exec splitting | Coalescing or holding slots across `cargo run` binary execution | `vox_build_queue::resolve::build_args_for_run` (`resolve.rs:76-83`) | Already splits `cargo run <flags> -- <args>` into `cargo build <flags>` (queued/coalesced) followed by lock-free `exec_real` for the binary. |
| Route capability policy under `--no-default-features` | Returning `None` (fail-open security regression!) | Mirror the 4 `VOX_ROUTE_*` env checks from `vox_actor_runtime::route_capability_policy` (`route_capability_policy.rs:18-64`) | Preserves identical `route_policy:network_disabled` / `provider_network_disabled` / `local_model_http_disabled` enforcement when `runtime` feature is disabled. |

---

## Runtime State Inventory

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| **Stored data** | `~/.vox/build-broker/metrics.jsonl` (existing `10,021+` rows with 8 fields) and `~/.vox/build-broker/broker.log`. | **Backward-compatible schema evolution only:** add `#[serde(default)]` on new `MetricRecord` fields (`target_wait_ms`, `slot_wait_ms`, `compile_ms`, `coalesced_hit`, `timed_out`) so `vox-broker stats` reads historical + new lines without migration or truncation. |
| **Live service config** | Login shell profile block `# >>> vox build broker >>>` in `~/.zshrc` / `~/.bashrc` / `~/.profile` managed by `scripts/broker-install.vox`. | No breaking change to PATH block format; re-running `vox run scripts/broker-install.vox -- --apply` after Phase 447 refreshes `~/.vox/build-broker/bin/{cargo,vox-broker}` atomically. |
| **OS-registered state** | Ephemeral advisory/mandatory file locks under `~/.vox/build-broker/{slots,inflight,targets}` (released automatically by OS kernel on process exit). | Add `~/.vox/build-broker/targets/<target_hash>/` for `FairQueue` locks and `<hash>.{lock,meta,stdout,stderr,exit}` under `inflight/` with automatic stale reaping on next `try_lock_exclusive`. |
| **Secrets / env vars** | No secrets (`vox-secrets` / Clavis unaffected). New non-secret tuning env vars: `VOX_BROKER_RUN_TIMEOUT_SECS` (alias `VOX_BROKER_BUILD_TIMEOUT_SECS`), `VOX_BROKER_NO_SCCACHE`, `VOX_WORKSTREAM`, `VOX_ISOLATED_TARGET`. | Document in `docs/src/contributors/build-broker-usage.md` (note: `docs/src/reference/env-vars.generated.md` is auto-generated from workspace `vox-config` / registry, NOT hand-edited per `AGENTS.md`). |
| **Build artifacts** | `~/.vox/build-broker/bin/cargo` and `~/.vox/build-broker/bin/vox-broker` installed binaries; optional `<repo>/target/ws-<slug>/` isolated target dirs. | Rebuild via `scripts/broker-ci.vox` during verification; operator refreshes live `~/.vox/build-broker/bin/` via `vox run scripts/broker-install.vox -- --apply`. |

---

## Common Pitfalls

### Pitfall 1: Breaking `vox-compiler/src/eval/caps.rs:700-728` When Gating `vox-terminal-core/src/vox_interp.rs`
- **What goes wrong:** Deleting `vox_interp.rs`, moving its implementation elsewhere, or rewriting `Interpreter::new` / `CapabilitySet::parse` causes `cargo test -p vox-compiler --lib eval::caps::tests::production_embedders_assign_caps_explicitly` to panic!
- **Why it happens:** `caps.rs:710` reads `crates/vox-terminal-core/src/vox_interp.rs` directly from disk and asserts `src.contains("Interpreter::new") && src.contains("CapabilitySet::parse")` (`[VERIFIED: crates/vox-compiler/src/eval/caps.rs:709-727]`).
- **How to avoid:** Keep `src/vox_interp.rs` in place with `#[cfg(feature = "vox-lang")] pub fn eval_line(...)` containing the exact existing body alongside a `#[cfg(not(feature = "vox-lang"))] pub fn eval_line(...)` `Err(...)` fallback.

### Pitfall 2: Windows `os error 33` (`ERROR_LOCK_VIOLATION`) on Coalescing Files
- **What goes wrong:** Leader locks `<hash>.out` or `<hash>.meta` with `fs2::lock_exclusive()`, and Follower calls `std::fs::File::open` + `read` on Windows -> `Err(Os { code: 33, kind: Uncategorized, message: "The process cannot access the file because another process has locked a portion of the file." })`.
- **Why it happens:** Win32 `LockFileEx` is a mandatory byte-range lock (`0..u64::MAX`), unlike Unix advisory `flock`.
- **How to avoid:** Call `fs2` lock methods **exclusively** on `<hash>.lock` (0 bytes, never read or written). Keep `<hash>.meta`, `<hash>.stdout`, `<hash>.stderr`, and `<hash>.exit` completely unlocked, using temp-file + `std::fs::rename` for `.meta` and `.exit`.

### Pitfall 3: `scripts/broker-ci.vox` Binary Path Check vs `CARGO_TARGET_DIR` / `VOX_WORKSTREAM`
- **What goes wrong:** `scripts/broker-ci.vox:119-125` (`[VERIFIED: scripts/broker-ci.vox:119-125]`) checks `env.get("CARGO_TARGET_DIR")`, falling back to `"crates/vox-cargo-shim/target/release/cargo"`. However, when `cargo build --manifest-path crates/vox-cargo-shim/Cargo.toml --release` runs from `<repo_root>` with `CARGO_TARGET_DIR` unset, Cargo reads `<repo_root>/.cargo/config.toml` (`CARGO_TARGET_DIR = { value = "target", relative = true }`) and places the binary in `<repo_root>/target/release/cargo` — OR if `VOX_WORKSTREAM` silently rewrote `CARGO_TARGET_DIR` inside the shim, `broker-ci.vox` wouldn't know where the binary went!
- **How to avoid:**
  1. Do **not** auto-rewrite `CARGO_TARGET_DIR` for `VOX_WORKSTREAM` unless `VOX_ISOLATED_TARGET=1` is also set (or skip `VOX_WORKSTREAM` target override when `--manifest-path` points to an excluded standalone `[workspace]` like `crates/vox-cargo-shim/Cargo.toml`).
  2. In `scripts/broker-ci.vox:119-126`, check `CARGO_TARGET_DIR` if set, else check `"crates/vox-cargo-shim/target"` AND fallback `"target"` (matching `.cargo/config.toml:95`) so `vox run scripts/broker-ci.vox` succeeds cleanly from `<repo_root>`!

### Pitfall 4: `.agents/rules/00-core.md` Is Gitignored (`/.agents/` at `.gitignore:305`)
- **What goes wrong:** Attempting `git add .agents/rules/00-core.md` during commit fails because `.gitignore:305` ignores `/.agents/`, whereas `.agents/cargo-safety.md` IS force-tracked in git (`git ls-files .agents` -> `.agents/cargo-safety.md`, `.agents/toestub-check.md`).
- **How to avoid:** Update `.agents/rules/00-core.md` locally for the active `drive-task` harness (per `447-CONTEXT.md`), AND commit the durable versioned policy to tracked `docs/src/contributors/build-broker-usage.md` and `.agents/cargo-safety.md` (reminding the orchestrator commit step not to `git add` gitignored `.agents/rules/00-core.md`).

### Pitfall 5: `sccache` Cache Misses Under Default `CARGO_INCREMENTAL`
- **What goes wrong:** `sccache` is set as `RUSTC_WRAPPER`, yet `sccache -s` shows `Non-cacheable reasons: incremental` for workspace crates during `cargo check` / `cargo test`.
- **Why it happens:** Cargo enables incremental compilation (`-C incremental=...`) by default for `dev`/`test` profiles, and `sccache` refuses to cache incremental `rustc` invocations (only caching non-incremental `deps/` crates unless `CARGO_INCREMENTAL=0`).
- **How to avoid:** When `vox-cargo-shim` activates an **isolated workstream target** (`target/ws-<slug>`) AND `sccache` is active AND `CARGO_INCREMENTAL` is unset in caller env, also set `CARGO_INCREMENTAL=0` so workspace crates share `.sccache` across `target/ws-a` and `target/ws-b` (`91.9%` hit rate measured in `docs/src/ci/shared-compile-cache.md`). For standard single-target builds (`target/`), leave `CARGO_INCREMENTAL` untouched so Cargo's native incremental cache keeps 1-file edits sub-second.

---

## Code Examples

### 1. Non-Existent-Safe Canonical Target Dir Resolution (`crates/vox-build-queue/src/resolve.rs`)
```rust
/// Canonicalize `path` even if its leaf directories (`target/ws-foo`) do not exist yet.
pub fn normalize_target_path(cwd: &Path, raw: &Path) -> PathBuf {
    let abs = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        cwd.join(raw)
    };
    let mut missing = Vec::new();
    let mut cur: &Path = &abs;
    while !cur.exists() {
        if let Some(name) = cur.file_name() {
            missing.push(name.to_os_string());
        }
        match cur.parent() {
            Some(p) if !p.as_os_str().is_empty() => cur = p,
            _ => break,
        }
    }
    let mut base = cur.canonicalize().unwrap_or_else(|_| cur.to_path_buf());
    for comp in missing.into_iter().rev() {
        base.push(comp);
    }
    base
}

/// Resolve effective `CARGO_TARGET_DIR` matching Cargo's precedence:
/// `--target-dir` CLI flag > `CARGO_TARGET_DIR` in child env > `<worktree_root>/target` > `<cwd>/target`.
pub fn effective_target_dir(
    cwd: &Path,
    args: &[String],
    child_env: &std::collections::BTreeMap<String, String>,
) -> PathBuf {
    let mut iter = args.iter().take_while(|a| a.as_str() != "--");
    while let Some(arg) = iter.next() {
        if arg == "--target-dir" {
            if let Some(val) = iter.next() {
                return normalize_target_path(cwd, Path::new(val));
            }
        } else if let Some(val) = arg.strip_prefix("--target-dir=") {
            return normalize_target_path(cwd, Path::new(val));
        }
    }
    if let Some(val) = child_env.get("CARGO_TARGET_DIR").filter(|v| !v.is_empty()) {
        return normalize_target_path(cwd, Path::new(val));
    }
    let base = worktree_root_of(cwd).unwrap_or_else(|| cwd.to_path_buf());
    normalize_target_path(&base, Path::new("target"))
}
```

### 2. Post-Queue Child Runner with Output Tee & Timeout (`crates/vox-cargo-shim/src/runner.rs`)
```rust
pub struct RunOutcome {
    pub exit_code: i32,
    pub compile_ms: u64,
    pub timed_out: bool,
}

/// Spawn `cmd` AFTER FairQueue + Slot acquisition; enforce `timeout_secs` on active runtime only.
/// When `tee_files` is `Some((stdout_path, stderr_path))`, stream child stdout/stderr to both
/// the caller's stdio and the unlocked leader log files for follower replay.
pub fn run_child_post_queue(
    mut cmd: std::process::Command,
    timeout_secs: Option<u64>,
    queue_wait_ms: u64,
    tee_files: Option<(&std::path::Path, &std::path::Path)>,
) -> anyhow::Result<RunOutcome> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    if tee_files.is_some() {
        cmd.stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
    }
    let t_run = std::time::Instant::now();
    let mut child = cmd.spawn()?;
    // Spawn stdout/stderr tee threads if leader, then poll `child.try_wait()` every 100ms...
    // On `timeout_secs` exceeded -> kill process group (`libc::killpg` / `taskkill /T /F`),
    // join tee threads, and return `Ok(RunOutcome { exit_code: 124, compile_ms, timed_out: true })`.
    # unimplemented!()
}
```

### 3. `route_policy.rs` Feature-Safe Fallback (`crates/vox-orchestrator/src/route_policy.rs`)
```rust
#[cfg(feature = "runtime")]
use vox_actor_runtime::route_capability_policy::{
    RouteCapabilityPolicySnapshot, exclusion_reason_for_llm_lane,
};

#[cfg(not(feature = "runtime"))]
fn exclusion_reason_without_runtime(is_local_http: bool) -> Option<&'static str> {
    let profile = std::env::var("VOX_ROUTE_POLICY_PROFILE")
        .unwrap_or_else(|_| "balanced".to_string())
        .to_ascii_lowercase();
    let restricted = profile == "restricted";
    let parse_bool = |key: &str, default: bool| -> bool {
        std::env::var(key)
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(default)
    };
    if !parse_bool("VOX_ROUTE_ALLOW_NET", !restricted) {
        return Some("route_policy:network_disabled");
    }
    if is_local_http {
        if !parse_bool("VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP", !restricted) {
            return Some("route_policy:local_model_http_disabled");
        }
    } else if !parse_bool("VOX_ROUTE_ALLOW_PROVIDER_NETWORK", !restricted) {
        return Some("route_policy:provider_network_disabled");
    }
    None
}
```

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `sccache` | `BUILD-CONTENTION-03` auto-wrapper | ✓ | `sccache 0.19.0` (verified on `PATH`) | Omit `RUSTC_WRAPPER` cleanly when absent or `VOX_BROKER_NO_SCCACHE=1`. |
| `vox-broker` / `~/.vox/build-broker` | `BUILD-CONTENTION-01` & `04` live shim | ✓ | Installed (`effective cap: 6`, verified via read-only `vox-broker status`) | Degrades to `exec_real` (`plain cargo`) on any coordinator error. |
| `fs2` / `libc` | `vox-build-queue` locks & Unix `killpg` | ✓ | `fs2 0.4` + workspace `libc` in `Cargo.toml` | Windows uses `CREATE_NO_WINDOW` + `taskkill /T /F` + `Child::kill`. |

**Missing dependencies with no fallback:** None.

---

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `libtest` (`cargo test`) + `.vox` broker verification scripts (`scripts/broker-ci.vox`, `scripts/broker-bypass-lint.vox`) |
| Config file | `crates/vox-cargo-shim/Cargo.toml` (standalone `[workspace]`) & root `Cargo.toml` |
| Quick run command | `timeout 300s cargo test -p vox-build-queue` + `timeout 300s cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml --all-targets` |
| Full suite command | Batched verification below |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| `BUILD-CONTENTION-01` | Effective `target_dir` precedence (`--target-dir` > `CARGO_TARGET_DIR` > `.cargo/config.toml`), target-first `FairQueue` ordering before global `Slot`, Windows-safe leader-follower coalescing (2 concurrent identical builds spawn real `cargo` once, follower gets output + exit code with `coalesced_hit=true`, stale mtime bypasses coalescing), 3-stage `MetricRecord` backward-compat & `vox-broker` read-only invariant | unit + integration | `timeout 300s cargo test -p vox-build-queue` & `timeout 300s cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml --all-targets` | ✅ `crates/vox-build-queue/src/{queue,metrics,resolve}.rs` + `crates/vox-cargo-shim/tests/vox_broker_readonly.rs` (extend + add `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs`) |
| `BUILD-CONTENTION-02` | `vox-orchestrator` compiles & passes tests under `--no-default-features` and `--no-default-features --features runtime`; `vox-terminal-core` compiles & passes tests with `vox-lang` OFF (default) and ON (`--features vox-lang`), while `vox-compiler` `caps.rs` embedder audit stays green | unit + integration | `timeout 600s cargo test -p vox-orchestrator --no-default-features --lib --tests` & `timeout 600s cargo check -p vox-orchestrator --no-default-features --features runtime` & `timeout 300s cargo test -p vox-terminal-core` & `timeout 300s cargo test -p vox-terminal-core --features vox-lang` & `timeout 300s cargo test -p vox-compiler --lib eval::caps` | ✅ Existing crates & tests |
| `BUILD-CONTENTION-03` | `sccache` auto-injected into `RUSTC_WRAPPER` when unset and executable on `PATH` (respecting `RUSTC_WRAPPER=""` and `VOX_BROKER_NO_SCCACHE=1`); `VOX_WORKSTREAM` + `VOX_ISOLATED_TARGET=1` routes `CARGO_TARGET_DIR` to `<wt>/target/ws-<slug>` with path-traversal sanitization; `dev-loop-guard` passes on repo | unit + CI guard | `timeout 300s cargo test -p vox-build-queue -p vox-cli-ci --lib` | ✅ `crates/vox-cli-ci/src/dev_loop_guard.rs` |
| `BUILD-CONTENTION-04` | `VOX_BROKER_RUN_TIMEOUT_SECS` / `--broker-run-timeout` excludes `FairQueue` + `Slot` wait time and kills active child returning exit `124` only when active compile/test runtime exceeds threshold; docs & scripts pass lint | integration + doc-lint | `timeout 300s cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml --test broker_coalesce_and_timeout` & `timeout 300s cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/contributors/build-broker-usage.md` & `vox run scripts/broker-ci.vox` & `vox run scripts/broker-bypass-lint.vox` | ❌ Wave 0 / Plan 02–03 creates `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs` using a tiny fake `cargo` script/binary in `TEMP` (`< 2s` runtime!) |

### Sampling Rate
- **Per task commit:** Scoped `cargo test` on only the crate(s) touched by that task (`vox-build-queue` + `vox-cargo-shim` execute in `< 10s` using a mock fake-`cargo` binary in `tempfile::tempdir()`!).
- **Per wave merge:** Batched crate tests + `vox run scripts/broker-ci.vox` + `vox run scripts/broker-bypass-lint.vox`.
- **Phase gate:** All 4 requirement test suites + `vox-doc-pipeline --lint-only` green before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs` — integration test harness using a synthetic `VOX_BROKER_HOME` + tiny fake `cargo` executable in `tempdir` to deterministically test: (a) target-first `FairQueue` blocking without consuming `slots/slot_*`, (b) 2-way leader-follower coalescing (`1` real spawn, identical stdout/stderr/exit), (c) post-queue `VOX_BROKER_RUN_TIMEOUT_SECS=1` surviving a `1.5s` queue wait for a `0.2s` build (`exit 0`) while killing a `3s` active hang (`exit 124`).

---

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | No | Local developer/CI build broker only; no auth boundary. |
| V3 Session Management | No | Stateless daemonless invocation per command. |
| V4 Access Control | Yes | Local user-owned `~/.vox/build-broker` directory; `vox-compiler` `CapabilitySet::parse("")` pure-only restriction preserved in `vox-terminal-core/src/vox_interp.rs`. |
| V5 Input Validation | Yes | Strict `[A-Za-z0-9_-]` allowlist sanitization on `VOX_WORKSTREAM` / `.vox/agent-target` slugs so untrusted branch/workstream names cannot traverse `../` outside `<worktree>/target/`. |
| V6 Cryptography | No | Non-cryptographic cache/queue keys (`DefaultHasher` permitted; no `vox-crypto` needed). |
| V14 Configuration | Yes | Preserve `VOX_BROKER_DEPTH >= 2` fork-bomb abort (`exit 70`), preserve `route_policy.rs` `VOX_ROUTE_*` fail-closed network restrictions when `runtime` feature is disabled, and never log secret env vars into `metrics.jsonl` or `broker.log` (only 64-bit `env_hash` of `FINGERPRINT_ALLOW` compiler flags is logged). |

### Known Threat Patterns for Build Broker & Process Wrappers
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal via `VOX_WORKSTREAM=../../etc` or `.vox/agent-target` | Tampering | Strip/reject any character outside `[A-Za-z0-9_-]` and cap length at 48 bytes; prefix directory with `ws-` under `<worktree_root>/target/`. |
| Recursive `cargo` / `sccache` wrapper fork bomb | Denial of Service | Keep `VOX_BROKER_DEPTH >= 2` hard exit `70` (`main.rs:80-83`), skip `own_dir` when scanning `PATH` for both `cargo` and `sccache`, and never set `RUSTC_WRAPPER` to `cargo` or shim path. |
| Silent route-policy bypass under `vox-orchestrator --no-default-features` | Elevation of Privilege | Implement exact `VOX_ROUTE_*` env check in `route_policy.rs`'s `#[cfg(not(feature = "runtime"))]` branch instead of returning `None`. |
| Orphaned `rustc` / test binaries holding `target/` file handles after timeout | Denial of Service | Spawn child in dedicated Unix process group (`process_group(0)`) and send `libc::killpg(pid, SIGKILL)` (Windows `taskkill /T /F /PID`) before returning exit `124`. |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `VOX_WORKSTREAM` target isolation (`target/ws-<slug>`) should be opt-in via `VOX_ISOLATED_TARGET=1` (or `.vox/agent-target`), rather than unconditionally rewriting `CARGO_TARGET_DIR` whenever `VOX_WORKSTREAM` is set, so existing scripts expecting artifacts in `target/` (like `scripts/broker-ci.vox`) are never surprised unless isolation is opted into. | Pattern 5 / Pitfall 3 | Low — honoring `VOX_ISOLATED_TARGET=1` (defaulting slug from `VOX_WORKSTREAM` when set, or `"isolated"` fallback) AND supporting `VOX_WORKSTREAM` when `VOX_ISOLATED_TARGET` is not `"0"` for non-`--manifest-path` workspace builds gives full flexibility with zero breakage. |

*(All other findings were verified directly against repository source files.)*

---

## Open Questions (RESOLVED)

1. **Should `vox-terminal-core` keep `vox-compiler` in `[dev-dependencies]` (as an old doc comment in `src/vox_interp.rs:4-5` suggested)?**
   - **What we know:** `src/vox_interp.rs:4-5` says `"available unconditionally in dev/test (vox-compiler is a dev-dep)"`, whereas `Cargo.toml:30-31` currently has **only** `tokio` in `[dev-dependencies]`.
   - **Resolution:** Do **NOT** put `vox-compiler` in `[dev-dependencies]`. Putting `vox-compiler` in `[dev-dependencies]` would force `cargo test -p vox-terminal-core` to compile `47k LoC` of `vox-compiler` on every terminal test run, defeating `BUILD-CONTENTION-02`! Instead, gate `tests/vox_interp.rs` with `#![cfg(feature = "vox-lang")]` and test both modes (`cargo test -p vox-terminal-core` fast path + `cargo test -p vox-terminal-core --features vox-lang`).

2. **Which env var name should activate the post-queue timeout: `VOX_BROKER_RUN_TIMEOUT_SECS` (`447-CONTEXT.md`) or `VOX_BROKER_BUILD_TIMEOUT_SECS` (`REQUIREMENTS.md`)?**
   - **What we know:** `447-CONTEXT.md:30` specifies `VOX_BROKER_RUN_TIMEOUT_SECS` and `--broker-run-timeout`, while `REQUIREMENTS.md:57` mentions `VOX_BROKER_BUILD_TIMEOUT_SECS`.
   - **Resolution:** Support `VOX_BROKER_RUN_TIMEOUT_SECS` as primary AND accept `VOX_BROKER_BUILD_TIMEOUT_SECS` as a fallback alias (plus `--broker-run-timeout[=]<secs>` CLI flag). Zero ambiguity for callers.

3. **How do we update `.agents/rules/00-core.md` cleanly given `.gitignore:305` ignores `/.agents/`?**
   - **What we know:** `git ls-files .agents` tracks `.agents/cargo-safety.md` and `.agents/toestub-check.md`, while `.agents/rules/00-core.md` is gitignored local driver config.
   - **Resolution:** Update `docs/src/contributors/build-broker-usage.md` and tracked `.agents/cargo-safety.md` for versioned repo SSOT, and update `.agents/rules/00-core.md` locally without passing `.agents/rules/00-core.md` to `git add`.

---

## Sources

### Primary (HIGH confidence — Verified In-Repo via `read_file`)
- `crates/vox-cargo-shim/src/main.rs:15, 132-229` — Current `run_global` lock acquisition order (`register_inflight` -> `acquire_slot_adaptive` without `FairQueue`), `VOX_BROKER_DEPTH`, `build_args_for_run`, and `MetricRecord` emission.
- `crates/vox-cargo-shim/Cargo.toml:19-51` & `crates/vox-cargo-shim/tests/vox_broker_readonly.rs:15-46` — Standalone `[workspace]` exclusion contract and read-only `vox-broker` test invariant.
- `crates/vox-build-queue/src/queue.rs:28-120` — Existing `FairQueue` (`hash_path`, `take_ticket`, `position`, Windows `os error 33` single-handle comment at lines 66–67).
- `crates/vox-build-queue/src/global.rs:98-285` (483 LoC total) — `should_admit`, `load_average_1m`, `acquire_slot_adaptive`, `register_inflight`.
- `crates/vox-build-queue/src/metrics.rs:11-95` — `MetricRecord` fields and `Summary::render` format (`builds=...`).
- `crates/vox-build-queue/src/resolve.rs:10-83` & `src/env_filter.rs:34-71` — `is_build_subcommand`, `worktree_root_of`, `resolve_real_cargo`, `build_args_for_run`, and `FINGERPRINT_ALLOW`.
- `crates/vox-orchestrator/Cargo.toml:50-117`, `src/victory.rs:48-61`, `src/orch_daemon/dei_dispatch.rs:216-256`, `src/route_policy.rs:6-33`, `src/orchestrator/task_dispatch/complete/success/healing.rs:12-39`, `tests/scaling_test.rs:1-25`, `tests/context_compaction_wiring_test.rs:96-105` — All 5 `--no-default-features` / `--features runtime` blockers.
- `crates/vox-actor-runtime/src/route_capability_policy.rs:1-65` — Exact 4-env-var `VOX_ROUTE_*` policy logic mirrored for `route_policy.rs`.
- `crates/vox-terminal-core/Cargo.toml:8-31`, `src/vox_interp.rs:1-39`, `src/host.rs:801-839`, `src/daemon_host.rs:11`, `crates/vox-compiler/src/eval/caps.rs:700-728` — `vox-lang` feature gating and `production_embedders_assign_caps_explicitly` source-text assertion.
- `.cargo/config.toml:15-109`, `crates/vox-cli-ci/src/dev_loop_guard.rs:1-248`, `scripts/broker-ci.vox:1-140`, `scripts/broker-bypass-lint.vox:1-285`, `docs/src/contributors/build-broker-usage.md:1-80`, `.agents/cargo-safety.md:1-45`, `.gitignore:304-306`.

---

## Metadata

**Confidence breakdown:**
- Standard stack: **HIGH** — 100% existing workspace/shim crates (`fs2`, `serde`, `serde_json`, `anyhow`, `libc`); zero external dependencies needed.
- Architecture: **HIGH** — Every lock interaction, Windows `os error 33` constraint, LoC budget (`global.rs` at 483 LoC), and cross-crate test assertion (`caps.rs:710`, `vox_broker_readonly.rs`, `broker-ci.vox`) verified directly from source.
- Pitfalls: **HIGH** — Uncovered 3 hidden `vox-orchestrator` `--no-default-features` / `--features runtime` compile blockers, 1 cross-crate source-text audit on `vox_interp.rs`, 1 standalone target-dir subtlety in `scripts/broker-ci.vox`, and `.gitignore` status of `.agents/rules/00-core.md` ahead of planning.

**Research date:** 2026-11-14
**Valid until:** 2026-12-14
