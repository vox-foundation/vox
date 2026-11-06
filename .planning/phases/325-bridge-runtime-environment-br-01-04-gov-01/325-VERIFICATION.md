---
phase: 325-bridge-runtime-environment-br-01-04-gov-01
verified: 2026-11-04T01:54:42Z
status: passed
score: 8/8 must-haves verified
covered_files: [
  ".planning/phases/325-bridge-runtime-environment-br-01-04-gov-01/325-01-PLAN.md",
  ".planning/phases/325-bridge-runtime-environment-br-01-04-gov-01/325-01-SUMMARY.md",
  ".planning/phases/325-bridge-runtime-environment-br-01-04-gov-01/325-02-PLAN.md",
  ".planning/phases/325-bridge-runtime-environment-br-01-04-gov-01/325-02-SUMMARY.md",
  "AGENTS.md",
  "Cargo.toml",
  "contracts/ci/crate-edges.allow.v1.json",
  "contracts/cli/command-registry.yaml",
  "contracts/operations/catalog.v1.yaml",
  "crates/vox-cli/src/cli_dispatch/mod.rs",
  "crates/vox-cli/src/commands/py/env.rs",
  "crates/vox-cli/src/commands/py/mod.rs",
  "crates/vox-cli/src/commands/run.rs",
  "crates/vox-compiler/Cargo.toml",
  "crates/vox-compiler/src/eval/expr.rs",
  "crates/vox-compiler/src/eval/python.rs",
  "crates/vox-compiler/src/typeck/builtins.rs",
  "crates/vox-compiler/src/typeck/effect_check.rs",
  "crates/vox-package-types/src/lockfile.rs",
  "crates/vox-package-types/src/manifest.rs",
  "crates/vox-python-bridge/Cargo.toml",
  "crates/vox-python-bridge/src/bin/fake_worker.rs",
  "crates/vox-python-bridge/src/frame.rs",
  "crates/vox-python-bridge/src/host.rs",
  "crates/vox-python-bridge/src/lib.rs",
  "crates/vox-python-bridge/tests/lifecycle.rs",
  "crates/vox-python-bridge/worker/vox_bridge/__main__.py",
  "docs/src/architecture/external-frontend-interop-phase5-component-interop-subspec-2026.md",
  "docs/src/architecture/layers.toml",
  "docs/src/architecture/vox-python-interop-design-2026-11-01.md",
  "docs/src/architecture/where-things-live.md",
  "docs/src/reference/syntax-k-complexity.md",
  "examples/golden/py_bridge_hello.vox"
]
covered_digest: "v3:sha256:9ccff312392a2457ddd702534657d2c6ae24650795c3d0efc28dfb885e4268d1"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "sandboxed spawn goes through vox-skill-runtime"
    reason: "CONTEXT D-06 and the verification brief: SkillRuntime cannot host a long-lived stdio worker. An unsandboxed worker plus the doctor warning satisfies this phase. Sandbox absence is not a phase 325 gap."
    accepted_by: "user"
    accepted_at: "2026-11-04T01:48:00Z"
decision_coverage:
  honored: 3
  total: 8
  not_honored:
    - D-01
    - D-05
    - D-06
    - D-07
    - D-08
---

# Phase 325: Bridge Runtime & Environment Verification Report

**Phase Goal:** `vox-python-bridge` spawns a real uv-managed worker, speaks framed Arrow IPC, survives crashes/timeouts, and lands with governance rows.
**Verified:** 2026-11-04T01:54:42Z
**Status:** passed
**Re-verification:** No — initial verification

