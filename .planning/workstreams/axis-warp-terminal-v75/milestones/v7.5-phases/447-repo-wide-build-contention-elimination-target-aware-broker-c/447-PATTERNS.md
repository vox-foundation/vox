# Phase 447: Repo-Wide Build Contention Elimination & Broker Coalescing - Pattern Map

**Mapped:** 2026-11-15
**Files analyzed:** 25 (3 new, 22 modified; 24 git-tracked + 1 gitignored local driver rule file)
**Analogs found:** 25 / 25

## Path Corrections & Non-Obvious Structural Discoveries (Read First)

1. **`vox-broker` binary filename is hyphenated:** `crates/vox-cargo-shim/src/bin/vox-broker.rs` (`[[bin]] name = "vox-broker"`, `path = "src/bin/vox-broker.rs"` in `crates/vox-cargo-shim/Cargo.toml:40-41`) — NOT `vox_broker.rs`.
2. **`dev_loop_guard.rs` lives in `vox-cli-ci`:** `crates/vox-cli-ci/src/dev_loop_guard.rs` (wired via `crates/vox-cli/src/commands/ci/pre_push.rs:600, 1513-1514` -> `vox_cli_ci::dev_loop_guard::run(root)`) — `crates/vox-cli/src/commands/ci/dev_loop_guard.rs` was moved out during the PR-4 CI split and does not exist.
3. **`MetricRecord` (not `BuildMetricEntry`):** The struct in `crates/vox-build-queue/src/metrics.rs:12-21` is `pub struct MetricRecord` (with `pub struct Summary` at `metrics.rs:45-50`). Only **3** struct literals exist across the entire repo: `crates/vox-build-queue/src/metrics.rs:117`, `crates/vox-cargo-shim/src/main.rs:192`, and `crates/vox-cargo-shim/src/bin/vox-broker.rs:237`.
4. **`libc` dependency boundary (`Rule 5` zero-`Cargo.lock`-churn guard):** `crates/vox-cargo-shim/Cargo.toml:43-45` depends **only** on `anyhow = "1"` and `vox-build-queue = { path = "../vox-build-queue" }` (no `libc`), whereas `crates/vox-build-queue/Cargo.toml:18-19` already declares `[target.'cfg(unix)'.dependencies] libc = { workspace = true }`. Put `kill_process_tree(pid: u32)` inside `vox_build_queue::resolve` (or `vox_build_queue::coalesce`) and call it from `crates/vox-cargo-shim/src/runner.rs` so **neither** `Cargo.toml` nor `crates/vox-cargo-shim/Cargo.lock` changes.
5. **`global.rs` is at 483 / 500 LoC:** Do **not** add functions to `crates/vox-build-queue/src/global.rs` (483 lines; `god_object` detector fails at 500 LoC). Place leader-follower coalescing in new `crates/vox-build-queue/src/coalesce.rs`.
6. **`.agents/rules/00-core.md` is gitignored (`/.agents/` at `.gitignore:305`), while `.agents/cargo-safety.md` IS force-tracked in git:** Update `.agents/cargo-safety.md` + `docs/src/contributors/build-broker-usage.md` for tracked commits; update `.agents/rules/00-core.md` locally if desired, and **never** pass `.agents/rules/00-core.md` to `git add`.

---

## 3-Plan File Partition Map

| Plan | Requirements | Non-Overlapping File Set |
|------|--------------|--------------------------|
| **Plan 01** | `BUILD-CONTENTION-01`, `BUILD-CONTENTION-03` (shim/queue), `BUILD-CONTENTION-04` (timeout) | `crates/vox-build-queue/src/{lib.rs,queue.rs,coalesce.rs,metrics.rs,resolve.rs}`, `crates/vox-cargo-shim/src/{main.rs,runner.rs,bin/vox-broker.rs}`, `crates/vox-cargo-shim/tests/{vox_broker_readonly.rs,broker_coalesce_and_timeout.rs}` |
| **Plan 02** | `BUILD-CONTENTION-02` | `crates/vox-orchestrator/src/{victory.rs,orch_daemon/dei_dispatch.rs,route_policy.rs,orchestrator/task_dispatch/complete/success/healing.rs}`, `crates/vox-orchestrator/tests/{scaling_test.rs,context_compaction_wiring_test.rs}`, `crates/vox-terminal-core/{Cargo.toml,src/vox_interp.rs,tests/vox_interp.rs,tests/session.rs}`, `crates/vox-term/Cargo.toml`, `crates/vox-orchestrator-d/Cargo.toml`, `crates/vox-gui/Cargo.toml` |
| **Plan 03** | `BUILD-CONTENTION-03` (CI guard + docs), `BUILD-CONTENTION-04` (docs + scripts) | `crates/vox-cli-ci/src/dev_loop_guard.rs`, `.agents/cargo-safety.md`, `docs/src/contributors/build-broker-usage.md`, `scripts/broker-install.vox`, `scripts/broker-ci.vox` (+ local untracked `.agents/rules/00-core.md`) |

