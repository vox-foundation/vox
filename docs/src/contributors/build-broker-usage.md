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
Explains how the daemonless `vox-cargo-shim` build broker eliminates multi-agent `target/.cargo-lock` contention across worktrees. The shim runs a four-stage pipeline: leader-follower flight coalescing under `inflight/flights/`, per-target `FairQueue` serialization under `targets/<target_hash>/` before global slot acquisition, adaptive machine-wide concurrency slots under `slots/slot_*`, and post-queue process-group execution with `VOX_BROKER_RUN_TIMEOUT_SECS`. Covers zero-config `sccache` discovery, opt-in `VOX_WORKSTREAM` target isolation, multi-package `cargo test -p` batching, and read-only `vox-broker` telemetry inspection.
<!-- /kb:summary -->

When many agents, IDE tabs, and verification jobs build across worktrees on one machine, they pile up on Cargo's `target/.cargo-lock` and package-cache locks while saturating CPU, RAM, and disk I/O. The build broker is a daemonless `cargo`-named shim (`crates/vox-cargo-shim`) placed on `PATH` ahead of the rustup proxy so concurrent builds coordinate through a deterministic, observable four-stage pipeline.

## Four-stage admission pipeline

Every intercepted `build`, `test`, `check`, `clippy`, `run`, or `bench` invocation passes through four ordered stages before invoking the real toolchain `cargo` (preserving `rust-toolchain.toml` and `+toolchain` overrides):

1. **Stage A — In-flight leader-follower coalescing (`inflight/flights/`):** After computing the child environment (`sccache` discovery and optional workstream target isolation) and canonical `CARGO_TARGET_DIR`, the shim hashes `(effective_subcmd, argv_hash, env_hash, target_hash, cwd_hash, toolchain)`. Eligible `check`, `build`, `clippy`, `test`, and `cargo run` build-phase invocations attach directly to an active leader's unlocked per-flight `.stdout`/`.stderr`/`.exit` streams guarded by a 0-byte `fs2`-locked `.lock` file without claiming any queue ticket or global slot. Coalescing automatically invalidates when workspace source, manifest, contract, or directory mtimes advance, and skips `--message-format*` calls or snapshot-mutating test runs (`INSTA_UPDATE`, `UPDATE_EXPECT`, `SNAPSHOT_UPDATE`, `TRYBUILD=overwrite`).
2. **Stage B — Per-target `FairQueue` (`targets/<target_hash>/`):** Callers targeting the same canonical `CARGO_TARGET_DIR` serialize in FIFO ticket order **before** acquiring a machine-wide concurrency slot, keeping global slots free for independent worktrees or isolated `--target-dir` builds. Immediately after `FairQueue::acquire`, a queued duplicate replays a fresh leader receipt (`try_replay_recent_leader`) in `0ms` without taking a global slot.
3. **Stage C — Adaptive machine-wide slot cap (`slots/slot_*` and `inflight/`):** Once at the head of its target queue, the leader claims an adaptive cross-process file-lock slot bounded by host parallelism, 1-minute load average, and `VOX_BROKER_RESERVED_SLOTS`.
4. **Stage D — Post-queue process-group runner (`runner`):** Active execution starts inside an isolated process group (`process_group(0)` on Unix; `CREATE_NO_WINDOW` + `taskkill /T /F` tree reaper on Windows). The `VOX_BROKER_RUN_TIMEOUT_SECS` timer begins strictly **after** Stages B and C complete; for `cargo run`, once the brokered build step exits `0`, the shim hands off to real `cargo run` via `exec_real` outside broker locks.

State lives outside any checkout at `~/.vox/build-broker/` (overridable via `VOX_BROKER_HOME`) so `cargo clean` and `git clean` never wipe active locks or history:

- `targets/<target_hash>/` — per-target FIFO ticket markers and `run.lock`.
- `inflight/flights/` — 0-byte `.lock` files paired with unlocked `.meta`, `.stdout`, `.stderr`, and `.exit` coalescing artifacts.
- `inflight/*` — top-level active build markers for concurrency accounting.
- `slots/slot_*` — machine-wide N-slot file-lock semaphore.
- `metrics.jsonl` — backward-compatible JSONL telemetry records.
- `broker.log` — human-readable per-build audit lines across all worktrees.

