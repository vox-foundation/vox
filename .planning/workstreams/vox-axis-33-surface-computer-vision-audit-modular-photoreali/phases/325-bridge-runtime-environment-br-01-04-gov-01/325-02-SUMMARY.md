---
phase: 325-bridge-runtime-environment-br-01-04-gov-01
plan: "02"
subsystem: runtime
tags: [python, uv, vox-py, capabilities, blake3]

requires:
  - phase: 325-01
    provides: vox-python-bridge worker and framed hello RPC
  - phase: 330-01
    provides: VoxManifest db field that [python] appends after
provides:
  - "[python] on VoxManifest and python_uv_lock_hash on vox.lock"
  - vox py sync and vox py doctor
  - python.version() through the interpreter when caps grant py
affects: [326, python-bridge]

actuals:
  tokens: 15952
  tasks: 4
  commits: 4

plan_head_before: d3fa49a849f10dc8133e84132e6edbcacfabf88b
plan_head_after: 829626863b103e12a40c5fd52463e34944f8aead

tech-stack:
  added: []
  patterns:
    - "uv is spawned as argv (uv --directory <dir> lock / sync --frozen), never a shell"
    - "The uv.lock BLAKE3 is vox_crypto::secure_hash in vox-cli; vox-package-types only stores the hex"
    - "python.version() runs the bridge on a dedicated current-thread runtime so the child is not dropped and block_on is not called inside the caller runtime"

key-files:
  created:
    - crates/vox-cli/src/commands/py/mod.rs
    - crates/vox-cli/src/commands/py/env.rs
    - crates/vox-compiler/src/eval/python.rs
    - examples/golden/py_bridge_hello.vox
  modified:
    - crates/vox-package-types/src/manifest.rs
    - crates/vox-package-types/src/lockfile.rs
    - contracts/operations/catalog.v1.yaml
    - contracts/cli/command-registry.yaml
    - contracts/ci/crate-edges.allow.v1.json
    - docs/src/architecture/layers.toml
    - AGENTS.md

key-decisions:
  - "No vox-package-types or vox-python-bridge dependency on vox-crypto. The CLI, which already depends on vox-crypto, writes the lock line."
  - "No vox-cli dependency on vox-python-bridge. vox run reaches the bridge through vox-compiler. The only new crate edge is vox-compiler → vox-python-bridge."
  - "Doctor prints a static unsandboxed warning. This phase does not take a vox-skill-runtime dependency."
  - "requires-python is emitted as >=<version> when the manifest version starts with a digit, because uv rejects a bare 3.12."

patterns-established:
  - "Bridge entry points return py-env: python not configured (add [python] to Vox.toml) and do not spawn when the section is absent."
  - "python.version() is gated by caps.allows_namespace(py) before the generic builtin namespace check."

requirements-completed: [BR-01, BR-04, GOV-01]

coverage:
  - id: D1
    description: "vox py sync creates .vox/python via uv and records the uv.lock BLAKE3 in vox.lock; vox py doctor reports uv, python, packages, lock match, and an unsandboxed warning."
    requirement: BR-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-cli --lib commands::py (7 passed, including sync_writes_env_and_doctor_matches_the_lock_when_uv_is_present)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Without [python], python.version() returns an error starting py-env: and does not spawn a worker."
    requirement: BR-01
    verification:
      - kind: unit
        ref: "cargo test -p vox-compiler --lib python_bridge_missing_config_does_not_spawn"
        status: pass
    human_judgment: false
  - id: D3
    description: "python.version() typechecks as Result[str, str], requires uses python, and vox run of the golden prints the worker version. A caps set without py is denied and does not spawn."
    requirement: BR-04
    verification:
      - kind: integration
        ref: "cargo test -p vox-compiler --lib python_bridge (8 passed); cargo test -p vox-cli --lib python_bridge_vox_run_prints_version (1 passed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "AGENTS.md exempts only the bridge worker tree. Stale syntax-k and HirType citations point at the current files. The design doc already records the unsandboxed deferral in §6 and §12."
    requirement: GOV-01
    verification:
      - kind: other
        ref: "cargo run -p vox-doc-pipeline -- --lint-only --paths reference/syntax-k-complexity.md,architecture/external-frontend-interop-phase5-component-interop-subspec-2026.md,architecture/vox-python-interop-design-2026-11-01.md"
        status: pass
    human_judgment: false

duration: 54min
completed: 2026-11-03
status: complete
---

# Phase 325 Plan 02: Bridge Runtime & Environment Summary

**`[python]` syncs a uv env, `vox py doctor` warns that the worker is unsandboxed, and `python.version()` returns that worker's version only when the `py` cap is granted.**

## Performance

- **Duration:** 54 min
- **Tasks:** 4/4
- **Commits:** 4 (measured `d3fa49a849..829626863`)

## Accomplishments

- `VoxManifest.python` is appended after `db`. `vox.lock` stores `python_uv_lock_hash`.
- `vox py sync` writes `.vox/python/pyproject.toml`, runs `uv lock` and `uv sync --frozen`, and records the lock hash. `vox py doctor` prints uv, Python, packages, lock status, `sandbox: unsandboxed`, and the deferral warning.
- `python.version()` is a `PythonModule` method returning `Result[str, str]`. The effect checker maps the `python` namespace to `HirCapability::Python`. The interpreter calls the bridge only after `py` is allowed and `[python]` is present.
- `examples/golden/py_bridge_hello.vox` runs through `vox run` in a temp project and prints the worker version (`3.12.10` on this machine).
- AGENTS.md exempts `crates/vox-python-bridge/worker/**` only. `syntax_k.rs` is cited under `vox-codegen`. `HirType` is cited at `stmt_expr.rs:111`.

