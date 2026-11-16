# Phase 447: Repo-Wide Build Contention Elimination & Broker Coalescing - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

<domain>
## Phase Boundary

Eliminate the 20–107 minute multi-agent `target/.cargo-lock` contention, sleeping-waiter broker slot starvation, 180k-LoC feature-cascade recompiles, feature-set ping-pong thrashing, and premature `timeout` `exit 124` build kills across the Vox repository (`BUILD-CONTENTION-01`, `BUILD-CONTENTION-02`, `BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`).

</domain>

<decisions>
## Implementation Decisions

### Target-First FairQueue Ordering, True In-Flight Coalescing & Honest Telemetry (`BUILD-CONTENTION-01`)
- Wire `vox_build_queue::queue::FairQueue` (keyed by canonicalized effective `CARGO_TARGET_DIR`, honoring `--target-dir`, `CARGO_TARGET_DIR`, and `.cargo/config.toml`) into `crates/vox-cargo-shim/src/main.rs` **before** calling `global::acquire_slot_adaptive`. Waiters on the same `target/` directory must queue on the per-target lock without consuming one of the machine-wide `cap` slots (fixing the bug where 6 sleeping `Blocking waiting for file lock on build directory` processes drop `load_average_1m()` near `0`, grab all 6 global slots at `wait=0ms`, and hide 30–107 minutes of lock wait inside `ran_ms`).
- Upgrade `register_inflight` / `FairQueue` from passive `would_coalesce` logging (`5.2%+` global, `~50%` during parallel agent waves) to **real leader-follower coalescing** for idempotent verification commands (`cargo check`, `cargo build`, and identical `cargo test` invocations when source mtimes have not advanced since leader start): followers attach to the active leader's output log + exit code and return in `0 ms` post-leader instead of re-running Cargo.
- Split `MetricRecord` (`metrics.jsonl`, `broker.log`, and `vox-broker stats`) into explicit `target_wait_ms`, `slot_wait_ms`, and `compile_ms` (backward-compatible `#[serde(default)]` on existing `metrics.jsonl` rows) so `vox-broker stats` never misreports `p95_wait=0ms` when builds spend 30 minutes queued on `target/.cargo-lock`.

### Lean `vox-orchestrator` `--no-default-features` & `vox-terminal-core` Dependency Decoupling (`BUILD-CONTENTION-02`)
- Gate unguarded `vox_code_audit` / `crate::validation` references in `crates/vox-orchestrator/src/victory.rs` and `crates/vox-orchestrator/src/orch_daemon/dei_dispatch.rs` behind `#[cfg(feature = "toestub-gate")]` so `cargo check/test -p vox-orchestrator --no-default-features` and `--no-default-features --features runtime` compile and pass cleanly.
- Activate the planned `vox-lang` feature in `crates/vox-terminal-core/Cargo.toml` (`vox-compiler` optional behind `vox-lang`, enabled only by crates needing inline `.vox` compilation) and switch `vox-terminal-core`'s `vox-orchestrator` dependency to `default-features = false` (aligning its feature fingerprint with `vox-orchestrator-mcp` so concurrent MENS edits in `vox-corpus` / `vox-populi` / `jj-lib` no longer trigger 180k-LoC cascade recompiles or `target/debug/deps` feature ping-pong).

### Workstream Isolation, Zero-Config `sccache` Auto-Discovery & Command Batching Guard (`BUILD-CONTENTION-03`)
- In `vox-cargo-shim`, automatically set `RUSTC_WRAPPER=sccache` for child Cargo invocations whenever `RUSTC_WRAPPER` is unset AND `sccache` is present and executable on `PATH` (preserving `.cargo/config.toml`'s CI safety when `sccache` is absent while sharing object caches across worktrees/targets when installed).
- Support optional per-workstream target isolation (`VOX_WORKSTREAM` / `VOX_ISOLATED_TARGET=1` routing `CARGO_TARGET_DIR` to `target/ws-<slug>` when `use_worktrees: false` forces multiple GSD workstreams into one checkout) plus a `vox ci dev-loop-guard` / `.agents/rules/00-core.md` update encouraging batched multi-package `cargo test -p a -p b --lib --tests` verification per wave instead of N sequential single-test lock acquisitions.

### Queue-Aware Build Timeouts Eliminating `98%` `exit 124` Kills (`BUILD-CONTENTION-04`)
- Add `VOX_BROKER_RUN_TIMEOUT_SECS` (and `--broker-run-timeout` / env support in `vox-cargo-shim`) that starts the timeout clock **only after** both the per-target `FairQueue` lock and global slot are acquired — preventing outer `timeout 300s`/`900s` wrappers from killing `rustc`/`ld` at `Building 964/965` simply because a build spent 14 minutes waiting in line, and document the canonical pattern in `.agents/rules/00-core.md` and `docs/src/contributors/build-broker-usage.md`.

### the agent's Discretion
- Exact file-lock protocol for leader-follower output replay (`<broker_home>/inflight/<key>.out` + `.exit`) and exact `MetricRecord` field names so long as Windows + macOS + Linux file-locking rules (`fs2` single-handle rule on Windows `os error 33`) and `< 500` LoC `god_object` limits are strictly respected.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/vox-build-queue/src/{queue.rs,global.rs,metrics.rs,env_filter.rs,resolve.rs}`: `FairQueue` (`take_ticket`, `position`, `coalesce_opportunity`, `acquire`) already exists and is unit-tested, ready to wire into `vox-cargo-shim`.
- `crates/vox-cargo-shim/src/{main.rs,bin/vox-broker.rs}`: Daemonless `cargo` shim and `vox-broker` read-only CLI (`stats`, `log`, `status`).
- `.cargo/config.toml`: Existing `SCCACHE_DIR = { value = ".sccache", relative = true }` and `CARGO_TARGET_DIR = { value = "target", relative = true }` configuration.
- `crates/vox-orchestrator/Cargo.toml` & `crates/vox-terminal-core/Cargo.toml`: Existing feature flags (`toestub-gate`, `runtime`, `corpus-db`, `jj`, and placeholder `vox-lang = []`).

### Established Patterns
- `vox-cargo-shim` and `vox-build-queue` are ultra-lightweight foundation/tooling crates (minimal deps, no `clap` in shim, graceful fallback to real `cargo` on any broker error).
- `vox-broker` binary (`src/bin/vox-broker.rs`) is strictly read-only (`vox_broker_readonly.rs` integration test enforces that `status`/`stats`/`log` never create or mutate files).
- Authored docs under `docs/src/contributors/build-broker-usage.md` require valid YAML frontmatter if created/substantially authored (`vox-doc-pipeline --lint-only`).

</code_context>

<specifics>
## Specific Ideas

- Live telemetry captured on 2026-11-14 from `vox-broker log -n 25` proved the exact failure mode: `wait=0ms ran=6443413ms ahead=2 cap=6` (`107 minutes` blocked inside Cargo's `target/.cargo-lock` while `vox-broker stats` reported `p50_wait=0ms p95_wait=0ms` over `10,021` builds). Fixing lock acquisition order (`FairQueue` per `target_dir` -> `global::acquire_slot_adaptive`) immediately frees global slots and makes queue telemetry truthful.

</specifics>

<deferred>
## Deferred Ideas

- Stateful persistent background build daemon (`sccache-dist` or remote Bazel/Buck2 migration) — explicitly out of scope; fixing `vox-cargo-shim` + `vox-build-queue` + feature decoupling solves contention daemonlessly within Cargo.

</deferred>
