---
phase: 326-std-array-on-numpy-arr-01-04-k-01
plan: 08
subsystem: testing
tags: [python-bridge, std.array, ci, parity, arr-04]

requires:
  - phase: 326-02
    provides: on_disk_expected_matches_behavior_cases and the python_bridge_parity cargo target
provides:
  - An in-memory row missing signature, handler, or expected returns Err
  - vox ci python-bridge-parity returns Err when that cargo test exits non-zero
affects: [326-04]

actuals:
  tokens: 6227
  tasks: 2
  commits: 4
commits: 4
plan_head_before: f88190152872f91e70a7ae3fc3681d701a86d897
plan_head_after: b364350c70d6a59b907112f61271c5bd6aa93acd

tech-stack:
  added: []
  patterns:
    - python_bridge_parity_from_status maps ExitStatus to Result, and the CI arm returns that Result
    - A handler counts only as a column-0 def plus a non-comment HANDLERS key

key-files:
  created: []
  modified:
    - crates/vox-compiler/tests/python_bridge_parity_test.rs
    - crates/vox-cli/src/commands/ci/run_body.rs
    - crates/vox-cli-ci/src/cmd_enums.rs
    - contracts/operations/catalog.v1.yaml
    - contracts/cli/command-registry.yaml
    - contracts/policy/policy-registry.v1.yaml
    - docs/src/reference/cli.md
    - docs/src/reference/cli-command-surface.generated.md

key-decisions:
  - "A comment that names an op is not a handler. The checker skips comment lines before looking for a column-0 def or a HANDLERS key."
  - "The policy registry gained only the python-bridge-parity entry. The full generator rewrite was not committed."

patterns-established:
  - "CiCmd::PythonBridgeParity shells out through cargo_bin() and returns python_bridge_parity_from_status."

requirements-completed: [ARR-04]

coverage:
  - id: D1
    description: "An in-memory row that drops signature, handler, or expected returns Err. A comment that names an op is not a handler. The on-disk yaml is not rewritten."
    requirement: ARR-04
    verification:
      - kind: integration
        ref: "cargo test -p vox-compiler --test python_bridge_parity synthetic_row_missing_signature_handler_or_expected_is_err"
        status: pass
    human_judgment: false
  - id: D2
    description: "python_bridge_parity_from_status returns Err for a non-zero process status and Ok for a zero status. The PythonBridgeParity arm returns that Result."
    requirement: ARR-04
    verification:
      - kind: unit
        ref: "cargo test -p vox-cli --lib python_bridge_parity_nonzero_status_is_err"
        status: pass
    human_judgment: false
  - id: D3
    description: "vox ci python-bridge-parity is in the operations catalog, the command registry, and cli.md."
    requirement: ARR-04
    verification:
      - kind: other
        ref: "cargo run -p vox-cli -- ci command-compliance"
        status: fail
    human_judgment: true
    rationale: "command-compliance exits 1 on this branch before it finishes. The first error is a pre-existing missing reachability row for top-level py. A diagnostic row then surfaced an unrelated missing TOOL_REGISTRY entry for vox_context_handoff. python-bridge-parity itself is reachability_required false and its cli.md row passed check_ref_cli."

duration: 44min
completed: 2026-11-04
status: complete
---

# Phase 326 Plan 08: python-bridge-parity fails closed Summary

**An incomplete `std.array` row returns Err, and `vox ci python-bridge-parity` returns that Err when the parity cargo test exits non-zero.**

## Performance

- **Duration:** 44 min
- **Started:** 2026-11-03T23:47:38Z
- **Completed:** 2026-11-04T00:31:56Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- `synthetic_row_missing_signature_handler_or_expected_is_err` builds the row in memory. Dropping signature, handler, or expected returns Err. A handler that is not a column-0 def and not a HANDLERS key returns Err. A comment that names the op returns Err. A complete `mean` row against the worker returns Ok. The yaml bytes are unchanged.
- `CiCmd::PythonBridgeParity` (`python-bridge-parity`, catalog id `ci.python-bridge-parity`) runs `cargo test -p vox-compiler --test python_bridge_parity` through `cargo_bin()` and returns `python_bridge_parity_from_status`.
- A non-zero status is Err with the reproducer string. A zero status prints `python-bridge-parity OK` and returns Ok. There is no second branch that returns Ok after a failed status.

## TDD Gate Compliance

Both tasks are RED then GREEN. The plan frontmatter is `type: execute`. The RED runs failed on the planned assertion.

| Task | RED | GREEN | RED evidence |
|------|-----|-------|--------------|
| 1 | `47156e299` | `68a1ee255` | Exit 101. `parity::synthetic_row_missing_signature_handler_or_expected_is_err` panicked at `dropping signature must return Err`. 0 passed, 1 failed, 1 filtered out. |
| 2 | `6a0ddf6e3` | `b364350c7` | Exit 101. `python_bridge_parity_nonzero_status_is_err` panicked at `a non-zero cargo status must return Err` in `unittests src/lib.rs`. 0 passed, 1 failed, 1147 filtered out. |

GREEN counts:

- `cargo test -p vox-compiler --test python_bridge_parity synthetic_row_missing_signature_handler_or_expected_is_err` — exit 0, **1 passed**, 0 failed, 1 filtered out (the on-disk test in the same binary).
- `cargo test -p vox-compiler --test python_bridge_parity` — **2 passed**, 0 filtered out.
- `cargo test -p vox-cli --lib python_bridge_parity_nonzero_status_is_err` — exit 0, **1 passed**, 0 failed, 1147 filtered out (other lib tests). The named test ran in the lib harness.

## Task Commits

1. **Task 1 RED: incomplete in-memory row** - `47156e299` (test)
2. **Task 1 GREEN: checker returns Err** - `68a1ee255` (feat)
3. **Task 2 RED: non-zero status still Ok** - `6a0ddf6e3` (test)
4. **Task 2 GREEN: CI arm returns that Result** - `b364350c7` (feat)

## Files Created/Modified

- `crates/vox-compiler/tests/python_bridge_parity_test.rs` - `check_row` and the synthetic-row test
- `crates/vox-cli/src/commands/ci/run_body.rs` - `PythonBridgeParity` arm and `python_bridge_parity_from_status`
- `crates/vox-cli-ci/src/cmd_enums.rs` - `CiCmd::PythonBridgeParity` and its gate id
- `contracts/operations/catalog.v1.yaml` - `ci.python-bridge-parity`, `spawns_process`, `reachability_required: false`
- `contracts/cli/command-registry.yaml` - projected by `operations-sync --target cli --write`
- `docs/src/reference/cli-command-surface.generated.md` - projected by `command-sync --write`
- `docs/src/reference/cli.md` - hand row beside `rust-ecosystem-policy`
- `contracts/policy/policy-registry.v1.yaml` - one `ci-gate/ci.python-bridge-parity` entry

## Decisions Made

- Comment lines are ignored when deciding that a worker defines a handler. A column-0 `def` and a HANDLERS key are both required.
- `policy-registry --write` also rewrote unrelated descriptions and inserted `ci.capability-index`. Only the python-bridge-parity entry was kept.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Hollow-function hook rejected the RED stub**
- **Found during:** Task 1 RED commit
- **Issue:** `check_row` returning `Ok(())` is `skeleton/hollow-fn`. The pre-commit hook exits 1, so the failing test could not land.
- **Fix:** The RED commit put `toestub-ignore(skeleton/hollow-fn)` on the function line under `#[rustfmt::skip]`. The GREEN commit replaced the body and removed the ignore.
- **Files modified:** `crates/vox-compiler/tests/python_bridge_parity_test.rs`
- **Verification:** The RED test failed on `dropping signature must return Err`. The GREEN test passed. The ignore is not in the GREEN commit.
- **Committed in:** `47156e299` (RED), removed in `68a1ee255`

**2. [Rule 2 - Missing Critical] Policy registry entry for the new gate id**
- **Found during:** Task 2
- **Issue:** `CiCmd::PythonBridgeParity` records `ci-gate/ci.python-bridge-parity`. That id is produced from the catalog row. A full `vox ci policy-registry --write` also dropped richer descriptions on other gates and added the pre-existing missing `ci.capability-index` row.
- **Fix:** Committed only the sorted `ci-gate/ci.python-bridge-parity` block.
- **Files modified:** `contracts/policy/policy-registry.v1.yaml`
- **Verification:** `git diff` for that file is 16 insertions and no description rewrites.
- **Committed in:** `b364350c7`

---

**Total deviations:** 2 auto-fixed (1 blocking hook, 1 registry projection)
**Impact on plan:** The checker and the CI arm match the plan. Unrelated generator churn was left unstaged.

## Issues Encountered

`cargo run -p vox-cli -- ci command-compliance` exits 1. `check_ref_cli` passed, including the new `python-bridge-parity` row. The next check is a pre-existing gap: `docs/src/reference/cli.md` has no `| `py` |` reachability row for top-level `py` (`reachability_required` is null). A diagnostic row, not committed, moved the failure to `vox_context_handoff` missing from `TOOL_REGISTRY`. No reachability row was added for `python-bridge-parity` or for `py`.

`command-sync --write` also updated `docs/src/reference/env-vars.generated.md` (pre-existing `VOX_REASONING_EVAL_MAX_TOKENS` drift) and the working tree already had `docs/src/index.mdx` and `contracts/ci/fan-in-snapshot.v1.json` changes. Those files were not staged.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- ARR-04's fail-closed checker and CI arm are in place for plan 04's signature ceiling to call the same cargo target.
- `command-compliance` on this branch stays red until the `py` reachability row and the `vox_context_handoff` registry gap are fixed. Those are outside this plan.

## Self-Check: PASSED

- FOUND: crates/vox-compiler/tests/python_bridge_parity_test.rs
- FOUND: crates/vox-cli/src/commands/ci/run_body.rs
- FOUND: crates/vox-cli-ci/src/cmd_enums.rs
- FOUND: contracts/operations/catalog.v1.yaml
- FOUND: contracts/cli/command-registry.yaml
- FOUND: contracts/policy/policy-registry.v1.yaml
- FOUND: docs/src/reference/cli.md
- FOUND: docs/src/reference/cli-command-surface.generated.md
- FOUND: 47156e299
- FOUND: 68a1ee255
- FOUND: 6a0ddf6e3
- FOUND: b364350c7

---
*Phase: 326-std-array-on-numpy-arr-01-04-k-01*
*Completed: 2026-11-04*