*(Note: Plan 01 and Plan 02 have zero overlapping files and can execute in Wave 1 in parallel; Plan 03 depends on Plan 01's final env/CLI names and runs `broker-ci.vox` + `broker-bypass-lint.vox` + `vox-doc-pipeline` in Wave 2.)*

---

## File Classification

| New/Modified File | Status | Cur LoC | Role | Data Flow | Closest Tracked Analog | Match Quality | Plan |
|-------------------|--------|---------|------|-----------|------------------------|---------------|------|
| `crates/vox-build-queue/src/lib.rs` | modify | 8 | config | module-export | `crates/vox-build-queue/src/lib.rs` | exact | 01 |
| `crates/vox-build-queue/src/coalesce.rs` | **new** | 0 | service | file-I/O | `crates/vox-build-queue/src/queue.rs` | exact | 01 |
| `crates/vox-build-queue/src/queue.rs` | modify | 223 | service | file-I/O | `crates/vox-build-queue/src/queue.rs` | exact | 01 |
| `crates/vox-build-queue/src/resolve.rs` | modify | 175 | utility | transform | `crates/vox-build-queue/src/resolve.rs` | exact | 01 |
| `crates/vox-build-queue/src/metrics.rs` | modify | 180 | model | file-I/O | `crates/vox-build-queue/src/metrics.rs` | exact | 01 |
| `crates/vox-cargo-shim/src/runner.rs` | **new** | 0 | service | streaming | `crates/vox-cargo-shim/src/main.rs` | role-match | 01 |
| `crates/vox-cargo-shim/src/main.rs` | modify | 229 | controller | request-response | `crates/vox-cargo-shim/src/main.rs` | exact | 01 |
| `crates/vox-cargo-shim/src/bin/vox-broker.rs` | modify | 297 | controller | file-I/O | `crates/vox-cargo-shim/src/bin/vox-broker.rs` | exact | 01 |
| `crates/vox-cargo-shim/tests/vox_broker_readonly.rs` | modify | 80 | test | file-I/O | `crates/vox-cargo-shim/tests/vox_broker_readonly.rs` | exact | 01 |
| `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs` | **new** | 0 | test | streaming | `crates/vox-cargo-shim/tests/vox_broker_readonly.rs` | role-match | 01 |
| `crates/vox-orchestrator/src/victory.rs` | modify | 105 | service | transform | `crates/vox-orchestrator/src/lib.rs` | exact | 02 |
| `crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs` | modify | 323 | controller | request-response | `crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs` | exact | 02 |
| `crates/vox-orchestrator/src/route_policy.rs` | modify | 149 | middleware | transform | `crates/vox-actor-runtime/src/route_capability_policy.rs` | exact | 02 |
| `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs` | modify | 80 | service | event-driven | `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs` | exact | 02 |
| `crates/vox-orchestrator/tests/scaling_test.rs` | modify | 247 | test | event-driven | `crates/vox-orchestrator/tests/chat_round_trip.rs` | exact | 02 |
| `crates/vox-orchestrator/tests/context_compaction_wiring_test.rs` | modify | 191 | test | request-response | `crates/vox-orchestrator/tests/chat_round_trip.rs` | exact | 02 |
| `crates/vox-terminal-core/Cargo.toml` | modify | 31 | config | transform | `crates/vox-orchestrator/Cargo.toml` | exact | 02 |
| `crates/vox-terminal-core/src/vox_interp.rs` | modify | 39 | service | transform | `crates/vox-terminal-core/src/vox_interp.rs` | exact | 02 |
| `crates/vox-terminal-core/tests/vox_interp.rs` | modify | 12 | test | transform | `crates/vox-orchestrator/tests/chat_round_trip.rs` | exact | 02 |
| `crates/vox-terminal-core/tests/session.rs` | modify | 130 | test | event-driven | `crates/vox-terminal-core/tests/session.rs` | exact | 02 |
| `crates/vox-term/Cargo.toml` | modify | 29 | config | transform | `crates/vox-terminal-core/Cargo.toml` | exact | 02 |
| `crates/vox-orchestrator-d/Cargo.toml` | modify | 50 | config | transform | `crates/vox-orchestrator-d/Cargo.toml` | exact | 02 |
| `crates/vox-gui/Cargo.toml` | modify | 83 | config | transform | `crates/vox-gui/Cargo.toml` | exact | 02 |
| `crates/vox-cli-ci/src/dev_loop_guard.rs` | modify | 248 | middleware | batch | `crates/vox-cli-ci/src/dev_loop_guard.rs` | exact | 03 |
| `.agents/cargo-safety.md` | modify | 45 | config | file-I/O | `.agents/cargo-safety.md` | exact | 03 |
| `docs/src/contributors/build-broker-usage.md` | modify | 180 | config | file-I/O | `docs/src/contributors/documentation-governance.md` | exact | 03 |
| `scripts/broker-install.vox` | modify | 412 | utility | batch | `scripts/broker-install.vox` | exact | 03 |
| `scripts/broker-ci.vox` | modify | 140 | utility | batch | `scripts/broker-ci.vox` | exact | 03 |

---

## Pattern Assignments — Plan 01 (`vox-build-queue` + `vox-cargo-shim`)

### 1. `crates/vox-build-queue/src/lib.rs` & `crates/vox-build-queue/src/queue.rs`

**Analog:** `crates/vox-build-queue/src/queue.rs` (lines 28–154)

- **`lib.rs` (lines 4–8):** Add `pub mod coalesce;`:
  ```rust
  pub mod coalesce;
  pub mod env_filter;
  pub mod global;
  pub mod metrics;
  pub mod queue;
  pub mod resolve;
  ```
- **`queue.rs` existing API (lines 28–147) — already complete, needs at most a `ticket_timestamp_ms` helper or `(u64, u128)` return if needed:**
  - `pub fn hash_path(p: &Path) -> String` (lines 28–32)
  - `FairQueue::new(queue_root: &Path, worktree_hash: &str) -> Result<Self>` (lines 47–51): pass `&root.join("targets")` and `&queue::hash_path(&effective_target)` so lock files live at `<broker_home>/targets/<target_hash>/` (surviving `cargo clean` and `git clean -fdx`).
  - `FairQueue::take_ticket(&self, key: &str) -> Result<u64>` (lines 57–84): writes `<ts_ms>\n<key>` into `t<seq>` while holding single-handle `f.lock_exclusive()` on `counter` (Windows `os error 33`-safe).
  - `FairQueue::position(&self, seq: u64) -> usize` (lines 119–121)
  - `FairQueue::coalesce_opportunity(&self, seq: u64, key: &str) -> bool` (lines 125–129)
  - `FairQueue::acquire(&self, seq: u64) -> Result<Ticket>` (lines 133–146): blocks on `run.lock` **without** holding any `slots/slot_*` handle!

---

### 2. `crates/vox-build-queue/src/coalesce.rs` (**NEW** — service, file-I/O)

**Analog:** `crates/vox-build-queue/src/queue.rs` (lines 14–84) & `crates/vox-build-queue/src/global.rs` (lines 223–302)

**Why a dedicated file:** `global.rs` is 483 LoC (`< 500` LoC `god_object` limit). Keep `coalesce.rs` under ~280 LoC.

**Imports & `vox-drift-check`-safe constants pattern:**
```rust
use anyhow::Result;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::metrics::now_ms;

// Named `const`s satisfy `vox-drift-check` (`drift/timeout-literal` skips `in_const == true`).
const FOLLOWER_POLL_MS: u64 = 50;
const RECEIPT_FRESH_WINDOW_MS: u128 = 30_000;
```

**Windows `os error 33`-safe file layout under `<broker_home>/inflight/`:**
- `<key_hash>.lock` — **0-byte lock-only file** (`try_lock_exclusive` / `unlock`). **Never read or write `<key_hash>.lock`** (on Windows, `LockFileEx` causes `ReadFile` on any second handle to fail with `os error 33`).
- `<key_hash>.meta` — Unlocked JSON written via `<key_hash>.meta.<pid>.tmp` + `std::fs::rename`:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct LeaderMeta {
      pub run_id: String,
      pub started_ms: u128,
      pub subcmd: String,
      pub target_dir: String,
  }
  ```
- `<key_hash>.stdout` & `<key_hash>.stderr` — Unlocked raw output stream files truncated by Leader on claim and opened read-only by Followers to replay exact stdout/stderr bytes in real time (`OpenOptions` shares `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE` by default on Windows as long as `fs2` never locks them).
- `<key_hash>.exit` — Unlocked JSON written via `<key_hash>.exit.<pid>.tmp` + `std::fs::rename` **before** unlocking `<key_hash>.lock`:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct LeaderExit {
      pub run_id: String,
      pub started_ms: u128,
      pub finished_ms: u128,
      pub exit_code: i32,
      pub timed_out: bool,
  }
  ```

**Key functions to expose from `coalesce.rs`:**
1. `pub fn is_coalesceable(sub: &str, args: &[String], env: &std::collections::BTreeMap<String, String>) -> bool`:
   - `check` | `build` | `clippy`: `true` unless `args.iter().any(|a| a.starts_with("--message-format"))` (protects `scripts/broker-install.vox:250` `--message-format=json`).
   - `test`: `true` unless snapshot-mutating env vars are set (`INSTA_UPDATE`, `UPDATE_EXPECT`, `SNAPSHOT_UPDATE`, or `TRYBUILD == "overwrite"`).
   - `run`: `resolve::build_args_for_run(args).is_some()` (coalesces **only** the `build` phase; binary execution runs via `exec_real` outside the broker).
   - `bench` or anything else: `false`.
2. `pub fn latest_source_mtime_ms(worktree: &Path) -> u128`:
   - Inspect root `Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`, and walk `crates/` (pruning `target`, `.git`, `node_modules`, `.sccache`, `.planning`, `docs`), tracking maximum file `modified()` in epoch ms (`0` on error).
3. `pub fn try_follow_inflight(root: &Path, key_hash: &str, min_started_ms: u128, mut on_stdout: impl FnMut(&[u8]), mut on_stderr: impl FnMut(&[u8])) -> Option<(i32, u64)>`:
   - Probe `<key_hash>.lock`: if `try_lock_exclusive()` succeeds, immediately `unlock()` (no active leader -> return `None`).
   - If locked by an active leader, read `<key_hash>.meta`; if `meta.started_ms < min_started_ms`, return `None` (workspace sources changed after leader started -> must not coalesce onto stale build).
   - Tail `<key_hash>.stdout` and `<key_hash>.stderr` every `Duration::from_millis(FOLLOWER_POLL_MS)` until `<key_hash>.lock` becomes acquirable (probe `try_lock_exclusive` -> immediately `unlock`). Flush remaining bytes, read `<key_hash>.exit`, verify `exit.run_id == meta.run_id && !exit.timed_out`, and return `Some((exit.exit_code, waited_ms))`. If leader crashed or timed out, return `None` so caller falls through and runs Cargo itself.
4. `pub fn try_replay_recent_leader(root: &Path, key_hash: &str, ticket_ts_ms: u128, min_started_ms: u128, mut on_stdout: impl FnMut(&[u8]), mut on_stderr: impl FnMut(&[u8])) -> Option<i32>`:
   - Called right after `FairQueue::acquire(seq)`: if an identical Leader started *while we were queued* (`exit.started_ms >= ticket_ts_ms && exit.started_ms >= min_started_ms && !exit.timed_out && now_ms().saturating_sub(exit.finished_ms) <= RECEIPT_FRESH_WINDOW_MS`), replay `.stdout` + `.stderr` and return `Some(exit.exit_code)` in `0 ms` without acquiring a global slot!
5. `pub struct LeaderGuard` (`claim_leader(root, key_hash, subcmd, target_dir) -> Result<Option<LeaderGuard>>` + `finish(self, exit_code: i32, timed_out: bool)`):
   - Locks `<key_hash>.lock` via `try_lock_exclusive`, truncates `.stdout`/`.stderr`, removes stale `.exit`, atomically writes `.meta`, exposes `(stdout_path, stderr_path)` for `runner.rs`, and on `finish` atomically writes `.exit` before unlocking `<key_hash>.lock`.

---

### 3. `crates/vox-build-queue/src/resolve.rs` (modify — lines 76–175)

**Analog:** `crates/vox-build-queue/src/resolve.rs` (lines 24–83) & `crates/vox-build-queue/src/global.rs` (lines 187–200)

**Add 5 pure/OS-isolated helpers:**
1. **`normalize_target_path(cwd: &Path, raw: &Path) -> PathBuf`**: Walk `.ancestors()` to the deepest existing ancestor, `.canonicalize()` it, and re-push non-existent leaf components (so fresh `target/ws-<slug>` canonicalizes identically before and after Cargo creates it, resolving macOS `/var` -> `/private/var` symlinks).
2. **`effective_target_dir(cwd: &Path, args: &[String], child_env: &BTreeMap<String, String>) -> PathBuf`**:
   - Precedence: `--target-dir <p>` / `--target-dir=<p>` before `--` > non-empty `child_env["CARGO_TARGET_DIR"]` > standalone `--manifest-path` `[workspace]` outside a Vox worktree > `worktree_root_of(cwd).join("target")` > `cwd.join("target")`.
3. **`resolve_sccache_wrapper(path_var: &str, own_exe: Option<&Path>) -> Option<PathBuf>`**:
   - Mirror `resolve_real_cargo` (`resolve.rs:52-74`): scan `std::env::split_paths(path_var)` for `sccache` (`sccache.exe` on Windows), skip `own_dir`, verify `is_file()` (and executable mode `& 0o111 != 0` under `#[cfg(unix)]` via `std::os::unix::fs::PermissionsExt`), return canonical `PathBuf`.
   - Inject into child `env` in `main.rs` **only** when `VOX_BROKER_NO_SCCACHE` is not truthy (`1`/`true`/`yes`) AND `!env.contains_key("RUSTC_WRAPPER")` (preserving explicit `RUSTC_WRAPPER=""` opt-outs).
4. **`apply_workstream_target_isolation(cwd: &Path, args: &[String], env: &mut BTreeMap<String, String>, sccache_active: bool) -> Option<PathBuf>`**:
   - Skip if `--target-dir` is in `args` (before `--`), OR `env.get("CARGO_TARGET_DIR").is_some_and(|v| !v.is_empty())`, OR `--manifest-path` is passed (protects `scripts/broker-ci.vox:119-125` standalone build check).
   - Resolve slug from:
     - `VOX_ISOLATED_TARGET == "1" | "true"` -> use `VOX_WORKSTREAM` (fallback `"isolated"`), OR
     - `<worktree_root>/.vox/agent-target` file contents if present (`wt.join(".vox").join("agent-target")` — avoids `drift/vox-path-literal`!).
   - Sanitize slug strictly to ASCII `[A-Za-z0-9_-]`, max 48 chars; if non-empty, set `CARGO_TARGET_DIR = <worktree_root>/target/ws-<slug>`. If `sccache_active && !env.contains_key("CARGO_INCREMENTAL")`, set `CARGO_INCREMENTAL = "0"` so workspace crates share `.sccache` across isolated targets.
5. **`kill_process_tree(pid: u32)`** (placed here in `vox-build-queue` because `vox-build-queue` already has `libc` in `Cargo.toml:18-19`):
   ```rust
   #[allow(unsafe_code)]
   pub fn kill_process_tree(pid: u32) {
       #[cfg(unix)]
       {
           if pid > 0 {
               // Child was spawned with `cmd.process_group(0)`, so PGID == child PID.
               unsafe {
                   let _ = libc::killpg(pid as libc::pid_t, libc::SIGKILL);
               }
           }
       }
       #[cfg(windows)]
       {
           use std::os::windows::process::CommandExt;
           const CREATE_NO_WINDOW: u32 = 0x0800_0000;
           let _ = std::process::Command::new("taskkill")
               .args(["/T", "/F", "/PID", &pid.to_string()])
               .creation_flags(CREATE_NO_WINDOW)
               .status();
       }
   }
   ```

---

### 4. `crates/vox-build-queue/src/metrics.rs` (modify — lines 11–180)

**Analog:** `crates/vox-build-queue/src/metrics.rs` (lines 11–110)

**Exact struct evolution (`#[serde(default)]` preserves all 10,021+ existing `metrics.jsonl` lines):**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricRecord {
    pub ts_ms: u128,
    pub worktree: String,
    pub subcmd: String,
    /// Total queue wait (`target_wait_ms + slot_wait_ms`).
    pub queue_wait_ms: u64,
    /// Active compile/test runtime (`compile_ms` for new rows).
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

**`Summary` & `Summary::render()` (lines 45–76):**
```rust
#[derive(Debug, PartialEq)]
pub struct Summary {
    pub count: usize,
    pub p50_wait_ms: u64,
    pub p95_wait_ms: u64,
    pub p95_target_wait_ms: u64,
    pub p95_slot_wait_ms: u64,
    pub p50_compile_ms: u64,
    pub coalesce_rate: f64,
    pub coalesced_hits: usize,
    pub timeouts: usize,
}
```
- Preserve `builds={}` prefix and `p50_wait={}ms p95_wait={}ms` + `coalesce_rate={:.1}%` + `daemon_recommended()` substrings in `Summary::render()` so existing tests in `metrics.rs:149-170` and `src/bin/vox-broker.rs:249` (`assert!(out.contains("builds=1"))`) pass unchanged:
  ```rust
  format!(
      "builds={} p50_wait={}ms p95_wait={}ms (target_p95={}ms slot_p95={}ms compile_p50={}ms) coalesce_rate={:.1}% coalesced={} timeouts={} -> {}",
      self.count,
      self.p50_wait_ms,
      self.p95_wait_ms,
      self.p95_target_wait_ms,
      self.p95_slot_wait_ms,
      self.p50_compile_ms,
      self.coalesce_rate * 100.0,
      self.coalesced_hits,
      self.timeouts,
      verdict
  )
  ```
- For historical rows where `compile_ms == 0 && !coalesced_hit && ran_ms > 0`, treat effective compile time as `r.ran_ms` when computing `p50_compile_ms`.

---

### 5. `crates/vox-cargo-shim/src/runner.rs` (**NEW**), `src/main.rs` (modify), & `src/bin/vox-broker.rs` (modify)

**Analog:** `crates/vox-cargo-shim/src/main.rs` (lines 71–229) & `crates/vox-cargo-shim/src/bin/vox-broker.rs` (lines 112–139, 234–250)

1. **Shim timeout flag & env parsing (`runner.rs` or `main.rs`):**
   - Strip `--broker-run-timeout <secs>` / `--broker-run-timeout=<secs>` from `rest` **before** `is_build_subcommand`, `argv_hash`, and `exec_real` (so fallback `exec_real` never passes `--broker-run-timeout` to real `cargo`!).
   - Precedence: CLI `--broker-run-timeout` > `VOX_BROKER_RUN_TIMEOUT_SECS` > `VOX_BROKER_BUILD_TIMEOUT_SECS` (`0` or unset = disabled).
2. **`runner.rs` (`run_child_post_queue`):**
   - Bind `const CHILD_POLL_MS: u64 = 50;` (named `const` avoids `drift/timeout-literal`).
   - On `#[cfg(unix)]`: `use std::os::unix::process::CommandExt; cmd.process_group(0);`.
   - On `#[cfg(windows)]`: `use std::os::windows::process::CommandExt; const CREATE_NO_WINDOW: u32 = 0x0800_0000; cmd.creation_flags(CREATE_NO_WINDOW);`.
   - If `tee_files: Option<(&Path, &Path)>` is `Some((stdout_p, stderr_p))`, set `Stdio::piped()` on child stdout/stderr and spawn 2 `std::thread::spawn` copy loops writing chunks to both `std::io::stdout()`/`stderr()` and the unlocked `.stdout`/`.stderr` files.
   - Start `let t_run = Instant::now();` **inside `run_child_post_queue`** (strictly after `FairQueue::acquire` AND `global::acquire_slot_adaptive` have returned!).
   - Poll `child.try_wait()?` every `Duration::from_millis(CHILD_POLL_MS)`. When `timeout_secs` is exceeded:
     - Call `vox_build_queue::resolve::kill_process_tree(child.id()); let _ = child.kill(); let _ = child.wait();`
     - Join tee threads, print `eprintln!("vox-broker: active build exceeded timeout ({secs}s, excluding {queue_wait_ms}ms queue wait); exiting 124");`, and return `RunOutcome { exit_code: 124, compile_ms, timed_out: true }`.
3. **`main.rs` lock acquisition order in `run_global` (replacing lines 143–219):**
   - Step 1: Build `mut env = env_filter::passthrough_env(std::env::vars())`.
   - Step 2: Inject `resolve_sccache_wrapper` into `env["RUSTC_WRAPPER"]` (if unset & `VOX_BROKER_NO_SCCACHE` not set) and `apply_workstream_target_isolation` into `env["CARGO_TARGET_DIR"]` **before** computing `env_hash` and `effective_target_dir`.
   - Step 3: Compute `effective_target = resolve::effective_target_dir(&cwd, args, &env)`, `target_hash = queue::hash_path(&effective_target)`, `run_cmd_args` (via `build_args_for_run`), `effective_sub = if build_args.is_some() { "build" } else { sub }`, `argv_hash = env_filter::argv_hash(run_cmd_args)`, `env_hash = env_filter::env_hash(&env)`, and `coalesce_key = queue::hash_path(Path::new(&format!("{effective_sub}:{argv_hash:016x}:{env_hash:016x}:{target_hash}")))`.
   - Step 4 (Stage A — In-Flight Follower): If `coalesce::is_coalesceable(sub, args, &env)`, compute `src_mtime = coalesce::latest_source_mtime_ms(&worktree)` and call `coalesce::try_follow_inflight(&root, &coalesce_key, src_mtime, ...)`. On `Some((code, target_wait_ms))`, record `MetricRecord` (`queue_wait_ms: target_wait_ms`, `target_wait_ms`, `slot_wait_ms: 0`, `compile_ms: 0`, `ran_ms: 0`, `would_coalesce: true`, `coalesced_hit: true`, `timed_out: false`) + `broker.log` and return (or `exec_real` if `cargo run` and `code == 0`).
   - Step 5 (Stage B — Per-Target `FairQueue` **BEFORE** Global Slot):
     ```rust
     let ticket_ts_ms = metrics::now_ms();
     let fq = vox_build_queue::queue::FairQueue::new(&root.join("targets"), &target_hash)?;
     let seq = fq.take_ticket(&coalesce_key)?;
     let would_coalesce_q = fq.coalesce_opportunity(seq, &coalesce_key);
     let t_target = Instant::now();
     let _target_ticket = fq.acquire(seq)?;
     let target_wait_ms = t_target.elapsed().as_millis() as u64;
     ```
     Immediately check `coalesce::try_replay_recent_leader(&root, &coalesce_key, ticket_ts_ms, src_mtime, ...)`: if `Some(code)`, drop `_target_ticket`, log `coalesced_hit = true` (`slot_wait_ms = 0`, `compile_ms = 0`), and return!
   - Step 6 (Stage C — Claim Leader + Global `Slot`):
     Claim `leader = coalesce::claim_leader(...)`, call `let (_inflight, would_coalesce_g) = global::register_inflight(&root, &coalesce_key)?;`, then `let t_slot = Instant::now(); let (_slot, _, busy) = global::acquire_slot_adaptive(&root, n)?; let slot_wait_ms = t_slot.elapsed().as_millis() as u64;`.
   - Step 7 (Stage D — Active Compile + Post-Lock Timeout):
     Call `runner::run_child_post_queue(...)`, finish `leader` before dropping `_slot` and `_target_ticket`, write `MetricRecord` + `broker.log` (`wait={}ms target_wait={}ms slot_wait={}ms compile={}ms ahead={} cap={} coalesce={} coalesced={} timeout={} exit={}`), and handle `cargo run` tail `exec_real`.
4. **`src/bin/vox-broker.rs` updates:**
   - Line 112–116 (`broker_has_run`): add `|| root.join("targets").is_dir() || root.join("inflight").is_dir()`.
   - Line 237 (`stats_with_data_renders_summary` test fixture): add `..MetricRecord` new fields (or `target_wait_ms: 0, slot_wait_ms: 0, compile_ms: 5, coalesced_hit: false, timed_out: false`).
   - **Read-only invariant:** Never call `create_dir_all` or open files with `write(true)` / `create(true)` in `vox-broker.rs` (`tests/vox_broker_readonly.rs:41-45` enforces `entries.is_empty()`).

---

### 6. `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs` (**NEW** — integration test)

**Analog:** `crates/vox-cargo-shim/tests/vox_broker_readonly.rs` (lines 10–79)

**Binary resolution pattern (`vox_broker_readonly.rs:10-12`):**
```rust
fn shim_cargo_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cargo"))
}
```
**Mock fake-`cargo` fixture pattern (runs in `< 2s`, zero network/rustc overhead):**
- Create `tempdir()`, put a tiny shell script (`#!/bin/sh` on Unix `chmod 0o755`; `.bat`/compiled helper or conditional Unix/portable script) at `<tmp>/fake-cargo-home/bin/cargo` and set `CARGO_HOME=<tmp>/fake-cargo-home` + `VOX_BROKER_HOME=<tmp>/broker-home`! Because `real_cargo` in `src/main.rs:46-49` checks `resolve::cargo_home_proxy()` (`$CARGO_HOME/bin/cargo`) FIRST, setting `CARGO_HOME` deterministically routes the `cargo` shim binary to the fake `cargo` script with zero rustup calls!
- Test 4 exact scenarios:
  1. **Target-first `FairQueue` frees global slots:** With `VOX_BROKER_MAX_CONCURRENT=2`, launch 2 builds targeting `target-A` (where build 1 sleeps `0.6s`) and 1 build targeting `target-B`; assert `target-B` finishes while `target-A`'s waiter is queued on `FairQueue`, and `metrics.jsonl` records `target_wait_ms >= 400` and `slot_wait_ms < 200` for the second `target-A` build.
  2. **Leader-follower coalescing:** Launch 2 concurrent identical `cargo check -p demo` invocations against the same fake `cargo` that increments an atomic line counter file and sleeps `0.4s` while printing `"checking demo\n"`; assert counter file has **1** invocation, both exit `0` with `"checking demo"`, and `metrics.jsonl` has 1 row with `coalesced_hit == true` and `compile_ms == 0`.
  3. **Post-lock timeout excludes queue wait (`BUILD-CONTENTION-04`):** Build 1 holds `target-A` for `1.2s`; Build 2 queues on `target-A` with `VOX_BROKER_RUN_TIMEOUT_SECS=1` and runs for `0.15s` once admitted -> Build 2 exits **`0`** (would have died under outer `timeout 1s`!). Build 3 runs actively for `2.0s` with `VOX_BROKER_RUN_TIMEOUT_SECS=1` -> exits **`124`** with `timed_out == true` in `metrics.jsonl`.
  4. **`sccache` + `VOX_ISOLATED_TARGET=1` env injection:** Fake `cargo` prints `RUSTC_WRAPPER` and `CARGO_TARGET_DIR`; with fake `sccache` on `PATH` and `VOX_WORKSTREAM=axis-v75` + `VOX_ISOLATED_TARGET=1`, assert child sees `RUSTC_WRAPPER=<fake_sccache>` and `CARGO_TARGET_DIR=<wt>/target/ws-axis-v75`.

---

## Pattern Assignments — Plan 02 (`vox-orchestrator` + `vox-terminal-core` + Downstream)

### 1. `crates/vox-orchestrator/src/victory.rs` (lines 48–104)

**Analog:** `crates/vox-orchestrator/src/lib.rs` (lines 250–252: `#[cfg(feature = "toestub-gate")] pub mod validation;`)

**Exact edit (`victory.rs:47-55`):** Gate `VictoryEvaluator::evaluate` behind `#[cfg(feature = "toestub-gate")]` (verified zero external callers across workspace; keeps `VictoryEvaluator`, `VictoryConfig`, `VictoryReport`, `VictoryTiers` unconditionally available):
```rust
    /// Aggregate all verification signals into a single verdict consistent with the persistent store.
    #[cfg(feature = "toestub-gate")]
    pub fn evaluate(
        &self,
        task_id: &str,
        validation: &crate::validation::ValidationResult,
        observation: Option<&ObservationReport>,
        socrates_score: Option<f32>,
    ) -> VictoryVerdict {
```

---

### 2. `crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs` (lines 216–256)

**Analog:** `crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs` (lines 216–220 `response_err` pattern)

**Exact edit (`dei_dispatch.rs:216-256`):**
```rust
#[cfg(feature = "toestub-gate")]
async fn handle_ai_check(id: &str, params: &Value) -> DispatchResponse {
    // existing lines 217-255 unchanged
}

#[cfg(not(feature = "toestub-gate"))]
async fn handle_ai_check(id: &str, params: &Value) -> DispatchResponse {
    match params.get("path").and_then(|v| v.as_str()) {
        Some(p) if !p.is_empty() => {}
        _ => return response_err(id, "params.path required"),
    }
    response_err(id, "ai.check requires feature `toestub-gate`")
}
```

---

### 3. `crates/vox-orchestrator/src/route_policy.rs` (lines 6–28)

**Analog:** `crates/vox-actor-runtime/src/route_capability_policy.rs` (lines 18–64 — exact SSOT for `VOX_ROUTE_*` env semantics)

**Why needed:** `vox-actor-runtime` is `optional = true` in `crates/vox-orchestrator/Cargo.toml:89` (enabled only by `runtime` at line 51). Without gating lines 6–8 & 25–28, `cargo check -p vox-orchestrator --no-default-features` fails at `route_policy.rs:6`.

**Exact edit (`route_policy.rs:6-28`):**
```rust
#[cfg(feature = "runtime")]
use vox_actor_runtime::route_capability_policy::{
    RouteCapabilityPolicySnapshot, exclusion_reason_for_llm_lane,
};

#[cfg(not(feature = "runtime"))]
fn exclusion_reason_without_runtime(is_local_http_provider: bool) -> Option<&'static str> {
    let profile = std::env::var("VOX_ROUTE_POLICY_PROFILE")
        .unwrap_or_else(|_| "balanced".to_string())
        .to_ascii_lowercase();
    let restricted = profile == "restricted";
    let parse_flag = |key: &str, default: bool| -> bool {
        std::env::var(key)
            .map(|v| matches!(v.to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(default)
    };
    if !parse_flag("VOX_ROUTE_ALLOW_NET", !restricted) {
        return Some("route_policy:network_disabled");
    }
    if is_local_http_provider {
        if !parse_flag("VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP", !restricted) {
            return Some("route_policy:local_model_http_disabled");
        }
    } else if !parse_flag("VOX_ROUTE_ALLOW_PROVIDER_NETWORK", !restricted) {
        return Some("route_policy:provider_network_disabled");
    }
    None
}

/// Returns a static denial code when [`ModelSpec`] cannot be used under the current route policy.
#[must_use]
pub fn route_policy_exclusion_reason(model: &ModelSpec) -> Option<&'static str> {
    #[cfg(feature = "runtime")]
    {
        let snap = RouteCapabilityPolicySnapshot::from_env();
        exclusion_reason_for_llm_lane(is_local_http_provider(&model.provider_type), &snap)
    }
    #[cfg(not(feature = "runtime"))]
    {
        exclusion_reason_without_runtime(is_local_http_provider(&model.provider_type))
    }
}
```

---

### 4. `crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs` (lines 6–79)

**Analog:** `crates/vox-orchestrator/Cargo.toml` (lines 51–52: `runtime = ["vox-actor-runtime", "vox-populi/mens", "lsp"]`, `toestub-gate = ["vox-code-audit", "vox-lsp", "lsp"]`)

**Why needed:** `healing.rs:25` calls `vox_lsp::validate_document(src)` inside `#[cfg(feature = "runtime")]`, which fails under `--no-default-features --features runtime` because `vox-lsp` is only enabled by `toestub-gate`.

**Exact edit (`healing.rs:12-13` and `77-79`):**
```rust
        #[cfg(all(feature = "runtime", feature = "toestub-gate"))]
        {
            // existing lines 14-76 unchanged
        }
        #[cfg(not(all(feature = "runtime", feature = "toestub-gate")))]
        {
            let _ = (agent_id, task_desc, write_files);
        }
        Ok(())
```

---

### 5. `crates/vox-orchestrator/tests/scaling_test.rs` & `tests/context_compaction_wiring_test.rs`

**Analog:** `crates/vox-orchestrator/tests/chat_round_trip.rs` (line 11: `#![cfg(feature = "runtime")]`)

- `crates/vox-orchestrator/tests/scaling_test.rs:1`: Prepend `#![cfg(feature = "runtime")]` above `use std::sync::Arc;` (`AgentFleet` at line 5 requires `runtime`).
- `crates/vox-orchestrator/tests/context_compaction_wiring_test.rs:1`: Add `#![cfg(feature = "runtime")]` right below `#![allow(missing_docs)]` (`SessionManager::assemble_llm_messages` at line 101 requires `runtime`).

---

### 6. `crates/vox-terminal-core` + Downstream `Cargo.toml`s

**Analogs:**
- `crates/vox-terminal-core/Cargo.toml` (lines 8–31)
- `crates/vox-compiler/src/eval/caps.rs` (lines 700–728 — **hard cross-crate source-text test invariant**)
- `crates/vox-terminal-core/src/host.rs` (lines 801–839) & `src/session.rs` (lines 313–339)

1. **`crates/vox-terminal-core/Cargo.toml` (lines 8–29):**
   ```toml
   [features]
   default = []
   # Gates inline `.vox` evaluation via vox-compiler (~47k LoC); off by default for fast TUI/PTY builds.
   vox-lang = ["dep:vox-compiler"]

   [dependencies]
   ...
   vox-compiler     = { workspace = true, optional = true }
   vox-journal      = { workspace = true }
   vox-orchestrator = { workspace = true, default-features = false }
   ```
   *(CRITICAL: Do **NOT** add `vox-compiler` to `[dev-dependencies]`, or `cargo test -p vox-terminal-core` will still compile `47k LoC` on default test runs! Zero `Cargo.lock` change occurs because `vox-compiler` and `vox-orchestrator` remain in `Cargo.lock`'s `vox-terminal-core` dependency list.)*

2. **`crates/vox-terminal-core/src/vox_interp.rs` (lines 1–39) — MUST preserve `"Interpreter::new"` AND `"CapabilitySet::parse"` literal strings for `vox-compiler/src/eval/caps.rs:710-726`:**
   ```rust
   use anyhow::{Result, anyhow};

   #[cfg(feature = "vox-lang")]
   pub fn eval_line(src: &str) -> Result<String> {
       use vox_compiler::eval::Interpreter;
       use vox_compiler::pipeline::{PipelineOptions, run_frontend_str_with_options};

       let options = PipelineOptions {
           script_mode: true,
           ..PipelineOptions::default()
       };
       let res =
           run_frontend_str_with_options(src, "terminal.vox", &options).map_err(|e| anyhow!("{e}"))?;

       if res.error_count() > 0 {
           let msgs: Vec<_> = res.diagnostics.iter().map(|d| format!("{d:?}")).collect();
           return Err(anyhow!("{}", msgs.join("; ")));
       }

       let mut interp = Interpreter::new(100_000);
       interp.caps = vox_compiler::eval::caps::CapabilitySet::parse("").unwrap();
       interp.run_module(&res.hir).map_err(|e| anyhow!("{e:?}"))?;
       let val = interp.call("main", vec![]).map_err(|e| anyhow!("{e:?}"))?;
       Ok(format!("{val:?}"))
   }

   #[cfg(not(feature = "vox-lang"))]
   pub fn eval_line(_src: &str) -> Result<String> {
       Err(anyhow!(
           "inline .vox evaluation requires `vox-terminal-core` feature `vox-lang`"
       ))
   }
   ```
3. **`crates/vox-terminal-core/tests/vox_interp.rs` (line 1):** Prepend `#![cfg(feature = "vox-lang")]` (and optionally add a `#[cfg(not(feature = "vox-lang"))]` test asserting `eval_line("fn main() -> Int { 1 }").is_err()`).
4. **`crates/vox-terminal-core/tests/session.rs` (lines 79–85):** Gate the `"40 + 2"` `VoxNative` `Ok` assertion on `#[cfg(feature = "vox-lang")]`, and under `#[cfg(not(feature = "vox-lang"))]` assert `host.submit(&id, "let x = 42")` produces `BlockKind::VoxNative` with `BlockStatus::Failed` while unprefixed `"echo organic-console-line"` (lines 87–91) still executes as `BlockKind::Shell` `BlockStatus::Ok`!
5. **Downstream `Cargo.toml` updates:**
   - `crates/vox-term/Cargo.toml` (lines 7–17): Add `[features]\ndefault = []\nvox-lang = ["vox-terminal-core/vox-lang"]` while keeping `vox-terminal-core = { workspace = true }` lean by default.
   - `crates/vox-orchestrator-d/Cargo.toml:19`: Change `vox-terminal-core = { workspace = true }` to `vox-terminal-core = { workspace = true, features = ["vox-lang"] }`.
   - `crates/vox-gui/Cargo.toml:62`: Change `vox-terminal-core = { workspace = true }` to `vox-terminal-core = { workspace = true, features = ["vox-lang"] }`.

---

## Pattern Assignments — Plan 03 (CI Guard, Docs & `.vox` Broker Scripts)

### 1. `crates/vox-cli-ci/src/dev_loop_guard.rs` (lines 81–167, 202–248)

**Analog:** `crates/vox-cli-ci/src/dev_loop_guard.rs` (lines 57–92, 129–167, 202–247)

**Existing pure-check + `run(root)` pattern (`dev_loop_guard.rs:82-92, 149-158`):**
```rust
/// True when `.cargo/config.toml` commits a hardcoded `[build] rustc-wrapper` or `[env] RUSTC_WRAPPER`
/// (which breaks hosts/CI without `sccache`; `vox-cargo-shim` injects `sccache` dynamically when on PATH).
pub fn hardcoded_rustc_wrapper(cargo_config: &str) -> bool {
    let mut section = "";
    for line in cargo_config.lines().map(str::trim) {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with('[') {
            section = line;
            continue;
        }
        let key = line.split('=').next().map(str::trim).unwrap_or("");
        if section == "[build]" && key == "rustc-wrapper" {
            return true;
        }
        if section == "[env]" && key == "RUSTC_WRAPPER" {
            return true;
        }
    }
    false
}
```
- Also add pure helpers checked in `run(root)` + unit-tested in `mod tests` (where `the_repository_passes_the_dev_loop_guard` at lines 241–247 runs against the real repo!):
  - `shared_sccache_or_target_missing(cargo_config: &str) -> bool`: ensures `[env]` defines both `SCCACHE_DIR` and `CARGO_TARGET_DIR`.
  - `broker_safety_docs_missing_guidance(safety_doc: &str, broker_doc: &str) -> Vec<&'static str>`: checks `.agents/cargo-safety.md` and `docs/src/contributors/build-broker-usage.md` document `VOX_BROKER_RUN_TIMEOUT_SECS` (queue-aware timeout) and batched multi-package `cargo test -p` verification (preventing sequential single-test lock storms).

---

### 2. `docs/src/contributors/build-broker-usage.md` (lines 1–180)

**Analogs:**
- `docs/src/contributors/build-broker-usage.md` (lines 1–56)
- `docs/src/contributors/documentation-governance.md` (lines 93–110, 149–199)
- `contracts/documentation/kb-taxonomy.v1.yaml` (lines 27, 379–390: `domain: "platform"`, `topic: "build-system"`)

**Frontmatter + `<!-- kb:summary -->` contract:**
- `docs/src/contributors/build-broker-usage.md` is currently listed in `contracts/documentation/kb-facet-backlog.v1.yaml:2809-2814` (`facet-missing-{authorship,domain,summary,topic}`). Upgrading it to full KB-facet compliance satisfies `AGENTS.md §Authored Markdown Frontmatter` and passes `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/contributors/build-broker-usage.md` with exit `0` (`facet-backlog-stale` is a non-failing soft warning per `facets_tests.rs:454`, and **never** hand-edit `contracts/documentation/kb-facet-backlog.v1.yaml`):
  ```markdown
  ---
  title: "Build broker — machine-wide cargo coordination"
  description: "How the vox-cargo-shim build broker serializes per-target builds, coalesces duplicate in-flight checks/tests, auto-discovers sccache, and enforces post-queue timeouts."
  category: "Contributors"
  status: "current"
  topic: "build-system"
  domain: "platform"
  authorship: "llm-assisted"
  authorship_model: "claude-opus-4-7"
  ---

  # Build broker (machine-wide cargo coordination)

  <!-- kb:summary -->
  Explains how the daemonless `vox-cargo-shim` build broker eliminates multi-agent `target/.cargo-lock` contention by acquiring a per-target `FairQueue` lock before claiming a machine-wide adaptive slot, coalescing identical in-flight `check`/`build`/`test` runs, injecting `sccache` and isolated `VOX_WORKSTREAM` targets, and starting `VOX_BROKER_RUN_TIMEOUT_SECS` strictly after queue admission.
  <!-- /kb:summary -->
  ```
  *(Note: summary above is 46 words — strictly within the enforced 40–120 word window, placed directly under H1! Do **not** set `training_eligible: true` with `authorship: "llm-assisted"`, which triggers `facet-training-eligible-contradiction`, and do **not** add `last_updated`.)*
- Document in body:
  1. Four-stage pipeline (`inflight/` leader-follower coalescing -> `targets/<hash>/` per-target `FairQueue` -> `slots/slot_*` adaptive global cap -> post-queue `runner`).
  2. New tunables: `VOX_BROKER_RUN_TIMEOUT_SECS` (alias `VOX_BROKER_BUILD_TIMEOUT_SECS`) + `--broker-run-timeout <secs>`, `VOX_WORKSTREAM` + `VOX_ISOLATED_TARGET=1` / `.vox/agent-target`, `VOX_BROKER_NO_SCCACHE=1`.
  3. Updated `vox-broker stats` (`target_p95`, `slot_p95`, `compile_p50`, `coalesced`, `timeouts`) and `broker.log` fields (`target_wait`, `slot_wait`, `compile`, `coalesced`, `timeout`).
  4. Multi-package batching rule (`cargo test -p crate-a -p crate-b --lib --tests` in one invocation instead of N sequential per-test invocations).

---

### 3. `.agents/cargo-safety.md` (tracked) & `.agents/rules/00-core.md` (gitignored local driver file)

**Analog:** `.agents/cargo-safety.md` (lines 5–12)

- **`.agents/cargo-safety.md` (tracked in git — safe to commit):**
  - Update rules 1–3 to document: (a) target-first `FairQueue` + leader-follower coalescing in `vox-cargo-shim`, (b) batching multi-crate checks/tests (`cargo test -p a -p b --lib --tests`) to acquire `target/.cargo-lock` once per wave, (c) preferring `VOX_BROKER_RUN_TIMEOUT_SECS=900` (or `--broker-run-timeout 900`) so queue wait never burns active compile timeout (`exit 124`), and (d) `VOX_ISOLATED_TARGET=1` + `VOX_WORKSTREAM=<slug>` (`target/ws-<slug>`) instead of ad-hoc `target_agent` folders.
- **`.agents/rules/00-core.md` (gitignored by `/.agents/` at `.gitignore:305`):**
  - If updated locally on lines 30–35 for active `drive-task` sessions, document in the plan that orchestrator commit steps **must not** `git add .agents/rules/00-core.md`.

---

### 4. `scripts/broker-install.vox` & `scripts/broker-ci.vox`

**Analogs:** `scripts/broker-install.vox` (lines 46–66, 216–238, 248–253) & `scripts/broker-ci.vox` (lines 118–139)

- **Vox compiler/codegen rules (verified in `scripts/broker-install.vox:287-308` & `scripts/broker-bypass-lint.vox:230-236`):**
  - Never call `.sorted()` on `list[str]` (unimplemented in Rust codegen).
  - Never write `match res { Ok(x) => { ... } Error(_) => {} }` with a unit `{}` arm unless the sibling arm exits via `process.exit(...)`; prefer `if res.is_ok() { ... }`.
  - Never name `.cargo/bin/cargo` or `rustup which cargo` in `scripts/*.vox` (`scripts/broker-bypass-lint.vox` scans `scripts/`!). Note `scripts/broker-install.vox:173` uses `path.join(home, ".cargo/bin")` (safe: case 1/3 only match when `/cargo` is included).
- **`scripts/broker-install.vox` (lines 62–66 & 216–238):** Update `usage()` and dry-run summary output to note per-target `FairQueue` + coalescing, dynamic `sccache` detection, and `VOX_BROKER_RUN_TIMEOUT_SECS`. Note line 250 runs `["build", "--release", "--manifest-path", manifest, "--message-format=json"]` — which is why `coalesce::is_coalesceable` MUST skip `--message-format*`!
- **`scripts/broker-ci.vox` (lines 118–130):** Check `CARGO_TARGET_DIR` if set, else check `"crates/vox-cargo-shim/target"` AND fallback `"target"` (since running `cargo` from `<repo_root>` picks up `.cargo/config.toml:95` `CARGO_TARGET_DIR = { value = "target", relative = true }`):
  ```vox
  let mut bin_path = path.join(path.join(root, target_dir), path.join("release", bin_name));
  if (not target_dir_opt.is_some()) and (not fs.is_file(bin_path)) {
      let ws_bin = path.join(path.join(root, "target"), path.join("release", bin_name));
      if fs.is_file(ws_bin) {
          bin_path = ws_bin;
      }
  }
  ```

---

## Shared Patterns & Guardrails Across All Plans

### 1. Windows `os error 33` Mandatory Lock Invariant
**Source:** `crates/vox-build-queue/src/queue.rs` (lines 66–68)
**Apply to:** `coalesce.rs`, `queue.rs`, `broker_coalesce_and_timeout.rs`
- On Windows, `fs2::FileExt::lock_exclusive` / `try_lock_exclusive` calls Win32 `LockFileEx` over bytes `0..u64::MAX`. Any `ReadFile`/`WriteFile` via a **second** `std::fs::File` handle on that locked path fails with `ERROR_LOCK_VIOLATION` (`os error 33`).
- Either read/write exclusively through the single locked handle (`queue.rs:68-75`) OR lock a dedicated 0-byte `<hash>.lock` file and store all shared data in unlocked sibling files (`<hash>.meta`, `<hash>.stdout`, `<hash>.stderr`, `<hash>.exit`).

### 2. `vox-drift-check` Static Detector Hygiene (`timeout-literal` & `vox-path-literal`)
**Source:** `crates/vox-drift-check/src/rules/timeout_literal.rs` (lines 8–35) & `vox_path_literal.rs` (lines 30–39)
**Apply to:** All modified/new `.rs` and `.vox` files
- Never pass inline `100`, `250`, `500`, `1000`, `5000` directly into `Duration::from_millis(...)` or `5`, `10`, `15`, `30`, `60`, `300`, `600` into `Duration::from_secs(...)` inside non-test `.rs` code: bind a `const POLL_MS: u64 = 50;` at module top (`in_const == true` is exempt).
- Never write a non-`const` string literal starting with `".vox/"` (e.g. `".vox/agent-target"`): either declare `const AGENT_TARGET_FILE: &str = ".vox/agent-target";` or construct `wt.join(".vox").join("agent-target")` (`global.rs:28` pattern).

### 3. `< 500` LoC `god_object` Ceiling & Individual `rustfmt`
**Source:** `.agents/rules/00-core.md` (Rule 7) & `AGENTS.md`
**Apply to:** All `.rs` files
- Every `.rs` file must stay strictly `< 500` lines (`global.rs` = 483 LoC — do not grow; keep `coalesce.rs`, `resolve.rs`, `main.rs`, `runner.rs`, `vox-broker.rs`, `dev_loop_guard.rs` each well below 450 LoC).
- Format only touched `.rs` files via `rustfmt --edition 2024 <file.rs>` (never `cargo fmt`).

---

## No Analog Found

None — every new and modified file has a direct, verified git-tracked repository analog.

---

## Metadata

**Analog search scope:** `crates/vox-build-queue/`, `crates/vox-cargo-shim/`, `crates/vox-orchestrator/`, `crates/vox-actor-runtime/`, `crates/vox-terminal-core/`, `crates/vox-compiler/src/eval/`, `crates/vox-term/`, `crates/vox-orchestrator-d/`, `crates/vox-gui/`, `crates/vox-cli-ci/`, `crates/vox-drift-check/`, `docs/src/contributors/`, `scripts/`, `.agents/`, `contracts/documentation/`
**Files scanned:** 34
**Pattern extraction date:** 2026-11-15
