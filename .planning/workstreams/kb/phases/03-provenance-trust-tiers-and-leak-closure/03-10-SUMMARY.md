---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 10
subsystem: mcp
tags: [ret-02, mcp-contract, operations-catalog, legacy-voxkb, absence-test, d-02]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "chat-turn KB leak closed and RetrievalBundle.kb_lines gone (03-07)"
provides:
  - "the 12 legacy vox_kb_* CRUD tools retired from catalog, tool/capability registries, model manifest, inventory, GUI coverage report, dispatch, input schemas and HTTP read-role list (no aliases)"
  - "kb_tools/handlers.rs and kb_tools/params.rs deleted (no model-callable KB write remains)"
  - "legacy_voxkb_surface_absent: boundary-aware, non-vacuous static absence test (extended by 03-12)"
affects: [03-11, 03-12, 03-13]
requirements-completed: [RET-02]
tech-stack:
  added: []
  patterns: ["retire a tool family through the catalog: delete exact blocks, regenerate every projection, never hand-edit generated files", "static absence test with identifier-boundary matching and a scanned-file floor"]
key-files:
  created:
    - crates/vox-orchestrator-mcp/tests/legacy_voxkb_retired.rs
  modified:
    - contracts/operations/catalog.v1.yaml
    - contracts/mcp/tool-registry.canonical.yaml
    - contracts/capability/capability-registry.yaml
    - contracts/capability/model-manifest.generated.json
    - contracts/reports/operations-catalog-inventory.v1.json
    - contracts/reports/gui-surface-coverage.v1.json
    - contracts/mcp/http-read-role-governance.yaml
    - crates/vox-orchestrator-mcp/src/dispatch.rs
    - crates/vox-orchestrator-mcp/src/input_schemas.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/mod.rs
    - crates/vox-orchestrator-mcp/src/scope_guard.rs
  deleted:
    - crates/vox-orchestrator-mcp/src/kb_tools/handlers.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/params.rs
key-decisions:
  - "D-02 option-a (retire now, no aliases), pre-approved in 03-CONTEXT.md; CHANGELOG break note is 03-13"
  - "Catalog ids are matched with a previous-char exclusion of [a-z0-9_.] so a field access like self.kb.clip never reads as a catalog id"
duration: ~75 min wall (dominated by cargo builds on a loaded host)
completed: 2026-10-08
status: complete
commits: 2
plan_head_before: 37a356ef84176ad15855fa704086ac55bc860edb
actuals:
  tokens: 13200
  tasks: 2
  commits: 2
---

# Phase 3 Plan 10: Retire the 12 legacy vox_kb_* tools Summary

The 12 legacy VoxKB CRUD tools (including the model-callable writers `vox_kb_clip` and `vox_kb_add_entry`) are gone from the MCP contract and from source, with no aliases; Phase 2's five tools are untouched and a static test keeps the old names out.

## Retirement decision

D-02 option-a (retire now, no aliases), pre-approved in 03-CONTEXT.md before this plan ran. No `MCP_TOOL_WIRE_ALIASES` entry was added. 03-12 and 03-13 build on this: 03-12 deletes KbStore/KbRouter and the vox-db CRUD (extend `BANNED` in `legacy_voxkb_retired.rs`); 03-13 owns the CHANGELOG note, retirement registries and docs.

## Commits

- `50bae7cd7` feat(03-10): retire the model-callable vox_kb_clip tool end to end (Task 1, tracer)
- `e63e75188` feat(03-10): retire the remaining 11 legacy vox_kb_* tools and their handlers (Task 2)

## What was built

