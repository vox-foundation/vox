---
phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c
plan: 03
subsystem: infra
tags: [rust, ci-guard, dev-loop-guard, build-broker, sccache, vox-scripts, documentation, kb-ssot]

requires:
  - phase: 447-01
    provides: "Target-first FairQueue, leader-follower coalescing, auto-sccache, VOX_WORKSTREAM isolation, VOX_BROKER_RUN_TIMEOUT_SECS, and split vox-broker stats/log tokens"
provides:
  - "Synchronous dev_loop_guard::run checks for hardcoded rustc-wrapper/RUSTC_WRAPPER, shared SCCACHE_DIR + CARGO_TARGET_DIR in .cargo/config.toml, .vox/agent-target slug hygiene, non-fatal sccache PATH advisory, and broker doc parity"
  - "Updated .agents/cargo-safety.md rules covering FairQueue + coalescing, batched multi-package cargo test -p, VOX_BROKER_RUN_TIMEOUT_SECS vs outer timeout, and VOX_ISOLATED_TARGET + VOX_WORKSTREAM"
  - "KB-compliant docs/src/contributors/build-broker-usage.md documenting the 4-stage pipeline, real broker env/CLI tunables, and exact vox-broker stats/log telemetry tokens"
  - "Updated scripts/broker-install.vox (Phase 447 usage + dry-run summary) and scripts/broker-ci.vox (dual target/release + crates/vox-cargo-shim/target/release binary check for cargo and vox-broker via path.join)"
affects:
  - vox-cli-ci
  - vox-cargo-shim
  - contributor-docs
  - agent-cargo-safety

actuals:
  tokens: 14077
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "In-process synchronous CI guard parity: pure TOML/slug/markdown helpers in crates/vox-cli-ci/src/dev_loop_guard.rs enforced against live repo files by the_repository_passes_the_dev_loop_guard"
    - "KB-compliant contributor SSOT frontmatter + 40-120 word kb:summary block without touching kb-facet-backlog.v1.yaml"
    - "Dual-layout release artifact verification in scripts/broker-ci.vox via path.join without triggering broker-bypass-lint Case 3 (.join(\"cargo\"))"

key-files:
  created: []
  modified:
    - crates/vox-cli-ci/src/dev_loop_guard.rs
    - .agents/cargo-safety.md
    - docs/src/contributors/build-broker-usage.md
    - scripts/broker-install.vox
    - scripts/broker-ci.vox

key-decisions:
  - "Constructed root.join(\".vox\").join(\"agent-target\") and avoided leading \".vox/\" string literals in error messages so vox-drift-check drift/vox-path-literal remains zero-finding without drift-allow comments."
  - "Bound bin_name and broker_bin_name variables before path.join(\"release\", bin_name) in scripts/broker-ci.vox so scripts/broker-bypass-lint.vox Case 3 (\\.join\\(\\s*\"cargo(\\.exe)?\"\\s*\\)) stays clean."
  - "Upgraded docs/src/contributors/build-broker-usage.md to full KB-compliant YAML frontmatter + 63-word kb:summary without editing contracts/documentation/kb-facet-backlog.v1.yaml (verified exit 0 under vox-doc-pipeline --lint-only)."

patterns-established:
  - "Doc-to-CI parity gate: dev_loop_guard::broker_safety_docs_missing_guidance locks critical broker env vars and telemetry tokens across .agents/cargo-safety.md and docs/src/contributors/build-broker-usage.md."

requirements-completed:
  - BUILD-CONTENTION-03
  - BUILD-CONTENTION-04

duration: 36min
completed: 2026-11-15
status: complete
---

# Phase 447 Plan 03: Repo-Wide Build Contention CI Guardrails, KB Documentation & Broker `.vox` Scripts Summary

**Synchronous `dev_loop_guard` enforcement for `sccache`/`.vox/agent-target`/timeout-doc parity, KB-compliant `build-broker-usage.md` + `.agents/cargo-safety.md` guidance, and dual-target `scripts/broker-ci.vox` + `scripts/broker-install.vox` updates**

## Performance

- **Duration:** 36 min
- **Started:** 2026-11-15T06:44:00Z
- **Completed:** 2026-11-15T07:20:00Z
- **Tasks:** 2 completed (TDD RED + GREEN verified on `dev_loop_guard.rs`; `vox-doc-pipeline`, `vox check`, `broker-install --dry-run`, and `broker-bypass-lint` verified)
- **Files modified:** 5 (0 created; uncommitted in working tree per orchestrator `NO GIT MUTATIONS` directive)

## Accomplishments

