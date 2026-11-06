---
phase: 325-bridge-runtime-environment-br-01-04-gov-01
plan: 01
subsystem: infra
tags: [python, bridge, stdio, tokio, framing]

requires:
  - phase: 324-effect-gate
    provides: python effect names the later compiler wiring will call
provides:
  - L2 vox-python-bridge crate and stdlib worker
  - framed request/response lifecycle with generationed handles
  - downward crate edge vox-python-bridge to vox-foundation
affects: [325-02, python-bridge, crate-edges]

tech-stack:
  added: []
  patterns:
    - "u32 LE header length, JSON DispatchRequest/DispatchResponse, u64 LE body length, opaque body"
    - "one lazy worker, requests serialized, SIGINT then 5s grace then kill"
    - "fake worker via BridgeConfig.worker_cmd; real worker via python3 -m vox_bridge"

key-files:
  created:
    - crates/vox-python-bridge/src/host.rs
    - crates/vox-python-bridge/src/frame.rs
    - crates/vox-python-bridge/src/bin/fake_worker.rs
    - crates/vox-python-bridge/worker/vox_bridge/__main__.py
    - crates/vox-python-bridge/tests/lifecycle.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - docs/src/architecture/layers.toml
    - docs/src/architecture/where-things-live.md
    - contracts/ci/crate-edges.allow.v1.json

key-decisions:
  - "Only edge added: vox-python-bridge -> vox-foundation. No vox-crypto edge; this plan does not hash."
  - "orphan_exempt until plan 325-02 adds vox-compiler and vox-cli consumers."
  - "vox-foundation max_dependents stays 18; live fan-in becomes 15."

patterns-established:
  - "Worker stdout is dup'd onto the RPC fd and sys.stdout points at stderr before any reply."
  - "A declared body over max_fetch_mb returns py: fetch exceeds cap before the body is read."

requirements-completed: [BR-02, BR-03, BR-04]

coverage:
  - id: D1
    description: "Fake-worker hello round-trip and batched release."
    requirement: BR-02
    verification:
      - kind: integration
        ref: crates/vox-python-bridge/tests/lifecycle.rs#fake_worker_hello_round_trips
        status: pass
      - kind: integration
        ref: crates/vox-python-bridge/tests/lifecycle.rs#release_ids_are_batched_into_the_next_call
        status: pass
    human_judgment: false
  - id: D2
    description: "Real Python hello, and debug.print on stderr without corrupting the reply."
    requirement: BR-02
    verification:
      - kind: integration
        ref: crates/vox-python-bridge/tests/lifecycle.rs#real_worker_hello_returns_version_platform_and_pid
        status: pass
      - kind: integration
        ref: crates/vox-python-bridge/tests/lifecycle.rs#debug_print_reaches_stderr_and_the_reply_still_parses
        status: pass
    human_judgment: false
  - id: D3
    description: "debug.sleep past the deadline bumps the generation and rejects the old handle; debug.crash restarts on the next request."
    requirement: BR-03
    verification:
      - kind: integration
        ref: crates/vox-python-bridge/tests/lifecycle.rs#debug_sleep_bumps_generation_and_rejects_the_old_handle
        status: pass
      - kind: integration
        ref: crates/vox-python-bridge/tests/lifecycle.rs#debug_crash_is_survived_by_the_next_request
        status: pass
    human_judgment: false
  - id: D4
    description: "A frame whose declared body exceeds max_fetch_mb returns py: fetch exceeds cap without reading the body."
    requirement: BR-04
    verification:
      - kind: integration
        ref: crates/vox-python-bridge/tests/lifecycle.rs#declared_body_over_cap_fails_before_the_body_is_read
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-11-03
status: complete
plan_head_before: 149e250e0d6944c59b2e15072060314fb876f0bf
plan_head_after: b891e557580c3a1a6658929c673f8cca0cb9cf33
actuals:
  tokens: 10000
  tasks: 3
  commits: 4
---

# Phase 325 Plan 01: `vox-python-bridge` crate, worker, lifecycle Summary

**Framed stdio host that round-trips a real Python worker and drops stale handles after a timeout or crash.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-11-03T10:37:44Z
- **Completed:** 2026-11-03T11:02:24Z
- **Tasks:** 3
- **Files modified:** 14

## Accomplishments

- Added the L2 `vox-python-bridge` library, workspace dependency, where-things-live row, and one downward edge to `vox-foundation`.
- A fake worker and `python3 -m vox_bridge` both complete a hello round-trip. `debug.print` lands on stderr and the host still parses the reply.
- `debug.sleep` past the deadline and `debug.crash` bump the generation. A handle from the old generation returns `py: stale handle (worker restarted)`. An oversized declared body returns `py: fetch exceeds cap` before the body is read.

## Task Commits

1. **Task 1: scaffold + governance rows** - `304abe3fb` (feat)
2. **Task 2: framing + lifecycle tests (RED)** - `df00ddf06` (test)
3. **Task 3: worker, framing, lifecycle (GREEN)** - `6441070d3` (feat)
4. **Bytecode ignore** - `b891e5575` (chore)

## Files Created/Modified