- **Task 1 (tracer).** `kb.clip` removed from `catalog.v1.yaml`; `operations-sync --target all --write`, `capability-sync --write`, `operations-verify` and `gui-surface-coverage --write` regenerated every projection; clip dispatch and input-schema arms deleted. `legacy_voxkb_retired.rs` written first: `legacy_voxkb_surface_absent` walks every `.rs` under `crates/` (skipping `target`, `node_modules`, `.git`, itself) plus six contract files with a whole-identifier matcher (next char not `[a-z0-9_]`, previous char not `[a-z0-9_.]`, case-sensitive) and fails if fewer than 100 files were scanned; `legacy_voxkb_matcher_respects_identifier_boundaries` pins the adjacency/case/empty-prefix edges.
- **Task 2.** The 11 remaining catalog blocks (220 lines, exactly `kb.create`..`kb.query`, stopping before Phase 2's `kb.search`), 21 dispatch lines and 15 schema lines deleted by exact arm; `kb_tools/handlers.rs` and `kb_tools/params.rs` removed with `git rm`; `kb_tools/mod.rs` now declares `findings` and `library` only; five legacy lines dropped from `http-read-role-governance.yaml`; `scope_guard.rs` comment no longer names legacy KB write tools. `BANNED` now holds all 12 tool names, their 12 catalog ids, `kb_lines`, `signal_chat`, `signal_research`, `signal_scientia`.

## Evidence

**Red before delete.**
- Task 1: `legacy_voxkb_surface_absent` FAILED against the unmodified tree, listing `vox_kb_clip` in input_schemas.rs, dispatch.rs, catalog, tool registry, capability registry, model manifest and `kb.clip` in the GUI coverage report (3 tries).
- Task 2: with the extended `BANNED`, FAILED on `vox_kb_create` in the same files plus `vox_kb_list` in `http-read-role-governance.yaml` and `kb_tools/handlers.rs` (`REM_KB_NOT_FOUND` string).

**Mutation proofs (grep -c, before / mutated / restored).**

| Guard | Mutation (Edit tool) | Counts | Result |
|-------|----------------------|--------|--------|
| T-03-17 / T-03-35 clip tool | re-added `"vox_kb_clip"` dispatch arm | `vox_kb_clip` in dispatch.rs: 0 / 1 / 0 | `legacy_voxkb_surface_absent` FAILED (`dispatch.rs mentions retired vox_kb_clip`) |
| T-03-35 legacy query tool | re-added `"vox_kb_query"` dispatch arm | `vox_kb_query` in dispatch.rs: 0 / 1 / 0 | `legacy_voxkb_surface_absent` FAILED (`dispatch.rs mentions retired vox_kb_query`) |

Both mutations were restored; the dispatch.rs diff against the task commit is empty.

**Single-definition / presence counts (after Task 2).** `fn every_registry_tool_has_static_dispatch` in dispatch.rs = 1; `pub(super) fn tool_input_schema` in input_schemas.rs = 1; `pub mod` lines in `kb_tools/mod.rs` = 2 (findings, library); catalog ids `kb.search|get|cite|status|reindex|eval` = 6 before and after; legacy ids 11 -> 0; `vox_kb_(search|get|cite|status|reindex)` lines in `tool-registry.canonical.yaml` = 5; `http-read-role-governance.yaml` keeps `vox_kb_cite`, `vox_kb_get`, `vox_kb_search`.

**Verification run locally.**
- `cargo nextest run -p vox-orchestrator-mcp -E 'test(/legacy_voxkb_|every_registry_tool_has_static_dispatch|kb_tool_surface|registry_tools_have_input_schema_coverage/)'`: 5 passed (`kb_tool_surface` reported LEAK, a harmless nextest leaked-subprocess note, not a failure).
- From source (`target/debug/vox`, `VOX_SKIP_FRESHNESS_CHECK=1`): `ci operations-sync --target all` OK (exit 0), `ci capability-sync` OK, `ci gui-surface-coverage` up to date, `ci operations-verify` OK (1293 rows), `ci crate-edges` OK (689 edges; prints a pre-existing "stale baseline edge vox-gui -> vox-db-types" warning, unrelated).
- Acceptance greps all empty / satisfied (see Evidence above); `handlers.rs` and `params.rs` absent.

## Verify lines deferred to CI

Per the lean-verification policy these were not run locally: workspace-wide `cargo clippy` and the full `vox-orchestrator-mcp` test suite; compilation of crates that depend on `vox-orchestrator-mcp` (a `git grep` over `crates/` found no use of the deleted `kb::*` handlers or params outside `kb_tools/` and `dispatch.rs`/`input_schemas.rs`); `vox ci sql-surface-guard --all` (known pre-existing offenders in `deferred-items.md`); `vox ci ssot-drift` and `docs/agents/doc-inventory.json` refresh (03-13 owns docs).

## Deviations from Plan

**1. [Rule 3 - Blocking] Generated projections carried Phase 2 drift.** `model-manifest.generated.json` and `gui-surface-coverage.v1.json` at HEAD were stale relative to Phase 2's kb.search/get/cite/status/reindex/eval rows, so regenerating them (the only permitted way) also added those Phase 2 entries (model-manifest +127/-11 lines in Task 1, GUI report gained `kb eval/reindex/status` paths and `kb.cite|eval|get|reindex|search|status`). This is correct generator output, not a hand edit. **Commit:** 50bae7cd7.

**2. [Rule 1 - Test ergonomics] Previous-char boundary added to the matcher.** The plan only specified the next-char rule; the previous-char exclusion `[a-z0-9_.]` was added so `.rs` field accesses (`self.kb.clip(..)`) cannot false-positive on catalog ids. Covered by `legacy_voxkb_matcher_respects_identifier_boundaries`.

**3. Housekeeping.** `rustfmt` on `kb_tools/mod.rs` reformatted the sibling `library.rs` (it formats child modules); that change was reverted with `git checkout -- <file>` before the commit, so `library.rs` is untouched. `rustfmt --check` on `dispatch.rs` reports a pre-existing import-order diff at line ~2068 (not from this plan; left alone).

**Total deviations:** 2 (1 blocking-by-generator, 1 test hardening). **Impact:** none on scope.

## Issues Encountered

Build host loaded (load average 6-11); first `cargo run -p vox-cli` took 3m40s, each nextest rebuild 2-3 minutes. No hangs. No cargo/rustc processes left running.

## Known Stubs

None.

## Threat Flags

None. Net effect removes surface: two model-callable KB writers (`vox_kb_clip`, `vox_kb_add_entry`) and ten more legacy tools. T-03-17 and T-03-35 mitigated and mutation-proven; T-03-18 accepted per RET-02/D-02 (CHANGELOG in 03-13).

## Self-Check: PASSED

- `legacy_voxkb_retired.rs` FOUND; `handlers.rs` / `params.rs` absent; commits `50bae7cd7` and `e63e75188` FOUND in `git log`; `git rev-list --count 37a356ef8..HEAD` = 2 at the time of the code commits.