- **Synchronous `dev_loop_guard` Contention & `sccache` Guardrails (`BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`):** Kept `#[inline(never)] pub fn run(root: &Path) -> Result<()>` synchronous (`444` total lines, `drift/vox-path-literal` clean) and added `hardcoded_rustc_wrapper`, `shared_sccache_or_target_missing`, `invalid_agent_target_slug` (validating optional `<worktree>/.vox/agent-target` against ASCII `[A-Za-z0-9_-]{1,48}`), `broker_safety_docs_missing_guidance`, and a non-fatal stderr `sccache` `PATH` advisory when `CI` is unset.
- **Agent Cargo Safety Rules (`.agents/cargo-safety.md`, `BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`):** Updated rules 1–3 to codify per-target `FairQueue` + flight coalescing, batched multi-package `cargo test -p crate-a -p crate-b --lib --tests` invocations, post-queue `VOX_BROKER_RUN_TIMEOUT_SECS` (`exit 124`) vs coarse outer `timeout 900s`, and `VOX_ISOLATED_TARGET=1` + `VOX_WORKSTREAM=<slug>` (`target/ws-<slug>`) paired with auto-injected `sccache`. Left gitignored `.agents/rules/00-core.md` untouched.
- **KB-Compliant Contributor Broker Guide (`docs/src/contributors/build-broker-usage.md`, `BUILD-CONTENTION-03`, `BUILD-CONTENTION-04`):** Upgraded frontmatter (`category: "Contributors"`, `status: "current"`, `topic: "build-system"`, `domain: "platform"`, `authorship: "llm-assisted"`, `authorship_model: "claude-opus-4-7"`, omitting `training_eligible` and `last_updated`), added a 63-word `<!-- kb:summary -->` directly under H1, documented the 4-stage pipeline, all shipping + Phase 447 tunables, multi-package batching, and exact `vox-broker stats` (`target_p95=`, `slot_p95=`, `compile_p50=`, `coalesced=`, `timeouts=`) and `broker.log` (`target_wait=`, `slot_wait=`, `compile=`, `coalesced=`, `timeout=`) tokens with zero single-backtick prose line starts.
- **VoxLang Broker Installer & CI Scripts (`scripts/broker-install.vox`, `scripts/broker-ci.vox`):** Updated `scripts/broker-install.vox` `usage()` and `--dry-run` summary with Phase 447 broker tunables (never running `--apply` against host `$HOME/.vox/build-broker`), and updated `scripts/broker-ci.vox` using `path.join` to fall back from `crates/vox-cargo-shim/target/release` to `<root>/target/release` when `CARGO_TARGET_DIR` is unset while verifying both `cargo` and `vox-broker` release binaries.

## TDD Gate Compliance & Commits

Per parent coordinator `<strict_execution_rules>` (`NO GIT MUTATIONS` on shared working tree), zero git commits were created; all 5 modified files remain cleanly in place for orchestrator verification and atomic commit.

1. **Task 1 (`dev_loop_guard.rs` + `.agents/cargo-safety.md` + `build-broker-usage.md`) TDD Verification:**
   - **RED:** Compiled `crates/vox-cli-ci/src/dev_loop_guard.rs` via `rustc --edition 2024 --test` against prebuilt `target/debug/deps/{libanyhow,libserde,libserde_yaml_ng}` before editing the two markdown files -> `8 passed; 1 failed` (`tests::the_repository_passes_the_dev_loop_guard` failed with `dev-loop-guard: 9 problem(s)` reporting all missing `cargo-safety.md` and `build-broker-usage.md` tokens, `exit 101`).
   - **GREEN:** Updated `.agents/cargo-safety.md` and `docs/src/contributors/build-broker-usage.md`, formatted `dev_loop_guard.rs` with `rustfmt --edition 2024`, recompiled and ran -> `9 passed; 0 failed` in `0.04s` (`exit 0`), `vox-drift-check` `exit 0`, and `./target/debug/vox-doc-pipeline --lint-only --paths contributors/build-broker-usage.md` `exit 0` (`no hard errors`).
2. **Task 2 (`scripts/broker-install.vox` + `scripts/broker-ci.vox`) Verification:**
   - `vox check scripts/broker-install.vox` -> `exit 0` (`Check passed`).
   - `vox check scripts/broker-ci.vox` -> `exit 0` (`Check passed`).
   - `vox run scripts/broker-install.vox -- --dry-run` & `--help` -> `exit 0`.
   - `vox run --interp --max-steps 50000000 scripts/broker-bypass-lint.vox -- -- scripts/broker-install.vox scripts/broker-ci.vox crates/vox-cli-ci/src/dev_loop_guard.rs` -> `exit 0` (`0 unallowlisted bypass(es), 0 allowlisted`).

