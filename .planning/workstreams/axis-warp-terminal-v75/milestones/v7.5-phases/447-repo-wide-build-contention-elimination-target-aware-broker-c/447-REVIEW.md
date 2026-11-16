---
phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c
reviewed: 2026-11-15T09:24:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - crates/vox-build-queue/src/lib.rs
  - crates/vox-build-queue/src/coalesce.rs
  - crates/vox-build-queue/src/queue.rs
  - crates/vox-build-queue/src/resolve.rs
  - crates/vox-build-queue/src/metrics.rs
  - crates/vox-cargo-shim/src/runner.rs
  - crates/vox-cargo-shim/src/main.rs
  - crates/vox-cargo-shim/src/bin/vox-broker.rs
  - crates/vox-cargo-shim/tests/vox_broker_readonly.rs
  - crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs
  - crates/vox-orchestrator/src/victory.rs
  - crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs
  - crates/vox-orchestrator/src/route_policy.rs
  - crates/vox-orchestrator/src/orchestrator/task_dispatch/complete/success/healing.rs
  - crates/vox-orchestrator/tests/scaling_test.rs
  - crates/vox-orchestrator/tests/context_compaction_wiring_test.rs
  - crates/vox-terminal-core/Cargo.toml
  - crates/vox-terminal-core/src/vox_interp.rs
  - crates/vox-terminal-core/tests/vox_interp.rs
  - crates/vox-terminal-core/tests/session.rs
  - crates/vox-term/Cargo.toml
  - crates/vox-orchestrator-d/Cargo.toml
  - crates/vox-gui/Cargo.toml
  - crates/vox-cli-ci/src/dev_loop_guard.rs
  - .agents/cargo-safety.md
  - docs/src/contributors/build-broker-usage.md
  - scripts/broker-install.vox
  - scripts/broker-ci.vox
findings:
  critical: 2
  warning: 6
  info: 2
  total: 10
status: issues_found
---

# Phase 447: Code Review Report

**Reviewed:** 2026-11-15T09:24:00Z
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Reviewed all 28 files in Phase 447 across `vox-build-queue`, `vox-cargo-shim`, `vox-orchestrator`, `vox-terminal-core`, `vox-term`, `vox-orchestrator-d`, `vox-gui`, `vox-cli-ci`, `.agents/cargo-safety.md`, `docs/src/contributors/build-broker-usage.md`, and `scripts/broker-{install,ci}.vox`.

