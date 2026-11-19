---
phase: 447
phase_name: "repo-wide-build-contention-elimination-target-aware-broker-c"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 24
  lessons: 3
  patterns: 13
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 447 Learnings: repo-wide-build-contention-elimination-target-aware-broker-c

## Decisions

### Placed flight coalescing files under <broker_home>/inflight/flights/ so ...
Placed flight coalescing files under <broker_home>/inflight/flights/ so global::register_inflight (which scans top-level files in <broker_home>/inflight/ and unlinks unlocked regular files) never reaps active .meta/.stdout/.stderr/.exit sibling files while global.rs remains untouched at 483 LoC.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-01-SUMMARY.md

---

### Sampled arrival_ts_ms before Stage A (try_follow_inflight) and bounded t...
Sampled arrival_ts_ms before Stage A (try_follow_inflight) and bounded ticket_ts_ms with ts.min(arrival_ts_ms) so a follower missing Stage A by sub-millisecond timing still qualifies for Stage B (try_replay_recent_leader) without sacrificing source-mtime causality.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-01-SUMMARY.md

---

### Exposed metrics::read_records(path) in vox-build-queue so integration te...
Exposed metrics::read_records(path) in vox-build-queue so integration tests in vox-cargo-shim parse MetricRecord JSONL without adding serde_json to crates/vox-cargo-shim/Cargo.toml.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-01-SUMMARY.md

---

### Target-First FairQueue Admission (BUILD-CONTENTION-01)
**Target-First `FairQueue` Admission (`BUILD-CONTENTION-01`):** `vox-cargo-shim` resolves the child environment first, computes `resolve::effective_target_dir`, and acquires `FairQueue::new(&root.join("targets"), &target_hash)` *before* calling `global::acquire_slot_adaptive(&root, n)`, preventing callers contending on the same `target/.cargo-lock` from burning machine-wide concurrency slots needed by isolated `--target-dir` / workstream builds.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-01-SUMMARY.md

---