Locked boundary from `325-CONTEXT.md`: this phase's frame body is opaque bytes. Arrow decode belongs to phase 326. The worker is unsandboxed, and `vox py doctor` warns. That warning is the sandbox requirement for this phase.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
| --- | --- | --- | --- |
| 1 | `vox py sync` on a `[python]` project creates `.vox/python` via `uv lock` and `uv sync --frozen`, and records a BLAKE3 of `uv.lock` in `vox.lock`. `vox py doctor` reports uv, Python, packages, lock match, and an unsandboxed warning. | ✓ VERIFIED | `sync_project` / `doctor_report` in `crates/vox-cli/src/commands/py/env.rs`. Hash is `vox_crypto::secure_hash`. Test `sync_writes_env_and_doctor_matches_the_lock_when_uv_is_present` asserts the venv, a 64-hex hash, `lock: match`, and the warning. `uv` 0.13.6 is on PATH, so the skip branch is not the only path. Not re-run: the functions and assertions match. |
| 2 | Without `[python]`, `python.version()` returns an error starting `py-env:` and does not spawn. | ✓ VERIFIED | `version_string` returns `PY_ENV_MISSING` before `Host::start`. `python_bridge_missing_config_does_not_spawn` asserts the prefix and `!worker_spawned`. `sync_without_python_section_does_not_ask_uv` asserts no `.vox` directory. |
| 3 | A hello round-trips through the fake worker and through `python3 -m vox_bridge`. `debug.print` does not corrupt the framed reply. | ✓ VERIFIED | Re-ran `cargo test -p vox-python-bridge --test lifecycle`: `fake_worker_hello_round_trips`, `real_worker_hello_returns_version_platform_and_pid`, and `debug_print_reaches_stderr_and_the_reply_still_parses` passed. Worker dups fd 1 onto the RPC fd and points `sys.stdout` at stderr before `print`. |
| 4 | `vox run` of `examples/golden/py_bridge_hello.vox` prints the worker Python version when `[python]` is set. A cap set without `py` is denied and does not spawn. Missing `uses python` fails typecheck. | ✓ VERIFIED | Golden calls `python.version()` under `@uses(python)`. `run.rs` loads `[python]` into `PythonProject` and prefers `.vox/python/.venv` python. `expr.rs` returns `CapabilityDenied { ns: "py", method: "version" }` before `call_version`. `python_bridge_missing_cap_does_not_spawn` and `python_bridge_missing_effect_fails_typecheck` assert those failures. `python_bridge_vox_run_prints_version` copies the golden, syncs, and asserts a dotted numeric version line. CLI test not re-run; the call path and assertions are direct. |
| 5 | A sleep past `timeout_s` errors, bumps the generation, and a previous-generation handle returns `py: stale handle`. A worker crash errors, the host survives, and the next hello runs on a new pid. | ✓ VERIFIED | Same lifecycle run: `debug_sleep_bumps_generation_and_rejects_the_old_handle` and `debug_crash_is_survived_by_the_next_request` passed (7 passed, 0 failed, 0 filtered). Timeout path is SIGINT, 5s grace, then kill, then `retire` increments generation. `stale_message` returns `py: stale handle (worker restarted)`. |
| 6 | A frame whose declared body exceeds `max_fetch_mb` returns `py: fetch exceeds cap` before the body is read. | ✓ VERIFIED | `read_frame` compares `body_len` to the cap and returns before `read_exact` of the body. Fake worker `test.oversize` declares 2 MiB and writes no body. `declared_body_over_cap_fails_before_the_body_is_read` passed against a 1 MiB cap. |
| 7 | The crate has a `layers.toml` row, a `where-things-live.md` row, and the two phase-owned crate edges. AGENTS.md has the scoped `.py` exemption. The two stale citations point at current lines. The design doc §6 and §12 record the sandbox deferral. | ✓ VERIFIED | `layers.toml` L2 library row, no `orphan_exempt`. Allow file has `vox-compiler → vox-python-bridge` and `vox-python-bridge → vox-foundation`, matching `Cargo.toml` deps. `vox-cli` does not depend on the bridge; that edge was not added. AGENTS.md exempts `crates/vox-python-bridge/worker/**` only. `syntax-k-complexity.md:65` cites `crates/vox-codegen/src/syntax_k.rs`. Phase5 subspec cites `stmt_expr.rs:111`, which is `enum HirType`. Design doc §6 and §12 state the unsandboxed deferral. Catalog and command registry have `py`, `py.sync`, and `py.doctor` with `handler_rust: commands::py`. |
| 8 | Sandboxed spawn goes through `vox-skill-runtime`. | ✓ PASSED (override) | Override: CONTEXT D-06 and the verification brief — accepted by user on 2026-11-04T01:48:00Z. Doctor prints a static `sandbox: unsandboxed` warning. No `vox-skill-runtime` dependency was added. |

**Score:** 8/8 truths verified (0 present, behavior-unverified)

