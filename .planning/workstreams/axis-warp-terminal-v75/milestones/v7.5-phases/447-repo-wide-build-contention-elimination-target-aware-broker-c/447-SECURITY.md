---
phase: "447"
slug: "repo-wide-build-contention-elimination-target-aware-broker-c"
status: verified
threats_open: 0
asvs_level: 1
created: "2026-11-15"
---

# Phase 447 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Shell / agent environment → `vox-cargo-shim` & `vox-build-queue::resolve` | Untrusted env vars (`VOX_WORKSTREAM`, `VOX_ISOLATED_TARGET`, `VOX_BROKER_RUN_TIMEOUT_SECS`, `RUSTC_WRAPPER`) and `.vox/agent-target` file contents read before spawning `cargo` | Workstream slug, isolated `CARGO_TARGET_DIR` path, `sccache` executable path, post-queue timeout seconds |
| Broker flight cache (`<broker_home>/inflight/flights/`) → Concurrent `vox-cargo-shim` invocations | Leader writes `.meta` / `<run_id>.stdout` / `<run_id>.stderr` / `<run_id>.exit` artifacts replayed by follower processes on identical build keys | Coalesce SHA-256 key, `FlightMeta` JSON (`pid`, `run_id`, `started_ms`, `src_mtime_ns`), captured compiler stdout/stderr bytes, exit status |
| `vox-cargo-shim::runner` → Child `cargo` / `rustc` / test process tree | Active post-queue process execution and timeout enforcement across nested compiler/test grandchildren | OS process group (`pgid`) / Windows process tree (`PID`), file-backed stdout/stderr descriptors, `SIGKILL` / `taskkill /T /F` |
| Minimal-feature library boundary → `vox-orchestrator::route_policy` & `vox-terminal-core::vox_interp` | Security-critical routing policy env vars (`VOX_ROUTE_*`) and embedded Vox interpreter capability sandboxes compiled with `--no-default-features` | Network/egress allowlists (`airgapped`, `VOX_ROUTE_ALLOW_*`), `Interpreter` fuel limit (`100_000`) and empty `CapabilitySet::parse("")` |
| Repo config & CI guard → `vox-cli-ci::dev_loop_guard` & `scripts/broker-*.vox` | `.cargo/config.toml`, `.vox/agent-target`, `.agents/cargo-safety.md`, and contributor docs audited in CI before merge | `[build]` / `[env]` wrapper settings, slug character whitelist, broker safety guidance markers |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-447-01 | Tampering | `crates/vox-build-queue/src/resolve.rs` (`sanitize_workstream_slug`, `apply_workstream_target_isolation`) | medium | mitigate | `sanitize_workstream_slug` (`resolve.rs:195-210`) enforces non-empty `[A-Za-z0-9_-]{1,48}`, rejecting `..`, `/`, `\`, whitespace, NUL, and >48-char inputs before constructing `<wt>/target/ws-<slug>` (`wt.join(".vox").join("agent-target")` keeps `.vox/` path assembly dynamic). Verified in `workstream_target_isolation_Precedence_and_slug_sanitization`. | closed |
| T-447-02 | Information Disclosure / Tampering | `crates/vox-build-queue/src/coalesce.rs` & `crates/vox-cargo-shim/src/main.rs` (`inflight/flights/`) | medium | mitigate | Flights live in `<broker_home>/inflight/flights/` (`0600` owner-only permissions on Unix at `coalesce.rs:76-82`); `compute_coalesce_key` (`main.rs:206-224`) hashes `(effective_sub, argv_hash, env_hash, target_hash, cwd_hash, tc_tag)` via SHA-256 (`CR-01`); monotonic `run_id` isolates per-run `.stdout`/`.stderr`/`.exit` files (`coalesce.rs:48-65`); `main.rs:311-315` re-samples `src_mtime_post_wait` after `FairQueue::acquire` so edits during queue wait invalidate stale leaders (`WR-02`). Verified in `coalesce_leader_follower_replay_and_mtime_invalidation` and `concurrent_followers_coalesce_single_cargo_invocation_and_invalidation_reruns`. | closed |
| T-447-03 | Denial of Service | `crates/vox-cargo-shim/src/runner.rs`, `crates/vox-build-queue/src/resolve.rs`, `crates/vox-build-queue/src/coalesce.rs` | high | mitigate | `run_child_post_queue` (`runner.rs:148-269`) starts the timeout clock strictly after `FairQueue` + global slot acquisition, uses `Stdio::from(File)` + non-blocking `tail_stream` (`CR-02`, preventing orphaned grandchild pipe hangs), spawns in `process_group(0)` (`CREATE_NO_WINDOW` on Windows), and invokes `kill_process_tree` (`libc::killpg(..., SIGKILL)` / `taskkill /T /F /PID`, `resolve.rs:313-332`) returning exit `124`. Coalescing locks only 0-byte `<key>.lock` (`coalesce.rs:305-322`), probes via non-exclusive `try_lock_shared()` (`WR-01`), and reaps unlocked flight files >1h old (`reap_stale_flights`, `IN-02`). Verified in `post_queue_timeout_excludes_queue_wait_and_kills_hung_child_with_124` and `grandchild_holding_inherited_stdio_does_not_hang_runner`. | closed |
| T-447-04 | Elevation of Privilege / Tampering | `crates/vox-orchestrator/src/route_policy.rs` (`#[cfg(not(feature = "runtime"))]`) | medium | mitigate | `exclusion_reason_without_runtime_with` (`route_policy.rs:24-73`) enforces all four `VOX_ROUTE_*` env guards (`VOX_ROUTE_POLICY_PROFILE=airgapped\|strict-local`, `VOX_ROUTE_ALLOW_NET=false`, `VOX_ROUTE_ALLOW_LOCAL_MODEL_HTTP=false`, `VOX_ROUTE_ALLOW_PROVIDER_NETWORK=false`) fail-closed when built without `vox-actor-runtime`, never silently permitting cloud or loopback egress. Verified in `airgapped_profile_blocks_remote_and_ollama_without_runtime` and `individual_allow_flags_false_block_respective_backends`. | closed |
| T-447-05 | Elevation of Privilege | `crates/vox-terminal-core/src/vox_interp.rs` (`eval_line`) | high | mitigate | `#[cfg(feature = "vox-lang")]` arm (`vox_interp.rs:13-37`) preserves exact `let mut interp = vox_compiler::eval::Interpreter::new(100_000);` fuel cap and `interp.caps = vox_compiler::eval::caps::CapabilitySet::parse("").unwrap();` zero-capability sandbox; `#[cfg(not(feature = "vox-lang"))]` (`vox_interp.rs:40-47`) fails closed without executing user source. Verified by `vox-compiler` `eval::caps::tests::production_embedders_assign_caps_explicitly` (`1 passed`) and `vox-terminal-core` `vox_interp` / `session` tests. | closed |
| T-447-06 | Denial of Service | `.cargo/config.toml` & `crates/vox-cli-ci/src/dev_loop_guard.rs` | medium | mitigate | `hardcoded_rustc_wrapper` and `shared_sccache_or_target_missing` (`dev_loop_guard.rs:220-264`) fail CI if `rustc-wrapper = "sccache"` or `RUSTC_WRAPPER = "sccache"` is hardcoded in `.cargo/config.toml` (breaking machines without `sccache`) or if shared `SCCACHE_DIR` / `CARGO_TARGET_DIR` keys are removed. Verified in `flags_hardcoded_rustc_wrapper_and_missing_sccache_dir` and `the_repository_passes_the_dev_loop_guard`. | closed |
| T-447-07 | Tampering | `crates/vox-cli-ci/src/dev_loop_guard.rs` (`invalid_agent_target_slug`) | low | mitigate | `invalid_agent_target_slug` (`dev_loop_guard.rs:266-285`) inspects `root.join(".vox").join("agent-target")` when present and rejects slugs outside `[A-Za-z0-9_-]{1,48}` (including traversal payloads like `../pwn`). Verified in `validates_agent_target_slug_and_broker_docs_parity`. | closed |
| T-447-SC | Supply Chain | `crates/vox-build-queue/Cargo.toml`, `crates/vox-cargo-shim/Cargo.toml`, `Cargo.lock` | low | accept | Zero new third-party crates or crate-edge exceptions introduced in Phase 447; all locking, hashing, serialization, and process-group primitives reuse existing workspace dependencies (`fs2`, `sha2`, `serde`, `serde_json`, `libc`, `tempfile`). Verified via `vox-arch-check` (`exit 0`). | closed |

---

## Accepted Risks Log

No accepted risks requiring remediation — `T-447-SC` confirms zero new dependencies were added.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-15 | 8 | 8 | 0 | Antigravity (`gsd-verifier` + `447-REVIEW-FIX`) |

---

## Sign-Off

- [x] All threats (`T-447-01..07`, `T-447-SC`) have a disposition (mitigate / accept / transfer) and are verified in codebase source + tests
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-15