## Task Commits

1. **Task 1: `[python]` manifest and lock hash** — `9b0f8f799` (feat)
2. **Task 2: `vox py sync|doctor`** — `dc187a93a` (feat)
3. **Task 3: `python.version()` end to end** — `b8cf7109c` (feat)
4. **Task 4: docs/governance** — `829626863` (docs)

## Files Created/Modified

- `crates/vox-cli/src/commands/py/` — sync, doctor, lock-hash read/write
- `crates/vox-compiler/src/eval/python.rs` — lazy worker host
- `contracts/ci/crate-edges.allow.v1.json` — `["vox-compiler", "vox-python-bridge"]`
- `docs/src/architecture/layers.toml` — `orphan_exempt` removed from `vox-python-bridge`
- `contracts/operations/catalog.v1.yaml` and `contracts/cli/command-registry.yaml` — `py`, `py.sync`, `py.doctor`
- `docs/src/reference/cli-command-surface.generated.md` — regenerated by `vox ci command-sync --write`, not hand-edited

## Decisions Made

- The lock digest is computed in `vox-cli` with `vox_crypto::secure_hash`. `vox-package-types` does not gain a `vox-crypto` dependency.
- `vox-cli` does not depend on `vox-python-bridge`. `run.rs` passes the discovered `[python]` config into the compiler, which owns the bridge.
- `requires-python` is a PEP 440 specifier. A manifest version of `3.12` is written as `>=3.12`.
- Effect checking still skips unannotated functions. The missing-effect test uses `@uses(net)` so the diagnostic is `uses python`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] uv rejected `requires-python = "3.12"`**
- **Found during:** Task 2
- **Issue:** `uv lock` failed because a bare version is not a specifier.
- **Fix:** Digit-leading versions are prefixed with `>=`.
- **Files modified:** `crates/vox-cli/src/commands/py/env.rs`
- **Commit:** `dc187a93a`

**2. [Rule 1 - Bug] `read_recorded_hash` returned None on the lock header comment**
- **Found during:** Task 2
- **Issue:** `strip_prefix(... )?` inside the line loop returned from the function on the first non-matching line.
- **Fix:** Non-matching lines continue the scan.
- **Files modified:** `crates/vox-cli/src/commands/py/env.rs`
- **Commit:** `dc187a93a`

**3. [Rule 3 - Blocking] Doc lint rejects `vox:skip`**
- **Found during:** Task 4
- **Issue:** The phase 5 subspec's illustrative fences were `vox` blocks with `vox:skip`, which `vox-doc-pipeline` now rejects.
- **Fix:** Those two fences are `text`. The three `stmt_expr.rs:90-103` citations still moved to `:111`.
- **Files modified:** `docs/src/architecture/external-frontend-interop-phase5-component-interop-subspec-2026.md`
- **Commit:** `829626863`

### Design doc

§6 (trust boundary) and §12 (deferred sandboxed worker) already stated that `SkillRuntime` cannot host the stdio worker and that `vox py doctor` warns. Task 4 linted that page and did not edit it.

## Crate edges

- Added: `vox-compiler` → `vox-python-bridge`
- Not added: `vox-cli` → `vox-python-bridge` (no Cargo dependency)
- Not added: `vox-crypto` on `vox-python-bridge` or `vox-package-types`
- `orphan_exempt` removed from `vox-python-bridge` because the compiler consumer exists
- `vox-foundation` `max_dependents` was not raised. `contracts/ci/fan-in-snapshot.v1.json` was not touched.

## Known Stubs

None. The unsandboxed worker is the behavior this phase specifies, and doctor prints the warning.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: process-spawn | crates/vox-compiler/src/eval/python.rs | `python.version()` spawns `python -m vox_bridge` when `[python]` is set and the `py` cap is granted. The worker is unsandboxed in this phase. |
| threat_flag: process-spawn | crates/vox-cli/src/commands/py/env.rs | `vox py sync` spawns `uv` with argv to lock and sync `.vox/python`. |

## Tests

- `cargo test -p vox-package-types --lib python_` — 4 passed (task 1)
- `cargo test -p vox-cli --lib commands::py` — 7 passed (task 2, after the two fixes)
- `cargo test -p vox-compiler --lib python_bridge` — 8 passed
- `cargo test -p vox-python-bridge --lib worker_root` — 1 passed
- `cargo test -p vox-cli --lib python_bridge_vox_run_prints_version` — 1 passed
- `vox-doc-pipeline --lint-only` on the three docs — no hard errors

## Self-Check: PASSED

- FOUND: `examples/golden/py_bridge_hello.vox`
- FOUND: `crates/vox-compiler/src/eval/python.rs`
- FOUND: `crates/vox-cli/src/commands/py/env.rs`
- FOUND: `9b0f8f799`
- FOUND: `dc187a93a`
- FOUND: `b8cf7109c`
- FOUND: `829626863`