- `crates/vox-python-bridge/src/host.rs` - lazy worker, serialized calls, timeout/kill, handle generation
- `crates/vox-python-bridge/src/frame.rs` - length-prefixed frames and the fetch cap
- `crates/vox-python-bridge/src/bin/fake_worker.rs` - Rust test double, including `test.oversize`
- `crates/vox-python-bridge/worker/vox_bridge/__main__.py` - stdlib worker (`hello`, `release`, `shutdown`, gated debug ops)
- `crates/vox-python-bridge/tests/lifecycle.rs` - the BR-02/03/04 truths
- `contracts/ci/crate-edges.allow.v1.json` - inserted `vox-python-bridge` → `vox-foundation`
- `docs/src/architecture/layers.toml` - L2 library row, `orphan_exempt` until plan 02
- `docs/src/architecture/where-things-live.md` - L2 row
- `Cargo.toml` / `Cargo.lock` - workspace member

## Decisions Made

- The only allow-file edge this plan inserts is `["vox-python-bridge", "vox-foundation"]`. `vox-compiler` and `vox-cli` edges wait until those crates depend on the bridge. No `vox-crypto` edge: this plan does not hash.
- `vox-foundation` `max_dependents` stays **18**. The new crate makes the live count 15, which does not exceed the cap.
- `contracts/ci/fan-in-snapshot.v1.json` still says `vox-foundation: 14`. That snapshot was not raised. `vox ci fan-in-budget` will report 15 until a later change bumps that one number.
- The worker is spawned with `tokio::process::Command` on the interpreter argv. No shell.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Correctness] `orphan_exempt` until consumers exist**
- **Found during:** Task 1
- **Issue:** Rule 4 is an error for a library with zero in-tree dependents. Plan 02 is what adds `vox-compiler` and `vox-cli`.
- **Fix:** `orphan_exempt = true` on the L2 row, with a comment that those consumers land in plan 325-02.
- **Files modified:** `docs/src/architecture/layers.toml`
- **Verification:** Fresh `cargo run -p vox-arch-check` does not list this crate as an orphan.
- **Committed in:** `304abe3fb`

**2. [Rule 3 - Blocking] in-file tests for the installed tdd-guard**
- **Found during:** Tasks 2 and 3
- **Issue:** The installed `toestub` rejected `pub fn`s whose only coverage was `tests/lifecycle.rs`.
- **Fix:** A default-config test in `lib.rs` and a generation-zero test in `host.rs`. The lifecycle file remains the behavioral suite.
- **Files modified:** `crates/vox-python-bridge/src/lib.rs`, `crates/vox-python-bridge/src/host.rs`
- **Verification:** Pre-commit `tdd-guard` passed on the RED and GREEN commits.
- **Committed in:** `df00ddf06`, `6441070d3`

**3. [Rule 2 - Correctness] ignore worker bytecode**
- **Found during:** Task 3
- **Issue:** `python3 -m vox_bridge` writes `__pycache__`, and the repo gitignore does not cover it.
- **Fix:** `crates/vox-python-bridge/worker/.gitignore`
- **Committed in:** `b891e5575`

**Total deviations:** 3 auto-fixed (2 Rule 2, 1 Rule 3)
**Impact on plan:** Governance and hook fixes only. No protocol change.

## Issues Encountered

- RED was confirmed before implementation: `cargo test -p vox-python-bridge` failed 7 lifecycle tests with `Protocol("not implemented")`, then passed 7 lifecycle tests plus 2 unit tests in 2.16s after the worker landed.
- `cargo run -p vox-arch-check` still exits 1 on 7 pre-existing `[raw-git-exec]` hits in `crates/vox-cli-ci` (`check_changed.rs`, `ci_changes.rs`, `evidence_asset_ban.rs`, `hit_every_surface.rs`, `suppression_review_gate.rs`). Also pre-existing fan-in warns for `vox-config` (41/40), `vox-db` (28/27), and `vox-secrets` (35/34), plus LoC-delta and `vox-cli` workspace-dep warns. Not fixed.
- `VOX_SKIP_FRESHNESS_CHECK=1 ./target/debug/vox ci crate-edges` accepts the new edge and still reports a pre-existing violation: `vox-plugin-mens-candle-core -> vox-eval`, plus a stale baseline edge `vox-gui -> vox-db-types`. The installed `vox` is one commit behind HEAD, so the freshness gate was skipped. Not fixed.
- A cached arch-check git index, written before these commits, briefly reported the new crate as stale. A cache rebuild does not.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Plan 02 can depend on `vox-python-bridge` from `vox-compiler` and `vox-cli` and insert those two edges in the same change as the `Cargo.toml` dependencies. It still owns Arrow-free `vox py sync`/`doctor` and `python.version()`. When those edges land, drop `orphan_exempt` if the crate then has a consumer, and bump `contracts/ci/fan-in-snapshot.v1.json` `vox-foundation` from 14 to the live count if that gate should go green.

## Self-Check: PASSED

- FOUND: `crates/vox-python-bridge/src/host.rs`
- FOUND: `crates/vox-python-bridge/worker/vox_bridge/__main__.py`
- FOUND: `304abe3fb`
- FOUND: `df00ddf06`
- FOUND: `6441070d3`
- FOUND: `b891e5575`

## Known Stubs

None. The lifecycle tests run the real worker and the fake worker.