Safety: the shim prefers the `~/.cargo/bin` rustup proxy, skips sibling copies in its own directory, and enforces a hard `VOX_BROKER_DEPTH` guard that aborts with exit `70` at depth `>= 2` so a misconfigured `PATH` never fork-bombs.

## Tunables (env and CLI)

- **Adaptive slot sizing (default):** The first build always starts; additional builds start while the 1-minute load average stays below host parallelism. On platforms without load-average reporting, builds serialize one at a time unless overridden.
- `VOX_BROKER_MAX_CONCURRENT` — optional upper bound on simultaneous machine-wide builds (default: available host parallelism modulated by load).
- `VOX_BROKER_RESERVED_SLOTS` — slots subtracted from the host cap (`max(1, base_cap - reserved)`) for containerized runners sharing CPU/RAM across a separate mount namespace. Non-numeric or negative values are treated as `0`.
- `VOX_BROKER_HOME` — relocate broker state directory (default: `~/.vox/build-broker`, used for test isolation).
- `VOX_BROKER_DEBUG` — print resolved shim/toolchain/argument metadata to stderr and exit `0` without building.
- `VOX_BROKER_NO_SCCACHE=1` — disable automatic `RUSTC_WRAPPER=sccache` injection. When unset and `RUSTC_WRAPPER` is not already defined in the caller environment, the shim scans `PATH` for `sccache` and injects it automatically (never commit hardcoded `[build] rustc-wrapper` or `[env] RUSTC_WRAPPER` in `.cargo/config.toml`; enforced by `vox ci dev-loop-guard`).
- `VOX_BROKER_COALESCE=0` — disable Stage A/B leader-follower flight coalescing (`1`/unset enables coalescing for eligible subcommands).
- `VOX_ISOLATED_TARGET=1` and `VOX_WORKSTREAM=<slug>` — opt-in workstream target isolation. When no explicit `--target-dir`, `--manifest-path`, or non-empty `CARGO_TARGET_DIR` is supplied, the shim routes `CARGO_TARGET_DIR` to `<worktree>/target/ws-<slug>` (or reads an ASCII `[A-Za-z0-9_-]{1,48}` slug from `<worktree>/.vox/agent-target`). When both isolated target routing and `sccache` are active, the shim sets `CARGO_INCREMENTAL=0` (if unset) so crate artifacts share `.sccache` across workstreams.
- `VOX_BROKER_RUN_TIMEOUT_SECS=<secs>` (alias `VOX_BROKER_BUILD_TIMEOUT_SECS=<secs>`) and CLI flag `--broker-run-timeout <secs>` — post-queue active execution timeout (precedence: `--broker-run-timeout` > `VOX_BROKER_RUN_TIMEOUT_SECS` > `VOX_BROKER_BUILD_TIMEOUT_SECS`; `0` or unset disables). Starts counting strictly after `target_wait` and `slot_wait` finish; on expiry, kills the child process tree and returns exit code `124`. Keep outer `timeout 900s` only as a coarse wall-clock backstop.

## Batching multi-package checks and tests

Even with per-target `FairQueue` fairness, invoking Cargo `N` times sequentially re-parses workspace metadata and re-acquires `target/.cargo-lock` `N` times. Combine independent package checks or unit/integration test targets into a single batched Cargo command:

```sh
VOX_BROKER_RUN_TIMEOUT_SECS=300 cargo test -p vox-build-queue -p vox-cli-ci --lib --tests
```

For PowerShell sessions on Windows, set the post-queue timeout before running the batched command:

```powershell
$env:VOX_BROKER_RUN_TIMEOUT_SECS = "300"
cargo test -p vox-build-queue -p vox-cli-ci --lib --tests
```

## Install (per machine, git-proof)

The installer defaults to dry-run mode and prints its plan without modifying disk until `--apply` is passed:

```sh
vox run scripts/broker-install.vox              # dry run: prints the plan, touches nothing
vox run scripts/broker-install.vox -- --apply   # builds, installs cargo + vox-broker, updates shell profile
```

