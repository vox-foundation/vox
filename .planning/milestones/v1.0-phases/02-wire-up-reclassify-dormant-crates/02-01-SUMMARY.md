---
phase: 02-wire-up-reclassify-dormant-crates
plan: 01
subsystem: orchestrator
tags: [tool-receipts, mcp-registry, fail-closed, crate-edges]
requires: []
provides:
  - "ToolReceiptLedger::issue_intent / issue and Orchestrator::issue_tool_receipt fail closed on tool names absent from vox_mcp_registry::TOOL_REGISTRY"
  - "vox-orchestrator -> vox-mcp-registry unconditional [dependencies] edge (one user-authorized exception)"
affects: [phase-05-trust-receipts]
tech-stack:
  added: []
  patterns: ["registry-backed allow-list guard as first statement of the issuing path"]
key-files:
  created:
    - crates/vox-orchestrator/tests/tool_receipt_registry_gate.rs
  modified:
    - crates/vox-orchestrator/src/tool_receipt.rs
    - crates/vox-orchestrator/src/orchestrator/safety.rs
    - crates/vox-orchestrator/src/lib.rs
    - crates/vox-orchestrator/Cargo.toml
    - contracts/ci/crate-edges.allow.v1.json
    - contracts/ci/crate-graph.v1.json
    - Cargo.lock
key-decisions:
  - "Guard lives in ToolReceiptLedger::issue_intent so every issuing path (issue, issue_tool_receipt) is covered, not just the facade."
  - "Exactly one crate-edge exception (vox-orchestrator -> vox-mcp-registry, 2026-09-25, user chat approval); edges array, other exceptions, fan-in snapshot untouched."
  - "crate-graph.v1.json regenerated in the same commit; crate-build-map.v1.json deliberately not regenerated (generator cannot run here without unrelated churn; already 60 rows out of parity on HEAD)."
metrics:
  duration: "about 27 min active (queued builds and a corrupt shared Cargo.lock added waiting time)"
  completed: 2026-09-25
status: complete
commits: 1
plan_head_before: e7477d6b3e219a7b9ed6789f3fa6698b123b1487
actuals:
  tokens: 3000    # rough chars/4 over the ~200 inserted lines; not precisely measured
  tasks: 2
  commits: 1
---

# Phase 2 Plan 01: Fail-closed MCP tool-name validation Summary

`ToolReceiptLedger` now rejects any tool name that is not an exact `vox_mcp_registry::TOOL_REGISTRY` entry with `ToolReceiptError::UnknownTool`, recording nothing, through `issue_intent`, `issue` and `Orchestrator::issue_tool_receipt`.

## Commits

- `dc0aa6cb9` feat(02-01): fail closed on unregistered MCP tool names in ToolReceiptLedger. It contains the 8 planned paths.
  - It was made by the parent session on `main`, on the user's instruction. The executor refused to commit on the protected branch by itself.
  - The fmt-fix and tdd-guard hooks passed.

## Evidence

- **RED (first error line, `target/phase02-01-red.txt`):** `error[E0432]: unresolved import `vox_orchestrator::ToolReceiptError``. The next line was `unresolved import `vox_mcp_registry``, plus 3 type errors, all in the new test file only.
- **GREEN integration:** `test result: ok. 3 passed; 0 failed`. The three tests are `issue_tool_receipt_rejects_unknown_tool_fail_closed`, `issue_tool_receipt_rejects_near_miss_names` and `issue_tool_receipt_accepts_every_registry_entry`.
- **GREEN in-file:** `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1209 filtered out`. The ok-line count was 6.
- **Ledger shape (verify block 3):** `jq -e` printed `true`.
  - `exceptions[0]` is the authorized object, with key order from,to,reason,date,authorized_by.
  - There are 35 exceptions.
  - The `exceptions[1:]` hash `88f4e5ed...0f495` and the `edges` hash `b8c62fbb...14c0e` are unchanged.
  - The dependency is under `[dependencies]`.