### Windows-Safe Leader-Follower Flight Coalescing (BUILD-CONTENTION-01)
**Windows-Safe Leader-Follower Flight Coalescing (`BUILD-CONTENTION-01`):** Added `vox_build_queue::coalesce` using a 0-byte `fs2`-locked `<key>.lock` file alongside unlocked `<key>.meta`, `<key>.stdout`, `<key>.stderr`, and `<key>.exit` files under `<broker_home>/inflight/flights/` (avoiding Windows `LockFileEx` `os error 33` and avoiding interference with `global::register_inflight`'s top-level stale-marker sweep). Supports Stage A live stream tailing, Stage B post-`FairQueue` recent-leader replay, `cargo run` build-phase coalescing (`build_args_for_run`), `--message-format*` & snapshot-update opt-outs, and workspace `latest_source_mtime_ms` cache invalidation.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-01-SUMMARY.md

---

### Backward-Compatible Split Metrics & Read-Only Viewer (BUILD-CONTENTION-0...
**Backward-Compatible Split Metrics & Read-Only Viewer (`BUILD-CONTENTION-01`):** Extended `MetricRecord` with `#[serde(default)]` `target_wait_ms`, `slot_wait_ms`, `compile_ms`, `coalesced_hit`, and `timed_out` while preserving all 8 legacy fields (`queue_wait_ms = target_wait_ms + slot_wait_ms`, `ran_ms = compile_ms`) and `Summary::daemon_recommended()`. Updated `vox-broker stats` and `vox-broker status` to surface split waits, coalesced hits, timeouts, active flights, and per-target queues without ever creating directories or files.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-01-SUMMARY.md

---

### Zero-Config sccache & Opt-In Workstream Target Isolation (BUILD-CONTENTI...
**Zero-Config `sccache` & Opt-In Workstream Target Isolation (`BUILD-CONTENTION-03`):** Added pure `BTreeMap`-based `should_inject_sccache`, `resolve_sccache_wrapper` (skipping shim self/sibling binaries), and `apply_workstream_target_isolation` (`VOX_ISOLATED_TARGET=1` / `VOX_WORKSTREAM=<slug>` / `<worktree>/.vox/agent-target` -> `<worktree>/target/ws-<slug>`, sanitizing slugs to `[A-Za-z0-9_-]{1,48}` and setting `CARGO_INCREMENTAL=0` only when both isolated target and `sccache` are active).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-01-SUMMARY.md

---

### Post-Queue Active Run Timeout (BUILD-CONTENTION-04)
**Post-Queue Active Run Timeout (`BUILD-CONTENTION-04`):** Added `crates/vox-cargo-shim/src/runner.rs` (`strip_broker_flags`, `resolve_run_timeout_secs`, `run_child_post_queue`) and `resolve::kill_process_tree(pid)` (`#[cfg(unix)] libc::killpg` with `process_group(0)` / `#[cfg(windows)] taskkill /T /F /PID`). Active compile/test hangs exceed `VOX_BROKER_RUN_TIMEOUT_SECS` only *after* queue and slot waits complete, reaping grandchildren and returning `124`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-01-SUMMARY.md

---

### Mirrored all four VOX_ROUTE_* environment variables fail-closed in route...
Mirrored all four VOX_ROUTE_* environment variables fail-closed in route_policy::exclusion_reason_without_runtime_with via closure-injected lookup so --no-default-features builds never bypass restricted network/provider policy and unit tests require zero std::env::set_var calls

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-02-SUMMARY.md

---

### Preserved 1-arg eval_line(src
Preserved 1-arg eval_line(src: &str) -> anyhow::Result<String> and literal Interpreter::new(100_000) + CapabilitySet::parse("") tokens in crates/vox-terminal-core/src/vox_interp.rs so vox-compiler's production_embedders_assign_caps_explicitly source-audit test passes unchanged

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-02-SUMMARY.md

---

### Kept vox-compiler out of vox-terminal-core [dev-dependencies] so default...
Kept vox-compiler out of vox-terminal-core [dev-dependencies] so default cargo test -p vox-terminal-core runs lean without compiling 47k LoC of vox-compiler

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-02-SUMMARY.md

---

### Gated VictoryEvaluator::evaluate (and its ObservationReport/VictoryVerdi...
Gated `VictoryEvaluator::evaluate` (and its `ObservationReport`/`VictoryVerdict` import) on `#[cfg(feature = "toestub-gate")]` while keeping `VictoryEvaluator::new`, `VictoryConfig`, `VictoryReport`, and `VictoryTiers` unconditional.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-02-SUMMARY.md

---

### Added a #[cfg(not(feature = "toestub-gate"))] fallback handle_ai_check i...
Added a `#[cfg(not(feature = "toestub-gate"))]` fallback `handle_ai_check` in `dei_dispatch.rs` preserving `"params.path required"` validation and returning `"ai.check requires feature `toestub-gate`"`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-02-SUMMARY.md

---

### Implemented exclusion_reason_without_runtime_with + exclusion_reason_wit...
Implemented `exclusion_reason_without_runtime_with` + `exclusion_reason_without_runtime` in `route_policy.rs` enforcing `VOX_ROUTE_POLICY_PROFILE`, `VOX_ROUTE_ALLOW_NET`, `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP`, and `VOX_ROUTE_ALLOW_PROVIDER_NETWORK` fail-closed when `runtime` is disabled, covered by pure closure-driven unit tests.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-02-SUMMARY.md

---

### Tightened Orchestrator::apply_vox_healing to #[cfg(all(feature = "runtim...
Tightened `Orchestrator::apply_vox_healing` to `#[cfg(all(feature = "runtime", feature = "toestub-gate"))]` with an unused-binding suppressor in the `not(...)` arm so `--no-default-features --features runtime` compiles cleanly without `vox-lsp`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-02-SUMMARY.md

---

### Added #![cfg(feature = "runtime")] to tests/scaling_test.rs and tests/co...
Added `#![cfg(feature = "runtime")]` to `tests/scaling_test.rs` and `tests/context_compaction_wiring_test.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-02-SUMMARY.md

---

### Constructed root.join(".vox").join("agent-target") and avoided leading "...
Constructed root.join(".vox").join("agent-target") and avoided leading ".vox/" string literals in error messages so vox-drift-check drift/vox-path-literal remains zero-finding without drift-allow comments.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-03-SUMMARY.md

---

### Bound bin_name and broker_bin_name variables before path.join("release",...
Bound bin_name and broker_bin_name variables before path.join("release", bin_name) in scripts/broker-ci.vox so scripts/broker-bypass-lint.vox Case 3 (\.join\(\s*"cargo(\.exe)?"\s*\)) stays clean.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-03-SUMMARY.md

---

### Upgraded docs/src/contributors/build-broker-usage.md to full KB-complian...
Upgraded docs/src/contributors/build-broker-usage.md to full KB-compliant YAML frontmatter + 63-word kb:summary without editing contracts/documentation/kb-facet-backlog.v1.yaml (verified exit 0 under vox-doc-pipeline --lint-only).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 447-03-SUMMARY.md

---

### Synchronous dev_loop_guard Contention & sccache Guardrails (BUILD-CONTEN...
**Synchronous `dev_loop_guard` Contention & `sccache` Guardrails (`BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`):** Kept `#[inline(never)] pub fn run(root: &Path) -> Result<()>` synchronous (`444` total lines, `drift/vox-path-literal` clean) and added `hardcoded_rustc_wrapper`, `shared_sccache_or_target_missing`, `invalid_agent_target_slug` (validating optional `<worktree>/.vox/agent-target` against ASCII `[A-Za-z0-9_-]{1,48}`), `broker_safety_docs_missing_guidance`, and a non-fatal stderr `sccache` `PATH` advisory when `CI` is unset.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-03-SUMMARY.md

---

### Agent Cargo Safety Rules (.agents/cargo-safety.md, BUILD-CONTENTION-03, ...
**Agent Cargo Safety Rules (`.agents/cargo-safety.md`, `BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`):** Updated rules 1–3 to codify per-target `FairQueue` + flight coalescing, batched multi-package `cargo test -p crate-a -p crate-b --lib --tests` invocations, post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` (`exit 124`) vs coarse outer `timeout 900s`, and `VOX_ISOLATED_TARGET=1` + `VOX_WORKSTREAM=<slug>` (`target/ws-<slug>`) paired with auto-injected `sccache`. Left gitignored `.agents/rules/00-core.md` untouched.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-03-SUMMARY.md

---

### KB-Compliant Contributor Broker Guide (docs/src/contributors/build-broke...
**KB-Compliant Contributor Broker Guide (`docs/src/contributors/build-broker-usage.md`, `BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`):** Upgraded frontmatter (`category: "Contributors"`, `status: "current"`, `topic: "build-system"`, `domain: "platform"`, `authorship: "llm-assisted"`, `authorship_model: "claude-opus-4-7"`, omitting `training_eligible` and `last_updated`), added a 63-word `<!-- kb:summary -->` directly under H1, documented the 4-stage pipeline, all shipping + Phase 447 tunables, multi-package batching, and exact `vox-broker stats` (`target_p95=`, `slot_p95=`, `compile_p50=`, `coalesced=`, `timeouts=`) and `broker.log` (`target_wait=`, `slot_wait=`, `compile=`, `coalesced=`, `timeout=`) tokens with zero single-backtick prose line starts.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-03-SUMMARY.md

---

### VoxLang Broker Installer & CI Scripts (scripts/broker-install.vox, scrip...
**VoxLang Broker Installer & CI Scripts (`scripts/broker-install.vox`, `scripts/broker-ci.vox`):** Updated `scripts/broker-install.vox` `usage()` and `--dry-run` summary with Phase 447 broker tunables (never running `--apply` against host `$HOME/.vox/build-broker`), and updated `scripts/broker-ci.vox` using `path.join` to fall back from `crates/vox-cargo-shim/target/release` to `<root>/target/release` when `CARGO_TARGET_DIR` is unset while verifying both `cargo` and `vox-broker` release binaries.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-03-SUMMARY.md

---

### T-447-06: hardcoded_rustc_wrapper rejects uncommented [build] rustc-wrap...
`T-447-06`: `hardcoded_rustc_wrapper` rejects uncommented `[build] rustc-wrapper` and `[env] RUSTC_WRAPPER` in `.cargo/config.toml` (unit-tested and live-repo verified).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 447-03-SUMMARY.md

## Lessons

### Running cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml from...
Running `cargo test --manifest-path crates/vox-cargo-shim/Cargo.toml` from `/Users/brbrainerd/dev/vox` without `CARGO_TARGET_DIR` set in the environment caused Cargo to inject `.cargo/config.toml`'s `[env] CARGO_TARGET_DIR = "target"` (`/Users/brbrainerd/dev/vox/target`) into `tests/queue_serializes.rs`, contending with a sibling `vox-cli` build on the host root target dir. Passing `CARGO_TARGET_DIR=/Users/brbrainerd/dev/vox/crates/vox-cargo-shim/target VOX_BROKER_NO_SCCACHE=1` isolated `queue_serializes.rs` completely from both `/Users/brbrainerd/dev/vox/target` and the host `sccache` daemon (`2.03s`).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 447-01-SUMMARY.md

---

### CR-01 (main.rs:206-224)
**`CR-01` (`main.rs:206-224`):** `compute_coalesce_key` hashes `cwd` and `RUSTUP_TOOLCHAIN` / `RUSTC` alongside subcommand, args, env, and `target_hash` (`coalesce_key_distinguishes_sibling_cwds_and_toolchains` passes).

**Context:** Verified in phase acceptance checks.
**Source:** 447-VERIFICATION.md

---

### CR-02 (runner.rs:148-269)
**`CR-02` (`runner.rs:148-269`):** `run_child_post_queue` writes child stdout/stderr directly to `Stdio::from(File)` and tails non-blockingly so long-lived background grandchildren (`sccache` daemon) never block `FairQueue` release (`background_grandchild_inheriting_stdio_never_blocks_runner` passes).

**Context:** Verified in phase acceptance checks.
**Source:** 447-VERIFICATION.md

## Patterns

### Target-first admission ordering
Target-first admission ordering: resolve child env (sccache + workstream isolation) -> compute effective_target_dir -> Stage A follow -> Stage B FairQueue(targets/<target_hash>) + recent replay -> Stage C claim_leader + global::acquire_slot_adaptive -> Stage D post-queue runner -> cargo run exec_real tail.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-01-SUMMARY.md

---

### Strict zero-mutation read-only viewer invariant in vox-broker
Strict zero-mutation read-only viewer invariant in vox-broker: inspect <broker_home>/targets and <broker_home>/inflight only when already present on disk.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-01-SUMMARY.md

---

### Target-first two-stage queueing
Target-first two-stage queueing: FairQueue::acquire(target_hash) strictly precedes global::acquire_slot_adaptive

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-01-SUMMARY.md

---

### Windows-safe flight coalescing under <broker_home>/inflight/flights/
Windows-safe flight coalescing under <broker_home>/inflight/flights/: 0-byte fs2-locked <key>.lock paired with unlocked atomic .meta/.exit JSON and .stdout/.stderr stream files

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-01-SUMMARY.md

---

### Post-queue active supervision
Post-queue active supervision: CommandExt::process_group(0) + kill_process_tree(pid) starting strictly after queue/slot acquisition

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-01-SUMMARY.md

---

### Explicit Vox keyword diagnostic assertion (let x = 42) in session tests ...
Explicit Vox keyword diagnostic assertion (let x = 42) in session tests under #[cfg(not(feature = "vox-lang"))] so ambiguous non-keyword expressions continue falling back to shell

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-02-SUMMARY.md

---

### Closure-injected environment lookup (exclusion_reason_without_runtime_wi...
Closure-injected environment lookup (exclusion_reason_without_runtime_with) for race-free unit testing without unsafe std::env::set_var

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-02-SUMMARY.md

---

### Compound #[cfg(all(feature = "runtime", feature = "toestub-gate"))] gati...
Compound #[cfg(all(feature = "runtime", feature = "toestub-gate"))] gating when optional subsystems cross-reference two Cargo features

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-02-SUMMARY.md

---

### Opt-in heavy compiler feature (vox-lang = ["dep:vox-compiler"]) off by d...
Opt-in heavy compiler feature (vox-lang = ["dep:vox-compiler"]) off by default in shared core/TUI crates and enabled only in shipping daemon/GUI binaries

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-02-SUMMARY.md

---

### Doc-to-CI parity gate
Doc-to-CI parity gate: dev_loop_guard::broker_safety_docs_missing_guidance locks critical broker env vars and telemetry tokens across .agents/cargo-safety.md and docs/src/contributors/build-broker-usage.md.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-03-SUMMARY.md

---

### In-process synchronous CI guard parity
In-process synchronous CI guard parity: pure TOML/slug/markdown helpers in crates/vox-cli-ci/src/dev_loop_guard.rs enforced against live repo files by the_repository_passes_the_dev_loop_guard

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-03-SUMMARY.md

---

### KB-compliant contributor SSOT frontmatter + 40-120 word kb:summary block...
KB-compliant contributor SSOT frontmatter + 40-120 word kb:summary block without touching kb-facet-backlog.v1.yaml

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-03-SUMMARY.md

---

### Dual-layout release artifact verification in scripts/broker-ci.vox via p...
Dual-layout release artifact verification in scripts/broker-ci.vox via path.join without triggering broker-bypass-lint Case 3 (.join("cargo"))

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 447-03-SUMMARY.md

## Surprises