## Files Modified & Line Counts

| File | Action | Total Lines (`wc -l`) | Role |
|---|---|---:|---|
| `crates/vox-cli-ci/src/dev_loop_guard.rs` | Modified | 444 | Synchronous CI guard for `rustc-wrapper`, shared `SCCACHE_DIR`/`CARGO_TARGET_DIR`, `.vox/agent-target` slug validation, `sccache` advisory, and doc parity |
| `.agents/cargo-safety.md` | Modified | 45 | Tracked agent Cargo safety doctrine (batching, post-queue timeout, workstream isolation, `sccache`) |
| `docs/src/contributors/build-broker-usage.md` | Modified | 129 | KB-compliant contributor SSOT for 4-stage broker pipeline, env/CLI tunables, and `vox-broker` tokens |
| `scripts/broker-install.vox` | Modified | 426 | Broker installer `usage()` and `--dry-run` Phase 447 tunable guidance |
| `scripts/broker-ci.vox` | Modified | 159 | Dual-layout release artifact verification for `cargo` and `vox-broker` via `path.join` |

## Decisions Made

- **Threat Model Mitigations Verified (`T-447-06`, `T-447-07`, `T-447-SC`):**
  - `T-447-06`: `hardcoded_rustc_wrapper` rejects uncommented `[build] rustc-wrapper` and `[env] RUSTC_WRAPPER` in `.cargo/config.toml` (unit-tested and live-repo verified).
  - `T-447-07`: `invalid_agent_target_slug` enforces ASCII `[A-Za-z0-9_-]{1,48}`, rejecting empty strings, `>48` chars, `../escape`, and `a/b` path separators.
  - `T-447-SC`: Zero `Cargo.toml` or `Cargo.lock` modifications; `.vox` scripts use `path.join` and `std.process.run_capture_ex` argv arrays exclusively and pass `broker-bypass-lint.vox`.

## Deviations from Plan

None - plan executed as written across all 5 files.

## Contention Status & Orchestrator Post-Lock Verification Notes

During execution, sibling Wave/Phase Cargo builds (`Phase 442-03` and Wave 1 `vox-orchestrator` updates) contended on shared `/Users/brbrainerd/dev/vox/target/.cargo-lock`:
1. **`cargo test -p vox-cli-ci --lib dev_loop_guard`:** Reached `Building 1058/1068: vox-doc-pipeline, vox-publisher` before outer `timeout` hit `exit 124` (`task-44` / `task-209`). Per `<strict_execution_rules>` #3, we did not retry blindly; `dev_loop_guard.rs` unit tests + live repository `run(root)` were verified RED (`8 passed; 1 failed`) and GREEN (`9 passed; 0 failed` in `0.04s`) via standalone `rustc --edition 2024 --test`, and only the final ~10 downstream crates remain in `./target/` for the orchestrator's post-contention `cargo test -p vox-cli-ci --lib dev_loop_guard` pass.
2. **`vox run scripts/broker-ci.vox`:** Because `std.process.run_capture_ex(cmd, args, root, [])` executes `cargo` from `root` with an empty override slice, Cargo picks up `.cargo/config.toml`'s `CARGO_TARGET_DIR = "target"` and queued behind `target/.cargo-lock` (`exit 124` on `task-180`/`188`/`198`; stopped after 3 attempts per Rule 5). Ready for orchestrator's post-contention `timeout 600s vox run scripts/broker-ci.vox` run once `target/.cargo-lock` is idle (and verifies the exact `<root>/target/release` fallback added in Task 2!).

## Verification Proof Outputs