Full `vox ci crate-edges` and `vox-arch-check` were not treated as phase failures. The brief excludes pre-existing misses that are not the python-bridge edges (`vox-plugin-mens-candle-core` → `vox-eval`, stale `vox-gui` → `vox-db-types`, and the known raw-git-exec hits). The uncommitted fan-in snapshot and git-exec migration were not staged and are not part of this phase.

### Required Artifacts

| Artifact | Expected | Status | Details |
| -------- | -------- | ------ | ------- |
| `crates/vox-python-bridge` | L2 framed host and stdlib worker | ✓ VERIFIED | Workspace member via `crates/*`. `Bridge::call` speaks `u32` + JSON `DispatchRequest` + `u64` + body. Spawn is `tokio::process::Command` argv. |
| `crates/vox-python-bridge/tests/lifecycle.rs` | BR-02/03/04 behaviors | ✓ VERIFIED | 7 tests, all passed this run. |
| `crates/vox-cli/src/commands/py/` | `vox py sync` and `vox py doctor` | ✓ VERIFIED | Dispatched from `Cli::Py` in `cli_dispatch/mod.rs`. |
| `crates/vox-compiler/src/eval/python.rs` | `python.version()` host | ✓ VERIFIED | Lazy worker; missing config returns before spawn; venv python when `.vox/python/.venv` exists. |
| `examples/golden/py_bridge_hello.vox` | One-op golden | ✓ VERIFIED | `@uses(python)` and `python.version()`. |
| `crates/vox-package-types` `[python]` / lock hash | Manifest and `vox.lock` field | ✓ VERIFIED | `python` is appended after `db`. Defaults `max_fetch_mb` 256 and `timeout_s` 300. |
| Governance rows | layers, where-things-live, edges, catalog | ✓ VERIFIED | Owned edges present. `orphan_exempt` is gone. |

### Key Link Verification

| From | To | Via | Status | Details |
| ---- | --- | --- | ------ | ------- |
| `run.rs` | `eval/python.rs` | `discover_python` then `set_python_project` | WIRED | Interpreter receives root, `max_fetch_mb`, and `timeout_s`. |
| `expr.rs` `python.version` | `Bridge::call("hello")` | `call_version` → `hello_version` | WIRED | Cap check runs first. Version string is `reply.value["python"]`. |
| `py/env.rs` `sync_project` | `uv` | `Command::new("uv")` with `--directory` | WIRED | `lock` then `sync --frozen`. No shell. |
| `frame.rs` `read_frame` | fetch cap | length check before body read | WIRED | Covered by the passing oversize test. |
| `vox-compiler` | `vox-python-bridge` | Cargo dependency and allow-file pair | WIRED | `vox-python-bridge.workspace = true`. |
| `vox-python-bridge` | `vox-foundation` | `DispatchRequest` / `DispatchResponse` | WIRED | Allow-file pair matches the dependency. |
| `effect_check.rs` | `HirCapability::Python` | `stdlib_module_capability("python")` | WIRED | Typeck `PythonModule.version` is `fn() -> Result[str, str]`. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
| -------- | ------------- | ------ | ------------------ | ------ |
| `python.version()` | version string | worker `hello` → `platform.python_version()` | Yes | ✓ FLOWING |
| `vox py doctor` python line | `python_version(venv)` | `.vox/python/.venv` `python --version` | Yes | ✓ FLOWING |
| `vox.lock` hash | `python_uv_lock_hash` | BLAKE3 of the written `uv.lock` | Yes | ✓ FLOWING |
| doctor sandbox line | static warning | specified unsandboxed status | The warning is the contract | ✓ FLOWING |
| fetch-cap error | declared `body_len` | frame header, before body bytes | Yes | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
| -------- | ------- | ------ | ------ |
| Hello, print, timeout/stale handle, crash restart, oversize cap | `cargo test -p vox-python-bridge --test lifecycle -- --test-threads=4` | `ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` in 1.23s. Named tests include `debug_sleep_bumps_generation_and_rejects_the_old_handle` and `debug_crash_is_survived_by_the_next_request`. | ✓ PASS |
| uv sync + golden `vox run` | inspected `commands::py::tests::python_bridge_vox_run_prints_version` | Not re-run. Assertions require a synced env and a numeric version line from `run`. | skipped re-run |
| Missing `[python]` does not spawn | inspected `python_bridge_missing_config_does_not_spawn` | Not re-run. Early return is before `Host::start`. | skipped re-run |