- **crate-edges (verify block 4):** rc=1 with `Error: crate-edges: 3 violation(s)`. All three are pre-existing on HEAD and unrelated: `vox-gui -> vox-db-types`, `vox-gui -> vox-research-shim` and `vox-research-shim -> vox-compiler`. No line mentions `vox-mcp-registry`.
- **tdd-guard command (toestub, verify block 5):** rc=0, "No issues found".
- **Crate graph (verify block 6):**
  - `affected-crates --regen` rc=0 and wrote `contracts/ci/crate-graph.v1.json`.
  - The regenerated graph equals the start-commit graph plus `vox-mcp-registry` in `vox-orchestrator`'s list. The numstat is `1 0`.
  - `affected-crates --check` rc=0.
  - The fan-in-budget report lines are identical before and after (27 lines each). The gate itself exits 1 with 10 pre-existing regressions. No snapshot edit was needed.
  - `crate-build-map.v1.json` and `fan-in-snapshot.v1.json` are unmodified.
- **Commit audit (Task 2 verify block 1):** rc=0. `dc0aa6cb9` contains exactly the 8 `files_modified` paths. The `Cargo.lock` numstat is `1 0` and the `crate-graph.v1.json` numstat is `1 0`.

## Mutation proof (AGENTS.md PR and Review Discipline)

Before the mutation, `git diff --quiet HEAD -- tool_receipt.rs` returned 0 and the guard text count was 1. I replaced the guard predicate `.any(|e| e.name == tool_name)` with `.any(|_| true)`. The presence check printed 1 immediately before the run and 1 after it. `target/phase02-01-mutant.txt`, rc=101:

```text
test issue_tool_receipt_rejects_unknown_tool_fail_closed ... FAILED
test issue_tool_receipt_rejects_near_miss_names ... FAILED
test issue_tool_receipt_accepts_every_registry_entry ... ok
test result: FAILED. 1 passed; 2 failed
```

I restored the predicate with a normal edit. `git diff -- crates/vox-orchestrator/src/tool_receipt.rs` is empty and `git diff --quiet HEAD` returns 0. The re-run in `target/phase02-01-postmutant.txt` had rc=0, with all 3 tests ok (`3 passed; 0 failed`). Task 2 verify block 2 passed (`verify2=0`).

## Clippy

`cargo clippy -p vox-orchestrator --all-targets -- -D warnings` returned rc=0 with zero diagnostics (0 `-->` location lines). No follow-up commit was needed.

## Deviations from Plan

1. **[Process] Commit made by the parent session.** The protected-branch pre-commit assertion (HEAD on `main`, no `git.allow_default_branch_commits`) halted the executor. The parent session then committed `dc0aa6cb9` on `main` per the user's instruction. The executor had staged the 8 paths and passed the Step 6(g) mechanical staged-set check beforehand. The commit message and hashes are as planned.
2. **[Rule 3 - Blocking, environmental] Corrupt working-tree `Cargo.lock`.** After the commit, another session left stray and duplicated lines in the `workspace-hack` section, so every cargo call failed with a TOML parse error at line 21164. The coordinator and user repaired it (HEAD's version restored, other sessions' entries re-derived by cargo). The mutation proof and clippy ran only after the repair. The executor did not touch the lockfile. This does not affect the committed `Cargo.lock`, which is 1 line added, 0 deleted.
3. **[Tooling] `cmp` replaced by `diff`.** `cmp` is blocked by the lean-ctx shell allowlist, so verify block 6's fan-in comparison used `diff` on the same two `.lines` files. The result was identical, `diff=0`.
4. **[Tooling] Log colors and lean-ctx roots.** Cargo logs contain ANSI color codes. I stripped them with `sed` before the greps that need exact strings. The Task 2 cargo runs used `LEAN_CTX_EXTRA_ROOTS=.../target` to get past a lean-ctx "path escapes project root" error caused by the shell hook rooting at another project. This did not affect results.
5. **[Note] Fmt hook.** The commit's test file has 79 lines versus 73 as I wrote it, so the fmt hook reformatted it inside the commit. I observed no other files being rewritten in the working tree.
6. **[Note] `actuals.tokens` is a rough estimate** and was not precisely measured.

Otherwise, the plan was executed as written.

## Known Stubs

None.

## Threat Flags

None. The only new surface is the first-party crate edge, covered by the plan's threat model (T-02-03 and T-02-SC).

## Self-Check: PASSED

- The integration test file exists, and the commit `dc0aa6cb9` exists and contains the 8 planned paths (verified).
- The mutation proof, the restore check and clippy all passed as recorded above.