### 1. `dev_loop_guard.rs` Unit Tests — TDD RED (Before Doc Edits)
Command: `/Users/brbrainerd/.gemini/antigravity/brain/04c023ad-bd49-44e5-8e94-bc6fd24fd6ea/scratch/dev_loop_guard_test`
```text
running 9 tests
test tests::flags_a_fast_tier_step_that_cargo_runs_a_tool ... ok
test tests::flags_a_hardcoded_build_jobs_count ... ok
test tests::flags_hardcoded_rustc_wrapper_and_missing_env_keys ... ok
test tests::flags_hooks_that_build ... ok
test tests::flags_missing_macos_dead_strip_or_global_panic_abort ... ok
test tests::flags_vox_subcommands_missing_from_the_registry ... ok
test tests::passes_hooks_that_only_call_installed_binaries ... ok
test tests::validates_agent_target_slug_and_broker_docs_parity ... ok
test tests::the_repository_passes_the_dev_loop_guard ... FAILED

failures:

---- tests::the_repository_passes_the_dev_loop_guard stdout ----
dev-loop-guard: broker docs parity: cargo-safety.md missing VOX_BROKER_RUN_TIMEOUT_SECS
dev-loop-guard: broker docs parity: cargo-safety.md missing VOX_WORKSTREAM
dev-loop-guard: broker docs parity: cargo-safety.md missing VOX_ISOLATED_TARGET
dev-loop-guard: broker docs parity: cargo-safety.md missing batched cargo test -p guidance
dev-loop-guard: broker docs parity: build-broker-usage.md missing VOX_BROKER_RUN_TIMEOUT_SECS
dev-loop-guard: broker docs parity: build-broker-usage.md missing VOX_WORKSTREAM
dev-loop-guard: broker docs parity: build-broker-usage.md missing VOX_BROKER_NO_SCCACHE
dev-loop-guard: broker docs parity: build-broker-usage.md missing target_p95
dev-loop-guard: broker docs parity: build-broker-usage.md missing target_wait

test result: FAILED. 8 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

### 2. `dev_loop_guard.rs` Unit Tests — TDD GREEN (After Doc Edits) + `rustfmt --check` + `vox-drift-check`
Commands:
- `/Users/brbrainerd/.gemini/antigravity/brain/04c023ad-bd49-44e5-8e94-bc6fd24fd6ea/scratch/dev_loop_guard_test`
- `rustfmt --edition 2024 --check crates/vox-cli-ci/src/dev_loop_guard.rs` -> `exit 0`
- `vox-drift-check --files crates/vox-cli-ci/src/dev_loop_guard.rs` -> `exit 0`
```text
running 9 tests
test tests::validates_agent_target_slug_and_broker_docs_parity ... ok
test tests::flags_a_fast_tier_step_that_cargo_runs_a_tool ... ok
test tests::flags_a_hardcoded_build_jobs_count ... ok
test tests::flags_hardcoded_rustc_wrapper_and_missing_env_keys ... ok
test tests::flags_hooks_that_build ... ok
test tests::flags_missing_macos_dead_strip_or_global_panic_abort ... ok
test tests::flags_vox_subcommands_missing_from_the_registry ... ok
test tests::passes_hooks_that_only_call_installed_binaries ... ok
test tests::the_repository_passes_the_dev_loop_guard ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

### 3. `vox-doc-pipeline` KB Lint on `docs/src/contributors/build-broker-usage.md`
Command: `./target/debug/vox-doc-pipeline --lint-only --paths contributors/build-broker-usage.md`
```text
vox-doc-pipeline: linting ./docs/src/contributors/build-broker-usage.md
  WARN   contributors/build-broker-usage.md:0 — backlog excuses facet-missing-authorship but the doc no longer violates it; run --tighten-facet-backlog (facet-backlog-stale)
  WARN   contributors/build-broker-usage.md:0 — backlog excuses facet-missing-domain but the doc no longer violates it; run --tighten-facet-backlog (facet-backlog-stale)
  WARN   contributors/build-broker-usage.md:0 — backlog excuses facet-missing-summary but the doc no longer violates it; run --tighten-facet-backlog (facet-backlog-stale)
  WARN   contributors/build-broker-usage.md:0 — backlog excuses facet-missing-topic but the doc no longer violates it; run --tighten-facet-backlog (facet-backlog-stale)
vox-doc-pipeline lint complete — no hard errors. (exit code 0)
```

### 4. `.vox` Type-Checks, Installer Dry-Run & `broker-bypass-lint`
Commands:
- `timeout 300s vox check scripts/broker-install.vox` -> `Check passed with 198 warning(s)` (`exit 0`)
- `timeout 300s vox check scripts/broker-ci.vox` -> `Check passed with 57 warning(s)` (`exit 0`)
- `timeout 300s vox run scripts/broker-install.vox -- --dry-run` -> `exit 0`
- `timeout 300s vox run --interp --max-steps 50000000 scripts/broker-bypass-lint.vox -- -- scripts/broker-install.vox scripts/broker-ci.vox crates/vox-cli-ci/src/dev_loop_guard.rs` -> `broker-bypass-lint: 0 unallowlisted bypass(es), 0 allowlisted` (`exit 0`)

## Known Stubs

None — zero stubs, `TODO`s, or placeholder values in any modified file.

## User Setup Required

None.

---
*Phase: 447-repo-wide-build-contention-elimination-target-aware-broker-c*
*Completed: 2026-11-15*