`python3` is `/opt/homebrew/bin/python3` (3.15.2) and `uv` is 0.13.6, so the lifecycle tests did not take their `python3` absent return.

### Probe Execution

No phase probe is declared. Step 7c skipped.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
| ----------- | ---------- | ----------- | ------ | -------- |
| BR-01 | 325-02 | uv sync, lock hash, doctor; `py-env:` and no spawn without `[python]` | ✓ SATISFIED | Truths 1 and 2 |
| BR-02 | 325-01, 325-02 | Golden and real worker; `print` does not corrupt the frame | ✓ SATISFIED | Truths 3 and 4; lifecycle run |
| BR-03 | 325-01 | Timeout and crash restart; `py: stale handle` | ✓ SATISFIED | Truth 5; lifecycle run |
| BR-04 | 325-01, 325-02 | Fetch cap; unsandboxed warning. Sandbox host deferred. | ✓ SATISFIED | Truths 6 and 8 (override on skill-runtime spawn) |
| GOV-01 | 325-01, 325-02 | layers, where-things-live, owned edges, AGENTS exemption, citation fixes | ✓ SATISFIED | Truth 7. `vox-cli → vox-python-bridge` was not added because `vox-cli` does not depend on the bridge. |

No requirement mapped to phase 325 is missing from the plans.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
| --- | --- | --- | --- | --- | --- | --- |
| `crates/vox-python-bridge/tests/lifecycle.rs` | BR-02, BR-03, BR-04 | 7 | 0 in this run | no | Behavioral | PASS |
| `crates/vox-cli/src/commands/py/env.rs` | BR-01, BR-04 | 6 | early return only if `uv` is absent | no | Value (hash, paths, doctor text) | PASS |
| `crates/vox-cli/src/commands/py/mod.rs` | BR-02 | 2 | early return only if `uv` or `python3` is absent | no | Behavioral for the golden run | PASS |
| `crates/vox-compiler/src/eval/python.rs` | BR-01, BR-02 | 5 bridge tests | early return only if `python3` is absent on the live hello | no | Behavioral (error prefix, cap denial, version shape) | PASS |
| `crates/vox-compiler/src/typeck/effect_check.rs` | BR-02 | 3 python tests | 0 | no | Value (`uses python` diagnostic) | PASS |

**Disabled tests on requirements:** 0
**Circular patterns detected:** 0
**Insufficient assertions:** 0

`python_bridge_print_capture_round_trip` only checks the test print recorder. The worker print requirement is the lifecycle stderr test, which passed.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
| ---- | ---- | ------- | -------- | ------ |
| `docs/src/architecture/vox-python-interop-design-2026-11-01.md` | — | untracked (`git ls-files` empty; not ignored) | ℹ️ Info | §6 and §12 do record the deferral. The file is in the worktree and not in HEAD. Phase 325 did not edit it. Committed doctor text and AGENTS.md carry the same decision. |
| phase-owned crates | — | `TBD` / `FIXME` / `XXX` | none | No debt markers in the bridge, `py` command, or `eval/python.rs`. |

Spawn sites are `tokio::process::Command` and `std::process::Command::new`. No shell string.

### Decision Coverage

### Decision Coverage (warning)

5 decision(s) not found in shipped artifacts by the substring gate:

- **D-01**: crate, `vox-foundation` protocol types, fake worker via `worker_cmd`
- **D-05**: `[python]`, uv sync, BLAKE3 lock hash, `py-env:` without spawn
- **D-06**: unsandboxed worker and doctor warning; skill-runtime host deferred
- **D-07**: `python.version()`, `HirCapability::Python`, golden
- **D-08**: layers, where-things-live, edges, catalog, AGENTS exemption, citation fixes

This is a soft warning. Manual reading found D-01, D-02, D-03, D-04, D-05, D-07, and D-08 in the committed code. D-06 is honored as the unsandboxed warning; live `vox_skill_runtime` detection was not added, which the override accepts. The gate does not change status.

### Human Verification

N/A — infrastructure phase. Timeout, crash, cap, and hello behaviors were exercised by the lifecycle tests. No user-facing flow is left unverified.

### Gaps Summary

No gaps. The phase goal holds in the committed bridge, CLI, and compiler wiring. The sandbox host is an accepted deferral, not missing work for this phase.

---

_Verified: 2026-11-04T01:54:42Z_
_Verifier: Claude (gsd-verifier)_