Passing `--apply` builds `crates/vox-cargo-shim` in release mode, copies every produced binary (`cargo` and `vox-broker`, with `.exe` on Windows) plus a self-install copy of `broker-install.vox` into `${VOX_BROKER_HOME:-$HOME/.vox/build-broker}/bin/`, and idempotently prepends that `bin/` directory ahead of `~/.cargo/bin` in your login-shell profile using `# >>> vox build broker >>>` / `# <<< vox build broker <<<` markers via atomic temp-file replacement.

Useful installer flags:

- `--profile <path>` — write the `PATH` marker block to an explicit file (useful for isolated testing).
- `--no-profile` — install binaries into `$VOX_BROKER_HOME/bin` without editing any shell profile.
- `--help` — print options and broker environment tunables.

Manual fallback (using `--manifest-path` because `crates/vox-cargo-shim` is intentionally excluded from the root `[workspace]`):

```sh
cargo build --release --manifest-path crates/vox-cargo-shim/Cargo.toml
mkdir -p ~/.vox/build-broker/bin
cp target/release/cargo target/release/vox-broker ~/.vox/build-broker/bin/
```

## Activate (per environment)

Prepend the shim directory to `PATH` ahead of `~/.cargo/bin` so bare `cargo` invocations route through the broker:

- **POSIX shells (macOS / Linux):** `vox run scripts/broker-install.vox -- --apply` updates `.zshrc`, `.bash_profile`/`.bashrc`, `.config/fish/config.fish`, or `.profile` for future login shells.
- **Persistent user scope (Windows):** prepend `%USERPROFILE%\.vox\build-broker\bin` ahead of `%USERPROFILE%\.cargo\bin` in the user `PATH` environment variable.
- **VS Code / Cursor / Antigravity / Windsurf:** add `~/.vox/build-broker/bin` to `terminal.integrated.env.osx.PATH`, `terminal.integrated.env.linux.PATH`, or `terminal.integrated.env.windows.PATH` and open a new integrated terminal (existing processes keep their launch-time `PATH`).

## Observe (`vox-broker` telemetry and log tokens)

Verify `PATH` activation in a fresh login shell rather than an already-running shell:

```sh
grep -A1 '>>> vox build broker >>>' ~/.zshrc
zsh -lc 'which -a cargo'
```

Use the strictly read-only `vox-broker` companion binary (installed alongside the shim; never creates directories or mutates state, and exits `0` cleanly on fresh machines) to inspect queue splits, coalescing hits, and timeouts:

```sh
vox-broker stats        # summary over metrics.jsonl with split waits, coalesce rate, and timeout counts
vox-broker log -n 20    # last 20 lines of broker.log
vox-broker status       # broker home, effective cap, reservations, and sampled busy slots
```

Output from `vox-broker stats` (`Summary::render()`) surfaces exact split-wait and coalescing tokens (`builds=`, `p50_wait=`, `p95_wait=`, `target_p95=`, `slot_p95=`, `compile_p50=`, `coalesce_rate=`, `coalesced=`, `timeouts=`):

```text
builds=42 p50_wait=120ms p95_wait=3400ms (target_p95=3100ms slot_p95=300ms compile_p50=6200ms) coalesce_rate=28.6% coalesced=9 timeouts=0 -> BUILD-DAEMON: recommended (coalesce>=10% and queue waits present)
```

Each `broker.log` entry (`vox-broker log`) records per-invocation split timings and outcomes (`wait=`, `target_wait=`, `slot_wait=`, `compile=`, `ahead=`, `cap=`, `coalesce=`, `coalesced=`, `timeout=`, `exit=`):

```text
1794724550233 test   wait=  4200ms target_wait=3900ms slot_wait=300ms compile=18000ms ahead=3 cap=6 coalesce=true coalesced=false timeout=false exit=0 /path/to/worktree
```

High `target_wait` with low `slot_wait` indicates same-worktree `target/.cargo-lock` serialization (mitigate by batching `cargo test -p` packages or enabling `VOX_ISOLATED_TARGET=1` with `VOX_WORKSTREAM`); high `slot_wait` indicates machine-wide CPU saturation across concurrent worktrees.