**Verified clean architectural invariants:**
- **Target-first lock ordering (`crates/vox-cargo-shim/src/main.rs:275-328`):** Per-target `FairQueue` acquisition strictly precedes `global::acquire_slot_adaptive`, eliminating global slot starvation by sleeping `target/.cargo-lock` waiters.
- **Windows `os error 33` safety (`crates/vox-build-queue/src/coalesce.rs:313-345`):** `fs2` locks are confined exclusively to 0-byte `<key>.lock` files; `.meta`/`.exit` use `.tmp` + `rename`, and `.stdout`/`.stderr` are unlocked files stored under `<broker_home>/inflight/flights/` (avoiding `global::register_inflight`'s stale-file reaper).
- **Backward-compatible `MetricRecord` JSONL (`crates/vox-build-queue/src/metrics.rs:11-31, 150-176`):** All 8 legacy fields are preserved verbatim, all 5 new fields carry `#[serde(default)]`, legacy `compile_ms == 0 && !coalesced_hit && ran_ms > 0` records fall back cleanly, and `vox-broker` remains 100% read-only.
- **Feature-gate decoupling & source-text contracts (`crates/vox-orchestrator/`, `crates/vox-terminal-core/`):** `vox-orchestrator` compiles cleanly across default, `--no-default-features`, and `--no-default-features --features runtime` (including compound `#[cfg(all(feature = "runtime", feature = "toestub-gate"))]` in `healing.rs:12,78` and fail-closed parity across all 4 `VOX_ROUTE_*` env vars in `route_policy.rs:24-59`); `vox-terminal-core/src/vox_interp.rs:30-35` preserves the exact 1-arg `eval_line`, `Interpreter::new(100_000)`, and `CapabilitySet::parse("").unwrap()` signatures asserted by `crates/vox-compiler/src/eval/caps.rs:700-728`.

**Key defects requiring remediation:**
- **2 Critical (`BLOCKER`) issues:** `coalesce_key` omits `cwd` and `+toolchain`/`RUSTUP_TOOLCHAIN` (causing false-green coalescing across different workspace member directories or toolchains), and `run_child_post_queue` performs an unbounded `JoinHandle::join()` on stdout/stderr tee pipes after child exit/timeout while holding `FairQueue` and global slot locks (deadlocking on background grandchildren such as `sccache`'s auto-spawned server daemon).
- **6 Warnings (`WARNING`):** Exclusive lock probing in `is_lock_held`, incomplete mtime invalidation in `latest_source_mtime_ms` / Stage B wait, argument swallowing in `strip_broker_flags` + `VOX_BROKER_RUN_TIMEOUT_SECS=0` precedence bug, external-`cwd` `--manifest-path` target resolution in `effective_target_dir`, unconditional `CARGO_TERM_COLOR=always`, and ambient env leakage (`VOX_BROKER_DEPTH=2` -> exit 70) in `broker_coalesce_and_timeout.rs`.
- **2 Info (`INFO`) items:** Mismatched `vox-broker stats` verdict text in `build-broker-usage.md` and un-reaped flight artifacts under `inflight/flights/`.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: Coalescing key omits `cwd` and `+toolchain` / `RUSTUP_TOOLCHAIN`, causing false-green cross-crate and cross-toolchain coalescing

**Severity:** BLOCKER
**File:** `crates/vox-cargo-shim/src/main.rs:243-245`
**Issue:** `coalesce_key` hashes only `"{effective_sub}:{argv_hash:016x}:{env_hash:016x}:{target_hash}"`:
1. Inside any Cargo workspace worktree, `resolve::effective_target_dir` resolves to `<worktree>/target` regardless of which member subdirectory `cwd` is in (`crates/vox-build-queue/src/resolve.rs:160-161`), and `env_filter::DENYLIST` strips `PWD`. When Caller A runs `cargo test` or `cargo check` (without `-p`/`--workspace`, or with a relative `--manifest-path`) from `/repo/crates/crate-a` and Caller B concurrently (or within 30s) runs `cargo test` or `cargo check` from `/repo/crates/crate-b`, both compute the **identical** `coalesce_key`. Caller B coalesces onto Caller A's run and exits `0` without ever compiling or testing `crate-b`.
2. `main.rs:156-161` strips `+toolchain` from `args` before `argv_hash` is computed, and `RUSTUP_TOOLCHAIN` is neither inserted into `env` prior to `env_hash` (`main.rs:218-241`) nor present in `env_filter::FINGERPRINT_ALLOW` (`crates/vox-build-queue/src/env_filter.rs:24-40`). Consequently, `cargo +nightly check -p foo` coalesces with stable `cargo check -p foo` and replays the wrong toolchain's exit status and diagnostics.

**Fix:** Include canonicalized `cwd` and the resolved toolchain in `coalesce_key` (`crates/vox-cargo-shim/src/main.rs:240-245`):
```rust
let cwd_hash = queue::hash_path(&cwd);
let effective_tc = toolchain
    .or_else(|| std::env::var("RUSTUP_TOOLCHAIN").ok().as_deref().map(|_| ""))
    .unwrap_or("");
let tc_tag = match toolchain {
    Some(tc) => tc.to_string(),
    None => std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_default(),
};
let _ = effective_tc;
let coalesce_key = queue::hash_path(Path::new(&format!(
    "{effective_sub}:{argv_hash:016x}:{env_hash:016x}:{target_hash}:{cwd_hash}:{tc_tag}"
)));
```

### CR-02: Unbounded post-exit `JoinHandle::join()` on tee threads deadlocks `FairQueue` and leaks global slots when grandchildren inherit stdio pipes

**Severity:** BLOCKER
**File:** `crates/vox-cargo-shim/src/runner.rs:153-200`
**Issue:** Every coalesceable leader invocation spawns Cargo with `Stdio::piped()` (`runner.rs:154`) and unconditionally blocks on `out_handle.join()` and `err_handle.join()` (`runner.rs:195-200`) *after* the timeout supervision loop exits — while `main.rs:338-356` still holds `_slot`, `_inflight`, and `_target_ticket` (`FairQueue` `run.lock`). Whenever Cargo or a build/test child spawns a long-lived background process that inherits the stdout/stderr pipe handles — most notably **`sccache` auto-spawning its background server daemon on cold start** (which `main.rs:219-226` auto-injects via `RUSTC_WRAPPER`), a test spawning a detached helper in its own `process_group(0)` (escaping `killpg`), or Windows `CreateProcess` handle inheritance — `child.try_wait()` (or `kill_process_tree` on timeout) returns, the deadline loop exits, and `h.join()` hangs indefinitely waiting for pipe EOF. This defeats `VOX_BROKER_RUN_TIMEOUT_SECS` and permanently wedges the target's `FairQueue` for all subsequent builds.

**Fix:** Signal tee-thread completion over a bounded channel and bound the post-exit drain wait instead of calling unbounded `JoinHandle::join()`:
```rust
fn spawn_tee_thread(
    mut reader: impl Read + Send + 'static,
    file_path: PathBuf,
    is_stderr: bool,
) -> std::sync::mpsc::Receiver<()> {
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .ok();
        let mut buf = [0u8; 8192];
        while let Ok(n) = reader.read(&mut buf) {
            if n == 0 {
                break;
            }
            let chunk = &buf[..n];
            if is_stderr {
                let mut err = std::io::stderr();
                let _ = err.write_all(chunk);
                let _ = err.flush();
            } else {
                let mut out = std::io::stdout();
                let _ = out.write_all(chunk);
                let _ = out.flush();
            }
            if let Some(f) = log_file.as_mut() {
                let _ = f.write_all(chunk);
                let _ = f.flush();
            }
        }
        let _ = tx.send(());
    });
    rx
}

// After child exit / timeout kill (lines 195-200):
let drain_wait = Duration::from_millis(500);
if let Some(rx) = out_rx {
    let _ = rx.recv_timeout(drain_wait);
}
if let Some(rx) = err_rx {
    let _ = rx.recv_timeout(drain_wait);
}
```

## Warnings

### WR-01: `is_lock_held` uses `try_lock_exclusive()`, causing false-positive leader detection (stale receipt replay) and starving `claim_leader`

**Severity:** WARNING
**File:** `crates/vox-build-queue/src/coalesce.rs:81-96, 221-267, 313-321`
**Issue:** Completed leaders leave `<key>.lock`, `.meta`, `.stdout`, `.stderr`, and `.exit` on disk indefinitely (`LeaderGuard::finish` at lines 47-59 only unlocks). `is_lock_held` probes `<key>.lock` by acquiring an *exclusive* lock (`f.try_lock_exclusive()`) and immediately unlocking:
1. When two callers probe an idle `<key>.lock` concurrently, Caller A briefly holds `try_lock_exclusive()`; Caller B observes `Err(_)` (`is_lock_held == true`), reads the leftover `.meta` from an arbitrarily old run (which passes `meta.started_ms >= min_started_ms` whenever no source mtime changed since that old run), sees `held == false` on the next poll, and replays an arbitrarily stale `.exit` because `try_follow_inflight` (`lines 262-267`) never checks `RECEIPT_FRESH_WINDOW_MS` or `exit.finished_ms >= loop_entry_ms`.
2. Conversely, if a follower's `is_lock_held` exclusive probe overlaps with the Stage C leader's single `lock_file.try_lock_exclusive()` in `claim_leader` (`line 319`), `claim_leader` returns `Ok(None)`, silently disabling coalescing for that leader build.
3. Back-to-back leaders overwrite `<key>.stdout`/`.stderr` (`lines 327-328`) and unlink `<key>.exit` (`line 326`) in place; if a Stage A follower from Run 1 is between its final poll and reading `.exit`, it already streamed Run 1's output to stdout/stderr, then gets `None` on `.exit` and runs Cargo again (emitting duplicate build output).

**Fix:** Probe `<key>.lock` with `FileExt::try_lock_shared()` (opening read-only on Unix/Windows) so read probes never contend with each other; retry `try_lock_exclusive()` briefly (e.g., 3x 5ms) in `claim_leader`; validate `exit.finished_ms` freshness in `try_follow_inflight`; and scope `.stdout`/`.stderr`/`.exit` filenames by `run_id` (or buffer Stage A follower output until `exit.run_id == meta.run_id` is verified):
```rust
fn is_lock_held(lock_path: &Path) -> bool {
    let Ok(f) = std::fs::OpenOptions::new().read(true).open(lock_path) else {
        return false;
    };
    match f.try_lock_shared() {
        Ok(()) => {
            let _ = FileExt::unlock(&f);
            false
        }
        Err(_) => true,
    }
}
```

### WR-02: `latest_source_mtime_ms` misses file deletions, non-`crates/` workspace inputs, and edits made while waiting in `FairQueue`

**Severity:** WARNING
**File:** `crates/vox-build-queue/src/coalesce.rs:161-209` (and `crates/vox-cargo-shim/src/main.rs:247-295`)
**Issue:** Three freshness gaps allow stale replay after workspace mutations:
1. `max_mtime_walk` (`coalesce.rs:178-182`) only checks `file_mtime_ms(&p)` when `ft.is_file()`, ignoring directory mtimes (`ft.is_dir()`). Deleting or renaming a `.rs` module or integration test updates only the parent directory's mtime, so `latest_source_mtime_ms` does not advance and a pre-deletion run is replayed as fresh.
2. When `worktree.join("crates").is_dir()` is true (`lines 199-205`), `latest_source_mtime_ms` skips `contracts/`, `.agents/`, `rust-toolchain.toml`, and `lefthook.yml`, which are embedded via `include_str!` or asserted by workspace tests (including `dev_loop_guard::the_repository_passes_the_dev_loop_guard`).
3. `main.rs:247-251` computes `src_mtime` once *before* `fq.acquire(seq)` (`line 283`) and passes that stale `src_mtime` into Stage B `try_replay_recent_leader` (`line 291`). If a file is edited while the follower waits in `FairQueue` (after a concurrent leader started), Stage B fails to notice the edit and replays the pre-edit leader result.

**Fix:** Record directory mtimes in `max_mtime_walk`, include root config/contract trees in `latest_source_mtime_ms`, and refresh `src_mtime` after `fq.acquire(seq)` in `main.rs`:
```rust
// In crates/vox-build-queue/src/coalesce.rs:178-183:
if ft.is_dir() {
    *acc = (*acc).max(file_mtime_ms(&p));
    max_mtime_walk(&p, acc);
} else if ft.is_file() {
    *acc = (*acc).max(file_mtime_ms(&p));
}
```
And in `crates/vox-cargo-shim/src/main.rs:286`:
```rust
let src_mtime_post_wait = if can_coalesce {
    coalesce::latest_source_mtime_ms(&worktree)
} else {
    0
};
```

### WR-03: `strip_broker_flags` swallows the following Cargo flag when `--broker-run-timeout` lacks a numeric argument, and `VOX_BROKER_RUN_TIMEOUT_SECS=0` fails to override `VOX_BROKER_BUILD_TIMEOUT_SECS`

**Severity:** WARNING
**File:** `crates/vox-cargo-shim/src/runner.rs:17-31, 53-62`
**Issue:**
1. In `strip_broker_flags` (`lines 53-60`), if `--broker-run-timeout` is followed by a Cargo flag instead of an integer (e.g. `cargo --broker-run-timeout -p vox-cli check`), `val.trim().parse::<u64>()` fails, `cli_secs` stays `None`, and `i += 2` unconditionally drops `"-p"`, turning a single-crate command into `cargo vox-cli check` or `cargo test foo` across the whole workspace.
2. `docs/src/contributors/build-broker-usage.md:50` states that `VOX_BROKER_RUN_TIMEOUT_SECS=0` disables the timeout and takes precedence over `VOX_BROKER_BUILD_TIMEOUT_SECS`. However, `resolve_timeout_secs_from` (`line 30`) chains `parse_nonzero_secs(run_env).or_else(|| parse_nonzero_secs(build_env))`; because `parse_nonzero_secs(Some("0"))` returns `None`, `VOX_BROKER_RUN_TIMEOUT_SECS=0` falls through to `VOX_BROKER_BUILD_TIMEOUT_SECS` instead of disabling the timeout.

**Fix:** Parse `run_env` as `Option<u64>` before filtering `> 0`, and only consume `args[i + 1]` when it parses as `u64` (or does not start with `'-'`):
```rust
fn parse_secs_opt(raw: Option<&str>) -> Option<u64> {
    raw.map(str::trim).filter(|s| !s.is_empty()).and_then(|s| s.parse::<u64>().ok())
}

fn resolve_timeout_secs_from(
    cli_secs: Option<u64>,
    run_env: Option<&str>,
    build_env: Option<&str>,
) -> Option<u64> {
    let chosen = cli_secs
        .or_else(|| parse_secs_opt(run_env))
        .or_else(|| parse_secs_opt(build_env));
    chosen.filter(|&n| n > 0)
}

// In strip_broker_flags (lines 53-62):
if cur == "--broker-run-timeout" {
    if let Some(val) = args.get(i + 1)
        && let Ok(parsed) = val.trim().parse::<u64>()
    {
        cli_secs = Some(parsed);
        i += 2;
        continue;
    }
    i += 1;
    continue;
}
```

### WR-04: `effective_target_dir` resolves `<cwd>/target` instead of `<worktree>/target` when `--manifest-path` points into a worktree from an external `cwd`

**Severity:** WARNING
**File:** `crates/vox-build-queue/src/resolve.rs:146-161`
**Issue:** When `cwd` is outside any git worktree (`worktree_root_of(cwd).is_none()`) and `--manifest-path /path/to/vox/crates/foo/Cargo.toml` points inside a git worktree (`worktree_root_of(manifest_dir)` is `Some(wt)`), the guard at line 149 (`if worktree_root_of(cwd).is_none() && worktree_root_of(manifest_dir).is_none()`) is `false`, so execution falls through to line 160 (`worktree_root_of(cwd).unwrap_or_else(|| cwd.to_path_buf()).join("target")`), returning `<external_cwd>/target` while Cargo actually compiles and locks `<wt>/target`. Two processes invoking `--manifest-path` into the same worktree from different external working directories will bypass the shared `FairQueue`.

**Fix:** Prefer `worktree_root_of(manifest_dir)` whenever `--manifest-path` is supplied:
```rust
if let Some(mp) = manifest_path_arg(args) {
    let abs_mp = if mp.is_absolute() { mp } else { cwd.join(mp) };
    if let Some(manifest_dir) = abs_mp.parent() {
        if let Some(wt) = worktree_root_of(manifest_dir) {
            return normalize_target_path(&wt, &wt.join("target"));
        }
        if worktree_root_of(cwd).is_none() {
            let ws_root = cargo_workspace_root_of(manifest_dir)
                .unwrap_or_else(|| manifest_dir.to_path_buf());
            return normalize_target_path(&ws_root, &ws_root.join("target"));
        }
    }
}
```

### WR-05: `run_child_post_queue` unconditionally forces `CARGO_TERM_COLOR=always`, overriding caller `CARGO_TERM_COLOR=never` and `NO_COLOR`

**Severity:** WARNING
**File:** `crates/vox-cargo-shim/src/runner.rs:135-136`
**Issue:** `cmd.envs(env); cmd.env("CARGO_TERM_COLOR", "always");` overwrites an explicit `CARGO_TERM_COLOR=never` in `env`, ignores `NO_COLOR`, and injects ANSI escape sequences into non-TTY stderr logs (including CI logs, `vox-cli` subprocess captures, and non-coalesced `--message-format=json` stderr diagnostics).

**Fix:** Set `CARGO_TERM_COLOR=always` only when neither `CARGO_TERM_COLOR` nor `NO_COLOR` was set by the caller and stderr is an interactive terminal:
```rust
if !env.contains_key("CARGO_TERM_COLOR")
    && std::env::var_os("NO_COLOR").is_none()
    && std::io::IsTerminal::is_terminal(&std::io::stderr())
{
    cmd.env("CARGO_TERM_COLOR", "always");
}
```

### WR-06: `broker_coalesce_and_timeout.rs` inherits ambient `VOX_BROKER_*` and `CARGO_TARGET_DIR` from parent `cargo test`, failing with exit 70 under nested broker runs

**Severity:** WARNING
**File:** `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs:89-102, 113-124, 138-150, 200-209, 261-270, 312-353`
**Issue:** Test child `Command::new(shim_cargo_bin())` invocations do not scrub `VOX_BROKER_DEPTH`, `CARGO_TARGET_DIR`, `VOX_ISOLATED_TARGET`, `VOX_WORKSTREAM`, `VOX_BROKER_COALESCE`, `VOX_BROKER_BYPASS`, or `VOX_BROKER_BUILD_TIMEOUT_SECS`. When `scripts/broker-ci.vox` is launched via `scripts/vox-dev.sh` (`cargo run -p vox-cli -- run ...`) on a developer machine where the `cargo` shim is installed on `PATH`, the outer `cargo run` sets `VOX_BROKER_DEPTH=1`, `broker-ci.vox`'s `cargo test -p vox-cargo-shim` sets `VOX_BROKER_DEPTH=2`, and every shim invocation inside `broker_coalesce_and_timeout.rs` immediately aborts at `main.rs:151-154` with exit code `70` (`recursion detected`). In addition, `.cargo/config.toml`'s `CARGO_TARGET_DIR = "target"` leaks into `different_target_dirs_run_concurrently_without_serializing` (`lines 192-218`, overriding the standalone fixture's default target dir to a relative `"target"`) and `post_queue_timeout_excludes_queue_wait_and_kills_stalled_build` (`lines 261-270`).

**Fix:** Construct all shim test commands via a sanitizing helper in `crates/vox-cargo-shim/tests/broker_coalesce_and_timeout.rs`:
```rust
fn clean_shim_cmd() -> Command {
    let mut c = Command::new(shim_cargo_bin());
    for k in [
        "VOX_BROKER_DEPTH",
        "VOX_BROKER_BYPASS",
        "VOX_BROKER_COALESCE",
        "VOX_BROKER_RUN_TIMEOUT_SECS",
        "VOX_BROKER_BUILD_TIMEOUT_SECS",
        "VOX_BROKER_RESERVED_SLOTS",
        "VOX_BROKER_MAX_CONCURRENT",
        "VOX_ISOLATED_TARGET",
        "VOX_WORKSTREAM",
        "CARGO_TARGET_DIR",
        "RUSTC_WRAPPER",
    ] {
        c.env_remove(k);
    }
    c
}
```

## Info

### IN-01: `docs/src/contributors/build-broker-usage.md` sample `vox-broker stats` verdict contradicts `Summary::render()`

**Severity:** INFO
**File:** `docs/src/contributors/build-broker-usage.md:120`
**Issue:** Line 120 renders `-> keep-cap-only (coalesce rate < 15% or p95 wait < 5s)` (immediately below `coalesce_rate=28.6%`), whereas `Summary::render()` in `crates/vox-build-queue/src/metrics.rs:77-81` exclusively emits `"BUILD-DAEMON: recommended (coalesce>=10% and queue waits present)"` or `"BUILD-DAEMON: not needed (daemonless shim sufficient)"`.
**Fix:** Update line 120 of `docs/src/contributors/build-broker-usage.md` to `  -> BUILD-DAEMON: recommended (coalesce>=10% and queue waits present)`.

### IN-02: Completed flight artifacts under `<broker_home>/inflight/flights/` lack stale-file reaping

**Severity:** INFO
**File:** `crates/vox-build-queue/src/coalesce.rs:70-72, 305-346`
**Issue:** Moving coalescing artifacts from `inflight/` to `inflight/flights/` deliberately bypassed `global::register_inflight`'s reaper (`crates/vox-build-queue/src/global.rs:133-155`), leaving 5 files (`.lock`, `.meta`, `.stdout`, `.stderr`, `.exit`) per unique `coalesce_key` on disk forever even though receipts expire after `RECEIPT_FRESH_WINDOW_MS` (`30_000ms`).
**Fix:** Opportunistically unlink unlocked `<hash>.{meta,stdout,stderr,exit,lock}` sets older than e.g. 5 minutes inside `claim_leader`.

---

_Reviewed: 2026-11-15T09:24:00Z_
_Reviewer: Antigravity (gsd-code-reviewer)_
_Depth: standard_
