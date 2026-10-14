---
phase: 02-library-index-and-kb-tool-surface
plan: 11
subsystem: mcp
tags: [kb, mcp, http-gateway, read-role, dispatch-timeout, gap-closure, wr-08, wr-02, mutation-proof]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-06 kb MCP tools and read-role governance; 02-10 walk_corpus signature (build coupling only)"
provides:
  - "vox_kb_status removed from HTTP read-role eligibility (catalog, governance list, regenerated registry)"
  - "dispatch_timeout::KB_REINDEX_TIMEOUT (600 s) with an EXPLICIT_TIMEOUTS row for vox_kb_reindex"
affects: [phase-03]
status: complete
actuals:
  tokens: 4000
  tasks: 2
  commits: 2
plan_head_before: c08f4e2bea8ffd7bd22455e87008b45fc00a473e
commits: 2
key-files:
  created: []
  modified:
    - contracts/operations/catalog.v1.yaml
    - contracts/mcp/http-read-role-governance.yaml
    - contracts/mcp/tool-registry.canonical.yaml
    - crates/vox-orchestrator-mcp/tests/kb_tool_surface.rs
    - crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
    - crates/vox-orchestrator-mcp/src/dispatch_timeout.rs
decisions:
  - "WR-08 fixed by removing read-role eligibility rather than sanitizing the payload: eligibility is the only gateway control, and stdio/CLI hosts keep the full status payload (AP-06 relies on corpus_root)"
  - "Gateway test lives in http_gateway/mod.rs tests; http_gateway_tests.rs is not declared as a module anywhere and never compiles"
requirements-completed: [RET-01]
---

# Phase 2 Plan 11: Read-role status leak and reindex timeout (WR-08, WR-02 limited) Summary

The HTTP read role can no longer list or call `vox_kb_status` (it returns absolute `corpus_root`, `index_dir` and per-file error paths), and `vox_kb_reindex` now gets an explicit 10-minute dispatch bound instead of the 120 s default.

## What changed

- Catalog row `kb.status`: `mcp.http_read_role_eligible: false`. `http-read-role-governance.yaml`: `vox_kb_status` line removed. `tool-registry.canonical.yaml` regenerated with `cargo run -p vox-cli -- ci operations-sync --target mcp --write` (one-line diff, no other files rewritten).
- `kb_tool_surface.rs`: `READ_ROLE` is now search/get/cite; `WRITE_TOOLS` renamed `NOT_READ_ROLE` = reindex + status, each asserted absent from the governance list.
- `http_gateway/mod.rs` tests: `metadata_read_role_excludes_vox_kb_status` (status and reindex absent, search/get/cite present).
- `dispatch_timeout.rs`: `KB_REINDEX_TIMEOUT` (600 s, doc comment names WR-02/WR-03), row in `EXPLICIT_TIMEOUTS`, test `kb_reindex_gets_explicit_timeout`.
- The kb_status handler and payload are untouched.

## Verification

- `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_tool_surface|metadata_read_role|kb_status_before_and_after_index/)'`: pass (RED seen first: `vox_kb_status must not be read-role eligible`).
- `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_reindex_gets_explicit_timeout|cargo_shelling_compiler_tools_get_the_longer_workspace_timeout|chat_message_uses_drive_aligned_ceiling/)'`: 3 run, 3 passed (RED seen first: unresolved name).
- `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_|metadata_read_role|dispatch_timeout::/)'`: 47 run, 47 passed.
- `cargo run -p vox-cli -- ci operations-verify`: OK (1305 catalog rows).

## Mutation proofs

- Registry `http_read_role_eligible` for `vox_kb_status` flipped back to `true` (presence checked before and after; build.rs compiles it into TOOL_REGISTRY): `metadata_read_role_excludes_vox_kb_status` FAILED. Restored from backup; `git diff --stat -- contracts` confirmed back to the intended 3 files.
- Table key typo `"vox_kb_reindexx"`: `kb_reindex_gets_explicit_timeout` FAILED. Restored; presence check (`grep -c reindexx` 1 then 0).
- A first registry-mutation attempt aborted on my own script assertion before changing anything; the resulting pass was discarded and the mutation redone with a verified edit.

## Deviations from Plan

**1. [Rule 3 - Blocking] Gateway test moved out of `http_gateway_tests.rs`**
- **Found during:** Task 1 (the filter ran 2 tests, not 3).
- **Issue:** `crates/vox-orchestrator-mcp/src/http_gateway_tests.rs` has no `mod` declaration anywhere, so it is never compiled; `metadata_read_role_eligible_contains_expected_tools` in it never runs. A test added there would be a green-by-absence.
- **Fix:** Added `metadata_read_role_excludes_vox_kb_status` to the live `mod tests` in `src/http_gateway/mod.rs` (where `metadata_read_role_eligible_tools` is in scope). The dead file is left unchanged. The plan's artifact `http_gateway_tests.rs` therefore does not contain the test.
- **Commit:** f6ab71900

**2. Plan's "fewer than 3 tests" check for Task 1** ran 2 tests (kb_tool_surface and the new gateway test) because the idiom test lives in the dead file; the intent (both guards run and pass) is met.

Auth gates: none.

## Deferred / out of scope

- Dead file `http_gateway_tests.rs` (never compiled, stale fields): worth deleting or wiring in separately.
- WR-02 cross-process single-flight, CLI client read deadline for `vox kb reindex`, WR-01/03/04/05/06/07 (per plan).
- Pre-existing, from 02-10 deferred-items.md: vox-search `kb_eval_golden_contract` baseline tests and clippy `cloned_ref_to_slice_refs` at `kb_eval/library.rs:185`.

## Known Stubs

None.

## Threat Flags

None. T-02-39 and T-02-40 mitigated; T-02-41 accepted/deferred; T-02-42 covered by the NOT_READ_ROLE assertion.

## Self-Check: PASSED

- Commits f6ab71900 and 6d3a6d03f exist; `dispatch_timeout.rs` contains `KB_REINDEX_TIMEOUT`; governance list has 0 occurrences of `vox_kb_status`.
